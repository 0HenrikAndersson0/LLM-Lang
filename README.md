# LLM-Lang

A token-dense intermediate representation (IR) language for LLM code
generation, with a working Rust transpiler (`irc`) targeting JavaScript
(C# next). Plain-language overview: [`docs/overview.md`](docs/overview.md).

The idea: LLMs pay per token for code generation. Production-language
boilerplate (modifiers, type ceremony, braces, XML docs) is a large
fraction of that spend. LLM-Lang is a dense, canonical, machine-native
format for agent codegen — write features in the IR, transpile to the
target, and round-trip existing code back to the IR for cheap reads.

**Measured (Phase 0, 2026-10-03):** hand-rendered variant D IR costs
**46.1% (o200k) / 46.2% (llama)** of real-world C#/JS sources — a ~54%
one-shot token reduction, against the ≥30% north star. Golden-corpus
syntax floor: 89.7% / 90.1% token counts vs the same code in C#/JS.

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
- `docs/PLAN.md` — 6-phase plan with per-step status. Phases 0–4 are
  DONE (measurement, reader, JS emit, JS depth, JS round-trip); Phase 5
  (C#) opens next.
- `bench/results.md` — every measured verdict: variant progression
  A→D (132.7% → 89.7%), BPE mechanics probe log, real-corpus go/no-go.
- `bench/results-coverage.md` / `bench/results-js-diff.md` /
  `bench/results-round-trip.md` — the JS corpus audits and both
  pipeline gates (compile + behavior, emit- and round-trip-side).
- `bench/corpus/` — golden fixtures (10 C#/JS↔IR pairs) + real corpus
  (9 C# from Humanizer, 7 TS from cheerio, both MIT, hand-rendered);
  `rt/` holds the round-trip artifacts.
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

## Status (2026-10-10)

**Working transpiler, verified in both directions for JavaScript.**

- **Reader/expander (Phase 1):** `irc lex|parse|expand` — parse→expand→
  print is identity on all 26 corpus fixtures; fuzzing suite green.
- **Emit IR→JS (Phase 2):** `irc to-js` (+ `--map` for line-mapped
  source maps) — 26/26 `node --check`, tsc 7/7 on the TS corpus; every
  tsc failure root-caused in the 2.4 report.
- **JS depth (Phase 3):** behavior identity gated (emitted code *runs*
  identically to originals, not merely parses); the LLM loop test passed
  (`bench/scripts/loop/report_2p7.md`) at three local-model scales;
  whole-pipeline token measurement done (`report_j6_pipeline.md`) —
  **savings hold vs typed/documented sources, break even vs bare
  doc-free runtime and small-feature loop traffic** (the honest framing
  is in `docs/overview.md`).
- **Round-trip JS→IR (Phase 4):** `irc from-js` — canon is identity
  stable over the whole corpus; double round-trip JS→IR→JS compiles
  (node 7/7, tsc 7/7) and behaves identically (3/3 harness subjects);
  actual fidelity budget in `docs/fidelity.md`.
- **Next:** Phase 5 — C# target (emit + loop test + round-trip).

See LICENSE notes inside the corpus sources (MIT, Humanizer + cheerio).