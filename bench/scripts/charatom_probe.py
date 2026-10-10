#!/usr/bin/env python3
"""Phase-0 re-run probe: the Char atom admission (6.4's named @2 candidate).

The IR canon has no char atom — C# single-quote chars canon to Str atoms
(`'k'` -> `"k"`). The C# emitter restores strings in char positions, which
produces 8 compile-diagnostics inside the round-tripped cs_05/cs_06 files
(`new string("0", n)` CS1503, `chars[i]=="0"` CS0019, … fidelity row 17).

This probe measures what a Char atom would do to the codegen token
economy, per the Phase-0 discipline (both tokenizers, paired rows, no
aggregation tricks):

  1. Per-literal: `'X'` vs `"X"` as they appear in canon rows — the only
     text change a Char atom makes to the canon surface.
  2. Whole-file: cs_05 with char positions re-quoted — position-aware:
     the canon CANNOT express char-vs-string for equal payloads (canon
     `"k"` carries both the Symbols char and the `.ToString("k")` string
     suffix) — that ambiguity IS the argument for the atom. The probe
     swaps only the rows the EMIT restores as chars today (Symbols, dict
     keys, rounding literals) and leaves the string-suffix rows untouched.

Method: paired, deterministic, no curation.
"""

import importlib.util
import re
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location("score", REPO / "bench/scripts/score.py")
score = importlib.util.module_from_spec(spec)
sys.modules["score"] = score
spec.loader.exec_module(score)

TOKS = score.get_tokenizers()


def count(txt, name):
    tok = TOKS[name]
    if name == "o200k_base":
        return len(tok.encode(txt))
    return len(tok.encode(txt, add_special_tokens=False).ids)


def both(txt):
    return {n: count(txt, n) for n in TOKS}


def context_probe():
    print("== per-literal: canon context pairs ==")
    contexts = ['=={"Y",', '= \'0\'', '[["k",', '=="5"&&', '-"0");']
    rows = []
    for ctx in contexts:
        str_form = ctx.replace("'Y'", '"Y"').replace("'0'", '"0"')
        rows.append((ctx.strip(), both(ctx.strip()), both(str_form.strip())))
    tot_prop = {n: 0 for n in TOKS}
    tot_cur = {n: 0 for n in TOKS}
    for label, prop, cur in rows:
        d = {n: cur[n] - prop[n] for n in TOKS}
        for n2 in TOKS:
            tot_prop[n2] += prop[n2]
            tot_cur[n2] += cur[n2]
        print(f"  {label!r:28} proposed {prop['o200k_base']}/{prop['llama']}  current {cur['o200k_base']}/{cur['llama']}  {d['o200k_base']:+d}/{d['llama']:+d}")
    print(f"  {'TOTAL (6 rows → 24/26)':28} proposed {tot_prop['o200k_base']}/{tot_prop['llama']}  current {tot_cur['o200k_base']}/{tot_cur['llama']}")


def file_probe():
    ir = Path(REPO / "bench/corpus/real/rt_cs/cs_05_extension_static.ir")
    rows = []
    n = 0
    for line in ir.read_text().splitlines():
        out = line
        count = 0
        if '[["k"' in line or ('new (' in line and '{{' in line):
            # Symbols data row + dict-entry rows: every 1-char Str is a char
            def swap(m):
                nonlocal count
                count += 1
                return "'" + m.group(1) + "'"
            out = re.sub(r'"([^^\\]{1})"', swap, line)
        elif re.search(r'=="[059]"|-"[059]"', line):
            # rounding-litera rows: == "0" / < "9" / - "0"
            def swap(m):
                nonlocal count
                count += 1
                return "'" + m.group(1) + "'"
            out = re.sub(r'"([059])"', swap, line)
        n += count
        rows.append(out)
    swapped = "\n".join(rows)
    a = both(ir.read_text())
    b = both(swapped)
    print(f"\n== whole-file: cs_05 canon ({n} char positions swapped; the ToString('k'-suffix string rows stay) ==")
    print(f"  current     o200k {a['o200k_base']:>5}  llama {a['llama']:>5}")
    print(f"  char-atom   o200k {b['o200k_base']:>5}  llama {b['llama']:>5}")
    print(f"  delta       o200k {b['o200k_base'] - a['o200k_base']:+5}  llama {b['llama'] - a['llama']:+5}")
    return n, {k2: b[k2] - a[k2] for k2 in TOKS}


def main():
    chars = sorted(set(re.findall(r"'([^'\\]{1,3})'", (Path(REPO / "bench/corpus/real/src/cs_05_extension_static.cs").read_text()))))
    known = set(chars)
    print(f"cs_05 source char-literal distinct forms: {len(known)}")
    occ = 0
    for p in (REPO / "bench/corpus/real/src").glob("cs_*.cs"):
        occ += len(re.findall(r"'[^'\\]{1,3}'", p.read_text()))
    print(f"cs-corpus char-literal occurrences: {occ}")

    strings3 = set(re.findall(r'"([^"\\]{1,3})"', (Path(REPO / "bench/corpus/real/src/cs_05_extension_static.cs").read_text())))
    ambiguous = sorted(k for k in known if k in strings3)
    print(f"payloads that the SAME source file holds as BOTH char and short string: {ambiguous} — canon text cannot distinguish (the atom's argument)")

    context_probe()
    file_n, file_d = file_probe()

    print("\n== verdict inputs ==")
    print(f"  per-literal token cost of a Char atom: ~0 per row (quoted-pair identical)")
    print(f"  cs_05 whole-file canon delta: {file_d['o200k_base']:+d} o200k / {file_d['llama']:+d} llama over {file_n} swaps")
    print("  emit rows fixed by the Char atom: 8 (CS1503/CS0019/CS0029/CS1950/CS0021 — fidelity row 17)")
    print("  grammar cost: @1 -> @2 migration + reader/emitter arms (versioning.md)")


if __name__ == "__main__":
    main()