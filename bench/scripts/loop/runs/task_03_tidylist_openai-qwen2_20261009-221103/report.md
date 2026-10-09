# Loop run — task_03_tidylist

- adapter: `openai`
- task: Tidy — chained pipelines + block-bodied lambdas (J4/J5 idioms)
- rounds: 4 (cap 4); started 2026-10-09T22:11:40+00:00
- verdict: **FAIL: 4 rounds exhausted**

| round | outcome | detail | IR chars | s |
|---|---|---|---|---|
| 1 | runtime-fail | FAIL Tidy(["Foo Bar","qux","","  Baz Qux  ","Hello World Again","one"]) -> ["fooBar","qux","BazQux"] (expected ["fooBar","qux","bazQux"]) | 307 | 5.3 |
| 2 | runtime-fail | SyntaxError: Unexpected token 'else' | 321 | 11.2 |
| 3 | runtime-fail | FAIL Tidy(["Foo Bar","qux","","  Baz Qux  ","Hello World Again","one"]) -> undefined (expected ["fooBar","qux","bazQux"]) | 316 | 11.1 |
| 4 | runtime-fail | FAIL Tidy(["Foo Bar","qux","","  Baz Qux  ","Hello World Again","one"]) -> ["foobar","qux","bazqux"] (expected ["fooBar","qux","bazQux"]) | 330 | 9.4 |

Artifacts: `prompt-R*.md` (transcripts), `reply-R*.md` (model replies), `feature-R*.ir`, `feature.ir` (last), `feature.js` (last emit), `feature.map.json` (IR↔JS line map), `rig.js` (cases).

## Final IR

```ir
@1 @ns Demo.Tidy

export fn Tidy(List<String> xs) List<String>{
  return xs.filter(s=>{
    var t=s.trim()
    t.length>0
  })
  .map(s=>{
    var words=s.split(" ")
    return words.map(w=>{
      if(words.indexOf(w)==0) return w.toLowerCase()
      return w[0].toUpperCase()+w.slice(1)
    })
    .join("")
  })
  .slice(0, 3)
}
```

