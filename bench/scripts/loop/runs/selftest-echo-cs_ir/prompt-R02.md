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

# Task: CompactLabel

Write the complete .ir file (one fenced ir block) implementing exactly:

```
export fn CompactLabel(String s) String
```

Semantics:
1. Trim surrounding whitespace from `s` (`s.Trim()`)
2. If the trimmed result is empty, return `-`
3. Split on single spaces; keep only nonempty words (runs of spaces collapse)
4. Take the first character of each word, UPPERCASED (`char.ToUpper(...)` / `char.ToUpperInvariant`)
5. Concatenate the characters; if the result is longer than 6 characters, return its first 6 characters, otherwise the concatenation

Example: `"hello brave new world"` -> `"HBNW"`; `""` -> `"-"`.

Runtime: real C# (.NET 10, ESM-free SDK console). Idiomatic .NET method names — `Trim()`, `Split(' ')`, `char.ToUpper(w[0])`, LINQ `Where`/`Select`/`string.Join`, `Substring(0, 6)`. NO JS-isms (no `toFirstUpper`, no JS `.toUpperCase()` method, no `slice`). The canon cases call `CompactLabel(s)` with one String argument (7 cases).


## ASSISTANT

@1 @ns Bench.Task01

export fn CompactLabel(String s) String{
  var t=s.Trim()
  if t.Length==0{return "-"}
  var ws=t.Split(' ').Where(w=>w.Length>0).ToArray()
  var label=string.Join("", ws.Select(w=>w[0]))
  return label.Length>6 ? label.Substring(0,6) : label
}


## USER

## Reported errors

```
/Users/henrikandersson/Developer/LLM-Lang/bench/scripts/loop/runs/selftest-echo-cs_ir/rig.cs(19,48): warning CS8632: The annotation for nullable reference types should only be used in code within a '#nullable' annotations context. [/Users/henrikandersson/Developer/LLM-Lang/bench/scripts/loop/runs/selftest-echo-cs_ir/t.csproj]
/Users/henrikandersson/Developer/LLM-Lang/bench/scripts/loop/runs/selftest-echo-cs_ir/rig.cs(24,46): warning CS8632: The annotation for nullable reference types should only be used in code within a '#nullable' annotations context. [/Users/henrikandersson/Developer/LLM-Lang/bench/scripts/loop/runs/selftest-echo-cs_ir/t.csproj]
/Users/henrikandersson/Developer/LLM-Lang/bench/scripts/loop/runs/selftest-echo-cs_ir/rig.cs(15,14): warning CS8632: The annotation for nullable reference types should only be used in code within a '#nullable' annotations context. [/Users/henrikandersson/Developer/LLM-Lang/bench/scripts/loop/runs/selftest-echo-cs_ir/t.csproj]
/Users/henrikandersson/Developer/LLM-Lang/bench/scripts/loop/runs/selftest-echo-cs_ir/rig.cs(29,27): warning CS8632: The annotation for nullable reference types should only be used in code within a '#nullable' annotations context. [/Users/henrikandersson/Developer/LLM-Lang/bench/scripts/loop/runs/selftest-echo-cs_ir/t.csproj]
PASS CompactLabel([""]) -> "-"
PASS CompactLabel(["   "]) -> "-"
PASS CompactLabel(["  Multiple   Spaces   Here  "]) -> "MSH"
FAIL CompactLabel(["hello world"]) -> "hw" (expected "HW")
FAIL CompactLabel(["hello brave new world"]) -> "hbnw" (expected "HBNW")
FAIL CompactLabel(["Hello world"]) -> "Hw" (expected "HW")
FAIL CompactLabel(["a b c d e f g h i"]) -> "abcdef" (expected "ABCDEF")
RESULT: 4 failing

```

## Your current IR (numbered)

```
  1 @1 @ns Bench.Task01
  2 
  3 export fn CompactLabel(String s) String{
  4   var t=s.Trim()
  5   if t.Length==0{return "-"}
  6   var ws=t.Split(' ').Where(w=>w.Length>0).ToArray()
  7   var label=string.Join("", ws.Select(w=>w[0]))
  8   return label.Length>6 ? label.Substring(0,6) : label
  9 }
```

Compare what your code does for each failing case above with the expected value. Name the exact line to change, then re-emit the COMPLETE corrected ir file in one fenced ir block.

## Reply

(complete ir file in one fenced ir block)
