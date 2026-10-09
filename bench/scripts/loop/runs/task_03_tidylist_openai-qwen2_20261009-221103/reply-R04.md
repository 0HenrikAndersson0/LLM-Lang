The issue in the current implementation is that the `map` function is not being returned properly. The `map` function should return a new list, and the current implementation is not doing that.

Here is the corrected IR file:

```
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