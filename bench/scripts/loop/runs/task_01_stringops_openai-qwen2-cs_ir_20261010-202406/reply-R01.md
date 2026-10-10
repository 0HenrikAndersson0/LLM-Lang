```
@1 @ns Acme.Labels

export fn CompactLabel(String s) String{
  var trimmed=s.Trim()
  if trimmed.length==0{return "-"}
  var chars=trimmed.Split(' ')
    .Where(w=>w.length>0)
    .Select(w=>char.ToUpperInvariant(w[0]))
  var result=string.Join("", chars)
  if result.length>6{return result.Substring(0, 6)}
  result
}
```