# Task 2.4 — JS output vs hand renditions (diff report)

> Footnote [2] in PLAN.md: hand renditions (0.5) are ground truth for printer
> quality when transpiler output doesn't compile — this report isolates
> transpiler bugs from design bugs. Gates measured 2026-10-09, TS 7.0.2 via
> `npx -p typescript`, binary = `target/debug/irc` at commit 053f11f+.

## Gates

- `node --check`: **26/26** (10 golden + 16 real corpus) — Phase-2 exit gate met.
- `tsc --noEmit --allowJs --noResolve --target es2022 --module esnext`
  (stricter evidence gate; `--noResolve` because v0 `@use P` lowers to
  side-effect `import "./P.js"` which doesn't resolve standalone):
  - golden 10/10 ✓, cs corpus 7/9 ✓ (cs_05, cs_06 fail on C#-isms — by design)
  - js corpus (originally TS sources) **3/7** ✓ (js_01, js_02, js_08) — the
    4 failures are fully attributed below.

Fix shipped with the gate: TS optional-param `?` now erased in param lists
(frozen design: types only in signatures, "which JS doesn't carry" — §1;
`function css(prop?, val?)` previously passed `?` through to JS and tripped
TS8009). After the fix js_08 passes.

## Gap catalog (per failing file, root-cause tagged)

**Tag legend:** ✍️ = hand-rendition drift (0.5 render spells something outside
frozen grammar v1.1); ⚙️ = v0 emitter gap (row missing; fixable additively);
📐 = design limit (verbatim atom-run expression layer cannot lower this; the
guide and fidelity budget already say so); 🔀 = C#-only construct — native
target is Phase 3.

| file | evidence (emitted line) | tag | verdict |
|---|---|---|---|
| js_03_parse:21 | `if (parent{parent.children=arr}else) { parent=null; }` — an if-EXPRESSION inside an inline atom run | ✍️📐 | rendition chose control flow in value position; emitter passes unknown shapes verbatim (design §2). Correct idiom = ternary. |
| js_04_forms:17 | `.map<AnyNode, {name: String, value: String}>(…)` | ✍️📐 | generics/annotations surviving inside expr runs; also multi-statement fused inline lambda bodies (`{var x=…if c{…}d()}`) — needs statement-level inline parsing (v0 boundary; 4.4 target-reader will classify properly) |
| js_05_css:7 | `return domEach(this, (el, i)=>{if isTag(el){setCss(…)}});` — if-statement inside a lambda inline body | 📐 | same inline-atom-run family; guide teaches explicit separate statements |
| js_05_css:50 | `let key: String \| undefined;` | ✍️ | bare `let` is not in grammar v1.1 (`[const\|volatile] var [mut]`) — line passes through as a verbatim expr; the annotation rides along. Fixture drift, not emitter bug. |
| js_06_slim:7+ | `const type;` ×3 (redeclare) — from `export type {…} from 'load'` lines; `const load;` (init fell into the objlit path) | ⚙️ | real emitter gap: **no re-export lowering row** (`export {…} from` / `export type * from`). Deferred to Phase 4 (4.4 target-reader/4.3 mapping table) — fixing now would churn canon classification; recorded in js-target.md §7. |
| cs_05/06 | `Math.Pow(10, Limit)` casing, `.HasValue`/`.Value`/`HasFlag`, `new Dictionary<Char, UnitPrefix>` generics-in-expr, `nameof(x)`→`"x"`, `out var` | 🔀 | C#-corpus patterns stay C#-side (js-target §3+§4 notes); Phase 3 is the native target |

## Verified-working lowering (spot checks vs hand renditions)

`: T?`/erasures, decimal suffix (`100m`→`100`), `{interp}`→template literal,
`.sum()`→`.reduce((a,b)=>a+b,0)`, `.toList()`→`[...x]`, `..`-range for →
C-style-for, `[^i]`→`.at(-i)`, `[.. x]`→`[...x]`, throw-ternary → IIFE,
`is`-patterns → `instanceof`/comparison chains, try/catch → instanceof-guard
chain + rethrow, `lock{…}` → body inlined (noted), receiver fns → free
functions (noted), `interface` → `/** @interface X */` + empty class,
`opaque{…}` members → verbatim passthrough, fn-tail bare exprs → `return`.

## Verdict

Phase-2 exit criteria (PLAN §Phase 2): node `--check` gate **met 26/26**;
tsc stricter gate recorded with **every failure root-caused** — no model-
rejection and no grammar failure in the set; the only true emitter gap
(re-exports) is scheduled for Phase 4 tooling where it can land additively.
LLM loop test (2.7): GO. **Phase 2 exit: met.**

## J1 behavior gate (v4 Phase 3, added post-plan-rewrite)

`bench/scripts/behavior/runner.py` (harness), gate report
`bench/scripts/behavior/report.md`. Method: emit IR→JS; transform the
original `.ts` with tsc (type-only imports erase — originals run standalone);
rig calls both sides with identical inputs; three-way compare
emitted==orig==hand-derived-expected.

| subject | cases | result |
|---|---|---|
| js_01_util | 22 + 1 side-effect identity (domEach) | **BEHAVIOR-MATCH** |
| js_08_options | 8 (flattenOptions incl. spread/xml/xmlMode) | **BEHAVIOR-MATCH** |

Found by J1 (and fixed additively with the gate): **obj-lit initializers
died on every path** — single-line `var x = {…}` lost its value entirely
(reader R3 hijacked the `{…}` pair into `inline_body`; the var branch never
read it — expander now re-wraps the pair as the initializer) and block
objlits (`var x: T = {` + member lines + `}`) hit a decl-only arm that
pre-empted the objlit arm (dead `(_, Some)` arm reordered). Corpus-wide,
`js_08`'s `defaultOpts`/`opts` now initialize; regression test
`objlit_initializers_survive` in tests/spans.rs.

The rig's NOTE row also caught *expectation drift* in my own hand-derived
cases (`... options` spreads kept `xml`/`xmlMode` keys — the derivation
missed them) while emit==orig held — the third check separating harness
from emitter worked as intended.

Coverage limits (explicit): js_03/js_04/js_05/js_06 need either named
imports (J4) or a DOM-ish `this` context — excluded from v1; js_02 is
type-only (nothing runs). Extending coverage is part of J2/J4.

## Next

- Phase 3 design note (3.1: C# AST + printer; `dotnet 10` available locally),
  with the receiver-fn lowering-note debt paid at the C# target.
- Phase 4's tree-sitter reader will reclassify re-exports and inline-lambda
  statement bodies (the ⚙️/📐 rows above).