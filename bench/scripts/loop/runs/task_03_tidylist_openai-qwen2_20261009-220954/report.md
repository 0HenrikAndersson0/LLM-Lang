# Loop run — task_03_tidylist

- adapter: `openai`
- task: Tidy — chained pipelines + block-bodied lambdas (J4/J5 idioms)
- rounds: 4 (cap 4); started 2026-10-09T22:10:16+00:00
- verdict: **FAIL: 4 rounds exhausted**

| round | outcome | detail | IR chars | s |
|---|---|---|---|---|
| 1 | runtime-fail | FAIL Tidy(["Foo Bar","qux","","  Baz Qux  ","Hello World Again","one"]) -> ["fooBar","qux","BazQux"] (expected ["fooBar","qux","bazQux"]) | 307 | 4.9 |
| 2 | runtime-fail | FAIL Tidy(["Foo Bar","qux","","  Baz Qux  ","Hello World Again","one"]) -> ["fooBar","qux","BazQux"] (expected ["fooBar","qux","bazQux"]) | 307 | 5.6 |
| 3 | runtime-fail | FAIL Tidy(["Foo Bar","qux","","  Baz Qux  ","Hello World Again","one"]) -> ["fooBar","qux","BazQux"] (expected ["fooBar","qux","bazQux"]) | 307 | 6.0 |
| 4 | runtime-fail | FAIL Tidy(["Foo Bar","qux","","  Baz Qux  ","Hello World Again","one"]) -> ["fooBar","qux","BazQux"] (expected ["fooBar","qux","bazQux"]) | 307 | 6.0 |

Artifacts: `prompt-R*.md` (transcripts), `reply-R*.md` (model replies), `feature-R*.ir`, `feature.ir` (last), `feature.js` (last emit), `feature.map.json` (IR↔JS line map), `rig.js` (cases).

## Final IR

```ir
@1 @ns Demo.Tidy

export fn Tidy(List<String> xs) List<String>{
  xs.filter(s=>{
    var t=s.trim()
    t.length>0
  })
  .map(s=>{
    var words=s.split(" ")
    words.map(w=>{
      if(words.indexOf(w)==0) w.toLowerCase()
      else w[0].toUpperCase()+w.slice(1)
    })
    .join("")
  })
  .slice(0, 3)
}
```

