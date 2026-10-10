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

# Task: three-feature module

Write the complete .ir file (one fenced ir block) implementing exactly three exported functions:

```
export fn CompactLabel(String s) String
export fn ParseDuration(String s) Int
export fn Tidy(List<String> xs) List<String>
```

## CompactLabel(String s) String
1. trim surrounding whitespace from `s`
2. if the result is empty, return `-`
3. split on single spaces; keep only nonempty words (runs of spaces collapse)
4. take the first character of each word, uppercased
5. concatenate those characters; if the result is longer than 6 characters, return its first 6 characters, otherwise the concatenation

Example: `"hello brave new world"` -> `"HBNW"`; `""` -> `"-"`.

## ParseDuration(String s) Int
1. `s` is a sequence of segments; each segment = one or more digit characters followed by exactly one unit character `h`, `m`, or `s` (e.g. `"90s"`, `"1m30s"`, `"1h0m30s"`).
2. `h` = 3600 seconds, `m` = 60 seconds, `s` = 1 second. Sum all segments.
3. If `s` is empty, has no segments at all, a segment has no digits, ends without a unit, has a unit other than h/m/s, or contains anything else — return `0`.
4. There are no spaces, no signs, no decimal points. Strings are ASCII.

API hints: parse an integer from an ASCII digit run with `parseInt(str)`; compare characters with `<`/`>`; index characters with `s[i]` (a 1-char string; out-of-range indexes give `undefined`); walk with `s.length`.

## Tidy(List<String> xs) List<String>
1. trim each entry with `s.trim()` and keep only nonempty results (an entry of only spaces is dropped)
2. camelize each: split the trimmed entry on single spaces into words (`s.split(" ")`, keep nonempty words); lowercase the first word's whole text with `w.toLowerCase()`; for every following word, capitalize only its first character via `w[0].toUpperCase()+w.slice(1)`; join the words with no separator
3. keep at most the first 3 entries (`xs.slice(0, 3)`)

Example: `["Foo Bar", "qux", "", "  Baz Qux  ", "many words here", "one"]` -> `["fooBar", "qux", "bazQux"]`.

API hints: list/string methods as in the guide (`xs.filter`, `xs.map`, `xs.slice`, `s.split`, `s.trim`, `w[0]`, `w.toUpperCase`). Block-bodied lambdas with several statements are fine (guide rule 10).
