## USER

# Task: Tidy

Write the complete .js module (one fenced js block) implementing exactly:

```
export function Tidy(xs)
```

Semantics:
1. trim each entry with `s.trim()` and keep only nonempty results (an entry of only spaces is dropped)
2. camelize each: split the trimmed entry on single spaces into words (`s.split(" ")`, keep nonempty words); lowercase the first word's whole text with `w.toLowerCase()`; for every following word, capitalize only its first character via `w[0].toUpperCase()+w.slice(1)`; join the words with no separator
3. keep at most the first 3 entries (`xs.slice(0, 3)`)

Example: `["Foo Bar", "qux", "", "  Baz Qux  ", "many words here", "one"]` -> `["fooBar", "qux", "bazQux"]`.

API hints: standard JS array/string methods (`xs.filter`, `xs.map`, `xs.slice`, `s.split`, `s.trim`, `w[0]`, `w.toUpperCase`). Block-bodied arrow functions with several statements are fine.
