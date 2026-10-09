# Loop run — task_01_stringops

- adapter: `openai`
- task: CompactLabel — trim/split/initials string ops
- rounds: 1 (cap 4); started 2026-10-09T21:08:53+00:00
- verdict: **PASS**

| round | outcome | detail | IR chars | s |
|---|---|---|---|---|
| 1 | ALL PASS | 8 rig lines | 322 | 8.0 |

Artifacts: `prompt-R*.md` (transcripts), `reply-R*.md` (model replies), `feature-R*.ir`, `feature.ir` (last), `feature.js` (last emit), `feature.map.json` (IR↔JS line map), `rig.js` (cases).

## Final IR

```ir
@1 @ns Demo.Guide

export fn CompactLabel(String s) String{
  var trimmed = s.trim()
  if trimmed == ""{return "-"}
  var words = trimmed.split(" ").filter(word => word != "")
  var compacted = words.map(word => word.charAt(0).toUpperCase()).join("")
  return compacted.length > 6 ? compacted.substring(0, 6) : compacted
}
```

