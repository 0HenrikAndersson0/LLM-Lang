The failing cases indicate that the code is not properly handling the case where the first word of a trimmed string is already in lowercase. We need to ensure that the first word is always converted to lowercase, regardless of its original case.

Here is the corrected IR code:

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
      if(words.indexOf(w)==0) w.toLowerCase()
      else w[0].toUpperCase()+w.slice(1)
    })
    .join("")
  })
  .slice(0, 3)
}
```