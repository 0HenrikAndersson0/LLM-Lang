#!/usr/bin/env python3
"""PLAN 2.7 — LLM loop test harness.

Drives: model writes a feature in IR -> transpile (irc to-js --map) -> run
(node rig) -> feed errors back (with IR lines recovered from the span map)
-> repair. Rounds until the rig passes or the round cap.

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

def resolve_irc() -> str:
    cand = REPO / "target" / "debug" / "irc"
    return str(cand) if cand.exists() else "irc"

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
    def reply(self, workdir: Path, messages: list, round_no: int) -> str:
        self.calls += 1
        return self.cfg["echo_draft"] if self.calls == 1 else self.cfg["echo_fix"]

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
        if not self.key:
            sys.exit("openai adapter: no API key (set --api-key-env target or LLM_API_KEY)")
        if not self.model:
            sys.exit("openai adapter: no model (set --model or LLM_MODEL)")
    def reply(self, workdir: Path, messages: list, round_no: int) -> str:
        import urllib.request
        body = json.dumps({
            "model": self.model,
            "messages": [dict(m) for m in messages],
            "temperature": 0,
        }).encode()
        req = urllib.request.Request(
            self.base.rstrip("/") + "/chat/completions",
            data=body,
            headers={"Content-Type": "application/json", "Authorization": f"Bearer {self.key}"},
        )
        with urllib.request.urlopen(req, timeout=180) as resp:
            data = json.loads(resp.read())
        try:
            return data["choices"][0]["message"]["content"]
        except Exception:
            return f"(adapter error: {json.dumps(data)[:2000]})"

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

def irc_emit(workdir: Path, ir_text: str):
    """write feature.ir; emit via irc to-js --map. Returns (ok, errors, js_path)."""
    ir_path = workdir / "feature.ir"
    ir_path.write_text(ir_text if ir_text.endswith("\n") else ir_text + "\n")
    proc = subprocess.run(
        [resolve_irc(), "to-js", "--map", str(ir_path)],
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

def feedback_text(node_out: str, ir_text: str, map_file: Path) -> str:
    """Errors + the mapped IR lines (span map, innermost rule)."""
    parts = ["## Reported errors\n", "```\n" + node_out.strip()[:4000] + "\n```"]
    entries = json.loads(map_file.read_text())["mappings"] if map_file.exists() else []
    lines = ir_text.splitlines()
    ir_refs = []
    for js_line in sorted({int(x) for x in re.findall(r"feature\.js:(\d+):", node_out)}):
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
    parts.append(
        "\nFix the problem and re-emit the COMPLETE corrected .ir file in one fenced ir block."
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

def setup_run(cfg: dict, args) -> Path:
    stamp = datetime.now(timezone.utc).strftime("%Y%m%d-%H%M%S")
    if args.adapter == "openai":
        tag = "openai-" + os.path.splitext((args.model or os.environ.get("LLM_MODEL") or "drive")
                                          .replace("/", "-"))[0]
    else:
        tag = args.adapter
    workdir = Path(args.workdir) if args.workdir else HERE / "runs" / f"{cfg['id']}_{tag}_{stamp}"
    if workdir.exists() and any(workdir.iterdir()):
        sys.exit(f"workdir not empty: {workdir}")
    workdir.mkdir(parents=True, exist_ok=True)
    return workdir

def cmd_run(args):
    cfg = json.loads(Path(args.task).read_text())
    guide = (HERE / cfg.get("guide", "teach.md")).read_text()
    workdir = setup_run(cfg, args)
    adapter = ADAPTERS[args.adapter](cfg, args)
    messages = [
        {"role": "system", "content": guide},
        {"role": "user", "content": cfg["spec"]},
    ]
    rounds_max = args.rounds
    verdict = f"FAIL: {rounds_max} rounds exhausted"
    round_log = []

    # optional pre-made IR (repair-phase experiment: a realistic draft error
    # the model must repair from mapped runtime errors)
    if args.seed_ir:
        text = Path(args.seed_ir).read_text()
        messages.append({"role": "assistant", "content": f"```ir\n{text}\n```"})
        ok, errs, js_path = irc_emit(workdir, text)
        if not ok:
            messages.append({"role": "user", "content": "The draft IR failed to transpile:\n" + "\n".join(f"- {e}" for e in errs)})
        else:
            passed, out = run_rig(workdir, cfg)
            print(out, flush=True)
            if passed:
                sys.exit("seed already passes — nothing to repair; remove --seed-ir")
            fb = feedback_text(out, text, js_path.with_suffix("").with_name(js_path.name[:-3] + ".map.json"))
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
            else render_transcript(messages) + "\n## Reply\n\n(complete .ir file in one fenced ir block)\n"
        )
        try:
            reply = adapter.reply(workdir, messages if args.adapter == "openai" else messages, r)
        except RuntimeError as e:
            round_log.append({"round": r, "outcome": "adapter-timeout", "detail": str(e), "ir_chars": 0, "elapsed": time.time() - t0})
            break
        (workdir / f"reply-R{r:02}.md").write_text(reply)
        messages.append({"role": "assistant", "content": reply})
        ir = extract_ir(reply)
        if ir is None:
            messages.append({"role": "user", "content": "Your reply contained no fenced ```ir block. Re-emit the complete .ir file in one fenced ir block."})
            round_log.append({"round": r, "outcome": "no-ir-block", "detail": "", "ir_chars": 0, "elapsed": time.time() - t0})
            continue
        (workdir / f"feature-R{r:02}.ir").write_text(ir + "\n")
        ok, errs, js_path = irc_emit(workdir, ir)
        if not ok:
            msg = "Your IR did not transpile. Errors:\n" + "\n".join(f"- {e}" for e in errs) + "\nRe-emit the complete corrected .ir file."
            messages.append({"role": "user", "content": msg})
            round_log.append({"round": r, "outcome": "transpile-error", "detail": "; ".join(errs)[:300], "ir_chars": len(ir), "elapsed": time.time() - t0})
            continue
        passed, out = run_rig(workdir, cfg)
        detail = ""
        if passed:
            verdict = "PASS"
            round_log.append({"round": r, "outcome": "ALL PASS", "detail": f"{len(out.splitlines())} rig lines", "ir_chars": len(ir), "elapsed": time.time() - t0})
            break
        errs_lines = [l for l in out.splitlines() if l.startswith(("FAIL", "ERROR")) or re.search(r"(SyntaxError|TypeError|ReferenceError|RangeError|Error:)", l)]
        detail = (errs_lines[0] if errs_lines else out.strip().splitlines()[-1] if out.strip() else "")[:300]
        fb = feedback_text(out, ir, js_path.with_suffix("").with_name(js_path.name[:-3] + ".map.json"))
        messages.append({"role": "user", "content": fb})
        round_log.append({"round": r, "outcome": "runtime-fail", "detail": detail, "ir_chars": len(ir), "elapsed": time.time() - t0})

    write_report(workdir, cfg, adapter.name, args, round_log, verdict, messages)
    print(f"[loop] {verdict} — report: {workdir / 'report.md'}")

def write_report(workdir, cfg, adapter_name, args, round_log, verdict, messages):
    rep = [f"# Loop run — {cfg['id']}", ""]
    rep.append(f"- adapter: `{adapter_name}`" + (f", model: `{args.model}`" if args.model else ""))
    rep.append(f"- task: {cfg['title']}")
    rep.append(f"- rounds: {len(round_log)} (cap {args.rounds}); started {datetime.now(timezone.utc).isoformat(timespec='seconds')}")
    rep.append(f"- verdict: **{verdict}**")
    rep.append("")
    rep.append("| round | outcome | detail | IR chars | s |")
    rep.append("|---|---|---|---|---|")
    for e in round_log:
        detail = e["detail"].replace("|", "\\|")
        rep.append(f"| {e['round']} | {e['outcome']} | {detail} | {e['ir_chars']} | {e['elapsed']:.1f} |")
    rep.append("")
    rep.append("Artifacts: `prompt-R*.md` (transcripts), `reply-R*.md` (model replies), `feature-R*.ir`, "
               "`feature.ir` (last), `feature.js` (last emit), `feature.map.json` (IR↔JS line map), `rig.js` (cases).")
    final_ir = workdir / "feature.ir"
    if final_ir.exists():
        rep.append("\n## Final IR\n\n```ir\n" + final_ir.read_text() + "```\n")
    report = "\n".join(rep) + "\n"
    (workdir / "report.md").write_text(report)

def cmd_selftest(_args):
    """Echo-adapter loop over task_01: verifies the deterministic mechanics
    (draft -> runtime error -> mapped feedback -> repair -> ALL PASS)."""
    import shutil
    workdir = HERE / "runs" / "selftest-echo"
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
        seed_ir=None,
        workdir=str(workdir),
    )
    cmd_run(ns)
    verdict = (workdir / "report.md").read_text()
    if "verdict: **PASS**" not in verdict:
        sys.exit("selftest FAILED — see bench/scripts/loop/runs/selftest-echo/report.md")
    print("[selftest] OK: echo loop completed (draft bug repaired, ALL PASS)")

def main():
    ap = argparse.ArgumentParser(description="PLAN 2.7 LLM loop harness")
    sub = ap.add_subparsers(dest="cmd", required=True)
    p = sub.add_parser("run")
    p.add_argument("--task", required=True, help="task .json under tasks/")
    p.add_argument("--adapter", default="echo", choices=sorted(ADAPTERS))
    p.add_argument("--rounds", type=int, default=4)
    p.add_argument("--reply-file", help="file-adapter: model writes the transcript here")
    p.add_argument("--timeout", type=float, default=3600.0, help="file-adapter poll timeout (s)")
    p.add_argument("--base-url", help="openai adapter override")
    p.add_argument("--api-key-env", help="name of the env var holding the API key")
    p.add_argument("--model")
    p.add_argument("--seed-ir", help="use this IR as the round-1 draft (debug)")
    p.add_argument("--workdir", help="explicit run directory")
    p.set_defaults(func=cmd_run)
    p2 = sub.add_parser("selftest", help="echo-adapter mechanics check (no model access)")
    p2.set_defaults(func=cmd_selftest)
    args = ap.parse_args()
    args.func(args)

if __name__ == "__main__":
    main()