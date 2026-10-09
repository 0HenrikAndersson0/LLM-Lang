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
4. `match` arms are literal patterns (`"A"`, `404`, `else`) whose bodies are
   statements — inside a function, arm bodies `return` their value (switch
   cases do not fall through and do not return implicitly). For inline
   value choices use ternaries instead of match.
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

## USER

# Task: ParseDuration

Write the complete .ir file (one fenced ir block) implementing exactly:

```
export fn ParseDuration(String s) Int
```

Semantics (must be exact):
1. `s` is a sequence of segments; each segment = one or more digit characters followed by exactly one unit character `h`, `m`, or `s` (e.g. `"90s"`, `"1m30s"`, `"1h0m30s"`).
2. `h` = 3600 seconds, `m` = 60 seconds, `s` = 1 second. Sum all segments.
3. If `s` is empty, has no segments at all, a segment has no digits, ends without a unit, has a unit other than h/m/s, or contains anything else — return `0`.
4. There are no spaces, no signs, no decimal points. Strings are ASCII.

API hints (real JS runtime): parse an integer from an ASCII digit run with `parseInt(str)`; compare characters with `<`/`>`; index characters with `s[i]` (a 1-char string; out-of-range indexes give `undefined`); walk with `s.length`.

Follow the guide. Deterministic output; explicit `return` everywhere is safest.

## Reply

(complete .ir file in one fenced ir block)
