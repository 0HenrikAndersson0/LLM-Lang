# LLM-Lang

A token-dense intermediate representation (IR) language optimized for
LLMs, with a planned Rust transpiler to C# and JavaScript.

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

- `docs/language-design.md` — the language itself (v1.1): EBNF reader
  grammar, ~24 core forms, type surface, sugar-as-expansions,
  readability floor, fidelity budget, golden examples, and the measured
  decision log (§8) + what got falsified along the way (§10).
- `docs/llm_optimized_language_architecture.md` — transpiler
  architecture summary (Rust reader + expander + printers).
- `docs/PLAN.md` — 6-phase plan. Phase 0 (decisions by measurement) is
  DONE with a GO; Phase 1 (Rust reader & expansion) is next.
- `bench/results.md` — every measured verdict: variant progression
  A→D (132.7% → 89.7%), BPE mechanics probe log, real-corpus go/no-go.
- `bench/corpus/` — golden fixtures (10 C#/JS↔IR pairs) + real corpus
  (9 C# from Humanizer, 7 TS from cheerio, both MIT, hand-rendered).
- `bench/variants/` — rendered variants: b, c (s-expr variants), d
  (Route 1 infix winner).
- `bench/scripts/` — `score.py` (tokenizer scoring harness),
  `toktest.py` (probe two tokenizers with arbitrary strings).

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

## Status

Phase 0 exit (GO) — grammar v1.1 frozen. Nothing is implemented yet.
Phase 1 starts from `docs/PLAN.md`.

See LICENSE notes inside the corpus sources (MIT, Humanizer + cheerio).