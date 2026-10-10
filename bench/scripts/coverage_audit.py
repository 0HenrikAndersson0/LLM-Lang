#!/usr/bin/env python3
"""PLAN 4.2 — IR-form coverage audit over the js corpus.

Joins the `irc js-kinds` histogram (tree-sitter-typescript CST kinds over
bench/corpus/real/src/js_*.ts) with a curated kind -> verdict table:

  core        direct core form (design doc §2) / verbatim surface (floor #1)
  sugar       canonicalized into a core form (e.g. template strings,
              function exprs -> arrows, update expr -> compound assign)
  signature   type-surface node: erased to IR signatures (§3) or
              reconstructed by the printer (§6.6)
  opaquetype  capturable as `opaque type` (reconcile row 21) — representable
              with the §6.5 bounded-loss form
  dropped     comments (§6.1, by design — not a coverage loss)
  grammar     tree-sitter gap, but the CONSTRUCT is IR-representable
              (Core::Reexport row)
  trivia      syntax atoms / structural tokens with no IR counterpart

Coverage % uses node counts EXCLUDING trivia/dropped (the constructs the
reader must map). Per-kind instances list corpus files for non-core kinds
as evidence.

Usage:
  bench/.venv/bin/python bench/scripts/coverage_audit.py [--md]
"""

import argparse
import json
import re
import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
SRC = REPO / "bench" / "corpus" / "real" / "src"

# kind -> (bucket, note). Anything unlisted -> 'unmapped' (audit failure).
VERDICTS = {
    # --- structural roots / punctuation / keyword atoms (trivia)
    "program": ("trivia", "CST root"),
    "!": ("trivia", ""), "!=": ("trivia", ""), "!==": ("trivia", ""),
    "${": ("trivia", "template delim"), "&&": ("trivia", "infix atom"),
    "'": ("trivia", ""), "(": ("trivia", ""), ")": ("trivia", ""),
    "*": ("trivia", ""), "+": ("trivia", ""), "++": ("trivia", ""),
    "+=": ("trivia", ""), ",": ("trivia", ""), "-": ("trivia", ""),
    ".": ("trivia", "member dot"), "...": ("trivia", "spread atom"),
    "/": ("trivia", ""), ":": ("trivia", ""), ";": ("trivia", ""),
    "<": ("trivia", ""), "<=": ("trivia", ""), "=": ("trivia", ""),
    "===": ("trivia", "infix atom"), "=>": ("trivia", "lambda atom"),
    ">": ("trivia", ""), ">=": ("trivia", ""), "?": ("trivia", ""),
    "?.": ("trivia", "optional-chain atom"), "??": ("trivia", ""),
    "[": ("trivia", ""), "]": ("trivia", ""), "`": ("trivia", ""),
    "{": ("trivia", ""), "}": ("trivia", ""), "|": ("trivia", "union sep"),
    "||": ("trivia", "infix atom"),
    "const": ("trivia", "var canon"), "let": ("trivia", "var canon"),
    "type": ("trivia", "alias keyword"),
    "of": ("trivia", ""), "from": ("trivia", ""), "extends": ("trivia", ""),
    "default_type": ("trivia", "type param default"),
    "is": ("trivia", "type predicate word (signature)"),
    # --- leaf values / names: pass through verbatim (floor #1)
    "identifier": ("core", "names verbatim"),
    "property_identifier": ("core", "member names verbatim"),
    "shorthand_property_identifier": ("core", "shorthand member"),
    "number": ("core", "numeric literal"),
    "string": ("core", "string literal"),
    "string_fragment": ("trivia", "string content"),
    "escape_sequence": ("trivia", "escape atom"),
    "true": ("core", "Bool literal"), "false": ("core", "Bool literal"),
    "null": ("core", "null literal"),
    "undefined": ("core", "undefined — JS-only value; emitted as `undefined`"),
    "this": ("core", "this verbatim"),
    # --- statements & declarations
    "expression_statement": ("core", "statement run"),
    "return_statement": ("core", "`return e`"),
    "return": ("trivia", ""),
    "if_statement": ("core", "`if c{…}else{…}`"),
    "if": ("trivia", ""), "else": ("trivia", ""),
    "else_clause": ("core", "else arm"),
    "for_statement": ("core", "C-style for (js-target §4 row)"),
    "for_in_statement": ("core", "`for x in xs{}`"),
    "for": ("trivia", ""), "of": ("trivia", ""),
    "statement_block": ("core", "body blocks"),
    "lexical_declaration": ("core", "`var name=expr` (mut via assignment)"),
    "variable_declarator": ("core", "declarator"),
    "function_declaration": ("core", "`fn Name(..)`"),
    "function": ("trivia", "keyword token"),
    "arrow_function": ("core", "`x=>{…}` lambdas"),
    "function_expression": ("sugar", "canonicalized to arrow"),
    "export_statement": ("core", "`export` modifier / re-export (Core::Reexport)"),
    "export": ("trivia", ""),
    "export_clause": ("core", "Core::Reexport"),
    "export_specifier": ("core", "Core::Reexport arms"),
    "import_statement": ("core", "`@use Path`"),
    "import": ("trivia", ""), "import_clause": ("trivia", ""),
    "named_imports": ("core", "@use named imports (J4)"),
    "import_specifier": ("core", "@use arm"),
    # --- expressions
    "call_expression": ("core", "call"), "arguments": ("trivia", "call args"),
    "member_expression": ("core", "member chain"),
    "binary_expression": ("core", "infix atom runs"),
    "unary_expression": ("core", "prefix ops (`!x`)",
                         ), 
    "update_expression": ("sugar", "`i++` → `i+=1` canon"),
    "augmented_assignment_expression": ("core", "`i+=1` core"),
    "assignment_expression": ("core", "`x=e`"),
    "ternary_expression": ("core", "`c ? a : b` (core expr-if)"),
    "parenthesized_expression": ("core", "paren groups"),
    "subscript_expression": ("core", "`x[i]`"),
    "new_expression": ("core", "`new T(..)` verbatim (cs fixtures)"),
    "new": ("trivia", "keyword"),
    "delete": ("trivia", "keyword of a unary-style expr; hand-render `delete styles[prop]` verbatim"),
    "spread_element": ("core", "`...x` (core spread admitted)"),
    "pair": ("core", "objlit member (J1)"),
    "object": ("core", "objlit"),
    "array": ("core", "array literal (type-position here; value form via inline runs)"),
    "template_string": ("sugar", 'string interpolation `"…{e}…"` (§4)'),
    "template_substitution": ("sugar", "interpolated expr"),
    "regex": ("core", "regex literal (lexer 1.2)"),
    "regex_pattern": ("trivia", "regex content"),
    "regex_flags": ("trivia", ""),
    "optional_chain": ("core", "hand render keeps `?.` verbatim (js_01.ir:6)"),
    "as_expression": ("core", "`x as T` cast, hand render verbatim"),
    "as": ("trivia", ""),
    "typeof": ("trivia", "keyword of unary/infix expr; hand render `typeof value === 'function'` verbatim"),
    # --- declarations: interfaces / signatures
    "interface_declaration": ("core", "`type I interface{}` (§2.1)"),
    "interface_body": ("core", "members"),
    "interface": ("trivia", ""),
    "function_signature": ("core", "signature fn (no body)"),
    "required_parameter": ("core", "params `name Type`"),
    "optional_parameter": ("core", "param `Type?` erasure (2.4 gate)"),
    "formal_parameters": ("core", "param list"),
    "property_signature": ("core", "field member"),
    # --- comment
    "comment": ("dropped", "§6.1 (25-40% of Humanizer files — the dominant saving)"),
    # --- type surface (erased to signatures / reconstructed)
    "type_annotation": ("signature", "§3: types only in signatures"),
    "type_arguments": ("signature", "`List<T>`-style generics"),
    "type_identifier": ("signature", "bare idents, never abbreviated"),
    "type_parameter": ("signature", "`fn T.Member<T>` param surface"),
    "type_parameters": ("signature", ""),
    "predefined_type": ("signature", "primitives Int/Bool/String/…"),
    "union_type": ("signature", "`A | B` unions in signatures"),
    "generic_type": ("signature", "parameterized idents"),
    "function_type": ("signature", "callback signatures"),
    "array_type": ("signature", "`Array<T>` / `T[]`"),
    "object_type": ("signature", "interface-shape member → interface"),
    "parenthesized_type": ("signature", ""),
    "literal_type": ("signature", "literal member types"),
    "lookup_type": ("signature", "`SelectOptions['quirksMode']` — alias-resolve or opaque"),
    "template_literal_type": ("signature", "§6.6 — printer reconstruction"),
    "template_type": ("signature", ""),
    "constraint": ("signature", "generic constraints — NOT encoded (§3)"),
    "extends_type_clause": ("signature", "`: Base` clause (§2.1 primary+base)"),
    "boolean": ("signature", "`Bool`"),
    "unknown": ("signature", "type word"),
    "void": ("signature", "type word"),
    "type_predicate": ("signature", "§6.6 printer reconstruction"),
    "type_predicate_annotation": ("signature", ""),
    # --- type aliases: interface-shaped → interface; machinery → opaque type
    "type_alias_declaration": ("opaquetype", "union/template aliases → `opaque type` (js_02.ir:5-9, reconcile row 21); interface-shaped → `type I interface{}`"),
    # --- grammar gap but IR-representable
    "ERROR": ("grammar", "tree-sitter-typescript 0.23.2 lacks `export type * from`; the construct is Core::Reexport (J3)"),
}

BUCKET_ORDER = ["core", "sugar", "signature", "opaquetype", "grammar",
                "unmapped", "dropped", "trivia"]

def fetch_histogram():
    out = subprocess.run([str(REPO / "target" / "debug" / "irc"), "js-kinds",
                          str(SRC)], capture_output=True, text=True)
    if out.returncode != 0:
        sys.exit(f"irc js-kinds failed: {out.stderr}")
    hist = {}
    for line in out.stdout.splitlines():
        m = re.match(r"\s*(\d+)\s{2}(.+)$", line)
        if m:
            hist[m.group(2)] = int(m.group(1))
    return hist


def instance_files(kind):
    """files containing the kind (per-file histograms, for evidence rows)"""
    hits = []
    for f in sorted(SRC.glob("js_*.ts")):
        out = subprocess.run([str(REPO / "target" / "debug" / "irc"),
                              "js-kinds", str(f)], capture_output=True, text=True)
        m = re.search(rf"^\s*(\d+)\s+{re.escape(kind)}\s*$", out.stdout, re.M)
        if m:
            hits.append((f.name, int(m.group(1))))
    return hits


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--md", action="store_true", help="write bench/results-coverage.md")
    args = ap.parse_args()

    hist = fetch_histogram()
    rows = []
    unmapped = []
    buckets = {}
    for kind, n in sorted(hist.items()):
        if kind in VERDICTS:
            b, note = VERDICTS[kind]
        else:
            b, note = "unmapped", ""
            unmapped.append(kind)
        rows.append((kind, n, b, note))
        buckets.setdefault(b, []).append((kind, n))

    def total(b):
        return sum(n for _, n in buckets.get(b, []))

    construct = total("core") + total("sugar") + total("signature") + \
        total("opaquetype") + total("grammar")
    coverage = 100.0 * (construct - total("unmapped")) / construct

    lines = []
    lines.append("# 4.2 — IR-form coverage audit (js corpus)\n")
    lines.append("tree-sitter-typescript CST node kinds over `bench/corpus/real/src/js_*.ts` "
                 "(7 files), joined with the kind→IR verdict table (design doc §2/§3/§6; "
                 "hand renditions are ground truth for verbatim/representable calls).\n")
    lines.append(f"- total CST nodes: **{sum(hist.values())}** over 145 kinds")
    lines.append(f"- constructs the reader must map (excluding trivia + dropped comments): **{construct}**")
    lines.append(f"- **representable: {coverage:.1f}%**"
                 + (f" — unmapped kinds remain ({', '.join(unmapped)})" if unmapped
                    else " — every kind classified"))
    lines.append("")
    lines.append("| bucket | nodes | kinds | reading |")
    lines.append("|---|---:|---:|---|")
    for b in BUCKET_ORDER:
        note = {
            "core": "direct core form / verbatim surface",
            "sugar": "canonicalizes into a core form",
            "signature": "erased to signatures or printer-reconstructed (§3/§6.6)",
            "opaquetype": "`opaque type` capture (§6.5, reconcile row 21) — bounded loss",
            "grammar": "tree-sitter gap; construct IS representable (Core::Reexport)",
            "unmapped": "AUDIT INCOMPLETE — kind has no verdict",
            "dropped": "comments, §6.1 by design",
            "trivia": "syntax atoms, no IR counterpart needed",
        }[b]
        kinds = buckets.get(b, [])
        lines.append(f"| {b} | {sum(n for _, n in kinds)} | {len(kinds)} | {note} |")
    lines.append("")
    if unmapped:
        lines.append("## Unmapped kinds (audit must resolve)\n")
        for k in unmapped:
            lines.append(f"- `{k}` ×{hist[k]} — " +
                         "; ".join(f"{f} ×{n}" for f, n in instance_files(k)))
        lines.append("")
    lines.append("## Evidence for non-core buckets\n")
    for b in ("sugar", "opaquetype", "grammar"):
        rows_b = [r for r in rows if r[2] == b]
        if not rows_b:
            continue
        lines.append(f"### {b}\n")
        for kind, n, _, note in rows_b:
            inst = instance_files(kind)
            inst_txt = ", ".join(f"{f} ×{n}" for f, n in inst) or "—"
            lines.append(f"- `{kind}` ×{n} ({inst_txt}): {note}")
        lines.append("")
    text = "\n".join(lines) + "\n"
    print(text)
    if args.md:
        Path(REPO / "bench" / "results-coverage.md").write_text(text)


if __name__ == "__main__":
    main()