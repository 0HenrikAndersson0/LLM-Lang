# J6 — whole-pipeline token measurement (PLAN footnote [1]) — v1, 2026-10-10

**Result: the ≥30% whole-pipeline savings do NOT hold at small-feature scale.
The static compression is real (write side), but loop scaffolding dominates
small tasks. Numbers below; framing and follow-ups at the end.**

Measured with `pipeline_tokens.py --latest` over `llm_loop.py` transcripts
(everything the model read + wrote, across rounds actually spent; o200k_base
tokenizers). Baseline world = `--lang js`: the same tasks/cases/rig, the
model writes `feature.js` directly — no guide, no transpile, no span map.
IR world = guide (system message) + spec + transpile + mapped feedback.

## 1. Delivered features, paired (same task + model, 1-round PASS)

Tokenizer o200k.

| pair | ir total | js total | % | ir read (guide+spec) | ir write | js read (spec) | js write | write % |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| task_01, agent-driven | 959 | 331 | 290% | 864 (guide 675) | 95 | 208 | 173 | 55% |
| task_01, qwen2.5-coder:7b | 1164 | 368 | 316% | 1069 (guide 880) | 95 | 198 | 198 | 48% |
| task_02, qwen2.5-coder:7b | 1184 | 574 | 206% | 967 (guide 675) | 217 | 315 | 259 | 84% |
| task_03, agent-driven | 1329 | 446 | 298% | 1124 (guide 675) | 205 | 290 | 156 | >100% |

The R01 excess over the js baseline is exactly the guide (`884 = 884 + 169 −
182` on task_01 7b: guide 880 vs spec 13 tokens smaller IR-side).

## 2. Session-amortized view (guide paid once per session; the agent-file model)

| pair | ir/feature | js/feature | ir % | break-even S | % @ S=5 | % @ S=10 | % @ S=20 |
|---|---:|---:|---:|---:|---:|---:|---:|
| task_01, agent (o200k) | 284 | 331 | 86% | 14 | 127% | 106% | 96% |
| task_01, 7b (o200k) | 284 | 368 | 77% | 11 | 125% | 101% | 89% |
| task_02, 7b (o200k) | 509 | 574 | 89% | 10 | 112% | 100% | 95% |

(task_03 excluded from this table: its IR marginal exceeds js even
amortized — spec reads heavy, writes equal.)

## 3. What the numbers say

1. **Static compression survives only on the write side.** Model-written IR
   drafts are 48–55% of the js drafts on task_01 (the ~46% one-shot Phase-0
   number, realized in drafts) — but roughly equal on task_02/03.
2. **Per-feature whole-pipeline spend is ~2–3× the js baseline at one-shot**
   (guide paid immediately), and ~77–90% amortized over an N-feature session
   with break-even at S≈10–15. The 30% whole-pipeline goal is NOT met at
   this feature scale; savings peak ≈ 10–23% asymptotically.
3. **Failures dominate everything**: one 6-round value near-miss costs 14.5k
   tokens vs 438 for the js one-shot (33×). Loop reliability > token
   arithmetic.
4. **Scaffolding, not the language, consumes the savings at small scale:**
   spec re-reads per feature (~170–480 tok), the guide, and numbered-IR
   repair feedback are constant-size overheads against 300-char features.
   The write-side advantage should scale with feature size — these tasks
   are ~300-char one-liners.

## 4. Follow-ups (J6 open)

- Re-measure on corpus-scale features (or chained multi-feature sessions) —
  if the write-side advantage scales, savings should approach the static
  number; if not, the design ceiling is ~1.2–1.4× total (write+read).
- Cut scaffolding: smaller repair feedback (only changed lines), spec
  delivered once per session (multi-feature transcript), shorter guide.
- Multi-feature-per-file tasks exercise the actual agent-file claim (the
  `.ir` file AMORTIZES — one header, shared context) — untested here.

## 5. Provenance

- Pairs in §1: `runs/task_0{1,2,3}_*_{openai,file}[_js]_*` (see
  `pipeline_tokens.py --latest --json` for the machine view).
- 2026-10-09 runs measured against the pre-fix emitter (see v4.4
  changelog: braceless-if statement tail + for/while run pieces); the
  paired clean sets here come from post-fix runs (2026-10-10) on
  qwen2.5-coder:7b + agent-driven file adapter.
- gemma4:12b is absent: operationally blocked (reasoning channel consumes
  the token budget, `content` stays empty; ~8 k ctx needed for sane speed
  on a 16 GB machine). Adapter now falls back to the reasoning channel,
  but the runs remain vacuous — recorded as an ollama adapter gap, not a
  language-design result.