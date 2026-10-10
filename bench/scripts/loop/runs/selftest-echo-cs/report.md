# Loop run — task_01_stringops (cs world)

- lang/world: `cs` — C# baseline world: no guide, no transpile
- adapter: `echo`
- task: CompactLabel — trim/split/initials string ops
- rounds: 2 (cap 4); started 2026-10-10T21:04:43+00:00
- verdict: **PASS**

| round | outcome | detail | src chars | s |
|---|---|---|---|---|
| 1 | runtime-fail | FAIL CompactLabel(["hello world"]) -> "hw" (expected "HW") | 354 | 1.0 |
| 2 | ALL PASS | 12 rig lines | 368 | 0.9 |

Artifacts: `prompt-R*.md`, `reply-R*.md`, `feature-R*.ir` (model drafts), `feature.cs` (last emit), `rig.cs` (dotnet rig), `t.csproj`.
