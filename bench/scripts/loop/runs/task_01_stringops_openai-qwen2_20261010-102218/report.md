# Loop run — task_01_stringops

- lang/world: `ir` — guide + transpile + span map
- adapter: `openai`
- task: CompactLabel — trim/split/initials string ops
- rounds: 1 (cap 6); started 2026-10-10T10:22:33+00:00
- verdict: **PASS**

| round | outcome | detail | src chars | s |
|---|---|---|---|---|
| 1 | ALL PASS | 8 rig lines | 314 | 14.5 |

Artifacts: `prompt-R*.md` (transcripts), `reply-R*.md` (model replies), `feature-R*.ir`, `feature.ir` (last), `feature.js` (last emit/run), `rig.js` (cases), `feature.map.json` (IR↔JS line map).

## Final IR

```ir
@1 @ns Demo.CompactLabel

export fn CompactLabel(String s) String{
  var trimmed = s.trim()
  if (trimmed.length == 0) return "-"
  var words = trimmed.split(" ").filter(w => w.length > 0)
  var initials = words.map(w => w[0].toUpperCase()).join("")
  return initials.length > 6 ? initials.slice(0, 6) : initials
}
```

