# Loop run — task_01_stringops (ir-cs world)

- lang/world: `ir-cs` — guide + to-cs emit + dotnet rig
- adapter: `echo`
- task: CompactLabel — trim/split/initials string ops
- rounds: 2 (cap 4); started 2026-10-27T17:29:42+00:00
- verdict: **PASS**

| round | outcome | detail | src chars | s |
|---|---|---|---|---|
| 1 | runtime-fail | FAIL CompactLabel(["hello world"]) -> "hw" (expected "HW") | 265 | 0.9 |
| 2 | ALL PASS | 12 rig lines | 279 | 0.9 |

Artifacts: `prompt-R*.md`, `reply-R*.md`, `feature-R*.ir` (model drafts), `feature.cs` (last emit), `rig.cs` (dotnet rig), `t.csproj`.
