# Plan 4.5 — double round-trip JS → IR → JS (report)

> Phase-4 exit gate (PLAN.md): "double round-trip holds over the js_* corpus".
> Method: `irc from-js <original .ts>` → canon IR → `irc to-js` → compile +
> behavior identity checks against the original sources. Binary =
> `target/debug/irc` at commit `71e8609`+ (4.4); TS 7.0.2 via npx; tree-sitter
> reader 0.23.2. Round-trip artifacts: `bench/corpus/real/rt/` (regenerable —
> two `irc` commands per file). Gates measured 2026-10-10.

## Gates

| gate | command | result |
|---|---|---|
| node --check | `node --check <rt>.js` | **7/7** |
| tsc stricter | `tsc --noEmit --allowJs --noResolve --target es2022 --module esnext <rt>.js` | **7/7** |
| canon identity | `print(read(js)) ≡ print(expand(print(read(js))))` | **7/7** (+30-row construct smoke) — `cargo test --lib roundtrip_tests` |
| behavior identity | `RT_MODE=1 python3 bench/scripts/behavior/runner.py` | **BEHAVIOR-MATCH all 3 subjects** (see below) |

**tsc 7/7 vs the Phase-2 forward pipeline's 3/7** (js_04/js_05/js_06 previously
failed on inline-atom-run statement shapes and the missing re-export row):
the reader classifies statements from the tree-sitter CST (block-lambda
pieces, braceless arms, re-exports, overload drops) so the accepted-v0
emission-time heuristic rows are gone. This flip is the strongest evidence
that 4.4's reader reclassification did the job.

## Behavior identity (3-way: rt-emit == tsc-transformed original == hand-derived expected)

Via the J1 harness (`bench/scripts/behavior/runner.py`, `RT_MODE=1` runs its
emit-source from `rt/` canon instead of the hand renditions — mode added with
this gate; hand-mode re-run after the edit still passes, no regression):

| subject | cases | round-trip emitted vs original | hand-mode (J1) |
|---|---|---|---|
| js_01_util | 22 + domEach side-effect identity | **BEHAVIOR-MATCH** | MATCH |
| js_08_options | 8 (flattenOptions incl. spread/xml/xmlMode) | **BEHAVIOR-MATCH** | MATCH |
| linked_modules (control) | 5 | **BEHAVIOR-MATCH** | MATCH |

Coverage notes: js_03/js_05 carry behavior-bearing logic (flatten/case/getCss)
but need DOM/doc-module context or non-constant side effects to rig — the
same exclusion set as J1 (js_02 type-only); js_06 is a barrel module.

## Diff vs hand renditions (row catalog)

Hand renditions remain ground truth for printer *quality*; the machine canon
differs by **convention** (layout/spelling), by **§8.4 canon-shape truths**,
and by **fidelity drops** (→ `docs/fidelity.md`, task 4.6). No semantic
drift was found that behavior testing didn't already cover.

Conventions (machine canon, stable by identity gate; hand spellings differ):

| row | hand | machine canon | why |
|---|---|---|---|
| params | type-first `String str` | name-first `str String` | printer `{n} {t}`; types erase at emit either way |
| decl keyword | `const` kept | `var` (mut over-label for `let`) | frozen grammar: `var [mut] ident` |
| tail expressions | bare (`array`) | explicit `return array` | IR value semantics are reader-visible |
| arrow block bodies | multi-line children | glued statement-pieced run `=>{stmt stmt}` | same atoms the J2 emission-side splitter consumes |
| `as`-union spacing | `T \| null \| undefined` | `T>|null \|undefined` | tight unions + frozen keyword-space rule (`null` spaces) |
| re-export paths | normalized `'cheerio'` | verbatim `'./cheerio.js'` | reader preserves the import specifier |

Fidelity drops visible in the diff (canonical → `docs/fidelity.md`):

- `@ns` dropped (reader cannot infer a namespace; hand added `@ns Cheerio`)
- fn type parameters dropped (`isCheerio<T>`, `domEach<T extends…>` → bare)
- overload declaration signatures dropped (hand js_05 notes the collapse)
- function-typed annotations erased (`fn: (T elem, Int index) => void` → `fn`)
- interface kind word / `: bases` / name-generics dropped by the canon print
  (`type X interface : A, B{` → `type X export{`) — **existing printer
  surface row** (symmetric on re-print, so identity holds; the C# fixtures
  ship the same drop)
- `export type * from` prints `export * from` (star row keeps no `type` word)
- alias type-params (`BasicAcceptedElems<T extends AnyNode>`) drop

Reader-side additions this gate surfaced (both fixed, both additive):

1. `alias_body_atoms` handled `array_type`/`lookup_type`/`object_type` leaves
   via `type_text` — an as-union like `content as AnyNode | AnyNode[] |
   Document` previously opaque-captured the array member, which then emitted
   a string literal and tripped tsc. After the fix the union spells tight
   (`AnyNode|Array<AnyNode>|Document`) and the whole `as`-type erases at
   emit → **tsc green**.
2. `runner.py` gained the `RT_MODE=1` emit-source switch + per-mode stub
   sets (rt canon derives `@use` aliases from import specifiers:
   `./cheerio.js` → `js`, `./parse5-htmlparser2-tree-adapter` →
   `parse5htmlparser2treeadapter`). Hand-mode output unchanged.

## Verdict

**Phase 4 exit criteria met**: the double round-trip holds over the js
corpus — JS → IR canon → JS compiles (node + tsc 7/7) and behaves
identically on every riggable subject (3/3 through the J1 harness canon).
Canon print is identity-stable corpus-wide, so round-trip N + 2 collapses
onto round N's text. Actual fidelity budget documented separately
(`docs/fidelity.md`, 4.6).