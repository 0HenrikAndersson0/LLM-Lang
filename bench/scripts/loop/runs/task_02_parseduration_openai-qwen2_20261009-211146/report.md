# Loop run — task_02_parseduration

- adapter: `openai`
- task: ParseDuration — compound durations (loop + match + parseInt)
- rounds: 1 (cap 4); started 2026-10-09T21:11:57+00:00
- verdict: **PASS**

| round | outcome | detail | IR chars | s |
|---|---|---|---|---|
| 1 | ALL PASS | 12 rig lines | 636 | 10.6 |

Artifacts: `prompt-R*.md` (transcripts), `reply-R*.md` (model replies), `feature-R*.ir`, `feature.ir` (last), `feature.js` (last emit), `feature.map.json` (IR↔JS line map), `rig.js` (cases).

## Final IR

```ir
@1 @ns Demo.ParseDuration

export fn ParseDuration(String s) Int{
  if s.length == 0 { return 0 }

  var mut total = 0
  var mut i = 0

  while i < s.length {
    var mut digits = ""
    while i < s.length && s[i] >= '0' && s[i] <= '9' {
      digits += s[i]
      i += 1
    }

    if digits.length == 0 { return 0 }

    var unit = s[i]
    i += 1

    if unit != 'h' && unit != 'm' && unit != 's' { return 0 }

    var value = parseInt(digits)
    if unit == 'h' { total += value * 3600 }
    else if unit == 'm' { total += value * 60 }
    else if unit == 's' { total += value }

    if i >= s.length { break }
  }

  return total
}
```

