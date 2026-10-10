# Loop run — task_03_tidylist

- lang/world: `ir` — guide + transpile + span map
- adapter: `file`
- task: Tidy — chained pipelines + block-bodied lambdas (J4/J5 idioms)
- rounds: 1 (cap 4); started 2026-10-10T10:21:54+00:00
- verdict: **PASS**

| round | outcome | detail | src chars | s |
|---|---|---|---|---|
| 1 | ALL PASS | 6 rig lines | 329 | 7.2 |

Artifacts: `prompt-R*.md` (transcripts), `reply-R*.md` (model replies), `feature-R*.ir`, `feature.ir` (last), `feature.js` (last emit/run), `rig.js` (cases), `feature.map.json` (IR↔JS line map).

## Final IR

```ir
@1 @ns Bench.Tidy

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

