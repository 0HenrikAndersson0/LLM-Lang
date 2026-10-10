# Loop run — task_04_multi

- lang/world: `ir` — guide + transpile + span map
- adapter: `file`
- task: Multi-feature module — CompactLabel + ParseDuration + Tidy (J6 session-scale)
- rounds: 1 (cap 4); started 2026-10-10T10:32:50+00:00
- verdict: **PASS**

| round | outcome | detail | src chars | s |
|---|---|---|---|---|
| 1 | ALL PASS | 24 rig lines | 967 | 5.6 |

Artifacts: `prompt-R*.md` (transcripts), `reply-R*.md` (model replies), `feature-R*.ir`, `feature.ir` (last), `feature.js` (last emit/run), `rig.js` (cases), `feature.map.json` (IR↔JS line map).

## Final IR

```ir
@1 @ns Bench.Multi

export fn CompactLabel(String s) String{
  var t=s.trim()
  if t.length==0{return "-"}
  var ws=t.split(" ").filter(w=>w.length>0)
  var label=ws.map(w=>w[0].toUpperCase()).join("")
  return label.length>6 ? label.slice(0,6) : label
}

export fn ParseDuration(String s) Int{
  var mut total=0
  var mut i=0
  while i<s.length{
    var numStart=i
    while i<s.length && s[i]>="0" && s[i]<="9"{i+=1}
    if i==numStart{return 0}
    var num=parseInt(s.slice(numStart, i))
    if i>=s.length{return 0}
    match s[i]{
      "h" total+=num*3600
      "m" total+=num*60
      "s" total+=num
      else return 0
    }
    i+=1
  }
  total
}

export fn Tidy(List<String> xs) List<String>{
  xs.filter(s=>{
    var t=s.trim()
    t.length>0
  })
  .map(s=>{
    var words=s.split(" ").filter(w=>w.length>0)
    var first=words[0].toLowerCase()
    var rest=words.slice(1).map(w=>w[0].toUpperCase()+w.slice(1))
    first+rest.join("")
  })
  .slice(0,3)
}
```

