The issue in the current implementation is that the `words.map` function is not properly returning the transformed words. The `map` function requires that each transformation returns a value, and in the current implementation, the transformation is being done in place without returning the transformed word.

To fix this, we need to ensure that each transformation returns the transformed word. We can do this by wrapping the transformation in a return statement.

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
      else return w[0].toUpperCase()+w.slice(1)
    })
    .join("")
  })
  .slice(0, 3)
}
```