```
@1 @ns Demo.CompactLabel

export fn CompactLabel(String s) String{
  s = s.trim()
  if s.length == 0 { return "-" }
  var words = s.split(" ").filter(word => word.length > 0)
  var initials = words.map(word => word[0].toUpperCase()).join("")
  if initials.length > 6 { return initials.slice(0, 6) }
  initials
}
```