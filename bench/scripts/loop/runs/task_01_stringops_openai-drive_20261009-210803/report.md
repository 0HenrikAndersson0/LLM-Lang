# Loop run — task_01_stringops

- adapter: `openai`
- task: CompactLabel — trim/split/initials string ops
- rounds: 1 (cap 4); started 2026-10-09T21:08:16+00:00
- verdict: **PASS**

| round | outcome | detail | IR chars | s |
|---|---|---|---|---|
| 1 | ALL PASS | 8 rig lines | 311 | 12.9 |

Artifacts: `prompt-R*.md` (transcripts), `reply-R*.md` (model replies), `feature-R*.ir`, `feature.ir` (last), `feature.js` (last emit), `feature.map.json` (IR↔JS line map), `rig.js` (cases).

## Final IR

```ir
@1 @ns Demo.CompactLabel

export fn CompactLabel(String s) String{
  s = s.trim()
  if s.length == 0 { return "-" }
  var words = s.split(" ").filter(word => word.length > 0)
  var initials = words.map(word => word[0].toUpperCase()).join("")
  if initials.length > 6 { return initials.slice(0, 6) }
  initials
}
```

