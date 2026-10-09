# LLM loop test harness (PLAN 2.7)

The product's central unverified claim: **an LLM can write and repair IR in a
loop** — unfamiliar-syntax risk means loop-clumsiness could consume the token
savings, so this is the real go/no-go (agreed 2026-10-08).

## Loop

```
model writes feature.ir
  → irc to-js --map          (feature.js + feature.map.json)
  → node rig.js              (task cases; PASS/FAIL/stack trace)
  → failure lines mapped IR-side via the span map (2.6)
  → repair prompt = errors + IR lines with context
  → model re-emits → round N+1 … cap
```

## Adapters

| adapter | driving | notes |
|---|---|---|
| `echo` | canned replies from the task config | verifies the deterministic mechanics with zero model access — `python3 llm_loop.py selftest` |
| `file` | out-of-band (agent session / human) | harness archives `prompt-R{N}.md` (full transcript); poll `--reply-file` for the model's answer |
| `openai` | chat-completions API | env `LLM_API_KEY`, `LLM_MODEL` (or `--model`), `LLM_BASE_URL` for llama.cpp/vLLM/etc |

## Usage

```sh
# mechanics selftest (no API; round 2 must show ALL PASS)
python3 llm_loop.py selftest

# real run — API adapter
LLM_API_KEY=sk-… LLM_MODEL=gpt-5.1 python3 llm_loop.py run \
  --task tasks/task_01_stringops.task.json --adapter openai --rounds 4

# real run — ollama (LLM_NUM_CTX is required; also disables thinking)
LLM_BASE_URL=http://localhost:11434/v1 LLM_API_KEY=ollama LLM_MODEL=qwen2.5-coder:7b \
  LLM_NUM_CTX=16384 python3 llm_loop.py run \
  --task tasks/task_01_stringops.task.json --adapter openai --rounds 6
```

# real run — driven by hand/agent (harness waits on the reply file)
python3 llm_loop.py run --task tasks/task_01_stringops.task.json \
  --adapter file --reply-file /tmp/reply.md
# → do the transcript at runs/<id>_file_<stamp>/prompt-R01.md, write answer to /tmp/reply.md
```

## Real-run findings (2026-10-09, ollama models)

Recorded in `runs/` per run; the operational findings so far:

- **temp-0 repeats verbatim**: a small model (qwen2.5-coder:7b) resends the
  same draft across repair rounds when temperature is 0. The adapter now
  samples repair rounds (`temperature 0.6`) and appends a
  byte-identical-repetition nudge when a reply matches the previous one.
- **ollama context clipping**: the `/v1` endpoint defaults to a small
  context and the transcript grows every round — replies went incoherent/
  verbatim. Set `LLM_NUM_CTX=16384` (sent as `options.num_ctx`).
- **thinking models**: ollama thinking models (qwen3.5) return EMPTY
  `message.content` by default; setting `LLM_NUM_CTX` also sends
  `think:false` so the final answer lands in content.
- **value-failures have no stack trace** (expected-vs-got only), so the
  span map can't localize; the feedback then includes the numbered IR +
  per-case got/expected.
- **task difficulty classes observed**: transpile-errors (repairable —
  models fix these well), value-fail near-misses (model-dependent; 7b
  iterated but didn't converge in 6 rounds on task_03), structural
  (Pythonisms / unindented lines from general — not code-tuned — models).

Each run gets `runs/<task>_<adapter>_<stamp>/` with per-round transcripts,
IR, emitted JS, the span map, the rig, and `report.md` (round table +
verdict). The 2.7 deliverable is the real-model report(s) in this directory.