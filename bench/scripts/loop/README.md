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

# real run — driven by hand/agent (harness waits on the reply file)
python3 llm_loop.py run --task tasks/task_01_stringops.task.json \
  --adapter file --reply-file /tmp/reply.md
# → do the transcript at runs/<id>_file_<stamp>/prompt-R01.md, write answer to /tmp/reply.md
```

Each run gets `runs/<task>_<adapter>_<stamp>/` with per-round transcripts,
IR, emitted JS, the span map, the rig, and `report.md` (round table +
verdict). The 2.7 deliverable is the real-model report(s) in this directory.