#!/usr/bin/env python3
"""Tokenizer scoring harness for the LLM-Lang IR (Phase 0, task 0.2).

Counts tokens for each file in a corpus under multiple tokenizers and reports
per-file counts, min/max/mean, and % of a baseline.

Usage:
  python3 bench/scripts/score.py                      # all corpora
  python3 bench/scripts/score.py --group variants     # only bench/variants
  python3 bench/scripts/score.py --baseline csharp    # % vs .cs files
"""

import argparse
import json
import statistics
import sys
from pathlib import Path
from typing import Optional

TIKTOKEN_ENCODING = "o200k_base"  # GPT-4o / Codex-class models
LLAMA_MODEL_ID = "hf-internal-testing/llama-tokenizer"

_tokenizers_cache = {}


def get_tokenizers():
    encoders = {}
    try:
        import tiktoken

        enc = tiktoken.get_encoding(TIKTOKEN_ENCODING)
        encoders["o200k_base"] = enc
    except Exception:
        pass
    try:
        from tokenizers import Tokenizer

        tok = Tokenizer.from_pretrained(LLAMA_MODEL_ID)
        encoders["llama"] = tok
    except Exception:
        pass
    return encoders


def count_tokens(text: str, tokenizer, name: str) -> int:
    if name == "o200k_base":
        return len(tokenizer.encode(text))
    return len(tokenizer.encode(text, add_special_tokens=False))


def score_file(path: Path, tokenizers: dict) -> dict:
    text = path.read_text(encoding="utf-8")
    counts = {}
    for name, tok in tokenizers.items():
        counts[name] = count_tokens(text, tok, name)
    return {"file": str(path), "counts": counts}


def summarize(rows: list, tokenizer_name: str, baseline_ext: Optional[str]):
    counts = [r["counts"][tokenizer_name] for r in rows]
    if not counts:
        return None
    summary = {
        "files": len(counts),
        "min": min(counts),
        "max": max(counts),
        "mean": round(statistics.mean(counts), 1),
    }
    if baseline_ext:
        base_rows = [
            r
            for r in rows
            if r["file"].endswith(baseline_ext) or r["file"].endswith("." + baseline_ext)
        ]
        base_total = sum(r["counts"][tokenizer_name] for r in base_rows)
        other_total = sum(
            r["counts"][tokenizer_name]
            for r in rows
            if r not in base_rows
        )
        if base_total:
            summary["baseline_total"] = base_total
            summary["other_total"] = other_total
            summary["other_vs_baseline_pct"] = round(
                100.0 * other_total / base_total, 1
            )
    return summary


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument(
        "--corpus-root",
        type=Path,
        default=Path(__file__).resolve().parents[2] / "bench" / "corpus",
        help="Root directory containing corpus files",
    )
    ap.add_argument("--group", help="Subdirectory to restrict scoring to")
    ap.add_argument(
        "--baseline",
        help="Extension treated as baseline for %% comparisons (e.g. cs)",
    )
    ap.add_argument("--json", action="store_true", help="Output JSON only")
    ap.add_argument(
        "--inspect",
        metavar="FILE",
        help="Show token-by-token breakdown of a single file (first tokenizer)",
    )
    args = ap.parse_args()

    tokenizers = get_tokenizers()
    if not tokenizers:
        print("error: no tokenizers available", file=sys.stderr)
        sys.exit(1)

    if args.inspect:
        tok_name, tok = next(iter(tokenizers.items()))
        text = Path(args.inspect).read_text(encoding="utf-8")
        if tok_name == "o200k_base":
            ids = tok.encode(text)
            toks = [tok.decode([i]) for i in ids]
        else:
            enc = tok.encode(text, add_special_tokens=False)
            toks = [tok.decode([e]) for e in enc.ids]
        print(f"tokenizer={tok_name} tokens={len(toks)} file={args.inspect}")
        for i, t in enumerate(toks, 1):
            print(f"{i:>4}  {t!r}")
        return

    root = args.corpus_root
    if args.group:
        root = root / args.group

    files = sorted(
        p for p in root.rglob("*") if p.is_file() and p.suffix in {".ir", ".cs", ".js"}
    )
    if not files:
        print(f"error: no corpus files under {root}", file=sys.stderr)
        sys.exit(1)

    rows = [score_file(f, tokenizers) for f in files]

    out = {"tokenizer_versions": {"o200k_base": TIKTOKEN_ENCODING}, "files": rows}
    for tok_name in tokenizers:
        s = summarize(rows, tok_name, args.baseline)
        if s:
            out[f"summary_{tok_name}"] = s

    total_counts = {
        tok_name: sum(r["counts"][tok_name] for r in rows)
        for tok_name in tokenizers
    }
    out["totals"] = total_counts

    if args.json:
        print(json.dumps(out, indent=2))
        return

    name_w = max(len(r["file"]) for r in rows) + 2
    print(f"{'file':<{name_w}}" + "".join(f"{t:>12}" for t in tokenizers))
    for r in rows:
        short = r["file"]
        print(f"{short:<{name_w}}" + "".join(f"{r['counts'][t]:>12}" for t in tokenizers))
    print(f"{'TOTAL':<{name_w}}" + "".join(f"{total_counts[t]:>12}" for t in tokenizers))
    for tok_name in tokenizers:
        s = out.get(f"summary_{tok_name}")
        if s and "other_vs_baseline_pct" in s:
            print(f"[{tok_name}] {s['other_total']} vs baseline {s['baseline_total']} -> {s['other_vs_baseline_pct']}% of baseline")
        elif s:
            print(f"[{tok_name}] min={s['min']} max={s['max']} mean={s['mean']}")


if __name__ == "__main__":
    main()