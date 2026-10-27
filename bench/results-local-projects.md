# Local-projects gate (2026-10-27) — fresh code, compiled + run + scored

> The release candidate test on **never-seen, locally authored** code: two
> small projects written the way the claim class supposes (typed +
> documented, idiomatic, fresh domain — a C# money/ledger library, a TS
> shift scheduler), driven end to end: canon → verify (hardened) → emit →
> **compile the emitted output and run it**, then token-scored against
> typed sources AND a bare doc-free baseline. This is what exposed six
> emit rows the rehearsed corpus never exercised.

## The projects

| project | source | why this code |
|---|---|---|
| `ledger_core/ledger_service` (C#) | `src/*.cs` | records + primary ctors, enums, operators, generic Result<T,E>, LINQ, switch expressions, extension-style statics, exceptions, interpolation, **full XML docs** — the C# surface the rig corpus (Humanizer fragments) only partially reaches |
| `shift_scheduler` (TS) | `src/shift_scheduler.ts` | interfaces, readonly arrays, unions, template literal types, destructure lambda params `([, q])`, object-lit shorthand, JSDoc — the TS surface with generics + destructuring |

All files written fresh for this gate; nothing derives from the corpus
files. Bare baselines in `bare/`: the same semantics, doc-free C# (docs /
comments stripped — C# cannot drop types) and clean minimal JS (the honest
"runtime floor" denominator from J6).

## Gates the emitted code passed

```
dotnet build  ledger.csproj (net10.0)   Build succeeded — 0 errors
dotnet run    harness (aggregation + rounding + parse)
              → "1 currencies reconciled, 0 conflicts"
              → "2 3 4"   (2.5→bankers 2 / nearest 3; 3.7→ceiling 4)
              → "12.5"    (Sek:1250 → 12.50 SKR)
node --check  shift_scheduler.js         OK
node          roster harness: bookable quarters, overlap=0 for adjacent
              windows, unsatisfied + weekly totals all match hand-computed
```

## The six rows it exposed (all fixed this pass)

| # | row | where | fix |
|---|-----|-------|-----|
| 1 | canon arm rows ambiguous: `RoundingMode.Floor decimal.Floor(value)` mis-split at re-parse (body-leading type word read as a pattern binder → `Floor(decimal)` garbage from `expr`'s bare-call rule) | printer `print_stmt` + expander `split_switch_arms` | C# trailing-switch canon arms carry ` => ` (structural split; `match` keeps the frozen hand-canon space shape — variants fixtures govern) |
| 2 | dict/array initializer payloads lost every canon round (reader brace-gluing parked `{…}` in the inline body; the expander's field path ignored it) | expander `expand_member` `_` arm | re-fuse the inline brace payload into the value atoms; emit restores valid `new Dictionary<…>{…}` |
| 3 | the old `verify` lint could not see row-2: canon1→canon2 drift happened *before* the compared pair | cli `verify` | machine-canon files must be **their own fixed point** (`file == print(expand(file))`); hand-canon files (comment/blank rows, pre-normalization shapes) keep the two-round check |
| 4 | opaque member captures re-emitted as corrupted string-literal fields (`readonly string  = $"opaque:…"`) | cs `member_var` | opaque-mod rows print the captured text raw (same discipline as the Expr-arm capture) |
| 5 | **file-level enums collapsed to empty classes** (invalid emitted C#: `Currency` had no members) | cs_read enum_declaration | file-level enums → verbatim opaque capture; emit re-prints raw at file scope |
| 6 | canon type-words inside expression positions unmapped (`new Dictionary<Currency,Int>` — `Int` has no .NET name → CS0246) | cs `expr` | type-word map applies in exprs except after `.` / before `(` |

Plus two JS-emission rows the same gate caught:

| # | row | fix |
|---|-----|-----|
| 7 | destructured lambda params canon'ed as opaque strings re-emitted as invalid quoted params (`("opaque:[, q]") => q>0`) | js_read `lambda_params`/`pattern_atoms`: destructure patterns carry as raw atoms; emit renders `[, q]` |
| 8 | object-lit keys that are keywords lost their colon (`{day: x, from slot}`) — the `:` arm dropped the colon when the keyword-space rule had already left a trailing space | js `expr_at` colon arm keeps the colon always |

Corpus side-effects: all C# canon + rt artifacts regenerated (the `=>`
arm rows + fuse change shift canon text; rt_cs re-emitted); the rt js
emissions regenerated (colon + destructure rows); **canon identity
35/35 under the hardened lint** (`ir/` + `rt/` + `rt_cs/` + `local/ir/`);
59 test rows; JS behavior 3/3; loop selftests ×4; and the Humanizer rig
re-run after the row pass: **61,581 tests — 61,580 pass / 0 fail /
1 vendor-disabled skip (unchanged, still all-green)**.

## Token measurement (fresh projects, machine canon)

| set | o200k | llama |
|---|-------|-------|
| src typed + documented | 3913 | 5037 |
| **machine canon** (`local/ir`) | **2241** | **2917** |
| bare doc-free baseline (`local/bare`) | 2025 | 2707 |
| emitted (compile+run) | rt_cs 1312, rt_js 760 | 1734, 1051 |

| ratio | o200k | llama |
|---|---|---|
| **canon vs typed + documented sources** | **57.3%** | **57.9%** |
| canon vs bare doc-free floor | 110.7% | 107.8% |
| emitted C# (rt_cs) vs C# sources | 56.4% | — |

Per file: ledger_core 56.8%, ledger_service 56.7%, shift_scheduler 58.0%.

**Conclusions (honest):**

1. **The headline holds on fresh, never-seen code** — 57.3% o200k /
   57.9% llama lands right on the real-corpus machine-canon class
   (weighted 58.3%; C# 64.2%); the language generalizes beyond the corpus
   it was built on, not just reproduces it.
2. **The J6 floor finding reproduces exactly**: the canon carries
   signatures + access/type structure (12k lines of type rows the bare JS
   strips), so canon ≈ 110% of the doc-free baseline — the agent-file
   value proposition is vs typed/documented sources, which is the stated
   claim class. The round-tripped JS emit itself (rt_js 760 vs bare 772 —
   98.4%) is as token-lean as a clean JS rendition.
3. **The emitted code is real**: compiles, runs, and behaves on both
   targets — including constructs the corpus never covered (enums,
   operators, generics-in-exprs, switch-expr arms, dict initializers,
   destructure lambdas).
4. The gate also hardened the corpus tooling: `verify` now catches every
   file→canon drift class, not just round-internal churn.

## Remaining

- 6.2 loop at scale (model time), the deferred crates.io publish.