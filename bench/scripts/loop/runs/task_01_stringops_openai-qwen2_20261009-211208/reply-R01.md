```ir
@1 @ns Bench.Task01

export fn CompactLabel(String s) String{
  var t=s.trim()
  if t.length==0{return "-"}
  var ws=t.split(" ").filter(w=>w.length>0)
  var label=ws.map(w=>w[0].toUpperCase()).join("")
  return label.length>6 ? label.slice(0,6) : label
}

```