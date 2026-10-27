# Loop run — task_01_stringops

- lang/world: `ir` — guide + transpile + span map
- adapter: `echo`
- task: CompactLabel — trim/split/initials string ops
- rounds: 2 (cap 4); started 2026-10-27T17:48:09+00:00
- verdict: **PASS**

| round | outcome | detail | src chars | s |
|---|---|---|---|---|
| 1 | runtime-fail | ERROR CompactLabel("hello world"): TypeError: w.toFirstUpper is not a function | 253 | 0.0 |
| 2 | ALL PASS | 8 rig lines | 255 | 0.0 |

Artifacts: `prompt-R*.md` (transcripts), `reply-R*.md` (model replies), `feature-R*.ir`, `feature.ir` (last), `feature.js` (last emit/run), `rig.js` (cases), `feature.map.json` (IR↔JS line map).

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

