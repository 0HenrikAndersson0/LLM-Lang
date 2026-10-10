# Actual fidelity budget (PLAN 4.6)

> PLAN.md Phase 4 calls this the "actual §5 budget"; the round-trip contract
> itself is §6 of language-design.md (7 ordered drops + double-round-trip
> acceptance). This document records what was **dropped in practice** by the
> shipped JS reader (`js_read.rs`) — evidenced by the 4.4 canon-identity gate
> (`bench/results-round-trip.md`, all 7 js_*.ts files) — and what held.
> Rows marked ✅ hold with no loss; ◐ bounded (behavior-neutral but
> information-losing); ⊘ dropped by contract.

## What holds (✅)

- **Statements, expressions, calls, assignments, control flow** — the full
  core-form surface: if/else-if/else (incl. braceless arms), while, for
  (range canon, for-in, non-range C-style heads verbatim), return/throw
  (incl. bare), try/catch/finally, ternaries, optional chains (`x?.y`, and
  `x?.[i]`), `new `, `typeof`, `delete`, `!=`/`!==`/`===` operators (canon
  prints `!==` with a leading space — the frozen `x!` suffix rule), spread,
  object/array literals, templates → interpolation, `i++` → `+=1`, regex
  literals, function expressions → arrows.
- **Identifiers** — never abbreviated (readability floor).
- **Imports** — `@use` aliases (last specifier segment); re-exports incl.
  `export type {…} from`, `export {…} from`, `export * from`.
- **Classes/interfaces** — members, methods, getters, heritage (into
  `TypeDecl.bases`), interface property/method signatures (`.d.ts` surfaces).
- **`as const`, enum-of-literals unions, template-literal type machinery** —
  verbatim `opaque type` atom runs (reconcile row 21; the printer re-emits
  captured text byte-identically; print→print identity includes captured
  rows — js_02 canon rows match the hand render modulo spacing).

## Bounded losses (◐ — recorded rows, emit-side behavior is preserved)

| # | what | practice | where |
|---|---|---|---|
| 1 | Comments | dropped entirely — by contract (§6.1), measured 25–40% of Humanizer/cheerio sources | reader skips `comment` nodes |
| 2 | Formatting/whitespace | canonicalized (§5 layout) — canon print is identity-stable across N+1 reparses | printer |
| 3 | Locals' explicit types | dropped (`var` canon); param/ret signatures erased at emit (`: T` never materializes in JS) | js.rs emitter |
| 4 | fn declaration type parameters | `isCheerio<T>` → `isCheerio` — the expander skips `<…>` on fn re-parse; keeping them would break canon identity | js_read fn_decl, §8.4 |
| 5 | Overload sets | declaration-only `function_signature`s drop; the implementation fn carries the callable shape | hand js_05.ir:3 note |
| 6 | Function-typed annotations | erased to bare params (`fn: (T elem, Int index) => void` → `fn`) — spellings need a grammar change; types are erased at emit anyway | type_text/function_type |
| 7 | Type predicates (`x is T` ret) | erase to `Bool` (hand canon js_01.ir:5 convention) | ret_text |
| 8 | Interface `?` optionals | canon `Bool? xmlMode` — the `?` rides the type; erased at emit (2.4 fix) | member_field |
| 9 | Private members (`#p`) | opaque-captured member row `"opaque:…"`. Canon-stable + compiles; **emit currently stringifies the capture instead of re-emitting verbatim** — emit-side passthrough gap, queued for the Phase-5 emit work | §8.1.3c, print_member |
| 10 | `@ns` | dropped — no ESM equivalent; reader cannot infer | header_core |
| 11 | Interface kind word, `: bases`, name-generics in canon print | `type X interface : A, B{` prints `type X export{` — **existing printer surface row** (symmetric drop; re-print stable; C# canon ships the same). TypeDecl keeps the data in-memory | print_type |
| 12 | `export type * from` | canon `export * from` (star row keeps no `type` word) | printer Reexport |
| 13 | Alias type-parameters | `BasicAcceptedElems<T extends AnyNode> = ...` → `opaque type BasicAcceptedElems = ...` (param name `T` survives inside the captured text) | alias_decl |
| 14 | Value-using postfix `i++` | canon `i+=1` — value-determining usages (`x = i++`) would shift by one; corpus has none (statement-tail only) | §8.2 sugar note |
| 15 | Destructured bindings | `{a, b} = obj` / array patterns → opaque var rows (`opaque:const {a, b} = …`) — canon-stable, emit verbatim-through-v0 | top_vars |
| 16 | Non-range C-style for heads | verbatim head atoms (`for(let x=…; …; …)`) — reparse-stable; emit splits the run statements | read_for |

## Nothing else was needed

The corpus's remaining constructs (arrow sugar, obj-lit + spread, chained
calls with lambdas, lookup types, `as`-erasure, `?.`, regex with flags,
template interpolation) map losslessly through the §8.2 mapping table — the
4.2 coverage audit's 100.0% representable claim held: six of the seven files
carry **zero** opaque captures; js_02 (type-alias module) peaks at 8 captured
`opaque type` rows, 2 of which contain an inline function-typed member
(`opaque:` atom) inside the union text — all within the §6 contract's
capture design, none a statement or expression.

## Phase-5 addendum (2026-10-10) — C# round-trip rows

The 4.5 JS budget above was JS-only. The C# reader (`cs_read.rs`) adds:

| # | row | practice |
|---|---|---|
| 17 | **char literals vs string literals** | ✅ **RESOLVED 2026-10-10 (the @2 admission)**: canon v1.1 gained `Atom::Char` (single-quoted 1-char; double-quoted 1-char stays Str — 1-char JS strings are meaningful). The C# emit restores `'X'`; JS emit renders `"X"` (JSON string; JS has no chars). Token-neutral (probe: ±1 over 36 swaps — `bench/results-char-atom.md`). The 8 char-family emit rows cleared; the round-trip corpus re-emits with zero non-stub diagnostics |
| 18 | accessibility nuance | canon keeps `export` (public) only; `internal`/`private`/`protected` collapse — the C# emitter emits `internal` for non-export members so multi-file cross-calls in the corpus assembly resolve as the originals do (`cs_06 → cs_08.DataUnitHumanizeExact`) |
| 19 | `where` generic constraints | dropped (contract); the C# emitter emits type params without them |
| 20 | data-braces vs interpolation in C# strings | **distinguished at the reader** (interpolated_string vs plain literal): plain literals canon-escape `{{…}}`, interpolation canon's `x{e}y`; the C# emit restores literal braces — *this one is preserved*, recorded because JS canon cannot make the same distinction |
| 21 | `<rest-omitted>` hand-canon elision rows | cs_08's hand rendition elided 2 long bodies — the machine canon re-reads the full source, so these exist only in the hand fixture |
| 22 | `#if` preprocessor regions | captured verbatim (`opaque:` atom rows in the canon); the C# emit flags them `/* opaque */` — the NET8_OR_GREATER guards pass textually, not semantically |

The JS-side rows 1–16 carry over unchanged.
**Acceptance (JS leg):** double round-trip (JS → IR → JS) compiles
(node --check 7/7, tsc 7/7) and behaves identically on every riggable
subject (3/3 harness subjects) — the §6 contract holds with the budget
above. **C# leg (5.6 leg-1):** C# → IR canon → C# re-emits syntax-clean
9/9 (`bench/results-cs-diff.md`); the behavior leg runs behind the
library-context rig.
