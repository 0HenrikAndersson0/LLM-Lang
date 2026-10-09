# Loop run — task_01_stringops

- adapter: `openai`
- task: CompactLabel — trim/split/initials string ops
- rounds: 1 (cap 3); started 2026-10-09T21:12:22+00:00
- verdict: **PASS**

| round | outcome | detail | IR chars | s |
|---|---|---|---|---|
| 1 | ALL PASS | 8 rig lines | 255 | 13.7 |

Artifacts: `prompt-R*.md` (transcripts), `reply-R*.md` (model replies), `feature-R*.ir`, `feature.ir` (last), `feature.js` (last emit), `feature.map.json` (IR↔JS line map), `rig.js` (cases).

## Final IR

```ir
@1 @ns Bench.Task01

export fn CompactLabel(String s) String{
  var t=s.trim()
  if t.length==0{return "-"}
  var ws=t.split(" ").filter(w=>w.length>0)
  var label=ws.map(w=>w[0].toUpperCase()).join("")
  return label.length>6 ? label.slice(0,6) : label
}
```

