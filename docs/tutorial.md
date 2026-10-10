# LLM-Lang tutorial — teaching the model

The model-facing walkthrough: from an empty file to publishing features
that compile and run on both targets (JavaScript and C#). Every construct
here is verified by the repo's gates (`cargo test`, the loop selftests,
the corpus identity suites). For the formal grammar see
`docs/language-design.md` §1; for release policy `docs/versioning.md`.

*(This tutorial is the target-agnostic superset of the loop's
`bench/scripts/loop/teach.md`, which shows the JS-verified subset.)*

## 1. A file in 60 seconds

```
@1 @ns Acme.Demo

export fn Cap(String s) String{...}
```

- `@1` — file-format version (always `@1` today; see `docs/versioning.md`)
- `@ns Acme.Demo` — a module namespace (free-form dotted path)
- `export fn Name(params) Ret{...}` — a function callers import
- Braces fuse to the header; expressions are tight (`a==b`, `x+y`)
- No comments, no semicolons. The printer owns the layout — write for
  meaning, read for the value.

## 2. Values and variables

```
var count=3              // immutable (by default)
var mut total=0          // `mut` opts into reassignment
```

Types ride signatures (`String s`, `Int n`, `Bool ok`, `List<String> xs`)
and optional marks: `Int? n` = nullable; `T[]`/`Array<T>` = array. Types
erase at emit — the target code is untyped/canonical-typed.

## 3. Control flow is statement-shaped

```
if a>b{return b}
for i in 0..n{total+=i}
for x in xs{acc+=x}
while q>0{return q}
```

For *value* choices inside expressions use ternaries: `cond ? a : b`.
`match` arms are literal patterns (`"A"`, `404`, `true`, `else`) whose
bodies are statements — inside a fn, arm bodies `return` their value.

## 4. Exceptions

```
try await Repo.Get id
catch NotFound return null
catch Error e throw e
finally Log("done")
```

## 5. Expressions

| idiom | canon |
|---|---|
| nullable fallback | `x ?? default` |
| safe navigation | `x?.prop`, `x?.[i]` |
| booleans | `!x`, `&&`, `||` (no `and`/`not` words) |
| string interpolation | `"Hello, {name}!"` |
| index-from-end (C#) | `xs[^1]` |
| spread (C# 12 / JS) | `[a, b, ...rest]` |
| cast | `(Int64)input` |
| pipeline | `xs.filter(f).map(g).toList()` |

A body may end in a bare expression — that value returns when a return
type is declared. Inside `try` and `if`/`else` branches prefer explicit
`return`.

## 6. Methods — write the target's idiom

The IR keeps your receiver-expressions verbatim, so spell library calls
in the **target language's** idiom:

**JS (the `to-js` default):**
```
s.trim()  s.split(" ")  s.toUpperCase()  s.slice(0,n)
xs.filter(p)  xs.map(f)  xs.join("")  xs.at(i)
```

**C# (the `to-cs` emit, .NET 10):**
```
s.Trim()  s.Split(' ')  char.ToUpper(w[0])
xs.Where(p)  xs.Select(f)  string.Join("", xs)
n.ToString("F2")  xs[^1]  input[..^1]
```

The lowerings are documented per-target: `docs/js-target.md` (§1–8) and
`docs/cs-target.md` (§1–6). Collection ops canonize: `.filter`→`Where`,
`.map`→`Select`, `.sum`→`Sum` (C#), so the same canon text lowers to both.

## 7. Types (the `Type` surface)

```
type Order export{                    // public class (export = public)
  List<LineItem> LineItems            // field
  Customer Customer                  // auto-property
  fn Total() Decimal{LineItems...}   // method
}

type IFormatter interface{            // C# interface / JS comment-class
  TimeHumanize(TimeUnit t, Int unit) String
}

type Point(Int64 x, Int64 y){...}     // primary constructor
type Cache : Base, IFace{...}         // bases (constraints drop
                                      // by contract)
```

Modifiers ride the decl line: `sealed abstract record struct interface`.
`const`/`volatile`/`mut` ride members (`volatile var flag`, `const var
Limit=27`).

## 8. Modules

```
@1 @ns Acme.Report @use Acme.Helpers
```

Cross-module calls qualify with the module's last path segment:
`Helpers.Double(n)`. Re-exports: `export type {T} from './mod'`,
`export * from './mod'`.

## 9. What the round trip carries

Writing against an existing codebase: the reader (`from-js`/`from-cs`)
turns real files into canon (comments drop, formatting canonicalizes,
`mut` is inferred from assignments, chars canon to strings, `where`
constraints drop — the full budget is `docs/fidelity.md`). What you read
is the *value* of the file; what you write regenerates the file on
either target.

## 10. Tooling

```
ircc verify feature.ir        # the lint: canon identity (exit 1 on drift)
ircc parse feature.ir         # does the file read?
ircc to-js feature.ir         # emit ESM
ircc to-cs feature.ir         # emit .NET 10
ircc from-js src.js           # round-trip leg: existing code → canon
```

`verify` is the cheap CI gate: canon files are print-stable by contract,
so any drift means the file would churn every tool round.

## 11. Loop workflow (agent-facing)

1. Emit the COMPLETE `.ir` file in one fenced `ir` block.
2. The harness transpiles (`to-js`/`to-cs`), runs the rig, and feeds
   failures back: emit-error rows are exact (`lower` errors carry the
   IR line), runtime failures map back through the span map where the
   target supports it.
3. Repair = re-emit the complete corrected file. The repetition-breaker
   will nudge if a reply is byte-identical.

Verify your feature compiles mentally against the emit rules, prefer
explicit `return`, and keep every row inside the canon (the gates in
`cargo test --lib cs_read`, `golden_snapshots` are the invariants).