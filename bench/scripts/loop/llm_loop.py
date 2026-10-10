#!/usr/bin/env python3
"""PLAN 2.7 — LLM loop test harness.

Drives: model writes a feature in IR -> transpile (ircc to-js --map) -> run
(node rig) -> feed errors back (with IR lines recovered from the span map)
-> repair. Rounds until the rig passes or the round cap.

With --lang js the same loop runs in the BASELINE world (J6 whole-pipeline
measurement): the model writes feature.js directly (no guide, no transpile,
no span map) and the rig runs it verbatim. Everything else (task cases,
rig, error feedback shape) is identical.

Adapters (pluggable):
  echo    deterministic canned replies from the task config — verifies the
          deterministic loop mechanics with zero model access (selftest)
  file    out-of-band driving (e.g. via an agent session): the harness
          writes prompt-R{N}.md + transcript.md, then polls --reply-file
  openai  OpenAI-compatible chat API (LLM_BASE_URL / LLM_API_KEY / LLM_MODEL)

Stdlib only; token counting is best-effort (tiktoken if importable).
"""

import argparse
import json
import os
import re
import subprocess
import sys
import time
from datetime import datetime, timezone
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[2]

# ----------------------------------------------------------------------------
# tooling

def resolve_ircc() -> str:
    cand = REPO / "target" / "debug" / "ircc"
    return str(cand) if cand.exists() else "ircc"

def tok_count(text: str) -> str:
    try:
        import tiktoken
        enc = tiktoken.get_encoding("o200k_base")
        return f"{len(enc.encode(text))} o200k"
    except Exception:
        return f"{len(text)} chars"

# ----------------------------------------------------------------------------
# adapters

MSG = dict  # {"role": "system"|"user"|"assistant", "content": str}

class EchoAdapter:
    name = "echo"
    def __init__(self, cfg: dict, opts):
        self.cfg = cfg
        self.calls = 0
        lang = getattr(opts, "lang", "ir")
        self.suffix = {"js": "_js", "cs": "_cs", "ir-cs": "_cs_ir"}.get(lang, "")
    def reply(self, workdir: Path, messages: list, round_no: int) -> str:
        self.calls += 1
        key = ("echo_fix" if self.calls > 1 else "echo_draft") + self.suffix
        if key not in self.cfg:
            sys.exit(f"echo adapter: task config is missing {key}")
        return self.cfg[key]

class FileAdapter:
    """Harness archives the transcript (engine writes prompt-R{N}.md); the
    model replies via a file. Polls --reply-file for a fresh write."""
    name = "file"
    def __init__(self, cfg: dict, opts):
        self.reply_file = Path(opts.reply_file or (HERE / "runs" / "reply.md"))
        self.timeout = opts.timeout
    def reply(self, workdir: Path, messages: list, round_no: int) -> str:
        print(f"[file-adapter] waiting for reply at {self.reply_file} (round {round_no}); transcript: {workdir}/prompt-R{round_no:02}.md", flush=True)
        start = time.time()
        while time.time() - start < self.timeout:
            if self.reply_file.exists() and self.reply_file.stat().st_mtime > start:
                text = self.reply_file.read_text()
                if text.strip():
                    print(f"[file-adapter] reply captured ({len(text)} bytes)", flush=True)
                    return text
            time.sleep(0.5)
        raise RuntimeError(f"file-adapter: no reply within {self.timeout}s ({self.reply_file})")

def render_transcript(messages: list) -> str:
    out = []
    for m in messages:
        role = m["role"].upper()
        out.append(f"## {role}\n\n{m['content']}")
    return "\n\n".join(out) + "\n"

class OpenAICompatAdapter:
    name = "openai"
    def __init__(self, cfg: dict, opts):
        self.base = opts.base_url or os.environ.get("LLM_BASE_URL") or "https://api.openai.com/v1"
        self.key = (os.environ.get(opts.api_key_env or "LLM_API_KEY") or "").strip()
        self.model = opts.model or os.environ.get("LLM_MODEL") or ""
        # ollama's /v1 endpoint clips context at the server default (~4k) —
        # the transcript grows across repair rounds; request more when told.
        # LLM_NUM_CTX also disables thinking (omitted content = broken loop
        # for ollama thinking models). LLM_NUM_PREDICT caps generation
        # (reasoning-heavy models can stream for hours with an unbounded
        # budget — the 600s socket timeout never trips a live stream).
        num_ctx = opts.num_ctx or os.environ.get("LLM_NUM_CTX")
        self.extra_body = {}
        if num_ctx:
            self.extra_body = {"options": {"num_ctx": int(num_ctx)}, "think": False}
            num_predict = os.environ.get("LLM_NUM_PREDICT")
            if num_predict:
                self.extra_body["options"]["num_predict"] = int(num_predict)
        if not self.key:
            self.key = "local" if os.environ.get("LLM_BASE_URL") else ""
        if not self.key:
            sys.exit("openai adapter: no API key (set --api-key-env target or LLM_API_KEY)")
        if not self.model:
            sys.exit("openai adapter: no model (set --model or LLM_MODEL)")
    def reply(self, workdir: Path, messages: list, round_no: int) -> str:
        import urllib.request
        # round 1 determinism (clean baseline evidence); repair rounds sample
        # (temp-0 small models repeat verbatim and never leave the loop)
        body = json.dumps({
            "model": self.model,
            "messages": [dict(m) for m in messages],
            "temperature": 0.0 if round_no == 1 else 0.6,
            **self.extra_body,
        }).encode()
        req = urllib.request.Request(
            self.base.rstrip("/") + "/chat/completions",
            data=body,
            headers={"Content-Type": "application/json", "Authorization": f"Bearer {self.key}"},
        )
        with urllib.request.urlopen(req, timeout=600) as resp:
            data = json.loads(resp.read())
        try:
            m = data["choices"][0]["message"]
        except Exception:
            return f"(adapter error: {json.dumps(data)[:2000]})"
        # thinking models on ollama put the whole answer in the reasoning
        # channel and leave content EMPTY (observed: gemma4:12b reasons
        # ~2.9k tokens, never reaches content, finish_reason=length); fall
        # back to the reasoning text — extract still scans for fenced blocks
        content = m.get("content") or ""
        if not content.strip():
            reasoning = m.get("reasoning") or m.get("reasoning_content") or ""
            if reasoning.strip():
                print("[adapter] empty content; using reasoning channel", flush=True)
                return reasoning
        return content

ADAPTERS = {"echo": EchoAdapter, "file": FileAdapter, "openai": OpenAICompatAdapter}

# ----------------------------------------------------------------------------
# loop steps

def extract_ir(text: str):
    """Prefer the fenced ```ir block; fall back to the last fenced block that
    contains 'fn '; fall back to the whole text if it starts with @."""
    fences = re.findall(r"```([^\n`]*)\n(.*?)```", text, re.S)
    for lang, body in fences:
        if lang.strip().lower() in ("ir", "irc", "") and body.strip().startswith("@"):
            return body.strip()
    for lang, body in reversed(fences):
        if "fn " in body:
            return body.strip()
    if text.strip().startswith("@"):
        return text.strip()
    return None

def extract_cs(text: str):
    """C# baseline world: prefer the fenced csharp/cs block with
    'static'/'class'/'=>'; fall back to the last fenced block; fall back to
    the whole text if it looks like C#."""
    fences = re.findall(r"```([^\n`]*)\n(.*?)```", text, re.S)
    for lang, body in fences:
        if lang.strip().lower() in ("csharp", "cs", "") and ("static" in body or "class " in body):
            return body.strip()
    for lang, body in reversed(fences):
        if "static" in body:
            return body.strip()
    t = text.strip()
    if t.startswith(("using ", "//", "public ", "internal ", "static ", "class ")):
        return t
    return None

def extract_js(text: str):
    """Baseline world: prefer the fenced ```js block that exports a fn; fall
    back to the last fenced block containing 'export'; fall back to the whole
    text if it looks like a module."""
    fences = re.findall(r"```([^\n`]*)\n(.*?)```", text, re.S)
    for lang, body in fences:
        if lang.strip().lower() in ("js", "javascript", "mjs", "") and "export" in body:
            return body.strip()
    for lang, body in reversed(fences):
        if "export" in body:
            return body.strip()
    t = text.strip()
    if t.startswith(("import ", "export ", "function ", "const ")):
        return t
    return None

def write_feature_js(workdir: Path, js_text: str):
    """Baseline world: the reply IS the module — no transpile, no map."""
    js_path = workdir / "feature.js"
    js_path.write_text(js_text if js_text.endswith("\n") else js_text + "\n")
    return js_path

def irc_emit(workdir: Path, ir_text: str):
    """write feature.ir; emit via ircc to-js --map. Returns (ok, errors, js_path)."""
    ir_path = workdir / "feature.ir"
    ir_path.write_text(ir_text if ir_text.endswith("\n") else ir_text + "\n")
    proc = subprocess.run(
        [resolve_ircc(), "to-js", "--map", str(ir_path)],
        capture_output=True, text=True, timeout=60,
    )
    out = (proc.stdout or "") + (proc.stderr or "")
    m = re.search(r"wrote (.+\.js) \((\d+) lines, (\d+) map entries\)", out)
    if not m:
        errs = []
        for line in out.splitlines():
            lm = re.match(r"// .*:? ?(ERROR|LOWER-ERR) (.*)", line)
            if lm:
                errs.append(lm.group(2))
        return False, (errs or [out.strip() or "no emit output"]), None
    js_path = Path(m.group(1))
    map_path = js_path.with_suffix("").with_name(js_path.name[:-3] + ".map.json")
    return True, [], js_path

def run_rig(workdir: Path, cfg: dict):
    """Generate + run the node rig. Returns (passed: bool, output: str)."""
    rig = workdir / "rig.js"
    (workdir / "package.json").write_text('{"type":"module"}\n')
    fns = cfg["requires"]
    rig_lines = [f"import {{ {', '.join(fns)} }} from \"./feature.js\";"]
    rig_lines.append("const cases = " + json.dumps(cfg["cases"]) + ";")
    rig_lines.append("const FNS = {" + ", ".join(f"{f}: {f}" for f in fns) + "};")
    rig_lines.append("""let fail = 0;
for (const [fn, args, expected] of cases) {
  const shown = args.map(a => JSON.stringify(a)).join(", ");
  try {
    const got = FNS[fn](...args);
    const g = JSON.stringify(got), e = JSON.stringify(expected);
    if (g === e) console.log(`PASS ${fn}(${shown}) -> ${g}`);
    else { fail++; console.error(`FAIL ${fn}(${shown}) -> ${g} (expected ${e})`); }
  } catch (err) {
    fail++;
    console.error(`ERROR ${fn}(${shown}): ${err.stack || err}`);
  }
}
if (fail > 0) { console.error(`RESULT: ${fail} failing`); process.exit(1); }
console.log("RESULT: ALL PASS");""")
    rig.write_text("\n".join(rig_lines) + "\n")
    proc = subprocess.run(["node", "rig.js"], cwd=workdir, capture_output=True, text=True, timeout=30)
    out = (proc.stdout or "") + (proc.stderr or "")
    return proc.returncode == 0, out

def js_to_ir(entries, line):
    best = None
    for e in entries:
        if e[0] <= line <= e[1] and (best is None or e[0] > best[0]):
            best = e
    if best:
        return best[2], best[3]
    prev = [e for e in entries if e[0] <= line]
    return (prev[-1][2], prev[-1][3]) if prev else None

def irc_emit_cs(workdir: Path, ir_text: str):
    """cs IR world: write feature.ir; emit via ircc to-cs. Returns (ok, errors, cs_path)."""
    ir_path = workdir / "feature.ir"
    ir_path.write_text(ir_text if ir_text.endswith("\n") else ir_text + "\n")
    proc = subprocess.run(
        [resolve_ircc(), "to-cs", str(ir_path)],
        capture_output=True, text=True, timeout=60,
    )
    out = (proc.stdout or "") + (proc.stderr or "")
    if "CS-ERR" in out or "ERROR" in out:
        errs = []
        for line in out.splitlines():
            m = re.match(r"// .*:? ?(CS-ERR|ERROR) (.*)", line)
            if m:
                errs.append(m.group(2))
        return False, (errs or [out.strip()]), None
    if not out.strip():
        return False, ["no emit output"], None
    cs_path = workdir / "feature.cs"
    cs_path.write_text(out if out.endswith("\n") else out + "\n")
    return True, [], cs_path

def write_feature_cs(workdir: Path, cs_text: str):
    """C# baseline world: the reply IS the target file."""
    cs_path = workdir / "feature.cs"
    cs_path.write_text(cs_text if cs_text.endswith("\n") else cs_text + "\n")
    return cs_path

def run_rig_cs(workdir: Path, cfg: dict):
    """Generate + run the dotnet rig. Reflection finds the fn by name+arity
    in the emitted assembly; the canon cases are JSON — args convert via
    System.Text.Json, results deep-compare via JsonNode.DeepEqual (net10)."""
    cases = cfg["cases"]
    rows = ",\n        ".join(
        f'("{fn}", {json.dumps(json.dumps(args))}, {json.dumps(json.dumps(expected))})'
        for (fn, args, expected) in cases
    )
    rig_body = """using System.Reflection;
using System.Text.Json;
using System.Text.Json.Nodes;

var cases = new (string Fn, string ArgsJson, string ExpectedJson)[] {
        """ + rows + """
};

static object? Conv(JsonNode n, Type t)
{
    if (n is JsonValue v)
    {
        if (t == typeof(string)) return (string?)v ?? "";
        if (t == typeof(int)) return (int)v!;
        if (t == typeof(long)) return (long)v!;
        if (t == typeof(bool)) return (bool)v!;
        if (t == typeof(double)) return (double)v!;
        if (t == typeof(char)) return (string?)v ?? "";
    }
    return n.Deserialize(t);
}

static (MethodInfo, object?[]) Resolve(string fn, string argsJson)
{
    var argsNode = (JsonArray)JsonNode.Parse(argsJson)!;
    var cand = typeof(Rig).Assembly.GetTypes()
        .SelectMany(t => t.GetMethods(BindingFlags.Public | BindingFlags.NonPublic | BindingFlags.Static))
        .Where(m => m.Name == fn && m.GetParameters().Length == argsNode.Count)
        .Select(m => (t: m.DeclaringType!, m: m))
        .FirstOrDefault();
    if (cand.m is null)
        throw new MissingMethodException($"fn not found: {fn}({argsNode.Count} args)");
    var argv = argsNode.Select((n, i) => Conv(n, cand.m.GetParameters()[i].ParameterType)).ToArray();
    return (cand.m, argv);
}

int fail = 0;
foreach (var (cf, aJson, eJson) in cases)
{
    try
    {
        var (m, argv) = Resolve(cf, aJson);
        var got = m.Invoke(null, argv);
        var gotJson = JsonSerializer.Serialize(got);
        var ok = JsonNode.DeepEquals(JsonNode.Parse(gotJson)!, JsonNode.Parse(eJson)!);
        if (ok) Console.WriteLine($"PASS {cf}({aJson}) -> {gotJson}");
        else { fail++; Console.Error.WriteLine($"FAIL {cf}({aJson}) -> {gotJson} (expected {eJson})"); }
    }
    catch (Exception err)
    {
        fail++;
        Console.Error.WriteLine($"ERROR {cf}({aJson}): {err.Message}");
    }
}
if (fail > 0) { Console.Error.WriteLine($"RESULT: {fail} failing"); return 1; }
Console.WriteLine("RESULT: ALL PASS");
return 0;
"""
    feat = workdir / "feature.cs"
    fsrc = feat.read_text() if feat.exists() else ""
    # the Rig type must live in its own file (C# top-level statements may
    # not be followed by type declarations) and in the global namespace
    # (typeof(Rig) resolves from the optionally-namespaced feature/world)
    (workdir / "riglib.cs").write_text("public static class Rig { }\n")
    (workdir / "rig.cs").write_text(rig_body)
    (workdir / "t.csproj").write_text(
        '<Project Sdk="Microsoft.NET.Sdk"><PropertyGroup><OutputType>Exe</OutputType>'
        "<TargetFramework>net10.0</TargetFramework><ImplicitUsings>enable</ImplicitUsings>"
        "<Nullable>disable</Nullable></PropertyGroup></Project>"
    )
    proc = subprocess.run(["dotnet", "run", "-v", "q", "--nologo"],
                          cwd=workdir, capture_output=True, text=True, timeout=180)
    out = (proc.stdout or "") + (proc.stderr or "")
    return proc.returncode == 0, out

def feedback_text(node_out: str, src_text: str, map_file, lang: str = "ir") -> str:
    """Errors + the mapped source lines (span map, innermost rule — IR world
    only). For value failures (no stack trace) include the numbered source so
    the model can act. The js baseline world has no span map: the source IS
    the emitted code, so feedback is the raw node output + numbered source."""
    fence = {"ir": "ir", "js": "js", "ir-cs": "ir", "cs": "cs"}.get(lang, "ir")
    parts = ["## Reported errors\n", "```\n" + (node_out if lang in ("ir-cs", "cs") else node_out.strip())[:4000] + "\n```"]
    src_lang = {"ir": "IR", "js": "JS", "ir-cs": "IR", "cs": "C#"}.get(lang, "IR")
    js_file = "feature.js"
    entries = json.loads(map_file.read_text())["mappings"] if map_file and Path(map_file).exists() else []
    lines = src_text.splitlines()
    ir_refs = []
    for js_line in sorted({int(x) for x in re.findall(rf"{re.escape(js_file)}:(\d+):", node_out)}):
        got = js_to_ir(entries, js_line)
        if got:
            ir_refs.append((js_line, got))
    if ir_refs:
        parts.append("\n## Mapped IR lines (from the emitted-file span map)\n")
        seen = set()
        for js_line, (lo, hi) in ir_refs:
            for ln in range(max(1, lo), min(len(lines), hi) + 1):
                if ln in seen:
                    continue
                seen.add(ln)
                parts.append(f"- IR line {ln}: `{lines[ln - 1]}`  (js line {js_line})")
        parts.append("\nContext:\n```\n" + numbered_excerpt(lines, sorted(seen)) + "\n```")
    if not entries and not ir_refs:
        # value-level failure (or baseline world): show the current source
        # with line numbers so the model can point at what to change
        parts.append(f"\n## Your current {src_lang} (numbered)\n")
        parts.append("```\n" + "\n".join(f"{n + 1:>3} {l}" for n, l in enumerate(lines)) + "\n```")
        parts.append(
            f"\nCompare what your code does for each failing case above with the expected value. "
            f"Name the exact line to change, then re-emit the COMPLETE corrected {fence} file in one fenced {fence} block."
        )
        return "\n".join(parts)
    parts.append(
        f"\nFix the problem and re-emit the COMPLETE corrected {fence} file in one fenced {fence} block."
    )
    return "\n".join(parts)

def numbered_excerpt(lines, focus):
    lo = max(1, min(focus) - 2)
    hi = min(len(lines), max(focus) + 2)
    out = []
    for ln in range(lo, hi + 1):
        mark = " ==>>" if ln in focus else "     "
        out.append(f"{ln:>3} {mark} {lines[ln - 1]}")
    return "\n".join(out)

# ----------------------------------------------------------------------------
# engine

def setup_run(cfg: dict, args, lang: str = "ir") -> Path:
    stamp = datetime.now(timezone.utc).strftime("%Y%m%d-%H%M%S")
    if args.adapter == "openai":
        tag = "openai-" + os.path.splitext((args.model or os.environ.get("LLM_MODEL") or "drive")
                                          .replace("/", "-"))[0]
    else:
        tag = args.adapter
    if lang == "js":
        tag = (tag or "file") + "-js"
    if lang == "ir-cs":
        tag = (tag or "file") + "-cs_ir"
    if lang == "cs":
        tag = (tag or "file") + "-cs"
    workdir = Path(args.workdir) if args.workdir else HERE / "runs" / f"{cfg['id']}_{tag}_{stamp}"
    if workdir.exists() and any(workdir.iterdir()):
        sys.exit(f"workdir not empty: {workdir}")
    workdir.mkdir(parents=True, exist_ok=True)
    return workdir

def cmd_run(args):
    lang = getattr(args, "lang", "ir")
    cfg = json.loads(Path(args.task).read_text())
    workdir = setup_run(cfg, args, lang)
    adapter = ADAPTERS[args.adapter](cfg, args)
    spec = cfg["spec_js"] if (lang == "js" and cfg.get("spec_js")) else cfg["spec_cs"] if (lang == "cs" and cfg.get("spec_cs")) else cfg["spec_cs_ir"] if (lang == "ir-cs" and cfg.get("spec_cs_ir")) else cfg["spec"]
    messages = []
    if lang != "js":
        # the language guide is IR-world overhead (the baseline needs none) —
        # this asymmetry is part of the J6 whole-pipeline measurement
        guide = (HERE / cfg.get("guide", "teach.md")).read_text()
        messages.append({"role": "system", "content": guide})
    messages.append({"role": "user", "content": spec})
    rounds_max = args.rounds
    verdict = f"FAIL: {rounds_max} rounds exhausted"
    round_log = []
    fence = {"ir": "ir", "js": "js", "ir-cs": "ir", "cs": "cs"}.get(lang, "ir")

    def extract(text):
        if lang == "ir":
            return extract_ir(text)
        if lang == "cs":
            return extract_cs(text)
        if lang == "ir-cs":
            return extract_ir(text)
        return extract_js(text)

    def emit_and_run(workdir, src_text):
        if lang == "ir":
            ok, errs, js_path = irc_emit(workdir, src_text)
            if not ok:
                return None, errs, None
            map_file = js_path.with_suffix("").with_name(js_path.name[:-3] + ".map.json")
            passed, out = run_rig(workdir, cfg)
            return (passed, out, js_path), [], map_file
        if lang == "ir-cs":
            ok, errs, cs_path = irc_emit_cs(workdir, src_text)
            if not ok:
                return None, errs, None
            passed, out = run_rig_cs(workdir, cfg)
            return (passed, out, cs_path), [], None
        if lang == "cs":
            cs_path = write_feature_cs(workdir, src_text)
            passed, out = run_rig_cs(workdir, cfg)
            return (passed, out, cs_path), [], None
        js_path = write_feature_js(workdir, src_text)
        map_file = None
        passed, out = run_rig(workdir, cfg)
        return (passed, out, js_path), [], map_file

    # optional pre-made feature (repair-phase experiment: a realistic draft
    # error the model must repair from mapped runtime errors)
    if args.seed_ir:
        text = Path(args.seed_ir).read_text().strip()
        messages.append({"role": "assistant", "content": f"```{fence}\n{text}\n```"})
        res, errs, map_file = emit_and_run(workdir, text)
        if res is None:
            messages.append({"role": "user", "content": f"The draft failed to {'transpile' if lang == 'ir' else 'load'}:\n" + "\n".join(f"- {e}" for e in errs)})
        else:
            passed, out, _js_path = res
            print(out, flush=True)
            if passed:
                sys.exit("seed already passes — nothing to repair; remove --seed-ir")
            fb = feedback_text(out, text, map_file, lang)
            messages.append({"role": "user", "content": fb})

    for r in range(1, rounds_max + 1):
        t0 = time.time()
        # archive the prompt/transcript for this round (every adapter —
        # report evidence; the file adapter's driver reads it)
        prompt_path = workdir / f"prompt-R{r:02}.md"
        prompt_path.write_text(
            render_transcript(messages).replace(
                "## Reply\n\nWrite your answer here",
                f"## Prompt file\n\nThis file: {prompt_path}\nReply by writing the transcript to: {args.reply_file or (HERE / 'runs' / 'reply.md')}\n\n## Reply\n\nWrite your answer here",
            ) if args.adapter == "file"
            else render_transcript(messages) + f"\n## Reply\n\n(complete {fence} file in one fenced {fence} block)\n"
        )
        try:
            reply = adapter.reply(workdir, messages if args.adapter == "openai" else messages, r)
        except (RuntimeError, TimeoutError, OSError) as e:
            # socket/HTTP timeouts surface as URLError/socket.timeout (OSError
            # subclasses) — record and stop cleanly instead of dying pre-report
            round_log.append({"round": r, "outcome": "adapter-error", "detail": str(e.__cause__ or e)[:300], "src_chars": 0, "elapsed": time.time() - t0})
            verdict = f"FAIL: adapter error in round {r}"
            break
        # repetition-breaker: an identical reply left the loop stuck (temp-0
        # small models repeat verbatim); tell the model it must change tack
        prev_reply = next((m["content"] for m in reversed(messages) if m["role"] == "assistant"), None)
        identical = prev_reply is not None and reply.strip() == prev_reply.strip()
        (workdir / f"reply-R{r:02}.md").write_text(reply)
        messages.append({"role": "assistant", "content": reply})
        if identical:
            messages.append({"role": "user", "content": "Your reply this round is byte-identical to the previous round despite the reported error. Re-examine the failing line and change something concrete (a method name, a condition, a return)."})
        src = extract(reply)
        if src is None:
            messages.append({"role": "user", "content": f"Your reply contained no fenced ```{fence} block. Re-emit the complete {fence} file in one fenced {fence} block."})
            round_log.append({"round": r, "outcome": f"no-{fence}-block", "detail": "", "src_chars": 0, "elapsed": time.time() - t0})
            continue
        # the cs worlds compile every .cs in the workdir (dotnet SDK glob) —
        # the per-round archives get non-compilable extensions (`.csrc`)
        round_ext = "csrc" if lang == "cs" else fence
        (workdir / f"feature-R{r:02}.{round_ext}").write_text(src + "\n")
        res, errs, map_file = emit_and_run(workdir, src)
        if res is None:
            stage = {"ir": "IR did not transpile. Errors", "ir-cs": "IR did not transpile. Errors", "js": "module failed to load. Errors", "cs": "C# module failed to build. Errors"}.get(lang, "IR did not transpile. Errors")
            msg = f"Your {stage}:\n" + "\n".join(f"- {e}" for e in errs) + f"\nRe-emit the complete corrected {fence} file."
            messages.append({"role": "user", "content": msg})
            round_log.append({"round": r, "outcome": "transpile-error" if lang == "ir" else "load-error", "detail": "; ".join(errs)[:300], "src_chars": len(src), "elapsed": time.time() - t0})
            continue
        passed, out, js_path = res
        detail = ""
        if passed:
            verdict = "PASS"
            round_log.append({"round": r, "outcome": "ALL PASS", "detail": f"{len(out.splitlines())} rig lines", "src_chars": len(src), "elapsed": time.time() - t0})
            break
        errs_lines = [l for l in out.splitlines() if l.startswith(("FAIL", "ERROR")) or re.search(r"(SyntaxError|TypeError|ReferenceError|RangeError|Error:)", l)]
        detail = (errs_lines[0] if errs_lines else out.strip().splitlines()[-1] if out.strip() else "")[:300]
        fb = feedback_text(out, src, map_file, lang)
        messages.append({"role": "user", "content": fb})
        round_log.append({"round": r, "outcome": "runtime-fail", "detail": detail, "src_chars": len(src), "elapsed": time.time() - t0})

    write_report(workdir, cfg, adapter.name, args, round_log, verdict, messages, lang)
    print(f"[loop] {verdict} — report: {workdir / 'report.md'}")

def write_report(workdir, cfg, adapter_name, args, round_log, verdict, messages, lang: str = "ir"):
    fence = "ir" if lang == "ir" else "js"
    world_note = {"js": " — no guide, no transpile, no span map (J6 baseline)", "cs": " — C# baseline world: no guide, no transpile", "ir-cs": " — guide + to-cs emit + dotnet rig"}.get(lang, " — guide + transpile + span map")
    rep = [f"# Loop run — {cfg['id']}" + ("" if lang in ("ir", "js") else f" ({lang} world)"), ""]
    rep.append(f"- lang/world: `{lang}`" + world_note)
    rep.append(f"- adapter: `{adapter_name}`" + (f", model: `{args.model}`" if args.model else ""))
    rep.append(f"- task: {cfg['title']}")
    rep.append(f"- rounds: {len(round_log)} (cap {args.rounds}); started {datetime.now(timezone.utc).isoformat(timespec='seconds')}")
    rep.append(f"- verdict: **{verdict}**")
    rep.append("")
    rep.append("| round | outcome | detail | src chars | s |")
    rep.append("|---|---|---|---|---|")
    for e in round_log:
        detail = e["detail"].replace("|", "\\|")
        rep.append(f"| {e['round']} | {e['outcome']} | {detail} | {e['src_chars']} | {e['elapsed']:.1f} |")
    rep.append("")
    if lang in ("ir-cs", "cs"):
        rep.append("Artifacts: `prompt-R*.md`, `reply-R*.md`, `feature-R*.ir` (model drafts), `feature.cs` (last emit), `rig.cs` (dotnet rig), `t.csproj`.")
    else:
        rep.append("Artifacts: `prompt-R*.md` (transcripts), `reply-R*.md` (model replies), `feature-R*." + fence + "`, "
                   f"`feature.{fence}` (last), `feature.js` (last emit/run), `rig.js` (cases)"
                   + (", `feature.map.json` (IR↔JS line map)" if lang == "ir" else " — no map in the js world") + ".")
    final_src = workdir / f"feature.{fence}"
    if final_src.exists():
        rep.append(f"\n## Final {'IR' if lang == 'ir' else 'JS'}\n\n```{fence}\n" + final_src.read_text() + "```\n")
    report = "\n".join(rep) + "\n"
    (workdir / "report.md").write_text(report)

def cmd_selftest(args):
    """Echo-adapter loop over task_01: verifies the deterministic mechanics
    (draft -> runtime error -> mapped feedback -> repair -> ALL PASS)."""
    import shutil
    tag = {"ir": "selftest-echo", "js": "selftest-echo-js", "ir-cs": "selftest-echo-cs_ir", "cs": "selftest-echo-cs"}.get(args.lang, "selftest-echo")
    workdir = HERE / "runs" / tag
    if workdir.exists():
        shutil.rmtree(workdir)
    ns = argparse.Namespace(
        cmd="run",
        task=str(HERE / "tasks" / "task_01_stringops.task.json"),
        adapter="echo",
        rounds=4,
        reply_file=None,
        timeout=None,
        base_url=None,
        api_key_env=None,
        model=None,
        num_ctx=None,
        seed_ir=None,
        workdir=str(workdir),
        lang=args.lang,
    )
    cmd_run(ns)
    verdict = (workdir / "report.md").read_text()
    if "verdict: **PASS**" not in verdict:
        sys.exit(f"selftest FAILED — see bench/scripts/loop/runs/{tag}/report.md")
    print(f"[selftest] OK: echo loop completed ({args.lang} world; draft bug repaired, ALL PASS)")

def main():
    ap = argparse.ArgumentParser(description="PLAN 2.7 LLM loop harness")
    sub = ap.add_subparsers(dest="cmd", required=True)
    p = sub.add_parser("run")
    p.add_argument("--task", required=True, help="task .json under tasks/")
    p.add_argument("--lang", default="ir", choices=["ir", "js", "ir-cs", "cs"], help="ir = transpiled-to-JS world (default); js = JS baseline world (J6); ir-cs = transpiled-to-C# world (to-cs + dotnet rig); cs = C# baseline world")
    p.add_argument("--adapter", default="echo", choices=sorted(ADAPTERS))
    p.add_argument("--rounds", type=int, default=4)
    p.add_argument("--reply-file", help="file-adapter: model writes the transcript here")
    p.add_argument("--timeout", type=float, default=3600.0, help="file-adapter poll timeout (s)")
    p.add_argument("--base-url", help="openai adapter override")
    p.add_argument("--api-key-env", help="name of the env var holding the API key")
    p.add_argument("--model")
    p.add_argument("--num-ctx", type=int, help="ollama-style options.num_ctx override")
    p.add_argument("--seed-ir", help="use this IR as the round-1 draft (debug)")
    p.add_argument("--workdir", help="explicit run directory")
    p.set_defaults(func=cmd_run)
    p2 = sub.add_parser("selftest", help="echo-adapter mechanics check (no model access)")
    p2.add_argument("--lang", default="ir", choices=["ir", "js", "ir-cs", "cs"])
    p2.set_defaults(func=cmd_selftest)
    args = ap.parse_args()
    args.func(args)

if __name__ == "__main__":
    main()