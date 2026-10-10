## USER

# Task: three-feature module

Write the complete .js module (one fenced js block) implementing exactly three exported functions:

```
export function CompactLabel(s)
export function ParseDuration(s)
export function Tidy(xs)
```

## CompactLabel(s)
1. trim surrounding whitespace from `s`
2. if the result is empty, return `-`
3. split on single spaces; keep only nonempty words (runs of spaces collapse)
4. take the first character of each word, uppercased
5. concatenate those characters; if the result is longer than 6 characters, return its first 6 characters, otherwise the concatenation

Example: `"hello brave new world"` -> `"HBNW"`; `""` -> `"-"`.

## ParseDuration(s)
1. `s` is a sequence of segments; each segment = one or more digit characters followed by exactly one unit character `h`, `m`, or `s` (e.g. `"90s"`, `"1m30s"`, `"1h0m30s"`).
2. `h` = 3600 seconds, `m` = 60 seconds, `s` = 1 second. Sum all segments.
3. If `s` is empty, has no segments at all, a segment has no digits, ends without a unit, has a unit other than h/m/s, or contains anything else — return `0`.
4. There are no spaces, no signs, no decimal points. Strings are ASCII.

API hints: parse an integer from an ASCII digit run with `parseInt(str)`; compare characters with `<`/`>`; index characters with `s[i]` (a 1-char string; out-of-range indexes give `undefined`); walk with `s.length`.

## Tidy(xs)
1. trim each entry with `s.trim()` and keep only nonempty results (an entry of only spaces is dropped)
2. camelize each: split the trimmed entry on single spaces into words (`s.split(" ")`, keep nonempty words); lowercase the first word's whole text with `w.toLowerCase()`; for every following word, capitalize only its first character via `w[0].toUpperCase()+w.slice(1)`; join the words with no separator
3. keep at most the first 3 entries (`xs.slice(0, 3)`)

Example: `["Foo Bar", "qux", "", "  Baz Qux  ", "many words here", "one"]` -> `["fooBar", "qux", "bazQux"]`.

API hints: standard JS array/string methods (`xs.filter`, `xs.map`, `xs.slice`, `s.split`, `s.trim`, `w[0]`, `w.toUpperCase`). Block-bodied arrow functions with several statements are fine.
