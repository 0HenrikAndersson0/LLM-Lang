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

## ASSISTANT

export function CompactLabel(s) {
  const t = s.trim();
  if (t.length === 0) return "-";
  const ws = t.split(" ").filter(w => w.length > 0);
  const label = ws.map(w => w.toFirstUpper()).join("");
  return label.length > 6 ? label.slice(0, 6) : label;
}


## USER

## Reported errors

```
PASS CompactLabel("") -> "-"
PASS CompactLabel("   ") -> "-"
ERROR CompactLabel("hello world"): TypeError: w.toFirstUpper is not a function
    at file:///Users/henrikandersson/Developer/LLM-Lang/bench/scripts/loop/runs/selftest-echo-js/feature.js:5:31
    at Array.map (<anonymous>)
    at Object.CompactLabel (file:///Users/henrikandersson/Developer/LLM-Lang/bench/scripts/loop/runs/selftest-echo-js/feature.js:5:20)
    at file:///Users/henrikandersson/Developer/LLM-Lang/bench/scripts/loop/runs/selftest-echo-js/rig.js:8:24
    at ModuleJob.run (node:internal/modules/esm/module_job:430:25)
    at async onImport.tracePromise.__proto__ (node:internal/modules/esm/loader:661:26)
    at async asyncRunEntryPointWithESMLoader (node:internal/modules/run_main:101:5)
ERROR CompactLabel("hello brave new world"): TypeError: w.toFirstUpper is not a function
    at file:///Users/henrikandersson/Developer/LLM-Lang/bench/scripts/loop/runs/selftest-echo-js/feature.js:5:31
    at Array.map (<anonymous>)
    at Object.CompactLabel (file:///Users/henrikandersson/Developer/LLM-Lang/bench/scripts/loop/runs/selftest-echo-js/feature.js:5:20)
    at file:///Users/henrikandersson/Developer/LLM-Lang/bench/scripts/loop/runs/selftest-echo-js/rig.js:8:24
    at ModuleJob.run (node:internal/modules/esm/module_job:430:25)
    at async onImport.tracePromise.__proto__ (node:internal/modules/esm/loader:661:26)
    at async asyncRunEntryPointWithESMLoader (node:internal/modules/run_main:101:5)
ERROR CompactLabel("Hello world"): TypeError: w.toFirstUpper is not a function
    at file:///Users/henrikandersson/Developer/LLM-Lang/bench/scripts/loop/runs/selftest-echo-js/feature.js:5:31
    at Array.map (<anonymous>)
    at Object.CompactLabel (file:///Users/henrikandersson/Developer/LLM-Lang/bench/scripts/loop/runs/selftest-echo-js/feature.js:5:20)
    at file:///Users/henrikandersson/Developer/LLM-Lang/bench/scripts/loop/runs/selftest-echo-js/rig.js:8:24
    at ModuleJob.run (node:internal/modules/esm/module_job:430:25)
    at async onImport.tracePromise.__proto__ (node:internal/modules/esm/loader:661:26)
    at async asyncRunEntryPointWithESMLoader (node:internal/modules/run_main:101:5)
ERROR CompactLabel("a b c d e f g h i"): TypeError: w.toFirstUpper is not a function
    at file:///Users/henrikandersson/Developer/LLM-Lang/bench/scripts/loop/runs/selftest-echo-js/feature.js:5:31
    at Array.map (<anonymous>)
    at Object.CompactLabel (file:///Users/henrikandersson/Developer/LLM-Lang/bench/scripts/loop/runs/selftest-echo-js/feature.js:5:20)
    at file:///Users/henrikandersson/Developer/LLM-Lang/bench/scripts/loop/runs/selftest-echo-js/rig.js:8:24
    at ModuleJob.run (node:internal/modules/esm/module_job:430:25)
    at async onImport.tracePromise.__proto__ (node:internal/modules/esm/loader:661:26)
    at async asyncRunEntryPointWithESMLoader (node:internal/modules/run_main:101:5)
ERROR CompactLabel("  Multiple   Spaces   Here  "): TypeError: w.toFirstUpper is not a function
    at file:///Users/henrikandersson/Developer/LLM-Lang/bench/scripts/loop/runs/selftest-echo-js/feature.js:5:31
    at Array.map (<anonymous>)
    at Object.CompactLabel (file:///Users/henrikandersson/Developer/LLM-Lang/bench/scripts/loop/runs/selftest-echo-js/feature.js:5:20)
    at file:///Users/henrikandersson/Developer/LLM-Lang/bench/scripts/loop/runs/selftest-echo-js/rig.js:8:24
    at ModuleJob.run (node:internal/modules/esm/module_job:430:25)
    at async onImport.tracePromise.__proto__ (node:internal/modules/esm/loader:661:26)
    at async asyncRunEntryPointWithESMLoader (node:internal/modules/run_main:101:5)
RESULT: 5 failing
```

## Your current JS (numbered)

```
  1 export function CompactLabel(s) {
  2   const t = s.trim();
  3   if (t.length === 0) return "-";
  4   const ws = t.split(" ").filter(w => w.length > 0);
  5   const label = ws.map(w => w.toFirstUpper()).join("");
  6   return label.length > 6 ? label.slice(0, 6) : label;
  7 }
```

Compare what your code does for each failing case above with the expected value. Name the exact line to change, then re-emit the COMPLETE corrected js file in one fenced js block.

## Reply

(complete js file in one fenced js block)
