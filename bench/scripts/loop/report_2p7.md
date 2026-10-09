# PLAN 2.7 — LLM loop test report

**Verdict: GO (local-model gate passed).** The loop — write IR → transpile →
run → feed mapped errors back → repair — works end to end; cold models write
corpus-real IR from the one-page guide, and a real model repaired from mapped
runtime errors. Runs live in `runs/` (regenerate: `python3 llm_loop.py selftest`).

## Run evidence (2026-10-09, ollama on localhost, temp 0)

| run | model | task | rounds | result |
|---|---|---|---|---|
| `task_01_stringops_openai-drive_20261009-210803` | qwen2.5-coder:7b | CompactLabel | 1/4 | **PASS**, 12.9 s |
| `task_01_stringops_openai-qwen2.5-coder:1_20261009-210845` | qwen2.5-coder:1.5b | CompactLabel | 1/4 | **PASS**, 8.0 s |
| `task_02_parseduration_openai-qwen2_20261009-211146` | qwen2.5-coder:7b | ParseDuration (compound h/m/s) | 1/4 | **PASS**, 10.6 s |
| `task_01_stringops_openai-qwen2_20261009-211208` | qwen2.5-coder:7b | CompactLabel — **repair phase** | 1/3 | **PASS**, 13.7 s |
| `task_02_parseduration_openai-qwen2_20261009-210900` | qwen2.5-coder:7b | ParseDuration | 4/4 | FAIL — emitter bug, not model bug (below) |

- **Writability:** both models one-shot task_01 cold from `teach.md` alone.
  The 1.5b used correct idioms at its own volition (`charAt(0)`, `substring`).
  task_02 (digit-walk loop, `parseInt`, char compares, `break`) also one-shot
  by the 7b.
- **Repairability:** the repair-phase run seeds a realistic draft bug (`w.ToFirstUpper()`
  — a C#-ism this corpus's authors would plausibly make). The model received
  the mapped error (`TypeError … feature.js` → **IR line** via the 2.6 span
  map) and repaired to `w[0].toUpperCase()` in one round. All runs' repair
  prompts are archived (`prompt-R*.md`: full transcripts with mapped IR-line
  excerpts and `==>>` markers).

## The loop as QA: emitter truths found and fixed while running 2.7

The FAIL row is the loop test doing its job — the model's draft was correct;
the emitter was broken. Repaired in this session, corpus re-verified 26/26
`node --check` after each fix:

1. `export fn` expanded as `var fn` → `const fn;` (canon printer emits
   export-first; corpus never spelled it — latent).
2. `var mut x=0` → `const mut;` + canon order (`mut var` → `var mut`,
   grammar `var [mut] ident`); mutability now emits `let`.
3. Fn-tail bare expressions didn't return (`acc;`) — corpus 06 returns
   `undefined` — now ret-gated `return`.
4. **else-if chain ownership** (this run's failure): `if c{…}` inline-closed
   + own-line `else if` drained into the enclosing while's arms (never
   emitted) instead of folding into the nested `if`. `expand_stmt` now
   drains fragments only for `if`/`match`; else-frags are sibling-folded by
   `assoc_list` (regression test in expander).
5. Earlier same session: first body statement fused onto `{`; C-style-for
   and `[.. x]` spread lowered (cs_02 restored).

## Known limits of this evidence (honest caveats)

- **Model scale:** qwen2.5-coder 1.5b/7b only — small, coding-tuned; frontier
  models are expected to do at least as well on write; repair skill should
  re-confirm when a frontier/local-70B run is available (folded into 5.2's
  multi-model scale report).
- **Repair was seeded:** cold drafts were too good to fail; the repair
  demonstration starts from a realistic buggy draft rather than the model's
  own round-1 mistake. Mechanics already proven by the echo selftest.
- **Guide is emit-verified but narrow:** value-position `match` (bool arms
  and bare-value arms) is a known v0 emitter gap — `teach.md` deliberately
  teaches ternaries for values and `return`-carrying match arms; until the
  gap is filled, tasks avoid those shapes.

## Next

- 2.4 exit: `tsc --noEmit` gate + diff-vs-hand-renditions (footnote [2]).
- Optional: second repair-task family + a bigger-model run for the scale
  note; then Phase 3 (C# target).