## SYSTEM

# IR language — minimal guide for writing code

This file is the model-facing teaching material for the loop test (PLAN 2.7).
Every example here is verified to emit runnable JS by the harness selftest.

## File shape

- First line: `@1 @ns Domain.Name`
- One declaration per logical line; two-space indent for block children;
  braceless child lines form the body of the line above them.
- `{` fuses to the preceding word (`fn F(Bool ok){`). Expressions are tight
  (`a==b`, `x+y`) but keep one space between words (`var mut total`, `Int id`).
- No comments, no semicolons.

## A complete file

```
@1 @ns Demo.Guide

export fn Cap(String s) String{s.length==0 ? "" : s[0].toUpperCase()+s.slice(1)}
export fn Sum(Int n) Int{
  var mut total=0
  for i in 0..n{total+=i}
  total
}
export fn Describe(String code) String{
  match code{
    "A" return "alpha"
    "B" return "beta"
    else return "?"
  }
}
export fn Guard(Int v) Int{
  try {
    if v<0{throw Error("neg")}
    return v*2
  }
  catch Error e return 0
  finally {}
}
```

## Rules that matter

1. `export` functions that callers need. Types (`String`, `Int`,
   `List<String>`) are written in signatures but carry no runtime weight.
2. `var name=expr` is immutable; `var mut name=expr` is reassignable.
3. Control flow (`if`/`for`/`while`/`match`/`try`) is *statement* shaped.
   For values inside expressions use ternaries: `cond ? a : b`.
4. `match` arms are literal patterns (`"A"`, `404`, `true`, `else`) whose
   bodies are statements — inside a function, arm bodies `return` their
   value (switch cases do not fall through and do not return implicitly).
   For inline value choices use ternaries instead of match.
5. Strings interpolate with braces: `"Hello, {name}!"` — a template literal.
6. Method calls spell like the runtime library: `s.trim()`, `s.split(" ")`,
   `s.toUpperCase()`, `s.slice(0,n)`, `xs.filter(p)`, `xs.map(f)`,
   `xs.join("")`, `xs.at(i)`, `n.toFixed(x)`.
7. A body may end with a bare expression — that value is returned when a
   return type is declared (`total` on its own line, above). Inside `try`
   blocks and `if`/`else` branches, always write explicit `return`. There is
   never harm in explicit `return`.
8. Null handling: `T?` (nullable type), `x ?? fallback`, `x?.method`.
   Boolean ops: `!x`, `&&`, `||` — there is no `and`/`not` keyword.
9. Cross-module use: declare `@use Path.To.Module` (after the `@ns` line);
   a module used with a qualified call (`Module.fn(...)`) lowers to a
   namespace import — write calls qualified with the module's last path
   segment:

   ```
   @1 @ns Acme.Report @use Acme.Helpers

   export fn Total(Int n) Int{Helpers.Double(n)+1}
   ```

10. Pipelines chain on dots; a chained call may sit on the NEXT line and a
    lambda may own a multi-statement block body — statements inside are
    separated by keywords and a bare terminal expression is the lambda's
    return value:

    ```
    export fn Tidy(List<String> xs) List<String>{
      xs.filter(s=>{
        var t=s.trim()
        t.length>0
      })
      .map(s=>{s.toLowerCase()})
      .slice(0, 3)
    }
    ```

## USER

# Task: Tidy

Write the complete .ir file (one fenced ir block) implementing exactly:

```
export fn Tidy(List<String> xs) List<String>
```

Semantics:
1. trim each entry with `s.trim()` and keep only nonempty results (an entry of only spaces is dropped)
2. camelize each: split the trimmed entry on single spaces into words (`s.split(" ")`, keep nonempty words); lowercase the first word's whole text with `w.toLowerCase()`; for every following word, capitalize only its first character via `w[0].toUpperCase()+w.slice(1)`; join the words with no separator
3. keep at most the first 3 entries (`xs.slice(0, 3)`)

Example: `["Foo Bar", "qux", "", "  Baz Qux  ", "many words here", "one"]` -> `["fooBar", "qux", "bazQux"]`.

API hints: list/string methods as in the guide (`xs.filter`, `xs.map`, `xs.slice`, `s.split`, `s.trim`, `w[0]`, `w.toUpperCase`). Block-bodied lambdas with several statements are fine (guide rule 10).

## ASSISTANT



## USER

Your reply contained no fenced ```ir block. Re-emit the complete .ir file in one fenced ir block.

## ASSISTANT

```ir
@1 @ns Tidy

export fn Tidy(List<String> xs) List<String>{
  xs.filter(s=>{var t=s.trim();t.length>0})
    .map(s=>{
      var words=s.split(" ")
      var result=""
      for i in 0..words.length{
        if i==0{result=words[i].toLowerCase()}
        else{var w=words[i];result=result+w[0].toUpperCase()+w.slice(1)}
      }
      result
    })
    .slice(0,3)
}
```

## USER

## Reported errors

```
file:///Users/henrikandersson/Developer/LLM-Lang/bench/scripts/loop/runs/task_03_tidylist_openai-qwen3_20261009-222702/feature.js:4
  return }).slice(0, 3);
          ^

SyntaxError: Unexpected token ')'
    at compileSourceTextModule (node:internal/modules/esm/utils:318:16)
    at ModuleLoader.moduleStrategy (node:internal/modules/esm/translators:99:18)
    at #translate (node:internal/modules/esm/loader:473:20)
    at afterLoad (node:internal/modules/esm/loader:529:29)
    at ModuleLoader.loadAndTranslate (node:internal/modules/esm/loader:534:12)
    at #getOrCreateModuleJobAfterResolve (node:internal/modules/esm/loader:577:36)
    at afterResolve (node:internal/modules/esm/loader:625:52)
    at ModuleLoader.getOrCreateModuleJob (node:internal/modules/esm/loader:631:12)
    at ModuleJob.syncLink (node:internal/modules/esm/module_job:160:33)
    at ModuleJob.link (node:internal/modules/esm/module_job:245:17)

Node.js v24.14.0
```

## Your current IR (numbered)

```
  1 @1 @ns Tidy
  2 
  3 export fn Tidy(List<String> xs) List<String>{
  4   xs.filter(s=>{var t=s.trim();t.length>0})
  5     .map(s=>{
  6       var words=s.split(" ")
  7       var result=""
  8       for i in 0..words.length{
  9         if i==0{result=words[i].toLowerCase()}
 10         else{var w=words[i];result=result+w[0].toUpperCase()+w.slice(1)}
 11       }
 12       result
 13     })
 14     .slice(0,3)
 15 }
```

Compare what your code does for each failing case above with the expected value. Name the exact line to change, then re-emit the COMPLETE corrected .ir file in one fenced ir block.

## ASSISTANT

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

## USER

Your IR did not transpile. Errors:
- expand: 7:1: line dedents under open brace (indent 0 <= 0)
Re-emit the complete corrected .ir file.

## Reply

(complete .ir file in one fenced ir block)
