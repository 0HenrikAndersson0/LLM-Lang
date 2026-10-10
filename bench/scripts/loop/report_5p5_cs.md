# 5.5 — LLM loop test for C# (first runs)

> The 2.7 loop harness (`bench/scripts/loop/llm_loop.py`) gained the two
> C# worlds: **`ir-cs`** (guide + IR + `irc to-cs` + dotnet rig — mirrors
> the ir/JS-world's transpile+repair loop) and **`cs`** (baseline: the
> model writes C# directly — the J6-analog). Task rows gained
> `spec_cs_ir`/`spec_cs` and `echo_draft/fix_cs_ir`/`echo_draft/fix_cs`
> (task_01_stringops; the draft/fix pair carries the same planted bug
> shape as the JS rows: a missing `char.ToUpper` — draft fails the
> case values, fix repairs). Measured 2026-10-10, qwen2.5-coder:7b,
> 4-round cap, 16k ctx? no — 8192 ctx, 1024/2048 num-predict.

## Echo selftests (deterministic; all four worlds)

| world | selftest | result |
|---|---|---|
| ir | selftest-echo | **PASS** (draft repaired via mapped feedback) |
| js | selftest-echo-js | **PASS** |
| ir-cs | selftest-echo-cs_ir | **PASS** (draft → dotnet runtime-fail → `/* mapped */` repair → ALL PASS) |
| cs | selftest-echo-cs | **PASS** |

Loop-as-QA found and fixed on landing: the C# rig emitted `public static
static` rows (double-`static` — the synth-class pre-push + the in_static
branch), the rig's `Rig` type trailed C# top-level statements (CS8803 —
the rig type now lives in `riglib.cs`, global namespace), `JsonNode
DeepEquals`/`JsonSerializer` usings, `JsonNode.Deserialize` extension
using, the case-`args` shadowing, and the per-round archive rows
compiling together under the dotnet SDK glob (cs-world round archives
now carry non-`.cs` suffixes — `.csrc`).

## Real-model runs (task_01_stringops, qwen2.5-coder:7b, 4 rounds)

| world | run dir | verdict | path |
|---|---|---|---|
| **cs baseline** | `task_01_stringops_openai-qwen2-cs_20261010-202520` | **PASS round 1** | one-shot cold; the model wrote idiomatic .NET (`Split(' ', StringSplitOptions.RemoveEmptyEntries)`, `char.ToUpperInvariant`) |
| ir-cs | `task_01_stringops_openai-qwen2-cs_ir_20261010-202406` | **FAIL 4 rounds** — round 1 build-fail → repaired to a runnable program round 2 (dotnet build errors fed raw → repaired); rounds 2–4 = value near-miss (`"hellow"` — the words-not-initials miss) |

## Reading

- **The cs baseline one-shot is the strongest C#-target loop signal yet**:
  qwen2.5-coder:7b cold-passed the full task in one round with idiomatic
  C# — mirroring the JS-world's task_01/qwen-7b one-shot (2.7).
- **The ir-cs transpiled world shows the same 2.7 repair shape**: build
  errors (raw dotnet feedback, no span map v0) were repaired in one
  round. The value miss (`"hellow"`, initials vs words) is the same
  class that held task_03's JS-world qwen-7b — a semantic near-miss the
  small-scale model doesn't climb out of from value-FAIL feedback alone;
  the JS loop solved it with the mapped-IR feedback v0 lacking here
  (to-cs spans are queued for the 5.6 sourcemap parity).
- The loop-as-QA discipline again paid for itself: 6 rig/emitter truths
  were fixed on landing (see Echo selftests above + the `Split(" ")`
  rows run real — `string.Split(string)` was *unknown-territory* and
  passed on net10).

## Boundary

- task rows: only task_01 carries the cs-world rows; task_02–04
  (parseduration/tidylist/multi) remain JS-only (v0 scope — the
  first runnable cs-world rows; the rest inherit the shapes).
- The ir-cs world has no span map yet (to-cs lacks `--map` parity);
  repair feedback uses raw dotnet errors + the numbered C# (the
  no-map branch of `feedback_text`).
- The char-atom design row (fidelity 17) also shapes what tasks the
  C# world can carry — task_01 is char-light (single-char indexing only,
  which survives), making it the right first task.