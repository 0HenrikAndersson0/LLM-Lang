# Loop run — task_01_stringops

- lang/world: `ir` — guide + transpile + span map
- adapter: `openai`
- task: CompactLabel — trim/split/initials string ops
- rounds: 6 (cap 6); started 2026-10-10T09:17:42+00:00
- verdict: **FAIL: 6 rounds exhausted**

| round | outcome | detail | src chars | s |
|---|---|---|---|---|
| 1 | runtime-fail | SyntaxError: Unexpected token 'return' | 314 | 10.0 |
| 2 | runtime-fail | SyntaxError: Unexpected token 'return' | 324 | 6.7 |
| 3 | runtime-fail | SyntaxError: Unexpected token 'return' | 314 | 6.4 |
| 4 | runtime-fail | SyntaxError: Unexpected token 'return' | 324 | 6.7 |
| 5 | runtime-fail | SyntaxError: Unexpected token 'return' | 324 | 6.8 |
| 6 | runtime-fail | SyntaxError: Unexpected token 'return' | 324 | 7.3 |

Artifacts: `prompt-R*.md` (transcripts), `reply-R*.md` (model replies), `feature-R*.ir`, `feature.ir` (last), `feature.js` (last emit/run), `rig.js` (cases), `feature.map.json` (IR↔JS line map).

## Final IR

```ir
@1 @ns Demo.CompactLabel

export fn CompactLabel(String s) String{
  var trimmed = s.trim()
  if (trimmed.length == 0) return "-"
  var words = trimmed.split(" ").filter(w => w.length > 0)
  var initials = words.map(w => w[0].toUpperCase()).join("")
  if (initials.length > 6) return initials.slice(0, 6)
  return initials
}
```

