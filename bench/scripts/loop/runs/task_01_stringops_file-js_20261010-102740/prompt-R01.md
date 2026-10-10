## USER

# Task: CompactLabel

Write the complete .js module (one fenced js block) implementing exactly:

```
export function CompactLabel(s)
```

Semantics:
1. trim surrounding whitespace from `s`
2. if the result is empty, return `-`
3. split on single spaces; keep only nonempty words (runs of spaces collapse)
4. take the first character of each word, uppercased
5. concatenate those characters; if the result is longer than 6 characters, return its first 6 characters, otherwise the concatenation

Example: `"hello brave new world"` -> `"HBNW"`; `""` -> `"-"`.

Runtime: real JS (ESM, plain V8 library only). The cases call `CompactLabel(s)` with the arguments listed below (7 cases: one arg, a String) — no CLI, no globals.
