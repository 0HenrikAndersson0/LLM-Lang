The filter drops empty trims, then camelize keeps the first word lowercased and capitalizes the first char of the rest. Empty trimmed entries are already gone before the map, so every entry has at least one word. The result for empty `words.slice(1)` is `""`, which concatenates cleanly.

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