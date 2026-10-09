The issue is that I'm using semicolons in lambda bodies, but the IR format prohibits semicolons (rule: "No comments, no semicolons"). Statements in lambda blocks should be separated by newlines only. The last expression becomes the return value.

```ir
@1 @ns Tidy

export fn Tidy(List<String> xs) List<String>{
  xs.filter(s=>{var t=s.trim()
t.length>0})
    .map(s=>{var words=s.split(" ")
var result=""
for i in 0..words.length{i==0?result=words[i].toLowerCase():var w=words[i];result=result+w[0].toUpperCase()+w.slice(1)}
result})
    .slice(0,3)
}
```