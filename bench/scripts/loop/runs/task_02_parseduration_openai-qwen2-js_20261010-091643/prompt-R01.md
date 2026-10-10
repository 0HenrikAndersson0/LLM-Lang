## USER

# Task: ParseDuration

Write the complete .js module (one fenced js block) implementing exactly:

```
export function ParseDuration(s)
```

Semantics (must be exact):
1. `s` is a sequence of segments; each segment = one or more digit characters followed by exactly one unit character `h`, `m`, or `s` (e.g. `"90s"`, `"1m30s"`, `"1h0m30s"`).
2. `h` = 3600 seconds, `m` = 60 seconds, `s` = 1 second. Sum all segments.
3. If `s` is empty, has no segments at all, a segment has no digits, ends without a unit, has a unit other than h/m/s, or contains anything else — return `0`.
4. There are no spaces, no signs, no decimal points. Strings are ASCII.

API hints: parse an integer from an ASCII digit run with `parseInt(str)`; compare characters with `<`/`>`; index characters with `s[i]` (a 1-char string; out-of-range indexes give `undefined`); walk with `s.length`.

Deterministic output; explicit `return` everywhere is safest.

## Reply

(complete js file in one fenced js block)
