# LLM-Lang

A token-dense intermediate representation (IR) language for LLM code
generation, with a working Rust transpiler (`ircc`) targeting JavaScript
and C#. Plain-language overview: [`docs/overview.md`](docs/overview.md).

The idea: LLMs pay per token for code generation. Production-language
boilerplate (modifiers, type ceremony, braces, XML docs) is a large
fraction of that spend. LLM-Lang is a dense, canonical, machine-native
format for agent codegen — write features in the IR, transpile to the
target, and round-trip existing code back to the IR for cheap reads.

## Install

```
cargo install ircc          # after the v0.1 crates.io release (queued)
```

Not published yet — build from the repo:

```
git clone https://github.com/henrikandersson/LLM-Lang
cargo install --path LLM-Lang/crates/cli
```

A self-contained Rust toolchain — no node/dotnet needed for authoring;
only for *running* emitted code (node for `to-js` output, .NET 10 for
`to-cs` output).

## Getting started

Write a feature in the IR (see `examples/` and `docs/tutorial.md`):

```
@1 @ns Examples.Compact

export fn CompactLabel(String s) String{
  var t=s.trim()
  if t.length==0{return "-"}
  return t.split(" ").filter(w=>w.length>0).map(w=>w[0].toUpperCase()).join("").slice(0,6)
}
```

Emit to a target:

```
ircc to-js feature.ir > feature.js        # runnable ESM
ircc to-cs  feature.ir > Feature.cs       # runnable .NET 10
```

Round-trip an existing codebase so an agent can read it densely:

```
ircc from-js src/util.js  > util.ir       # the cheap canon
ircc from-cs src/Lib.cs   > lib.ir        # C# → IR, same contract
# ... edit *.ir ...
ircc to-js   util.ir > src/util.js        # regenerate
ircc to-cs   lib.ir  > src/Lib.cs         # regenerate
```

Lint before committing — canon files must be print-stable:

```
ircc verify feature.ir                    # exit 1 on drift/parse failure
```

Reference: `ircc parse/expand` (debug views), `ircc canon`,
`ircc js-kinds`. The library (`llm-lang` crate) exposes the same
pipeline programmatically.

**Measured:** machine-generated canon (from-js/from-cs over the real
corpus) scores **36.7% / 64.2%** of the original JS/C# source tokens
(o200k; llama 37.7% / 64.2%) — ≈42% cheaper across both languages
(`bench/results-6p1.md`; re-measured after the member-row pass, which
carries member access/readonly rows the canon previously dropped). The
Phase-0 hand-rendition headline was 46.1% / 46.2%; the machine canon
beats it because comments/formatting drop mechanically rather than by
curation.

## What's here

- `docs/overview.md` — non-technical explanation of the idea, what was
  proven, and the plan state.
- `docs/language-design.md` — the language itself (v1.1): EBNF reader
  grammar, ~24 core forms, type surface, sugar-as-expansions,
  readability floor, fidelity budget, golden examples, and the measured
  decision log (§8) + what got falsified along the way (§10).
- `docs/llm_optimized_language_architecture.md` — transpiler
  architecture summary (Rust reader + expander + printers).
- `docs/fidelity.md` — the actual round-trip fidelity budget in practice
  (what holds, what's bounded, per-row evidence).
- `docs/js-target.md` — JS lowering table, emission notes/sourcemap
  design (§1–7), JS→IR round-trip mapping + canon shapes (§8).
- `docs/cs-target.md` — the C# counterpart (lowering table, type-word
  map, emission conventions).
- `docs/PLAN.md` — 6-phase plan with per-step status. Phases 0–5 are
  DONE (measurement, reader, JS emit + depth, JS round-trip, C# emit +
  round-trip incl. the vendored-library behavior rig); Phase 6
  (hardening & release) is mostly done — 6.2 (loop at scale) and the
  crates.io publish remain.
- `bench/results.md` — every measured verdict: variant progression
  A→D (132.7% → 89.7%), BPE mechanics probe log, real-corpus go/no-go.
- `bench/results-coverage.md` / `bench/results-js-diff.md` /
  `bench/results-round-trip.md` — the JS corpus audits and both
  pipeline gates (compile + behavior, emit- and round-trip-side);
  `bench/results-cs-diff.md` — the C# emit + round-trip gates;
  `bench/results-6p1.md` — the machine-canon bench rerun;
  `bench/results-humanizer-rig.md` — the vendored-library behavior rig.
- `bench/corpus/` — golden fixtures (10 C#/JS↔IR pairs) + real corpus
  (9 C# from Humanizer, 7 TS from cheerio, both MIT, hand-rendered);
  `rt/` and `rt_cs/` hold the machine-canoned round-trip artifacts and
  their re-emits.
- `bench/variants/` — rendered variants: b, c (s-expr variants), d
  (Route 1 infix winner).
- `bench/scripts/` — `score.py` (tokenizer scoring), `behavior/`
  (emitted-vs-original behavior harness), `loop/` (the LLM
  write→transpile→run→repair loop harness + reports), `coverage_audit.py`.

## The language in one page (`bench/variants/d`)

```
@1 @ns Acme.Users

fn GetUser(Int id) User?{await Users.Find id}

type Order export{
  List<LineItem> LineItems
  Customer Customer
  fn Total() Decimal{LineItems.map(li=>li.Amount).sum()}
}

async fn Load(Int id) User?{
  try await Repo.Get id
  catch NotFound return null
  catch Error e throw e
  finally Log("done")
}
```

## Status (2026-10-11)

**Working transpiler, verified in both directions for both targets.**

- **Reader/expander (Phase 1):** `ircc lex|parse|expand` — parse→expand→
  print is identity on all 26 corpus fixtures; fuzzing suite green.
- **Emit IR→JS (Phase 2):** `ircc to-js` (+ `--map` for line-mapped
  source maps) — 26/26 `node --check`, tsc 7/7 on the TS corpus; every
  tsc failure root-caused in the 2.4 report.
- **JS depth (Phase 3):** behavior identity gated (emitted code *runs*
  identically to originals, not merely parses); the LLM loop test passed
  (`bench/scripts/loop/report_2p7.md`) at three local-model scales;
  whole-pipeline token measurement done (`report_j6_pipeline.md`) —
  **savings hold vs typed/documented sources, break even vs bare
  doc-free runtime and small-feature loop traffic** (the honest framing
  is in `docs/overview.md`).
- **Round-trip JS→IR (Phase 4):** `ircc from-js` — canon is identity
  stable over the whole corpus; double round-trip JS→IR→JS compiles
  (node 7/7, tsc 7/7) and behaves identically (3/3 harness subjects);
  actual fidelity budget in `docs/fidelity.md`.
- **C# emit + round-trip (Phase 5):** `ircc to-cs` / `ircc from-cs` —
  golden gate 8/9 (1 proven-upstream fragment), canon identity 9/9,
  double round-trip re-emits clean; the LLM loop test passed for the C#
  baseline (`bench/scripts/loop/report_5p5_cs.md`); the   **vendored
  Humanizer behavior rig compiles the round-tripped corpus inside the
  real library tree with 0 errors** (was ~834 diagnostics,
  `bench/results-humanizer-rig.md`); **Humanizer's own test suite ran
  against that   assembly on SDK 10.0.403: after one row pass (get-only properties,
  `where` constraints, unsigned type-words), the full suite is green —
  **61,581 tests, 61,580 pass / 0 fail / 1 vendor-disabled skip**.
- **Phase 6 (hardening):** machine-canon bench re-measured (36.7% JS /
  64.2% C#, weighted 58.3% — `results-6p1.md`), spec + tutorial +
  versioning policy shipped (`docs/spec.md`, `docs/tutorial.md`,
  `docs/versioning.md`), LSP tooling cut confirmed. Remaining: 6.2
  (loop test at scale — needs model time) and the crates.io publish
  (deferred; `llm-lang`/`ircc` names verified free).

See LICENSE notes inside the corpus sources (MIT, Humanizer + cheerio).