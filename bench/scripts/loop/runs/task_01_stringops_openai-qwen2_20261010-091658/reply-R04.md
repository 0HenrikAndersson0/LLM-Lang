```
@1 @ns Demo.CompactLabel

export fn CompactLabel(String s) String{
  var trimmed = s.trim()
  if (trimmed.length == 0) return "-"
  var words = trimmed.split(" ").filter(w => w.length > 0)
  var initials = words.map(w => w[0].toUpperCase()).join("")
  if (initials.length > 6) return initials.slice(0, 6)
  return initials
}
```