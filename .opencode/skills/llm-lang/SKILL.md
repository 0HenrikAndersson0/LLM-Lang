---
name: llm-lang
description: Use when working with LLM-Lang (LLM-Lang) files or tooling — .ir canon rows, the ircc CLI (to-js / to-cs / from-js / from-cs / verify), canon round-trips in this repo, or when asked to author/edit/emit/verify IR canon. Covers the workflow and the rules that keep canon identity stable.
---

# LLM-Lang canon workflow

> AGENTS.md at the repo root is the canonical copy of this workflow
> (Codex/Gemini/Cursor adapters read it natively). This skill is the
> opencode trigger subset — keep the two in sync when editing either.

LLM-Lang is a compact transfer representation for code: sources
compile INTO canon (`.ir` text rows), canon emits back OUT to targets
(JS/TS, C#), round-trip stable. Its whole point is agents reading,
authoring, and repairing code in canon — the token savings (~57% of
typed+documented sources on o200k with 100% behavior identity in the
Humanizer gate) come from dropping comments/doc-bloat while keeping every
executable byte's semantics.

## The tool: `ircc`

```
ircc from-cs file.cs > file.ir     # C# source → canon
ircc from-js file.ts > file.ir     # JS/TS source → canon
ircc to-cs file.ir > file.cs       # canon → C# emit
ircc to-js [--map] file.ir > file.js   # canon → JS emit (+ line map sidecar)
ircc verify file.ir...             # canon-identity lint — RUN THIS
ircc lex|parse|expand <files>      # reader introspection (expand = core forms)
```

## The rules that matter (violating these breaks the pipeline)

1. **Canon identity is the contract**: a canon file must be its own
   fixed point — `ircc verify <files>` checks `file == print(expand(file))`
   AND print→print stability. Never hand-tweak canon so that this fails.
   `ircc verify` exits 1 on drift.
2. **Core forms are additive-only**: never rename or reshape existing canon
   text; growth lands as new rows (versioning policy: `docs/versioning.md`).
   Canon v1.1 grammar reference: `docs/spec.md`.
3. **Fidelity is budgeted with `opaque`**: constructs with no core form
   capture verbatim as `opaque:` rows and re-emit byte-identically (enums,
   operators, `#if` regions, unmatched statements). Don't invent new
   escape shapes — the budget is documented in `docs/fidelity.md`.
4. **Comments drop by contract.** Hand canon MAY carry `//` rows (they are
   fixture annotations); machine canon never re-emits them.

## Canon shape in one glance

```
@1 @ns Ledger.Core        ← header: version, namespace
type Money export record{
  export ctor (minor Long, curr Currency)
  export Decimal Major => Minor / (Decimal)Math.Pow(10, Scale[Curr])
  export fn TryAdd(other Money) Result<Money,String>{…}
}
fn Of(amount Decimal, curr Currency) Money{…}
```

- params are `(name Type[, default])`; types canonize capital
  (`String Int Bool Float Int64 ULong …` → target keywords on emit)
- modifiers ride rows as words: `export`, `static`, `readonly`, `volatile`,
  `sealed`, `abstract`, `virtual`, `override`, `get` (get-only auto-props)
- generic constraints canon after bases: `type C<T> : B where T : class{`
- access words stay verbatim (`protected CultureInfo Culture`)

## Verification gates (run before proposing a change)

```
cargo test -p llm-lang -p ircc                        # 59 test rows
ircc verify bench/corpus/real/ir/*.ir bench/corpus/real/rt/*.ir \
            bench/corpus/real/rt_cs/*.ir               # canon identity, all corpora
python3 bench/scripts/behavior/runner.py               # JS behavior 3/3 BEHAVIOR-MATCH
for lang in ir js cs ir-cs; do python3 bench/scripts/loop/llm_loop.py selftest --lang $lang; done
```

Emitted-code compile gates: `node --check` for JS emits; `dotnet build`
/net10.0 for C# emits. The strongest gate on file: the vendored Humanizer
rig runs the vendor's own 61,581-test suite against round-tripped code
(`bench/results-humanizer-rig.md`).

## Where the deep references live

- `docs/spec.md` — canon v1.1 grammar (the authoritative form list)
- `docs/tutorial.md` — step-by-step authoring walkthrough (JS + C# idiom tables)
- `docs/fidelity.md` — the round-trip budget, per-row
- `docs/versioning.md` — the additive-only core-forms policy
- `bench/results-local-projects.md` — the fresh-code gate (what "holds up" means)

## Repo structure

- `crates/lang/` — the library (reader, expander, printer, js/cs readers+emitters)
- `crates/cli/` — the `ircc` binary
- `bench/corpus/real/` — the measured corpus (`src/` originals, `ir/` canon,
  `rt/` + `rt_js/` + `rt_cs/` round-trip artifacts)
- `bench/corpus/local/` — the fresh-code gate projects (emit these and compile them)