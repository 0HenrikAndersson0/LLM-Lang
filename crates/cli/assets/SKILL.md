---
name: llm-lang
description: Use when authoring, reading, emitting, or verifying IR canon (.ir files) with the ircc CLI (from-js / from-cs / to-js / to-cs / verify / doc / install-skill) — LLM-Lang is a compact agent-first transfer representation for JS/TS and C# code. Covers canon authoring, round-trips, and the canon-identity rules that keep tooling stable.
---

# LLM-Lang canon

LLM-Lang compiles typed + documented code (JS/TS, C#) INTO compact canon
text (`~57%` of typed+documented sources on o200k, behavior identity held
down to Humanizer's 61,581-test vendored suite) and emits it back OUT to
JS/TS or C#, round-trip stable. Canon is designed to be authored and
repaired BY agents: every round-trip is lossless, comment/doc-bloat drops
by contract (an agent in a repair loop re-reads the canon, not the docs),
and every executable byte's semantics ride through verbatim.

## Commands

```
ircc from-js file.ts  > file.ir     # JS/TS source → canon
ircc from-cs file.cs  > file.ir     # C# source → canon
ircc to-js file.ir    > file.js     # canon → JS emit
ircc to-js --map ...                # + line map sidecar (repair-loop feedback)
ircc to-cs file.ir    > file.cs     # canon → C# emit
ircc verify file.ir...              # canon-identity lint — RUN THIS, exit 1 on drift
ircc doc spec|tutorial|fidelity|versioning|skill|agents   # print embedded docs
ircc install-skill                  # install this skill for your agent tools
```

## The workflow

1. **Author canon directly** (`.ir` rows) or transpile an existing source:
   `ircc from-js service.ts > service.ir`
2. **Always `ircc verify service.ir`** — canon must be its own fixed
   point (`file == print(expand(file))`); verify exits 1 on drift.
3. **Emit + build**: `ircc to-js service.ir > service.js` (then
   `node --check`) or `ircc to-cs service.ir > service.cs` (then
   `dotnet build`). An agent in a repair loop feeds compiler errors back
   against the canon rows; `to-js --map` carries a source map for precise
   row feedback.
4. Multiple canon files in one module chain: each starts with its own
   `@1 @ns` header; cross-file types resolve at emit via `@ns` paths.

## The rules that matter

1. **Canon identity**: never hand-tune canon so `ircc verify` fails; canon
   text regenerates from rows every round and must be stable.
2. **Core forms are additive-only**: existing canon text never changes
   shape; growth lands as NEW rows (policy: `ircc doc versioning`).
3. **Fidelity is budgeted with `opaque`**: constructs without a core form
   capture verbatim as `"opaque:…"` rows and re-emit byte-identically
   (enums, operators, `#if` regions, unmatched statements).
4. **Comments drop by contract**: machine canon never carries `//` rows.

## Canon rows (cheat sheet)

```
@1 @ns App.Billing                      ← version + namespace header
@use System.Runtime.CompilerServices    ← imports repeat per row

type Invoice export record{             ← types: kind after name, mods before
  export ctor (id Long, total Decimal)
  export Bool Settled => Total >= 0
  export fn TryPost(acct String) Result<String,String>{…}
}
class Repo : IStore { … }               ← bases after name; mods verbatim
type Cache<T> : Object where T : class{ … }

fn Charge(amount Decimal, curr Currency = Currency.Usd) Bool{…}
fn Ext.Round(this Decimal d, Int p = 2) Decimal{…}   ← extension receiver

var mut Balance = Money.Of(10.50m, Currency.Sek)     ← `var` / `var mut`
export get String Label { get; }                     ← get-only auto-property

if a == b{ return 0 }                                ← braced control flow
for x in items{ total += x }
while !done{ step() }
mode switch{ Round.FLOOR => floor(mode), _ => bang } ← switch arms carry =>
try risky() catch NotFound return null finally Log()
lock gate{ … }
return [.. items . Where(p => p.Active)]             ← LINQ ops lowercase in canon
```

Type words canonize capital: `String Int Bool Float Int64 ULong Decimal
Char Byte Short UShort UInt SByte Object` — targets map back to lowercase
keywords on emit. Anything else (`Currency`, `TLocaliser`) passes through
verbatim.

## Deep references

- `ircc doc spec` — the canon v1.1 grammar (authoritative form list)
- `ircc doc tutorial` — the agent walkthrough (JS + C# idiom tables)
- `ircc doc fidelity` — the round-trip budget, per-row
- `ircc doc versioning` — the additive-only core-forms policy
- Upstream + measured gates: https://github.com/0HenrikAndersson0/LLM-Lang