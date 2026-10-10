#!/usr/bin/env python3
"""PLAN J6 — whole-pipeline token measurement (footnote [1]).

The north-star metric is the whole-pipeline view: not just the one-shot IR
token count, but everything the model reads and writes while working until
the feature passes — the number that predicts agent-loop spend.

Two worlds, same tasks/cases/rig (see llm_loop.py --lang):

  ir world   system=teach.md + task spec, model writes IR, transpile+map,
             errors fed back with mapped/numbered IR.
  js world   task spec only (no guide), model writes feature.js directly,
             raw node errors fed back. This is the baseline agent-file
             world (the model could have written JS in the first place).

Spend per run = tokens(read prompts) + tokens(written replies), over the
rounds actually spent (a 1-round PASS spends R01 only; a FAIL spends all).
`†` rows = seeded repair experiments (R01 contains a draft+error already);
not cold one-shots — excluded from one-shot claims.

Headline: ir-world total vs js-world total on the same task+model pair,
plus the one-shot ratio (R01 spend only) for context. Counted with the
Phase-0 tokenizers (o200k_base + llama when importable — run under the
bench venv: `bench/.venv/bin/python pipeline_tokens.py`).

Usage:
  python3 pipeline_tokens.py                 # scan all runs/
  python3 pipeline_tokens.py --json          # machine output
  python3 pipeline_tokens.py runs/task_01..  # explicit dirs only
"""

import argparse
import json
import re
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
RUNS = HERE / "runs"

_tokenizers = {}

def get_tokenizers():
    if _tokenizers:
        return _tokenizers
    try:
        import tiktoken
        _tokenizers["o200k"] = tiktoken.get_encoding("o200k_base")
    except Exception:
        pass
    try:
        from tokenizers import Tokenizer
        _tokenizers["llama"] = Tokenizer.from_pretrained(
            "hf-internal-testing/llama-tokenizer")
    except Exception:
        pass
    return _tokenizers


def count(text: str, name: str) -> int:
    tok = get_tokenizers().get(name)
    if tok is None:
        return -1
    if name == "o200k":
        return len(tok.encode(text))
    return len(tok.encode(text, add_special_tokens=False))


# older run dirs truncated model names (setup_run splitext on ':');
# normalized to the canonical ollama names for paired comparison
CANON = {
    "qwen2": "qwen2.5-coder:7b",
    "qwen3": "qwen3.5:9b",
    "qwen2.5-coder:1": "qwen2.5-coder:1.5b",
    "drive": "(agent-driven)",
}

def parse_report(run: Path):
    """(world, model, task, rounds_used, verdict) from report.md."""
    rep = run / "report.md"
    if not rep.exists():
        return None
    text = rep.read_text()
    world = "js" if ("js baseline world" in text or "-js_" in run.name) else "ir"
    m = re.search(r"adapter: `([^`]+)`", text)
    adapter = m.group(1) if m else "unknown"
    if adapter == "file":
        model = "(agent-driven)"
    else:
        m = re.search(r"model: `([^`]+)`", text)
        if m:
            model = m.group(1)
        else:
            # dir name: {task}_{tag}_{stamp}; tag = openai-<model[-js]>
            m = re.search(r"openai-([A-Za-z0-9.:\-]+?)(?:-js)?_\d{8}-\d{6}$", run.name)
            model = m.group(1) if m else run.name
        model = CANON.get(model, model)
    m = re.search(r"# Loop run — ([\w\-]+)", text)
    task = m.group(1) if m else run.name.split("_", 1)[0]
    verdict_m = re.search(r"verdict: \*\*(.+?)\*\*", text)
    verdict = verdict_m.group(1) if verdict_m else "unknown"
    rounds = len(re.findall(r"\n\| \d+ \| ", text))
    r1 = run / "prompt-R01.md"
    seeded = r1.exists() and "## ASSISTANT" in r1.read_text()
    return world, model, task, rounds, verdict, seeded


def guide_tokens(run: Path, counters):
    """tokens of the system block in THIS run's prompt-R01 (the guide as the
    model actually saw it — teach.md has grown across sessions, so a global
    count would mislabel historical runs). Empty for js-world runs."""
    p = run / "prompt-R01.md"
    if not p.exists():
        return None
    t = p.read_text()
    m = re.match(r"## SYSTEM\n\n(.*?)\n+## USER", t, re.S)
    if not m:
        return None
    block = m.group(1)
    return {n: count(block, n) for n in counters}


def run_spend(run: Path, rounds_used: int, counters):
    """tokens(read) + tokens(write) over the rounds spent, per tokenizer.
    Rounds past the archive are not counted (adapter-timeout rows)."""
    out = {}
    for name in counters:
        read = write = 0
        for r in range(1, rounds_used + 1):
            p = run / f"prompt-R{r:02}.md"
            q = run / f"reply-R{r:02}.md"
            if p.exists():
                read += count(p.read_text(), name)
            if q.exists():
                write += count(q.read_text(), name)
        out[name] = {"read": read, "write": write, "total": read + write}
    rounds_archived = len(list(run.glob("reply-R*.md")))
    return {"rounds_archived": rounds_archived, "spend": out,
            "guide": guide_tokens(run, counters)}


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("dirs", nargs="*", help="run dirs (default: scan all under runs/)")
    ap.add_argument("--latest", action="store_true",
                    help="keep only the most recent run per (task, model, world) — "
                         "drops obsolete experiments (e.g. pre-fixer-fail runs)")
    ap.add_argument("--json", action="store_true")
    ap.add_argument("--out", metavar="FILE", help="also write a markdown report")
    args = ap.parse_args()

    counters = sorted(get_tokenizers())
    if not counters:
        sys.exit("error: no tokenizers importable (use bench/.venv/bin/python)")

    dirs = [Path(d) for d in args.dirs] or sorted(
        p for p in RUNS.iterdir()
        if p.is_dir() and p.name.startswith("task_"))
    rows = []
    for run in dirs:
        pr = parse_report(run)
        if pr is None:
            continue
        world, model, task, rounds, verdict, seeded = pr
        sp = run_spend(run, max(rounds, 1), counters)
        rows.append({"dir": run.name, "world": world, "model": model,
                     "task": task, "rounds": rounds, "verdict": verdict,
                     "seeded": seeded, **sp})

    if args.latest:
        # dir names sort by task then stamp — keep the last per (task, model, world)
        keep = {}
        for r in rows:
            keep[(r["task"], r["model"], r["world"])] = r
        rows = [keep[k] for k in sorted(keep)]

    if args.json:
        print(json.dumps({"tokenizers": counters, "runs": rows}, indent=2))
        return

    # markdown table (per tokenizer — o200k first)
    lines = ["# Whole-pipeline token spend (J6, footnote [1])", ""]
    lines.append("Read+write tokens over the rounds actually spent per run "
                 "(`llm_loop.py` transcripts). `ir` = guide+transpile world; "
                 "`js` = baseline agent world (no guide, direct JS). "
                 f"`guide` row = one-shot teach.md overhead paid once per IR session.\n")

    # paired headline: same task + model across the two worlds
    by_key = {}
    for r in rows:
        by_key.setdefault((r["task"], r["model"], r["world"]), []).append(r)

    lines.append("## Session-amortized view (per delivered feature)\n")
    lines.append("The guide is paid ONCE per loop session; in the agent-file model the same"
                 " session delivers many features. Marginal per-feature cost drops the guide"
                 " from the ir spend; js cost unchanged. Break-even S = guide / saving per feature.\n")
    for name in counters:
        lines.append(f"\n*tokenizer: {name}*\n")
        lines.append("| task | model | ir/feat | js/feat | ir % of js | guide | break-even S | ir % @S=5 | @S=10 | @S=20 |")
        lines.append("|---|---|---:|---:|---:|---:|---:|---:|---:|---:|")
        seen = set()
        for (task, model, world), rs in sorted(by_key.items()):
            if (task, model) in seen or world != "ir":
                continue
            pair = {w: by_key.get((task, model, w)) for w in ("ir", "js")}
            if not pair["ir"] or not pair["js"]:
                continue
            seen.add((task, model))

            def pass_run(rs):
                # delivered feature = the PASS run (latest if several)
                ps = [r for r in rs if r["verdict"] == "PASS"]
                return ps[-1] if ps else None

            ir_r, js_r = pass_run(pair["ir"]), pass_run(pair["js"])
            if not ir_r or not js_r:
                continue
            ir_tot = ir_r["spend"][name]["total"]
            js_tot = js_r["spend"][name]["total"]
            guide = (ir_r.get("guide") or {}).get(name, 0)
            ir_fe = ir_tot - guide
            js_fe = js_tot
            if js_fe <= ir_fe:
                continue
            s_be = round(guide / (js_fe - ir_fe), 1)
            def at(S):
                return f"{round(100.0 * (guide + S * ir_fe) / (S * js_fe), 1)}%"
            lines.append(
                f"| {task} | {model} | {ir_fe} | {js_fe} "
                f"| {round(100.0 * ir_fe / js_fe, 1)}% | {guide} | {s_be} "
                f"| {at(5)} | {at(10)} | {at(20)} |")

    lines.append("\n## Paired: ir vs js world (same task + model)\n")
    for name in counters:
        lines.append(f"\n*tokenizer: {name}*\n")
        lines.append("| task | model | ir world | js world | ir tokens | js tokens | ir % of js |")
        lines.append("|---|---|---|---|---:|---:|---:|")
        seen = set()
        for (task, model, world), rs in sorted(by_key.items()):
            if (task, model) in seen:
                continue
            pair = {w: by_key.get((task, model, w)) for w in ("ir", "js")}
            if not pair["ir"] or not pair["js"]:
                continue
            seen.add((task, model))
            def cell(rs):
                t = sum(r["spend"][name]["total"] for r in rs)
                detail = ", ".join(
                    (f"{r['rounds']}r {r['verdict'].replace(': ', '/')}"
                     + (" (seeded)" if r.get("seeded") else ""))
                    for r in rs)
                return detail, t
            (ir_d, ir_t), (js_d, js_t) = cell(pair["ir"]), cell(pair["js"])
            pct = f"{round(100.0 * ir_t / js_t, 1)}%" if js_t else "—"
            lines.append(f"| {task} | {model} | {ir_d} | {js_d} | {ir_t} | {js_t} | {pct} |")

    for name in counters:
        lines.append(f"\n## All runs — tokenizer: {name}\n")
        lines.append("| run | world | model | seeded | task | rounds | verdict | read | write | total |")
        lines.append("|---|---|---|---|---|---|---|---:|---:|---:|")
        for r in rows:
            s = r["spend"][name]
            lines.append(
                f"| {r['dir']} | {r['world']} | {r['model']} | {'†' if r.get('seeded') else ''} | {r['task']} "
                f"| {r['rounds']} | {r['verdict']} | {s['read']} | {s['write']} | {s['total']} |")
        if rows:
            g = next(({"guide": r.get("guide")} for r in rows if r.get("guide")), None)
            if g:
                lines.append(f"- guide in R01 (per-run system block): {g['guide'][name]} tokens (paid once per IR loop session)")
    text = "\n".join(lines) + "\n"
    print(text)
    if args.out:
        Path(args.out).write_text(text)


if __name__ == "__main__":
    main()