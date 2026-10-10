# Loop run — task_03_tidylist

- lang/world: `ir` — guide + transpile + span map
- adapter: `openai`
- task: Tidy — chained pipelines + block-bodied lambdas (J4/J5 idioms)
- rounds: 6 (cap 6); started 2026-10-10T10:23:18+00:00
- verdict: **FAIL: 6 rounds exhausted**

| round | outcome | detail | src chars | s |
|---|---|---|---|---|
| 1 | runtime-fail | FAIL Tidy(["Foo Bar","qux","","  Baz Qux  ","Hello World Again","one"]) -> ["fooBar","qux","BazQux"] (expected ["fooBar","qux","bazQux"]) | 307 | 7.1 |
| 2 | runtime-fail | FAIL Tidy(["Foo Bar","qux","","  Baz Qux  ","Hello World Again","one"]) -> ["fooBar","qux","BazQux"] (expected ["fooBar","qux","bazQux"]) | 307 | 7.4 |
| 3 | runtime-fail | FAIL Tidy(["Foo Bar","qux","","  Baz Qux  ","Hello World Again","one"]) -> ["fooBar","qux","BazQux"] (expected ["fooBar","qux","bazQux"]) | 307 | 6.9 |
| 4 | runtime-fail | FAIL Tidy(["Foo Bar","qux","","  Baz Qux  ","Hello World Again","one"]) -> ["fooBar","qux","BazQux"] (expected ["fooBar","qux","bazQux"]) | 307 | 7.8 |
| 5 | runtime-fail | FAIL Tidy(["Foo Bar","qux","","  Baz Qux  ","Hello World Again","one"]) -> ["fooBar","qux","BazQux"] (expected ["fooBar","qux","bazQux"]) | 307 | 7.5 |
| 6 | runtime-fail | FAIL Tidy(["Foo Bar","qux","","  Baz Qux  ","Hello World Again","one"]) -> ["fooBar","qux","BazQux"] (expected ["fooBar","qux","bazQux"]) | 307 | 8.3 |

Artifacts: `prompt-R*.md` (transcripts), `reply-R*.md` (model replies), `feature-R*.ir`, `feature.ir` (last), `feature.js` (last emit/run), `rig.js` (cases), `feature.map.json` (IR↔JS line map).

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

