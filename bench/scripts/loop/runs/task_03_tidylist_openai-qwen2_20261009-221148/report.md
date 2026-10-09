# Loop run — task_03_tidylist

- adapter: `openai`
- task: Tidy — chained pipelines + block-bodied lambdas (J4/J5 idioms)
- rounds: 6 (cap 6); started 2026-10-09T22:12:43+00:00
- verdict: **FAIL: 6 rounds exhausted**

| round | outcome | detail | IR chars | s |
|---|---|---|---|---|
| 1 | runtime-fail | FAIL Tidy(["Foo Bar","qux","","  Baz Qux  ","Hello World Again","one"]) -> ["fooBar","qux","BazQux"] (expected ["fooBar","qux","bazQux"]) | 307 | 5.0 |
| 2 | runtime-fail | FAIL Tidy(["Foo Bar","qux","","  Baz Qux  ","Hello World Again","one"]) -> ["fooBar","qux","BazQux"] (expected ["fooBar","qux","bazQux"]) | 307 | 9.5 |
| 3 | runtime-fail | FAIL Tidy(["Foo Bar","qux","","  Baz Qux  ","Hello World Again","one"]) -> ["fooBar","qux","BazQux"] (expected ["fooBar","qux","bazQux"]) | 307 | 9.7 |
| 4 | runtime-fail | FAIL Tidy(["Foo Bar","qux","","  Baz Qux  ","Hello World Again","one"]) -> ["fooBar","qux","BazQux"] (expected ["fooBar","qux","bazQux"]) | 307 | 10.0 |
| 5 | runtime-fail | FAIL Tidy(["Foo Bar","qux","","  Baz Qux  ","Hello World Again","one"]) -> ["fooBar","qux","BazQux"] (expected ["fooBar","qux","bazQux"]) | 307 | 10.1 |
| 6 | runtime-fail | FAIL Tidy(["Foo Bar","qux","","  Baz Qux  ","Hello World Again","one"]) -> ["fooBar","qux","BazQux"] (expected ["fooBar","qux","bazQux"]) | 307 | 10.5 |

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

