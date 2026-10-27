# C# target (PLAN 5.1–5.2)

> Phase 5 opens with the original promise kept: C# stays a target. This
> document mirrors `js-target.md` in structure: the C# surface design
> (§1), the emitter architecture decision (§2), atom→C# mapping (§3),
> the Core → C# lowering table (§4 — task 5.2), canonical layout (§5),
> and the type-word/signature conventions (§6). Grounded in the cs_*
> canon fixtures (ground truth, working agreements) and the Phase-0
> semantics table (language-design §2).

## 1. What the C# surface keeps and drops

Dropped by contract (same fidelity doc rows as JS, §6.1/§6.2 of
language-design): comments, XML-doc blocks (25–40% of Humanizer sources —
the dominant Phase-0 saving), whitespace/formatting, generic parameter
constraints (`where TLocaliser : class` — signatures only), leading
`@ns`→namespace is the ONE header directive that *gains* a C# equivalent.

Kept as canonical text and preserved through the round trip: attributes as
opaque rows if a corpus file carries them (none do today — noted in §7),
modifiers (`export/volatile/const/sealed/abstract`), primary constructors,
`ctor` overloads, `lock`, computed properties, receiver (extension) fns,
collection expressions `[.. x]`, interpolated strings, switch/match, `is`
patterns, out-args.

## 2. Emitter architecture decision

**Direct Core → C# emitter over `Core` (the `js.rs` shape), not a
separate Roslyn-style AST layer.** The js.rs direct emitter already
survived behavior gates + sourcemap + round-trip; a second AST layer
would be speculative machinery. "Roslyn-inspired" shows up as OUTPUT
conventions (§5), not as an AST dependency. The emitter lives in a new
`crates/lang/src/cs.rs` with the same shape as `js.rs`:

- `emit(core: &[Core]) -> Result<String, CsLowerError>` (+ `emit_with_map`
  for the IR↔CS sourcemap, 2.6-style, if the loop test maps C# errors).
- Expression rendering reuses the *atom-run* expression rewriter approach:
  most expressions pass through verbatim (C# and the IR expression surface
  agree: calls, binaries, lambdas, `?.`, `??`, collection expressions,
  interpolated strings — see §3); the rewriter handles only the
  IR-specific spellings (`[^i]`→C# `[^1]` index-from-end verbatim is
  already valid C# 12; bare-call paren rule; `Name args` → `Name(args)`;
  ternary/if-exprs; statement runs inside lambdas — `split_stmt_runs`
  machinery is shared shape, `emit`-local implementation).

## 3. Atom → C# expression mapping (verbatim-first)

The IR expression surface was designed C#-compatible (Phase-0 decision,
`bench/results.md` route section). Mapping:

| IR surface | C# | note |
|---|---|---|
| `recv.Member(args)` / chains | verbatim | calls, member access, `?.` |
| `a[i]`, `xs[^1]` | verbatim | C# indexers + index-from-end |
| `[a, b]`, `[.. xs]` | verbatim | C# 12 collection expressions (dotnet 10 target) |
| `new(args)` / `new Type(args)` | `new(args)` / `new Type(args)` | target-typed new stays |
| `(a)=>e`, `(a)=>{…}` | verbatim lambdas | block bodies statementified like js |
| `"x{e}y"` (interp canon) | `$"x{e}y"` | canon interpolation → interpolated string |
| `x is > 0 and < 5`, `x is Type`, `x is null` | verbatim | IR `is`-patterns were C#-derived |
| `cond ? a : b` | verbatim | ternary |
| `Name args` | `Name(args)` | bare-call paren rule (§1 paren rule) |
| `x.Value!`, `x!` | `x.Value!`, `x!` | null-forgiving is C# surface; lexer keeps `!` suffix — emit as `x!` |
| `_ => e` | verbatim | discard lambda |
| `^export`-era attributes | n/a today | none in corpus; opaque rows if they appear (§7) |
| `=>` in `Type name => expr` members | `Type Name => expr;` | computed property |
| `..`-range heads | C# range/indexer where value-position, classic `for` where loop | see §4 range row |

Erased at emit (like JS): param `?` optionals, param/ret type annotations
on *locals*, fn type-parameter declarations, `as const`.

## 4. Core-form → C# lowering table (task 5.2)

| Core | C# sketch | conventions |
|---|---|---|
| `Header{version,ns,uses}` | `namespace Humanizer;` + usings | version: dropped (tooling metadata, no C# equivalent of the JS side-effect import story). `@ns P` → file-scoped `namespace P;`. `@use Q[.R]` → `using Q.R;` — dotted paths verbatim; usings sorted, placed after the namespace (file-scoped convention) |
| `Var{name,ty,value,modifiers}` (top-level) | `public static …` fields in a class-context or locals at fn scope | top-level `var X=e` in a `type … export` = static field row; bare module-level vars lower inside the enclosing class as `public static readonly`-shaped (spell per fixture: cs_05 `const var Limit=27` → `public const int Limit = 27;` — const REQUIRES an explicit C# type: `Int? ty` carries it; inferred consts use the ty or fall back per §6) |
| `Var` member field `Type name[=init]` | `private readonly Type name = e;` / auto-property | fixture convention decides: cs_02 plain members = private readonly fields; cs_03 `String DefaultSeparator` with ctor init — IR row is plain; emit default = field unless the original had `{ get; }` (v0: plain field; property-shape rows `Type name => expr` are computed; `{ get; init }`-ish rows are captured as fields when the IR couldn't express them — budget row) |
| `Var` local `var name=expr` / `var mut` | `var name = e;` | `mut` is default-mutability in C# locals — both spell `var`; `const var` local → `const T name = e;` with the §6 type word |
| `volatile var` | `private volatile bool useGeneratedCultureResolver;` | field placement + volatile keyword verbatim |
| `Fn{name,receiver:none}` in class | `public Ret Name(params)` | modifiers: `export`→`public`; `async`→`async`; overrides/abstract/sealed pass via their modifier words when canon carries them |
| `Fn` top-level (no type) | static method on a synthetic static class | C# needs a containing type: `internal static class Module{…}` — the cs fixtures are all type-contained (canon `fn Metric…` under `type … export`), so v0 covers the fixture set; synthetic class name = file stem PascalCase |
| `Fn{name="X.Member", receiver}` | extension method | `fn String.FromMetric() Float` → `public static float FromMetric(this string input)` inside `static class` type (canon already places receiver fns inside `type … export` — cs_05); receiver name = `input` (design §2.1 measured row) |
| `ctor(params){}` | `public Type(params){…}` | overloads verbatim; primary `type Name(params) : Base` → C# 12 primary constructor |
| `Type{name,kind,modifiers,bases}` | `public class/interface X : Base{…}` | `interface` kind word → `interface`; `record`/`struct` words → their C# keywords; bases join with `: `; constraints dropped (§1) |
| `Core::Var` sig-only member rows (ty = call-shaped text) | interface/method signature | canon rows like `DateHumanize_Now() String` parse (expander) as Var{ty:"DateHumanize_Now()", name:"String"} — the C# emitter INVERTS: split the ty text at the first `(`/default tail, re-parse params (§6), emit `Ret Name(params);` — fixture rows carry defaults (`Bool toWords=false`) which stay |
| `Core::Stmt{head,body,arms}` — `if/for/while/lock/match/try…` | statements | `if c{a}else{b}` → braced; `for i in a..b` → `for (var i = a; i < b; i++)`; `for x in coll` → `foreach (var x in coll)`; `lock e{…}` → `lock (e){…}`; `match e{arms}` → switch expression (`e switch {…}`) when value-position, switch statement when statement-position; arms `pat expr` with `else`→`_`; `try/catch pat binder?/finally` → blocks; catch patterns (type names) → `catch (NotFoundException)` etc. |
| `Core::Expr(atoms)` | expression statement | verbatim run + `;` — the ret-tail value rule (bare tail expr = return) applies inside fns like js (`fn ResolveForUiCulture() T{…}` fusing `ResolveForCulture(…)` → `=>` expression-bodied method when the body is ONE bare expr — cs_02 originals ARE expression-bodied; v0: `return …;` normal form, expression-body only for the single-statement case) |
| `Reexport{…}` | drop + fidelity flag | C# has no file-level re-export; canon keeps the row (identity), emit drops — budget row in `docs/fidelity.md` |

## 5. Canonical C# layout

- 4-space indent (Roslyn/.NET convention); braces K&R (`{` on the header line
  for types/methods — fixture style: `public class X\n{` is brace-new-line…
  **decision: brace-new-line for type/method bodies, same-line for control
  flow** — matching the cs corpus originals' dominant shape; verify at 5.3
  against fixtures per working agreements).
- File order: header → namespace → usings → types (one per canon row) →
  top-level static class (if any) last.
- `;` after every field/local/return; expression-bodied members get `=>`.

## 6. Type words and signatures

IR canon stores capital type words (`String/Int/Bool/Float/Int64/ULong/Decimal`)
— measured tie in Phase 0, chosen for C# familiarity. C# keywords differ —
emitter maps (corpus-grounded):

| IR | C# | note |
|---|---|---|
| `String` | `string` | |
| `Int` | `int` | |
| `Bool` | `bool` | |
| `Float` | `double` | cs_01 original: `double count` |
| `Int64` | `long` | |
| `ULong` | `ulong` | fidelity 24 (suite-gate pass): the EB wrap-boundary is semantics-carrying — `ULong` folded to `Int64` flipped the vendor suite's `--1 EB` row |
| `Decimal` | `decimal` | |
| `Array<T>` | `T[]`` | generics otherwise verbatim (`Dictionary<Char, UnitPrefix>`, `Func<CultureInfo, TLocaliser>`) |
| other idents | verbatim | `TimeUnit`, `CultureInfo`, `TLocaliser`… |

- Params: canon `(id Int, name String)`-shaped name-first pairs → C#
  `Type name` (design §2.1: IR is type-first in HAND fixtures, machine
  canon is name-first — the emitter consumes the `(name, ty)` pairs
  directly, so both orders converge).
- Defaults: canon `Bool toWords=false` → C# `bool toWords = false`.
- Nullable: `T?` → `T?` verbatim.

## 7. Deferred rows / likely 5.4 findings

Recorded up front, in the 2.4-report spirit: attributes (`[Attribute]`
rows — none in corpus; opaque if they appear), field access-modifier
precision (IR keeps only `export`—public/private/internal nuance is a
budget row), `{ get; set; }` vs plain fields, `#region`, C# switch
guard-clauses (`when`), LINQ query syntax (canonicalized to method chains
by contract), `checked`/`unsafe` (opaque), operator overloads (`fn op_Addition`
— none in corpus; opaque if present).

## 8. Acceptance gates (5.3–5.6)

- **5.3** `dotnet build` zero errors on the 10 golden C#↔IR pairs
  (`bench/corpus/*.cs` + `.ir`).
- **5.4** cs_* real corpus (9 Humanizer files): `dotnet build` zero
  errors; diff-vs-hand-renditions report (`bench/results-cs-diff.md`),
  every failure root-caused (⚙️ emitter gap / ✍️ rendition drift /
  📐 design limit tags, 2.4 style).
- **5.5** LLM loop test for C#: the `bench/scripts/loop/` harness gains a
  `--lang cs` baseline + dotnet-run feedback loop (stack-trace repair).
- **5.6** tree-sitter-c-sharp reader + double round-trip over cs corpus
  (mirror `js_read.rs`; the §8/§1 rows carry over).