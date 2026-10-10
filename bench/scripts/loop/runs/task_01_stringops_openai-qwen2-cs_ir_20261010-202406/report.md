# Loop run — task_01_stringops (ir-cs world)

- lang/world: `ir-cs` — guide + to-cs emit + dotnet rig
- adapter: `openai`, model: `qwen2.5-coder:7b`
- task: CompactLabel — trim/split/initials string ops
- rounds: 4 (cap 4); started 2026-10-10T20:25:10+00:00
- verdict: **FAIL: 4 rounds exhausted**

| round | outcome | detail | src chars | s |
|---|---|---|---|---|
| 1 | runtime-fail | The build failed. Fix the build errors and run again. | 318 | 14.2 |
| 2 | runtime-fail | FAIL CompactLabel(["hello world"]) -> "hellow" (expected "HW") | 318 | 16.4 |
| 3 | runtime-fail | FAIL CompactLabel(["hello world"]) -> "hellow" (expected "HW") | 318 | 13.7 |
| 4 | runtime-fail | FAIL CompactLabel(["hello world"]) -> "hellow" (expected "HW") | 318 | 20.2 |

Artifacts: `prompt-R*.md`, `reply-R*.md`, `feature-R*.ir` (model drafts), `feature.cs` (last emit), `rig.cs` (dotnet rig), `t.csproj`.
