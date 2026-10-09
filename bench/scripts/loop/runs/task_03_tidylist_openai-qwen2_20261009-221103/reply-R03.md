The error is due to the incorrect placement of the `else` statement inside the `map` function. The `else` statement should be after the `return w[0].toUpperCase()+w.slice(1)` statement, not before it.

Here is the corrected IR file:

```
@1 @ns Demo.Tidy

export fn Tidy(List<String> xs) List<String>{
  xs.filter(s=>{
    var t=s.trim()
    t.length>0
  })
  .map(s=>{
    var words=s.split(" ")
    words.map(w=>{
      if(words.indexOf(w)==0) return w.toLowerCase()
      return w[0].toUpperCase()+w.slice(1)
    })
    .join("")
  })
  .slice(0, 3)
}
```