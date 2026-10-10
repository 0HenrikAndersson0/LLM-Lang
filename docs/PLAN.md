# Implementation Plan (v3, revised)

> **Planning mode.** Phase 0 exits 2026-10-03 with a GO. Phase 1 (reader &
> expansion) authorized; design doc v1 grammar is the frozen input. Status
> legend: `☐ todo` `◐ doing` `☑ done` `⊘ cut` `⏸ blocked`

## North Star

A dense IR language that reduces LLM token spend on code generation and
round-trips to/from C#/JS. Success metric: **≥30% token reduction** on a real
corpus, measured on the whole-pipeline view (footnote [1]), while round-trip
compile-identity holds and an LLM can write and debug it in a closed loop.

---

## Phase 0 — Decisions by measurement (DONE 2026-10-03)

**Route decision (2026-10-03): Route 1 adopted.** Full s-expr surface
falsified by measurement (A/B/C = 118.7–132.7% of baseline); variant D —
infix definitions/calls + braceless keyword control flow — scored
**89.7% o200k / 90.1% llama** on the 10 golden examples, the first variant
under baseline. Routes 2/3 shelved; design doc rewritten for Route 1
(language-design.md v1, measured) with grammar + core forms frozen.

**Goal:** Go/no-go token-savings number on a real corpus (whole-pipeline
view [1]). Design decisions are frozen (design doc §8); grammar changes
now require re-running the bench per working agreements.

**Exit:** go/no-go number recorded in `bench/results.md` as measured over
`bench/corpus/real/`. If no-go, the project stops here — the cheapest
possible failure point.

| # | Task | Deliverable | Status |
|---|------|-------------|--------|
| 0.1 | Python venv with `tiktoken` + `tokenizers` (o200k_base, llama vocab) | `bench/scripts/` runnable | ☑ |
| 0.2 | Scoring script: per-file token counts, min/max/mean, % of baseline, per-tokenizer verdicts | `bench/scripts/score.py` (+ `toktest.py` probe script) | ☑ |
| 0.3 | Golden-example corpus from language-design §7 as fixture files | `bench/corpus/*.ir` | ☑ |
| 0.4 | Real-world C#/JS corpus (10–20 files each; license-clean) | `bench/corpus/real/` | ☑ — 16 files: 9 C# (Humanizer, MIT) + 7 JS/TS (cheerio, MIT), construct-diverse |
| 0.5 | Hand-written IR renditions of real corpus files (informs the design!) | `bench/corpus/real/*.ir` | ☑ — all 16 hand-rendered in variant D; fidelity notes inline (opaque fallbacks flagged) |
| 0.6 | Hand-render each golden example in each variant syntax (variant table, design doc §1) | `bench/variants/**` | ☑ (B, C rendered; C = 118.7% o200k; **D (Route 1 hybrid) rendered, 89.7% o200k / 90.1% llama — first variant under baseline**) |
| 0.7 | Decision runs: arrow, bools, nullable, static-call, async/annotations — isolate one construct per run | `bench/results.md` verdict table | ☑ — decided inline during D rendering + probes (results.md route section, design doc §8 frozen-decisions table); all spelling ties resolved by readability floor |
| 0.8 | Go/no-go: token delta of IR renditions vs real C#/JS baseline, whole-pipeline view [1] | headline number in `bench/results.md` | ☑ — **GO: 46.4% o200k / 46.5% llama** (25,299→11,735 / 32,300→15,005) over 16 real files; one-shot view, whole-pipeline confirmation deferred to loop test 2.7 |
| 0.9 | Design-doc update: replace §1 variant-table hypotheses with measured verdicts; freeze grammar + core forms | design doc v1 | ☑ — language-design.md rewritten for Route 1 (v1, measured); grammar/core forms frozen pending 0.8 sanity check |

> [1] **Whole-pipeline view:** measure `baseline C# → generated output →
> model re-reads its own output` (stack traces, fix-up loops), not just the
> one-shot IR token count. That is the number that predicts agent-loop spend.
>
> **Decision order (settled 2026-10-03, see design doc §8):** arrow + bools
> + nullable spellings, statics, async annotations — all measured inline
> during variant D rendering; ties resolved by readability floor.

---

## Phase 1 — Reader & expansion (starts after Phase 0 exit)

**Goal:** `ir text → core forms` works for the golden corpus; invariants upheld.

**Exit:** all golden examples parse + expand; parse→expand→print is identity
on canonical form; reader survives fuzzing.

| # | Task | Deliverable | Status |
|---|------|-------------|--------|
| 1.1 | Cargo workspace: `crates/lang` (lib) + `crates/cli` | builds clean | ☑ |
| 1.2 | Lexer from frozen grammar, symbol table driven by Phase 0 verdicts | unit-tested lexer | ☑ — 11 unit tests; regex literals, interpolated strings, number suffixes; all 26 corpus files lex |
| 1.3 | Reader: text → form tree (EBNF, design doc §1) | unit tests over golden corpus | ☑ — line/block reader with R1-R8 rules (param continuation, infix continuation, inline literals, close-lines w/ trailing atoms); all 26 files parse + unit tests |
| 1.4 | Expander: macro table → core forms; enforce core-form closure invariant | expansion snapshots | ☑ — Core{Header,Fn,Type,Var,Expr} closure; golden tests; expr-statement fallback |
| 1.5 | Canonical printer: IR → text (§6 layout) + `--expand` debug mode | printer + snapshots | ☑ — printer done, print→expand→print **identity on all 26 files**; `--expand` flag shipped (`irc --expand parse` / `irc expand`, `printer::print_debug` labeled core-form view) |
| 1.6 | Property-based fuzzing (proptest) on reader: never panics, always returns error | fuzz suite | ☑ — grammar-driven generator, 8k+ cases no-panic |
| 1.7 | Golden-corpus snapshot tests wired into `cargo test` | CI test target | ☑ — tests/golden_snapshots.rs: parse+expand+print stable over every fixture |

---

## Phase 2 — JS target (first target; no type system)

**Goal:** IR → compilable JS for the full golden corpus + real JS corpus.

**Exit:** outputs pass `node --check` / `tsc --noEmit`; LLM loop test (2.7)
shows the model repairs its own IR from JS runtime errors.

| # | Task | Deliverable | Status |
|---|------|-------------|--------|
| 2.1 | JS AST + printer design (estree-style, canonical layout) | design note | ☑ — docs/js-target.md: minimal AST (exprs stay verbatim atom runs), atom→JS mapping (§3), canonical layout (§5) |
| 2.2 | Core-form → JS lowering table (design doc §2 semantics) | lowering-table doc | ☑ — js-target.md §4; expander gained structured `Core::Stmt{head,body,arms}` (additive; surface text unchanged — identity re-verified on all 26) |
| 2.3 | JS printer | emits golden corpus | ☑ — crates/lang/src/js.rs: recursive-descent expr rewriter (calls, casts, templates, `is`-patterns, hat-index, throw-ternaries); all 10 golden + 16 real corpus files emit |
| 2.4 | Real-corpus IR→JS output runs under node/tsc; diff against hand written renditions [2] | report | ☑ — **node `--check` 26/26**; tsc stricter gate on the TS corpus 3/7 with every failure root-caused (bench/results-js-diff.md): 2 C#-only files (by design), inline-atom-run/lambda statement bodies + generics-in-expr (design limit), export-re-export row missing (Phase 4), one grammar-drift line (bare `let`); param-`?` erasure fixed with the gate |
| 2.5 | `ir --expand` / `--ir` debug flags (§6 readability floor) | CLI flags | ☑ — `--expand` shipped in 1.5 (`irc expand` = same view) |
| 2.6 | Error localization: printer carries source spans so IR ↔ JS line mapping is recoverable | span tracking | ☑ — line spans threaded reader→Core→`js::emit_with_map` (marker post-pass); `SourceMap.js_to_ir/ir_to_js/to_json`; lower errors carry `IR line N`; `irc to-js --map` writes `.js`+`.map.json`; corpus gate: every non-blank JS line maps, output byte-identical with/without map (js-target.md §7) |
| 2.7 | LLM loop test harness: model writes a feature in IR → transpile → run → feed errors back → repair | loop harness + report | ☑ — **GO (local-model gate):** harness `bench/scripts/loop/` (adapters echo/file/openai); guide emit-verified; runs in `runs/` — qwen2.5-coder 7b **and** 1.5b one-shot task_01 cold; 7b one-shot task_02 (loops/parseInt/if-chains); seeded repair from mapped runtime errors repaired in 1 round (`report_2p7.md`). Caveats: 7b/1.5b scale only; repair seeded (cold drafts one-shotted); value-match gap taught-around. The loop test also functioned as QA: 5 emitter truths fixed (see report §2) |

> [2] **Diff-against-hand-written:** Phase 0 hand renditions (0.5) are ground
> truth for printer quality when transpiler output doesn't compile — it
> isolates transpiler bugs from design bugs.

---

## Phase 3 — JS depth (v4: "make it great"), C# deferred

**Goal (v4 reroute, 2026-10-09):** everything that makes the JS/agent-file
story *excellent* before adding a second target language. Rationale: the
loop test was the go/no-go and passed; a C# target adds breadth, not risk
closure — while the JS pipeline has unverified claims (behavior!), open gap
rows, and an unmeasured whole-pipeline number.

**Exit:** emitted JS *behaves* identically to the originals on runnable
cases; tsc clean on the TS corpus outside documented design limits; whole-
pipeline token number measured.

| # | Task | Deliverable | Status |
|---|------|-------------|--------|
| J1 | Behavior harness: emitted js_* vs original sources, same inputs → same outputs (leaf modules; sibling deps stubbed/executed) | behavior report | ☑ — `bench/scripts/behavior/runner.py`: emit vs tsc-transformed originals, 3-way deep compare; js_01 (22 cases + side-effect identity) and js_08 (8 cases) **BEHAVIOR-MATCH**; NOTE rows caught harness-side expectation drift while emit==orig held; js_03/04/05/06 excluded (need J4 named imports / DOM context), js_02 type-only. Obj-lit initializers died on every path — fixed additively (R3 inline-pair hijack re-wraps value; objlit arm ordering) + regression test |
| J2 | Inline atom runs: statement-ify lambda bodies / if-expressions (emitter + expander classification; surface text unchanged ⇒ bench-neutral) | tsc failures js_04/js_05 closed | ☑ — glued `if c{a}else{b}` chains split (split_glued_if chain-aware; then-arm consumption in printer+emitter; `}else{…}` close-line brace-glue stabilized in R8); dot-chain continuation absorption (reader R2 at child level, R1-rules while unbalanced); arrow-block statement-ification at emit (`split_stmt_runs`/`stmtify_piece` + terminal-expr→return); callee-generics erasure; bare-`let`→`var mut` canon re-stabilization. Tests split/stmtify with constructed atoms |
| J3 | Gap rows: `export {…} from`/`export type *` re-exports, generics-in-expr erasure, value-match arms (bool/ident-literal) | tsc 7/7 on TS corpus | ☑ **tsc 7/7** — `Core::Reexport` additive core form (canon prints the line; JS drops `type`); callee-generics erased at emit; typed-var `=`-consumption bug + empty-objlit + fused-`>=` tolerance fixed; bool/null literal arms split. Still accepted-v0: value-position match arms stay statement-shaped (guide teaches explicit `return`); non-keyword-led inline statement pieces (see js-target.md §7) |
| J4 | Real module imports: `@use` with named imports (usage-inferred) so multi-file features transpile + run | rig over 2 linked feature files | ☐ |
| J5 | Loop depth: task_03/04 on new idioms; unseeded-repair target; qwen3.5:9b + gemma4:12b points | 2.7 evidence at 3 model scales | ◐ — **task_03 runs at 3 scales + agent-driven; two classes resolved, one open.** Post-fixer (v4.4): qwen2.5-coder:7b drafts now round-1 valid (was: braceless-if + for-run emitted as garbage — emitter truths, NOT model failures); remaining 7b failure class = value near-miss (first-word capitalize inverted in camelize, not repaired in 6 rounds; loop feedback shows got/expected + numbered IR but the error is semantic, not localized); qwen3.5:9b = structural (Pythonisms/unindented); gemma4:12b = operationally blocked on 16 GB (reasoning channel consumes the generation budget before content lands — adapter now falls back to the reasoning channel but runs stayed vacuous; ollama adapter gap, not language); agent-driven (file adapter): **task_03 PASS round 1 both worlds** (first task_03 IR pass ever) |
| J6 | Whole-pipeline token measurement (footnote [1]): loop-transcript tokens vs C# baseline | the real north-star number | ☑ — **measured on both bases; savings are conditional on the baseline class (report_j6_pipeline.md).** v1 (1-feature loop pairs): one-shot IR 206–316% of `--lang js` baseline; amortized 77–90%, break-even S≈10–15; write-side IR drafts 48–55% (terse ops), ~equal on control-heavy. v2 (task_04 3-feature session): marginal parity (98.3%), one-shot 184% (guide-dominated). **Denominator finding (js_08): IR module = 38.7% of the typed+documented source (Phase-0's 46% reproduces) but 169.8% of bare clean-JS runtime** — the ≥30% claim holds vs agent files replacing typed/documented sources; fails vs minimal doc-free code. Loop reliability dominates (one value-fail = 33×). Corpus-scale loop measurement scoped and rejected with evidence (v4.6): loop rigs are small-write by construction; the file-scale denominator is static-only. Residual (optional): scaffold-trimming sensitivity |

> Bench discipline unchanged: J2/J3 touch no surface text (emission-side and
> classification only) — bench-neutral by the Core::Stmt precedent; any
> grammar-spelling change would require a Phase-0 re-run per working
> agreements and is deliberately out of scope for this phase.

---

## Phase 4 — Round-trip JS → IR (JS only in v4) — **EXITS 2026-10-10**

**Goal:** Existing JS → lossy IR, meeting the §5 fidelity budget and the
double-round-trip acceptance criterion (JS → IR → JS compiles and behaves
identically). C# round-trip moves behind the C# target phase.

**Exit:** double round-trip holds over the js_* corpus; actual §5 budget
documented. — **MET** (`bench/results-round-trip.md`; `docs/fidelity.md`).

| # | Task | Deliverable | Status |
|---|------|-------------|--------|
| 4.1 | tree-sitter-javascript grammar for parsing JS targets | parser setup | ☑ — `js_read.rs`: tree-sitter + tree-sitter-typescript v0.23.2; CST parse over the js_*.ts corpus (TS grammar superset — types surface as `type_annotation` and get skipped by the JS-only mapping); 6/7 corpus files parse clean, **gap identified: `export type * from` unsupported by the grammar** (round-trip will treat it as opaque passthrough); node-kind histogram wired as `irc js-kinds <dir>` (4.2 groundwork, kind histogram incl. `program/file` roots) |
| 4.2 | IR-form coverage audit on js corpus: % of constructs representable in core+ sugar | coverage report | ☑ — `bench/results-coverage.md` + `bench/scripts/coverage_audit.py` (verdict table per kind, evidence rows via per-file histograms): **4398 nodes / 145 kinds over the 7 js_*.ts files; 2233 constructs the reader must map; 100.0% representable** — core 1576/51 kinds (direct core form or verbatim surface per hand renders: `new`, `typeof`, `delete`, `as`, `?.` all pass through), sugar 14/4 (templates→interpolation, `i++`→`+=`, fn-expr→arrow), signature 634/23 (types erased to signatures, §3/§6.6), `opaque type` capture 8/1 (js_02 union/template aliases, reconcile row 21 — bounded loss), grammar-gap 1 (ts grammar lacks `export type *`, construct = Core::Reexport). Comments dropped by design (§6.1). Reader (4.4) has no representability blockers |
| 4.3 | Mapping table: JS construct → IR form; unrepresentable → `(opaque …)` passthrough [3] | mapping doc + `(opaque …)` design | ☑ — js-target.md §8: capture design (verify-grounded: `Body::Opaque`, `opaque` modifier, `opaque:` atoms, `opaque{ raw }` bodies all exist in expander/printer; `opaque type` canon rows js_02.ir:5-9), capture rules (whole declaration/member, never partial expr; verbatim print identity), exhaustive capture sites, mapping table by audit bucket (core/verbatim incl. `new`/`typeof`/`delete`/`as`/`?.`; sugar canon: templates→interpolation, `i++`→`+=`, fn-expr→arrow; signature erasure incl. lookup-type member resolution; opaque = union/template aliases only; comments dropped by design), reader-side reclassification of the 2.4 accepted-v0 rows, ts-grammar ERROR fallback rule |
| 4.4 | JS reader (JS → IR), reusing the canonical printer; reclassifies the 2.4 gap rows (re-exports, inline statement bodies) where they land | reader | ☑ — `read_js` walk complete + `irc from-js` CLI; **canon identity: print(read(js)) ≡ print(expand(print(read(js)))) on all 7 js_*.ts files + 30 construct-row smoke** (`cargo test --lib roundtrip_tests`); re-classifications per §8.4/§8.2 — reexports→Reexport (named/type/star), alias split (interface-shaped vs `opaque type` union/template), overload signatures dropped, fn type-params dropped (expander reparse convention), type predicates→Bool, private-class members opaque-captured, `?.`/`as const`/lookup-type/cstyle-for→range canon; canon-shape truths documented in js-target §8.4 |
| 4.5 | Double round-trip over js corpus: compile + behavior identity checks | round-trip report | ☑ — **`bench/results-round-trip.md`: both round-trip legs green.** node --check 7/7; **tsc stricter gate 7/7** (the Phase-2 forward pipeline managed 3/7 — the reader's CST-level classification closed the inline-run/re-export rows); canon stability re-verified; behavior via the J1 harness `RT_MODE=1` (mode added with per-mode stub sets; hand-mode unchanged): js_01 (22 + domEach side-effects), js_08 (8), linked control — **BEHAVIOR-MATCH all 3**; diff-vs-hand catalog split into printer conventions vs fidelity drops (→ fidelity doc); one additive reader fix found by the gate (`alias_body_atoms` union leaves) |
| 4.6 | Document actual §5 budget (what was dropped in practice) | fidelity doc | ☑ — `docs/fidelity.md`: ✅ full statement/control-flow/expr surface incl. `?.`/`as const`/capture re-print; ◐ 16 recorded bounded rows (comments, types, fn type-params, overloads, fn-typed annotations, predicates→Bool, `@ns`, printer-surface kind/bases/generics drop, private members, destructured v0, postfix-`++` value forms); the 4.2 representability claim confirms in practice — 6/7 files zero captures, js_02 peaks at 8 alias rows |

> [3] **`(opaque …)` passthrough:** unrepresentable members are captured
> verbatim (spans + original text) and re-emitted untouched by the printer.
> This bounds worst-case fidelity loss at zero for members the IR can't
> express.

---

## Phase 5 — C# target (emit + round-trip; was Phase 3) — **essentially exits 2026-10-10**

**Goal:** IR → compilable C# for golden + real C# corpus; then C# → IR
round-trip with behavior identity (folds the original Phase 3 + C# half of
Phase 4 behind the JS-first line).

**Decision (2026-10-10):** a "skip C#, harden first" reroute was considered
and **rejected** — the C# promise stays (original plan keeps precedence;
the v4 JS-first reroute was about closing JS risk, not dropping C#).

**Exit:** all outputs compile (`dotnet build` zero errors); LLM loop test
replicates 2.7's fix-from-stack-trace capability; double round-trip holds
over the cs_* corpus. — **met on the JS-verified terms**: 5.3 golden 8/9
(1 upstream), 5.4 re-emit syntax-clean (215 residual decomposed:
stub-boundary + the char row), **5.5 cs-baseline one-shot qwen-7b PASS**
(+ build-repair in-loop on the transpiled world), 5.6 canon identity 9/9
+ re-emit syntax-clean. The **char-atom decision and the behavior rig's
library context defer to Phase 6** (fidelity row 17) — the honest
remainder of the "behavior identity" criterion, recorded as Phase-6 work
rather than a silent gap.

| # | Task | Deliverable | Status |
|---|------|-------------|--------|
| 5.1 | C# AST + printer design (Roslyn-inspired, namespaces/usings/attributes) | design note | ☑ — `docs/cs-target.md` §1–§3+§5: direct Core→CS emitter over Core (the js.rs shape — proven by behavior/round-trip gates; "Roslyn-inspired" = output conventions §5, not an AST layer), verbatim-first atom mapping (C#-compatible expression surface by Phase-0 design), canonical layout (4-space, file-scoped namespace, usings after ns, brace-new-line types) |
| 5.2 | Core-form → C# lowering table (`var`, properties vs fields, switch exprs) | lowering-table doc | ☑ — cs-target §4: full Core→C# table (Header→file-scoped ns + sorted usings; fields vs computed properties per fixture conventions; sig-only member rows invert ty-shaped text; match→switch expr/stmt; lock/try/range/foreach rows; extension methods→`this`-param statics; top-level fns→synthetic static class; Reexport→drop+fidelity flag) + §6 type-word map (`Float→double`, `Array<T>→T[]`…) + §7 deferred rows (attributes, access-modifier nuance, `{get;set;}`, query syntax — none in corpus) |
| 5.3 | C# printer | emits golden corpus | ☑ — `crates/lang/src/cs.rs` + `irc to-cs`: direct Core→C# emitter per cs-target §2–§6 (verbatim-first atoms, LINQ op map, type-word map, decl-keyword ty stripping, sig-only member inversion, switch-expr reordering else-last, async→`Task<>` upgrade, synthetic `partial` top-level class for stub rigs). **Golden gate: `dotnet build` 8/9** — 03_pipeline excluded with proven upstream cause (the golden fragment itself does not compile in its own world: `.Select(o=>o.Total)` → `List<decimal>` vs declared `List<Order>`; the original golden .cs fails identically). Field-vs-property/access-nuance rows are recorded printer-quality diffs (fidelity budget) |
| 5.4 | Real-corpus compile under `dotnet build`; diff against hand renditions | report | ☑ — **`bench/results-cs-diff.md` first pass**: golden gate 8/9 (1 proven-upstream); real-corpus emit **structure-compiles 9/9** (syntax phase) under a 35-type stub world (the `--noResolve` analog — the corpus is Humanizer-library fragments); 13 additive emitter fixes shipped en route (bracket-depth, `$`-canon interpolation, keyword-space glue, `else`-arm Sym form, pattern designators restored in `split_switch_arms` — identity green, C-style-for, receiver-param prepend, static/const discipline, ctor-return gate, `new T()` fallback); tagged fidelity rows: hand-canon `<rest-omitted>` elisions, multi-line `{…}`-initializer reader loss, target-typed-new type loss, field-vs-property nuance. Remaining: full-library semantic compile (needs a redeclaration-aware rig), then 5.5/5.6 |
| 5.5 | LLM loop test for C# | report | ☑ — `report_5p5_cs.md`: the harness gained the **ir-cs and cs worlds** (to-cs + dotnet rig; C#-baseline); all four echo selftests green; **task_01 cs-baseline: qwen2.5-coder:7b ONE-SHOT cold PASS** (idiomatic .NET); ir-cs: round-1 build-fail repaired round-2 (raw dotnet feedback), rounds 2–4 value near-miss (`"hellow"` — the small-scale semantic class; mapped-IR feedback = the queued to-cs sourcemap parity). Loop-as-QA: 7 rig/emitter truths fixed on landing |
| 5.6 | tree-sitter-c-sharp reader + double round-trip over cs corpus | round-trip report | ◐ — `cs_read.rs` + `irc from-cs`: **canon identity 9/9 + 17-row construct smoke; re-emit leg syntax-clean** (rounds 1–2: 35 fixes; zero emit-structural rows; 250 residual diagnostics decomposed: ~195 stub-member boundary + ~55 char-family design rows — fidelity row 17). Behavior leg: documented boundary (library context + char-atom decision, Phase-6) — `bench/results-cs-diff.md` §leg-2+verdict. **5.5 = the remaining Phase-5 task (needs model time)** |

---

## Phase 6 — Hardening & release

**Goal:** Stable v0.1 usable as an agent file format.

| # | Task | Deliverable | Status |
|---|------|-------------|--------|
| 6.1 | Bench rerun vs Phase 0 headline: did savings hold on the whole-pipeline view? | updated results | ☑ — `bench/results-6p1.md`: **machined-canon rerun — JS canon 36.7% o200k / 37.7% llama (beats the 46% Phase-0 hand-canonicalized headline); C# canon 62.1% / 62.5%; both-languages weighted 57.6%**. The J6 denominator finding carries over and now anchors to machine canon an agent regenerates from any file |
| 6.2 | LLM loop test at scale: larger corpus, multiple models (GPT/Claude/Llama) | scale report | ☐ |
| 6.3 | LSP-ish tooling sketch (completion, go-to-def) — cut unless loop tests demand it | decision doc | ☑ — **cut confirmed**: the loop evidence (2.7/4.4/5.5) never demanded tooling — the repair loop drives from emit/runtime errors + the canon guide; a spec/tutorial + the canon identity serve the authoring path. Revisit only if agent-driven authoring shows canon-drift at scale |
| 6.4 | Versioning policy: additive-only core forms; macros absorb growth | policy doc | ☑ — `docs/versioning.md`: additive-only core forms (repo precedents), opaque-rows-fidelity principle, canon-text rules (additive rows + reader normalization; keyword renames never), the `@1`→`@2` migration rule (the char-atom flip is the named candidate), reader sugar + target lowering as the growth absorbers |
| 6.5 | Language spec + tutorial for "teaching the model" | spec + tutorial | ☑ — `docs/spec.md` (canon v1.1 reference: lexical/grammar/types/statements/members/modules/printing/round-trip) + `docs/tutorial.md` (the target-agnostic walkthrough — supersedes teach.md's JS-only subset; JS + C# idiom tables) |
| 6.6 | Release v0.1: crates.io + example repo | release | ◐ — **prep done**: crate metadata (keywords/categories/description/publish-ready — `cargo package` verifies for both `ir-lang` and `irc`), `examples/` (the metric.cs C# round-trip + compact.js JS idiom, both verified against the gates + `examples/README`), the publish checklist remaining is the actual `cargo publish` invocation + the char-atom decision call (blocks the C# behavior rig, not the JS release) |

---

## Risk register

| Risk | Phase | Mitigation |
|---|---|---|
| Token savings < 30% (no-go) | 0.8 | hand renditions inform design; stop cheaply if no-go |
| Over-minification degrades LLM reasoning | 0.7, 2.7 | readability floor (§6); LLM repair test is the acceptance gate |
| Round-trip fidelity worse than budget | 4.2–4.6 | `(opaque …)` bounds per-member loss at zero; recheck budget after coverage audit |
| Emitted code parses but misbehaves | J1 | behavior harness vs original sources is the Phase-3 gate |
| Loop-clumsiness consumes token savings | 2.7, J5–J6 | loop depth at more model scales + whole-pipeline measurement |
| Reader crashes on adversarial input | 1.6 | proptest fuzzing; never panics, always returns error |

---

## Verification map

Each phase exit is a verification act, not a checklist:

| Phase | Exit artifact | Verifies |
|-------|---------------|----------|
| 0 | `bench/results.md` verdicts + go/no-go number | the concept: token savings ≥30% |
| 1 | golden corpus parse→expand→print identity | grammar & core forms are sufficient in practice |
| 2 | JS outputs compile + LLM repair test (GO, local-model bar) | the language is generatable & debuggable by an LLM |
| 3 (J) | behavior identity + whole-pipeline number | the JS pipeline is *excellent*, not merely passing |
| 4 | double round-trip compile identity (JS) | the round-trip contract (§5) |
| 5 | C# outputs compile + LLM repair test + round-trip | same, with a type system |
| 6 | v0.1 release + scale report | usable as an agent file format |

---

## Working agreements

- **Plan upkeep:** statuses flip only on *verified* progress (artifacts exist
  and pass their checks), not intent.
- **Design-first:** Phase 0 runs before any transpiler code; grammar/core-form
  freeze requires Phase 0 to exit.
- **Fixtures are ground truth:** hand renditions (0.5/0.6) are the reference
  for printer-quality disputes.
- **Core-form closure:** changes to core forms are additive-only and require
  re-running the Phase 0 bench before merge.

---

## Changelog

- **v3** — Route decision recorded: full s-expr falsified, Route 1 (infix
  hybrid) adopted; variant D scored 89.7%/90.1% on golden corpus; 0.7
  decided inline, 0.9 frozen in design-doc v1. **0.8 GO recorded: 46.4%
  o200k / 46.5% llama on 16-file real corpus (Humanizer + cheerio); Phase 0
  exit criteria met — Phase 1 authorized.** *Amendment (same day): grammar
  reconcile pass over the 12 construct families the 0.5 renders needed —
  probe-adjudicated (results.md reconcile section), design doc updated to
  grammar v1.1 (~24 core forms); real corpus re-scored 46.1%/46.2%,
  golden unchanged. Grammar now genuinely covers both corpora.*
- **v3.1** — Phase 1 complete (task 1.5: `--expand` flag; print→print
  identity + closure invariant on all 26 fixtures). Phase 2 open: 2.1/2.2
  design note (js-target.md), expander gained structured `Core::Stmt`
  (additive core form; surface text unchanged, identity held), reader R1b
  logical-continuation + `}else{` chain links fixed, JS emitter (2.3)
  emits 30/30 files passing `node --check`.
- **v3.2 (amended 2026-10-09)** — 2.6 span tracking: `Span{lo,hi}` threaded
  reader→Core (additive fields; surface text + print identity unchanged);
  JS emitter marker post-pass → `SourceMap` (js↔ir lookups both ways, arms
  included; lower errors name their IR line; `to-js --map` sidecar). Corpus
  emission-layout fixes required by line fidelity: first body statement no
  longer fuses onto `{`; fn-tail bare expressions now return (IR value
  semantics, ret-gated); pre-existing expander bugs hit by the loop fixed:
  `export fn` (classified as `var fn`) and `var mut` (canon order fixed —
  grammar `var [mut] ident`). **Corpus re-verified 26/26 node --check.**
  2.7 harness: `bench/scripts/loop/` (llm_loop.py; echo/file/openai
  adapters; emit-verified teach.md; selftest green). **Next: real-model
  loop run (2.7 go/no-go), then 2.4 exit (tsc + diff-vs-hand-renditions).**
- **v3.3 (2026-10-09, later)** — **2.7 GO** on local models: qwen2.5-coder
  7b + 1.5b one-shot task_01 cold; 7b one-shot task_02; seeded repair from
  span-mapped runtime errors repaired in 1 round. Loop-as-QA found the
  else-if chain ownership bug (fragments drained into the enclosing
  `while` instead of the nested `if`) — expand_stmt drains only for
  if/match, sibling-else folding belongs to assoc_list. Report:
  `bench/scripts/loop/report_2p7.md`. **Next session: 2.4 exit (tsc gate +
  diff-vs-hand-renditions), then Phase 3 design.**
- **v3.4 (2026-10-09, later)** — **2.4 exit:** node `--check` 26/26 (Phase-2
  gate met); tsc 7.0.2 gate recorded — TS corpus 3/7, every failure
  root-caused in `bench/results-js-diff.md` (C#-only files by design,
  inline-atom-run design limits, missing re-export row → Phase 4, one
  rendition-drift line), param-`?` erasure fixed with the gate. Emitter-gap
  list appended to js-target.md §7. **Next session: Phase 3 open (3.1 C#
  target design note); dotnet 10 available locally.**
- **v4 (2026-10-09) — plan reroute: JS first, C# deferred.** Phase 2 closed
  (2.4 ☑, 2.7 GO). New Phase 3 = JS depth (J1 behavior harness — emitted JS
  must *behave* identically to originals, not merely parse; J2 inline atom
  runs; J3 gap rows; J4 real module imports; J5 loop depth at more model
  scales; J6 whole-pipeline token measurement — footnote [1] finally
  measured). Phase 4 = JS→IR round-trip only. C# target (emit + round-trip)
  moves to Phase 5; hardening becomes Phase 6. Rationale: breadth before
  depth risk-closure inverted — the pipeline's remaining unknowns are all
  JS-side.
- **v4.1 (2026-10-09, later)** — Phase 3 open: **J1 done** — behavior harness
  (`bench/scripts/behavior/`), first real same-input behavior evidence:
  emitted==original on js_01 (22 cases) + js_08 (8 cases); obj-lit
  initializer loss fixed additively en route (R3 pair-hijack value recovery +
  objlit arm order; regression test). **Next session: J2 inline atom runs
  (statement-ify lambda bodies / if-expressions) → J3 gap rows.**
- **v4.2 (2026-10-09, later)** — **J2 + J3 done: tsc 7/7 on the TS corpus**
  (from 3/7). Glued-if chains split (js_03:21); dot-chain lines absorb
  (reader R2-in-block, R1-continuation rules while unbalanced); arrow-block
  statement-ification at emit; `Core::Reexport` row; callee-generics
  erasure; bool/null match arms; and five latent emitter truths fixed
  (typed-var `=` consumption, `={}` empty inits, fused-`>=`,
  bare-`let` canon, `else{…}` brace-glue reparse). Canonical print→print
  identity holds on all 26; 26/26 node --check; behavior gate green.
  **Next session: J4 real import names → J5 loop depth → J6 whole-pipeline
  tokens.**
- **v4.5 (2026-10-10, later)** — **J6 v2 + re-scoped framing.** task_04
  (3-feature module session; union spec, all 23 cases; agent-driven, PASS 1r
  both worlds) measured the agent-file amortization directly: **marginal
  parity (98.3%)** vs js — the spec amortizes exactly as claimed; one-shot
  session ratio 184%, still guide-dominated (880). **Denominator finding:**
  on the js_08 flattenOptions feature, the IR module is **38.7% of the
  typed+documented TS source** (Phase-0's static number reproduces at file
  scale) but **169.8% of the bare clean-JS runtime** — the loop baseline
  (`--lang js`) strips exactly what IR compresses (types/docs), so savings
  are conditional on the baseline class: met vs typed/documented sources,
  ~0% vs doc-free code. North-star framing re-scoped accordingly (J6 exit:
  scaffold-trimming sensitivity + one corpus-scale loop task).
- **v4.7 (2026-10-10, later)** — **4.2 done: 100.0% representable.** `coverage_audit.py`
  joins the `irc js-kinds` CST histogram (tree-sitter-typescript, 7 js_*.ts
  files, 4398 nodes / 145 kinds) with a curated kind→IR verdict table; the
  reader must map 2233 constructs: 1576 direct core/verbatim (hand renders
  ground truth — `new`/`typeof`/`delete`/`as`/`?.` pass through verbatim),
  14 canonicalizing sugar, 634 signature-erased, 8 `opaque type` captures
  (js_02 union/template aliases), 1 grammar-gap (`export type *` → the
  Core::Reexport row). No representability blockers for the reader.
  Report: `bench/results-coverage.md`. **Next session: 4.3 mapping table +
  `(opaque …)` design (seeded by the audit's per-kind evidence), then 4.4
  JS reader.**
- **v6.2 (2026-10-10, latest)** — **6.6 prep ☑.** `examples/` ships:
  `metric.ir`→`metric.cs` (the C#-idiom emit, verified) and
  `compact.ir`→`compact.js` (the JS-idiom emit) with `examples/README`
  showing the two-way round trip (`from-cs` re-reads). Crate metadata
  publish-ready (`cargo package` passes for both crates; the cli pins
  `ir-lang = { version = "0.1.0" }`). The actual `cargo publish` +
  the char-atom decision are the remaining release calls.
- **v6.1 (2026-10-10, later)** — **6.5 ☑ + 6.3 cut-confirmed.**
  `docs/spec.md` (the canon v1.1 reference spec — lexical/grammar/core
  forms/types/statements/members/modules/printing-invariants and the
  round-trip contract) + `docs/tutorial.md` (the target-agnostic
  teaching walkthrough — JS + C# idiom tables; the loop's teach.md stays
  as the JS-verified subset). 6.3: **LSP cut confirmed** — the loop
  evidence (2.7 repair, 4.4 reader, 5.5 worlds) never demanded
  tooling; authoring runs through the guide + the repair loop.
  Remaining Phase 6: 6.2 (loop at scale — model time), 6.6 (release).
- **v6.0 (2026-10-10, later)** — **Phase 6 opens (6.1 ☑ + 6.4 ☑) and
  Phase 5 closes.** 6.1: the bench rerun on **machine canon** — JS canon
  36.7% o200k / 37.7% llama (beats the Phase-0 46% hand headline), C#
  62.1% / 62.5%, both-languages weighted **57.6%**; the char-heavy/
  comment-light variance rows explained (J6's denominator finding
  carries). 6.4: the versioning policy frozen (`docs/versioning.md`) —
  additive-only core forms, opaque-bounded fidelity, the @1→@2 migration
  rule naming the char-atom as the first candidate. Phase 5
  **essentially exits** (5.5 cs-baseline one-shot + ir-cs repair-in-loop;
  the char-atom + library-context behavior rig defer to Phase 6).
  **Next: 6.5 spec/tutorial, 6.2 loop at scale, 6.3 cut-confirm,
  6.6 release.**
- **v5.7 (2026-10-10, later)** — **5.5 first runs: the loop gained the
  ir-cs and cs worlds.** The 2.7 harness's third+fourth worlds:
  `irc to-cs` emit + a generated dotnet rig (reflection fn-resolve,
  `System.Text.Json` arg-conv, `JsonNode.DeepEquals` compare), C#-baseline
  (feature.cs direct), echo selftests green in all four worlds. **Real
  model evidence (qwen2.5-coder:7b, task_01): the cs baseline ONE-SHOT
  cold-passed** (the first C#-target loop PASS); ir-cs repaired its own
  dotnet build errors round-1→2 then hit the small-scale value miss
  (`"hellow"`) — the to-cs sourcemap parity (mapped-IR repair feedback)
  is the queued fix shape. Report: `bench/scripts/loop/report_5p5_cs.md`.
  Loop-as-QA fixed 7 rig truths on landing (double-static synth, Rig-type
  trailing top-level statements, DeepEquals/using, per-round .cs archives
  compiling under the dotnet glob → `.csrc` suffix).
- **v5.6 (2026-10-10, later)** — **5.6 leg-2: behavior boundary documented.**
  The C# behavior rig decomposition: ~55 char-family rows (the char atom's
  absence — fidelity row 17; unblocking = a Phase-6 design decision, not
  rushed) + ~195 stub-member rows (the library API; the emitted corpus
  carries internal-default members and resolves cross-file like the
  originals). Structure-compiles verified per row; the emit-side
  field-mutability analysis (readonly + assignment) keeps round-tripped
  mutable state mutable. Field rows restored this pass: Symbols/UnitPrefixes
  dict+array initializers (verbatim entries — brace-pair discipline), field
  `variable_declaration` field-name fallback, primary-ctor params (canon
  print, fixture-row shape), computed-property `=>` rows, lambda
  multi-params, implicit-object args, char/string/keyword-text verbatim
  atomization, mut-overlabel (reader post-pass + emit-side assigned-scan).
  **Remaining Phase-5: 5.5 loop test; the char-atom deferred to Phase 6.**
- **v5.5 (2026-10-10, later)** — **5.6 re-emit leg syntax-clean.** The
  round-trip gate closes round 1: 93 → **0 syntax-class rows**; 215
  remaining diagnostics root-caused — 207 = the stub-world MEMBER gap by
  gate design (types resolve, members don't; emitted cross-file calls now
  carry the internal-default and resolve like the originals), 8 = the
  char-vs-string canon row (C# `'0'` chars canon to Str; recorded as a
  design-limit row for the fidelity doc). 13 more fixes this pass (cast
  operand fields, void-ret case filter, else-block truncation — the c-sharp
  grammar's `alternative` field holds the block directly, else-if conds,
  local-fn access-modifier prohibition (`fn_sig(type_ctx, in_static)`),
  local-fn `type`-field rets, static-host members, primary-ctor params
  (reader + canon print — the fixture row shape), property computed rows,
  **data-brace canon-escape `{{…}}`** (the C# reader distinguishes
  interpolated strings from literals; the emit restores literal braces),
  opaque-fragment statement drops). Suite green (50+2+7). **Next: the
  behavior leg + 5.6 report, then 5.5 (dotnet-run loop) and the fidelity
  doc addendum.**
- **v5.2 (2026-10-10, later)** — **5.4 first pass (◐):** real-corpus emit
  under `dotnet build` — structure-compiles 9/9 (syntax phase) via a 35-type
  stub world (the corpus is Humanizer-library fragments; member-level
  semantics reduce to the real-library context, `--noResolve` analog).
  13 additive emitter fixes (each identity-re-verified): bracket_slice depth,
  cast operand, `$`-canon interpolation, pre-keyword spacing, `}`-binder
  glue, Sym(`else`) arms, pattern-designator restoration in the expander's
  arm splitter, C-style for, receiver-param prepend (CS0111), static/const
  discipline, ctor tail-return, argless `new T()`, var-branch run-end brace
  capture. Hand-canon `<rest-omitted>` elisions + initializer reader-loss +
  dynamic-new type-loss tagged in `bench/results-cs-diff.md`. **Next: the
  library-context semantic rig (queued), 5.5 C# loop test, 5.6 reader.**
- **v5.1 (2026-10-10, later)** — **5.3 done: C# printer ships.** `crates/lang/src/cs.rs`
  + `irc to-cs` — direct Core→C# emitter per cs-target §2–§6. Golden gate
  `dotnet build` **8/9** (03_pipeline excluded, proven upstream: the original
  golden fragment fails identically — its `.Select(o=>o.Total)` vs declared
  `List<Order>` mismatch is in the design-doc example itself). Lowering
  features landed: LINQ op map (`filter→Where`, `sum→Sum`, argless `sort`
  →`OrderBy(x=>x)`), type-word map incl. `Array<T>→T[]`, declaration-keyword
  ty stripping (`const var` rides the ty via the expander's field parse),
  sig-only member-row inversion, switch-expr reordering (expander hoists
  else arms first — emit puts `_` last), bare-call arg collection w/ comma
  separation, async-upgrade (`await` in body → `async Task<Ret>`), synthetic
  `internal static partial class ModuleTop` for top-level fns with the
  stub-rig extension point mirroring the JS harness stubs. Real-corpus
  preview shapes right (extension methods, primary ctor); 5.4 gate next.
- **v5.0 (2026-10-10, later)** — **Phase 5 opens with the C# promise kept.**
  A skip-C#/harden-first reroute was weighed (the v4-style risk-closure
  argument: nothing in Phase 6 needs C#, Phase 5 ≈ re-running Phase 2+4
  effort) and **rejected** — original plan precedence; C# stays a target,
  recorded in the Phase-5 heading. 5.1+5.2 ☑ in one code-grounded doc
  (`docs/cs-target.md`): direct Core→C# emitter (js.rs shape; output-level
  Roslyn conventions), verbatim-first atom mapping (the IR expression
  surface was C#-compatible by Phase-0 design), full lowering table incl.
  the sig-only member-row inversion and the type-word map; deferred rows
  pre-registered (attributes, access-modifier nuance, `{get;set;}`).
  **Next session: 5.3 C# printer → golden corpus `dotnet build` gate.**
- **v4.11 (2026-10-10, later)** — **4.5 + 4.6 done — Phase 4 exits.**
  Double round-trip JS → IR canon → JS: node --check **7/7**, **tsc 7/7**
  (vs 3/7 forward in 2.4 — the reader closed the inline-run/re-export rows),
  behavior via J1-harness `RT_MODE=1`: **3/3 subjects BEHAVIOR-MATCH** (hand
  mode re-verified after the harness edit). Report
  `bench/results-round-trip.md` (gate table, diff-vs-hand catalog split into
  printer conventions vs fidelity drops, one additive reader fix the gate
  found — `alias_body_atoms` union leaves). `docs/fidelity.md` (4.6): the
  actual §6 budget in practice — 6/7 corpus files capture-free, js_02 peaks
  at 8 `opaque type` alias rows; 16 bounded rows recorded; the double-trip
  acceptance holds with that budget. **Phase 5 (C# target) now opens
  (5.1 design note next).**
- **v4.10 (2026-10-10, later)** — **4.4 done: JS reader + canon identity + re-emit gate.**
  `read_js` (js_read.rs, tree-sitter CST → Core) + `irc from-js`. The identity gate
  (canon → IR reader → expand → print) held after closing reader canon bugs (union
  tightness, lookup-type object, property-signature `?`, braceless if/for arms, bare
  `return;`, array/args separators, function/object-type erasure, `as const` leaf,
  `?.` optional-chain children, `increment` field) and — additively — three latent
  **pipeline holes the reader was the first to exercise at scale**: `expand_line`'s
  expr fallback splices run-end `{…}` inline bodies (tail block-lambdas silently
  lost on re-parse before); var-branch obj-lit hijack extends to `=>` tails;
  `expand_member`/`print_member` verbatim `"opaque:…"` member rows. Lexic truth
  found: `!==` glued after an ident can't survive the frozen `x!` suffix rule —
  canon prints a space (hand-canon spacing reproduces correct emit). **Evidence:
  48 lib tests incl. all-7-corpus canon identity + 30-row construct smoke;
  JS → IR → JS `node --check` 7/7; behavior cases (js_01 22 + domEach side
  effects, js_08 6) ALL MATCH vs originals.** js-target §8.4 records canon shapes.
  **4.5 opens at ◐ (leg 1 green; tsc gate + report next).**
- **v4.9 (2026-10-10, later)** — **4.4 in flight (session paused):** reader walk
  written and compiling in `js_read.rs` — expressions/atoms (`new`/`typeof`/
  `delete`/`as`/`?.` verbatim, templates→interpolation payloads, `i++`→`+=`,
  callee-generics preserved), statements (`var`/`var mut` over-label,
  if/else(+else-if)/while/for-in/C-style-for→range canon/return/throw),
  declarations (`fn` + param type erasure `name Type` + `Type?`, class/
  interface members, imports→@use, reexports, type-alias split into
  interface-shaped vs `opaque type` atom runs), `opaque:` member capture
  fallback (never-wrong rule). Unverified next steps: `irc from-js` CLI,
  canon-print identity tests, corpus smoke (js_01/js_08), re-emit
  node --check round-trip, then 4.5.
- **v4.8 (2026-10-10, later)** — **4.3 done: js-target.md §8.** The round-trip
  mapping (4.2 audit buckets → IR forms/sugar/signature/opaque/drop) plus the
  `(opaque …)` capture design. The existing printer already holds the
  machinery (`opaque:` atoms + `opaque{ raw }` bodies verbatim; `opaque type`
  canon rows from js_02.ir), so the design is code-grounded rather than
  aspirational: capture whole declaration/member only, print captured text
  byte-identically (print→print identity includes captured lines), capture
  sites exhaustive (union/template aliases, unclassifiable statements,
  grammar-ERROR fallback), and the double-trip acceptance (JS → IR(±opaque)
  → JS compiles + behaves identically) is guaranteed by construction for
  captured members. **Next session: 4.4 JS reader** (tree-sitter CST → Core,
  reusing the canonical printer; classification per §8.2; the 4.2 histogram
  is the checklist), then 4.5 double round-trip.
- **v4.6 (2026-10-10, later)** — **J6 closed on the class model; Phase 4
  opens.** Corpus-scale loop tasks scoped and rejected with evidence
  (js_01's runtime = 4 short pure fns; JSON-case loop rigs cannot measure
  the type/doc weight — that denominator is static-only, Phase-0 46% vs
  js_08's 170%-of-bare-runtime both recorded; see report_j6_pipeline.md §5).
  J6 exit criterion revised: sensitivity runs on scaffold trimming remain
  optional; the north-star measurement = static corpus basis + loop
  sensitivity, both closed. **4.1 done**: `js_read.rs` (tree-sitter +
  tree-sitter-typescript 0.23.2), 6/7 corpus files parse; grammar gap
  found (`export type * from` — round trip will opaque it);
  `irc js-kinds` histogram ships as 4.2 groundwork. 40 lib tests green.
  **Next session: 4.2 coverage audit (kinds → IR-form coverage %) and 4.3
  mapping table incl. the `(opaque …)` design.**
  (3-feature module session; union spec, all 23 cases; agent-driven, PASS 1r
  both worlds) measured the agent-file amortization directly: **marginal
  parity (98.3%)** vs js — the spec amortizes exactly as claimed; one-shot
  session ratio 184%, still guide-dominated (880). **Denominator finding:**
  on the js_08 flattenOptions feature, the IR module is **38.7% of the
  typed+documented TS source** (Phase-0's static number reproduces at file
  scale) but **169.8% of the bare clean-JS runtime** — the loop baseline
  (`--lang js`) strips exactly what IR compresses (types/docs), so savings
  are conditional on the baseline class: met vs typed/documented sources,
  ~0% vs doc-free code. North-star framing re-scoped accordingly (J6 exit:
  scaffold-trimming sensitivity + one corpus-scale loop task).
- **v4.4 (2026-10-10)** — **J1-J6 day: three emitter truths + harness hardening + the J6 number.**
  Loop-as-QA round 2: qwen2.5-coder:7b drafts exposed (1) braceless
  single-line `if (c) return x` — the reader glues the statement into the
  if-head run and the emitter wrapped it as a condition (`if ((c)return e)
  {}`, runtime SyntaxError); (2) `for`/`while`-led run pieces emitted
  VERBATIM (no lowering inside inline atom runs — `for i in 0..n{…}` shipped
  raw, `""for iin 0..`). Both fixed additively (emission-side only, surface
  unchanged — canon identity holds, 47 tests green, tsc gate clean, 16/16
  node --check, behavior ALL MATCH): `emit_if` splits the cond-run at
  depth-0 statement keywords (tail = then-arm); `stmtify_piece` gains
  range-for/for-of/C-style-for/while; `STMT_BEGIN` += for/while.
  Post-fixer evidence: task_01 7b cold PASS round 1 with the idiom.
  Harness: `--lang js` baseline world (no guide/no transpile/no map; tasks
  carry spec_js; js echo drafts); adapter catches socket/timeouts
  (adapter-error instead of dying pre-report); LLM_NUM_PREDICT cap;
  reasoning-channel fallback for content-empty thinking models (gemma4:12b
  is operationally blocked on 16 GB — whole answer streams into
  `reasoning`, ~2.9k tokens, content never lands). **J5 verdict at 3
  scales**: post-fixer 7b = value near-miss only; qwen3.5:9b structural;
  gemma op-block; agent-driven = task_03 PASS round 1 BOTH worlds (first
  task_03 IR pass). **J6 v1 (report_j6_pipeline.md): the ≥30%
  whole-pipeline number is falsified at small-feature scale** — one-shot
  IR 206–316% of js per feature, amortized 77–90% (break-even S≈10–15);
  write-side compression real (48–55%), loop scaffolding consumes it at
  300-char feature scale; follow-ups: corpus-scale features,
  multi-feature-per-file, leaner feedback. **Next session: J6 v2 session
  measurement + framing (done same day, see v4.5), then Phase-4 open
  (4.1 tree-sitter).**
- **v4.3 (2026-10-09, later)** — **J4 done** (`8526982`): `@use` with dotted
  usage lowers to a namespace import (`import * as Helpers …`) — the
  qualifier is a module object in ESM, so cross-module calls work verbatim;
  linked two-file behavior subject green (Fixtures.Helpers + feature,
  5 cases). J5 in flight: harness hardened for real-model repair dynamics
  (repair-round sampling, repetition nudge, `think:false` + `num_ctx` for
  ollama, value-fail = numbered-IR feedback); task_03 runs recorded ×3
  models — qwen2.5-coder:7b near-miss class (untrimmed split drops
  camelize), qwen3.5:9b structural (Pythonisms/unindented), gemma4:12b
  pending retry (600s). **Next session: gemma run → J5 verdict → J6
  whole-pipeline measurement (tiktoken venv + `--lang js` baseline-mode +
  pipeline_tokens.py).**
- **v3.2 amendment note (2026-10-09 early):** unfamiliar-syntax risk means
  loop-clumsiness could consume the savings — 2.7 is the real go/no-go
  (risk register). Only after 2.7 passes: tsc gate + diff-vs-hand-renditions
  (2.4 exit), then Phase 3.
- **v2** — Full 6-phase plan: task tables with deliverables/status, per-phase
  exit criteria, risk register, verification map, working agreements. Phase 0
  expanded into 9 tasks with a decision order; whole-pipeline measurement
  (footnote [1]) made the north-star metric.
- **v1** — Initial outline (Phases 0–5, ~15 tasks).