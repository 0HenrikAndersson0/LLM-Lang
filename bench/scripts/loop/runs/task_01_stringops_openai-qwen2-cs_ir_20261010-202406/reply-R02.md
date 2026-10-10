The error messages indicate that the `length` property and `Substring` method are not recognized by the compiler. This could be due to missing using directives or an issue with the project configuration. However, since the issue is related to the language syntax and not the project setup, we can address it by ensuring that the correct method names are used.

The correct method names for length and substring in C# are `Length` and `Substring`, respectively. Let's update the code accordingly.

Here is the corrected IR file:

```
@1 @ns Acme.Labels

export fn CompactLabel(String s) String{
  var trimmed=s.Trim()
  if trimmed.Length==0{return "-"}
  var chars=trimmed.Split(' ')
    .Where(w=>w.Length>0)
    .Select(w=>char.ToUpperInvariant(w[0]))
  var result=string.Join("", chars)
  if result.Length>6{return result.Substring(0, 6)}
  result
}
```