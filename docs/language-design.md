# Language Design (v1.1 — Route 1, measured, reconciled)

This document is the design-first track for the IR language itself. The transpiler
architecture in `llm_optimized_language_architecture.md` stays valid but is
**downstream** of everything here: nothing gets implemented until the grammar,
core forms, and golden examples reach a stable verdict.

Design flow:

```
1. Grammar (this doc, EBNF)          -> frozen first
2. Core forms (~15 primitives)       -> frozen second
3. Type system surface               -> frozen third
4. Sugar / macro layer               -> specified as expansions, not new syntax
5. Tokenizer-scoring studies         -> gate every decision above
6. Readability floor (never-minify)  -> policy doc
```

The structure mirrors the Scheme report model: **one tiny reader grammar + a
table of special forms**. The transpiler is then just a reader + expander +
printer, which is why the language stays small.

**Route decision (2026-10-03, bench/results.md).** The original full s-expr
surface (v0, "variant A") was falsified by measurement: variants A/B/C scored
118.7%–132.7% of the C#/JS token baseline — s-exprs trade infix punctuation
(which BPE fuses into shared tokens) for prefix parens (which BPE cannot
fuse). **Route 1 adopted**: a normal infix language with short, unabbreviated
keywords and braceless blocks, scored **89.7% (o200k) / 90.1% (llama)** of
baseline on the golden corpus. This document is the v0 draft rewritten for
Route 1; §10 records the decisions the v0 variant table got wrong.

---

## 0. The one-page language

From the golden fixtures (`bench/variants/d/*.ir`) — note 05/07 fuse their
single-expr bodies into the header line, 09 uses braceless single-clause
lines:

```ir
@1 @ns Acme.Users

fn GetUser(Int id) User?{await Users.Find id}

type Order export{
  List<LineItem> LineItems
  Customer Customer
  fn Total() Decimal{LineItems.map(li=>li.Amount).sum()}
}

async fn Load(Int id) User?{
  try await Repo.Get id
  catch NotFound return null
  catch Error e throw e
  finally Log("done")
}
```

Design rule this syntax was selected by: **infix punctuation fuses, prefix
parens don't** — `Users.Find` = `'Users'+'.Find'` (2 tokens), `Find(id`
fuses `'(`', but `(Users.` never fuses. Every construct below was chosen
by the probes in `bench/results.md`, not by taste.

---

## 1. Reader Grammar (EBNF v1.1)

The reader is lexical-only: it does not know about `fn`, types, or method
resolution. Special forms (Section 2) are a table consulted by the expander,
not grammar constructs.

```ebnf
program     = { directive | decl } ;
directive   = "@" ( version | "ns" path | "use" path ) ;   (* one per line *)
path        = ident { "." ident } ;
version     = digit { digit } ;

decl        = "export"? ( fn-decl | type-decl | var-decl ) ;
fn-decl     = "async"? "fn" ident "(" params? ")" type? body ;
params      = param { "," param } ;  param = ident type ;
body        = "{" stmt-list "}" | stmt ;   (* braceless = single stmt *)
type-decl   = "type" ident type-params? primary? base? type-modifiers? "{" { member } "}" ;
type-params = "<" type-param { "," type-param } ">" ;
primary     = "(" param-list ")" ;         (* primary constructor *)
base        = ":" type ;
type-modifiers = { modifier-word } ;
modifier-word = "export" | "interface" | "record" | "struct" | "sealed" | "abstract" ;
member      = fn-decl | ctor-decl | field | property ;
ctor-decl   = "ctor" "(" params? ")" body ;
field       = type ident [ "=" expr ] ;
property    = type ident "=>" expr ;       (* computed; auto-prop = field w/ init *)
var-decl    = ["const" | "volatile"] "var" ["mut"] ident "=" expr ;

type        = ident [ "<" type { "," type } ">" ] [ "?" ] ;

match-arm   = pattern expr ;
stmt-list   = stmt { stmt } ;
control     = if-expr | match-expr | for-expr | while-expr | lock-expr
            | "throw" expr | "return" [expr] | "await" expr ;
for-expr    = "for" ident "in" iterable "{" stmt-list "}" | "for" ident "in" iterable stmt ;
while-expr  = "while" expr body ;
lock-expr   = "lock" expr body ;
pattern     = literal | ident | "_" | "else" ;

expr        = expr op expr | expr "=>" expr        (* lambda arrow *)
            | prefix-call | method-chain | operand ;
op          = "&&" | "||" | "==" | "!=" | "<" | "<=" | ">" | ">="
            | "+" | "-" | "*" | "/" | "%"
            | "=", "+=", "-=", ... (assignment family) ;
operand     = int | float | string | ident | "null" | "true" | "false"
            | "(" expr ")" | "(" expr-list ")" | list-lit | obj-lit
            | "..." expr ;                          (* spread *)
prefix-call = ident { operand } ;      (* Users.Find id *)
method-chain= operand { "." ident [ "(" [expr-list] ")" ] } ;
list-lit    = "[" [expr-list] "]" ;      (* collection literal *)
obj-lit     = "{" [ ident ":" expr { ident ":" expr } ] "}" ;

string      = '"' { plain-char | escape | "{" expr "}" } '"' ;  (* interpolation *)
escape      = backslash char ;

opaque-decl = "opaque" type-decl          (* verbatim passthrough block *)
comment     = "//" { any-char } newline ;
```

Call-site arg modifiers: `out` marks a by-ref out arg (`f(k, out v)`,
declaring `out var v`); `ref`/`in` same slot if a target needs them.
An `fn` whose body the round-trip cannot represent is marked `fn …
opaque{ capture verbatim }` (fidelity §6.5) — the reader stores the
original source span, the printer re-emits it untouched.

Nonterminals `stmt`, `if-expr`, `match-expr`, `for-expr`, try clauses and
`pattern` shapes are spelled in §2 — the reader only fixes token shapes;
keyword association happens in the expander.

Grammar rules measured into existence:

- **Interpolation is a token shape** — `"…{u.Name}…"` is exactly as cheap
  as backtick `${...}` forms (16/20 measured on the 7.7 fixture text); the
  `""`-interpolation shape is canonical because it matches C#/JS and needs
  no second string token type.
- **Only spaces that carry meaning** — space *between* atoms is required
  (grammar), spaces around operators are optional (`o.Paid&&o.Total>100m`
  scores 1 token better than spaced in o200k, llama indifferent); the
  canonical printer emits minimal spaces.
- **Call-paren rule:** prefix-call `Users.Find id` (3/4) beats
  `Users.Find(id)` (4/6) *when the parser can end the arg list unaided*
  (prefix-call args are atoms, never infix chains); otherwise emit `(...)`.
  `(` fuses with the following ident (`'(Int'`), and `')'` fuses with what
  follows (`')\n'`, `');'`), so statement-head parens are nearly free.
- **Comments `//`** — line comments allowed in source, dropped on round-trip
  (§6 fidelity). `;;` from v0 retired (`//` = 3/3, same as `;;`, and both
  fuse with the following word).

### 1.1 Lexical notes

- Number suffixes are part of the number token: `100m` (decimal) = 2 o200k
  tokens / 5 llama (splits); `100.0m` = 4/7 — suffix form wins, keep `m`.
- Bools are `true false`; single-letter `T F` measured a tie and the
  readability floor rejects it.

---

## 2. Core Forms (the ~24 primitives)

Everything else must be sugar expanding into this table (§5). After macro
expansion, no remaining form is outside the table. (v1 counts 24 after the
real-corpus reconcile: interface/record/struct modifier words, primary
ctor + base, ctor, computed property, const/volatile, while, lock, spread,
out-args, ranges — each admitted on a measured basis, §8.)

### 2.1 Definitions

| Form | Surface | Expands to (target sketch) |
|---|---|---|
| `@1` / `@ns Path` / `@use Path` | as written | file header: version, namespace, using/import |
| `var name=expr` / `var mut name=expr` / `const var name=expr` / `volatile var name=expr` | as written | `var`/`const`/`volatile` decl (immutability is the default: `mut` opts in) |
| `fn Name(params) Ret{body}` | as written | method / free function |
| `async fn Name(params) Ret{…}` | as written | async method / function |
| `type Name{members}` / `type Name export{…}` | as written | `class` / `public class` |
| `type I interface{members}` | modifier word | `interface` (C#) / plain `class` (JS) |
| `type Name(params) : Base{…}` | primary ctor + base | `class Name(params) : Base` / JS ctor chain |
| `ctor(params){}` | member | constructor |
| `Type name=init` (member) / `Type name => expr` | field / computed property | auto-property / expression-bodied property |
| `fn T.Member(args?)` receiver-first extension | `fn String.FromMetric() Float` = 10/14 vs `fn FromMetric(this String input)` = 12/16 (measured both in-type and bare); body refers to receiver as `input` |

Param order is `name Type`, matching C# and `bench/results.md` (pure BPE tie
with `Type name`). `export`/`interface`/`record`/`struct`/`sealed`/
`abstract` are modifier words on the decl line (`type Order export{`,
`type IFormatter interface{`) — attribute-ish prefixes (`^export`) measured
ties and lose to plain words on readability. Immutability is the default:
`mut` opts a `var` in; `const`/`volatile` are modifier words when a target
needs the distinction.

### 2.2 Expressions & control

| Form | Meaning |
|---|---|
| `recv.Member(args?)` | instance call / member chain |
| `Type.Member(args?)` | static access/call |
| `Name(args?)` / `Name args` | call (paren rule in §1) |
| `if c{a}else{b}` (statement) / `if c a else b` (expr arm) | conditional; braceless 3-operand form saves 3–4 over braced one-liners (5/5 vs 8/9) |
| `match e{ arm… }` | switch/switch-expr; arms `pattern expr`, `else` = default |
| `for x in coll{…}` / `for i in a..b{…}` (`..` half-open) | foreach / range loop |
| `while c{…}` / `lock e{…}` | core loop / monitor scope (4/4 `lock lockObject{`) |
| `try stmt` / `catch pattern binder? stmt` / `finally stmt` | clauses, one per line; see §2.4 |
| `throw e` / `return e?` / `await e` | escapes |
| `ctor(params){}` / local `fn` in a body | constructor / local function (no new syntax) |

### 2.3 Blocks and clause bodies (measured)

- **Bodies are braceless by default.** A *statement-per-line* at one indent
  level is a body (see 7.2, 7.9). A single-expression body fuses onto the
  header line (`fn Greet(..) String{"…"}`, `catch N return null`) or onto
  the control header (`for x in xs{acc+=x}`), saving the `' {\n'` +
  indent + `'\n }'` scaffold — ~4 tokens/file.
- Braces `{}` appear only on: fn bodies of >1 statement (7.6-style), match
  headers (7.4), and wherever ambiguity would otherwise arise. A single
  statement never gets its own braces.
- Sequence semantics: a body evaluates its statements in order; value of
  the block is the value of the last statement. No explicit `begin`.

### 2.4 try/catch spellings

| IR | C# | note |
|---|---|---|
| `try await Repo.Get id` | `try await Repo.Get(id);` | try body braceless |
| `catch NotFound return null` | `catch (NotFound){return null;}` | **pattern-only when no binder used** |
| `catch Error e throw e` | `catch (Error e){throw e;}` | binder `e` when used |

Pattern-only catches avoid one binder per clause and were the largest
single win of the D pass (fixture 09: 51/61 → 37/43, 70–73% of baseline).

---

## 3. Type System Surface

Types appear **only in signatures** — params, returns, field declarations;
locals are untyped (`var name=expr`); explicit local types were probed at
5 tokens vs 4 and add nothing. v0 `mut`/`let` retired: `var` fused 1-token
in both vocabs in our probes (`var acc=0` = 4/4) and matches C# 1:1.

- Primitives: `Int I64 Float Decimal Bool String`
- Containers: `List<T> Map<K,V> Array<T> Set<T>` — infix with angle
  brackets. Measured: `List<Int>` = 3/4, `List Int` = 2/2, `List~Int` = 3/3;
  infix `<>` chosen to match C#/JS/TS and because `~`-glue never fused in
  llama either. *Rejects v0's type-s-expr hypothesis.*
- Nullable: `Type?` (2 tokens, both vocabs) — `~User`, `Opt T`, `(Opt T)`
  all falsified.
- Async wrapper: **none in signatures** — `async fn` carries asyncness; the
  printer emits `Task<T>` / `Promise<T>` around the declared return type
  per target (probe: `User?` + `async fn` = 8/9 beats `Promise~User` + fn
  = 9/11 with the same end program).
- User types: bare idents, never abbreviated (readability floor).

The IR does **not** encode generics constraints, overloads, or attribute
targets; the printer reconstructs conventional defaults, and round-trip
*loses* what IR cannot express (Section 6).

---

## 4. Sugar Layer (macros — spec'd as expansions)

Macros are defined *in this doc* as expansions into core forms; the Rust
expander implements the table, never new syntax.

| Macro | Example | Expansion core form |
|---|---|---|
| string interpolation | `"Hello, {u.Name}!"` | `Concat "Hello, " (u.Name) "!"` per target |
| method-chain→LINQ | `xs.map(li=>li.Amount)` | `xs.Select(li => li.Amount)` (C#) / native (JS) |
| chain fns | `filter/map/sort/toList` | native LINQ names when 1:1, else expansion |
| if / if-let | `if c{a}else{b}` | core `if`; if-let sugar over `match` (see 7.5) |
| switch-expr | `match e{…}` as an expr | target switch-expression |
| async/await | `async fn` | fn-level async + `await` keyword per target |

Rule: **if a form could be a macro, it must be a macro.** The core form
table above is closed; PRs adding primitives are rejected in favor of
macro syntax.

---

## 5. Canonical Formatting & Readability Floor

The IR ships in exactly one canonical layout (mirrors `bench/variants/d/`):

1. `@`-directives at top, one line per directive, blank line after header.
2. One decl per top-level line; statement bodies indented 2 spaces.
3. Minimal spacing: `{`/`(`/`?` fuse to the preceding word; binary-operator
   spacing is canonical *tight* (`o.Paid&&o.Total>100m`) — llama indifferent,
   o200k saves 1.
4. Single-expression bodies fuse onto the header line; multi-statement
   bodies use braces + indent. One shape per construct — no alternatives.

The readability floor rules (unchanged from v0, and now *measurement-backed*):

1. **User identifiers are never abbreviated.** `GetUser`, `lineItems`,
   `customerEmail` pass through verbatim. Only *grammar* compresses.
2. Structural keywords are unabbreviated English (`fn var type match try
   catch finally if for in while lock await throw return null true false
   async export ctor interface record struct sealed abstract const volatile
   out else`). Probe: single-letter variants (`w`, `T`, `F`, `afn`) saved
   **zero** tokens in both vocabularies — abbreviating keywords costs
   readability for nothing.
3. Punctuation may shrink only when the probe confirms the fused form is
   stable (`Users.Find`, `User?`, `=>`, `&&`) — never when it breaks an
   anchor word apart.
4. `--expand` debug printer mode shows macro-expanded core form for review.

---

## 6. Fidelity Budget (round-trip contract)

The reader (C#/JS → IR) is explicitly **lossy**; the writer (IR → C#/JS) is
lossless *relative to the IR*. Dropped in order of acceptance:

1. Comments — dropped entirely. (Real-corpus impact measured: XML-doc
   blocks were 25–40% of Humanizer files — *the* dominant saving.)
2. Formatting — normalized to canonical layout.
3. Locals' explicit types — dropped (`var` everywhere; both targets emit/expect it).
4. Syntactic sugar variety (LINQ query syntax vs method call) — canonicalized to
   method chains.
5. Overload sets, advanced generic constraints, unsafe blocks —
   unrepresentable; pass through as opaque `(opaque …)`/`fn … opaque{}` + `opaque type` forms the printer
   emits verbatim from a captured source string.
6. TS-specific type machinery (union template types, `as const`, type
   predicates) — canonicalized where it maps (predicates → printer
   reconstruction), else `opaque type …` (reconcile row 21).
7. Interface members with default bodies — representable as `Type name =>
   expr` / `fn` with body; multiple-impl dispatch stays in target.

Acceptance criterion: round-trip twice (C# → IR → C#) and the result must
compile and behave identically even if it does not match the original text.

---

## 7. Golden Examples

Worked in both directions. These are the *fixtures* the lexer, expander,
and printers are built against; they double as the bench corpus
(`bench/corpus/*.cs|js`, `bench/variants/d/*.ir`).

### 7.1 Async member + optional return

```
fn GetUser(Int id) User?{await Users.Find id}
```
```csharp
static async Task<User?> GetUser(int id)
    => await Users.Find(id);
```

### 7.2 Type with fields and a method

```
type Order export{
  List<LineItem> LineItems
  Customer Customer
  fn Total() Decimal{LineItems.map(li=>li.Amount).sum()}
}
```
```csharp
public class Order
{
    public List<LineItem> LineItems { get; set; }
    public Customer Customer { get; set; }
    public decimal Total()
        => LineItems.Select(li => li.Amount).Sum();
}
```

### 7.3 Pipeline over collections

```
fn ActiveHeavy() List<Order>{
  Orders.filter(o=>o.Paid&&o.Total>100m).map(o=>o.Total).toList()
}
```
```csharp
static List<Order> ActiveHeavy()
    => Orders
        .Where(o => o.Paid && o.Total > 100m)
        .Select(o => o.Total)
        .ToList();
```

### 7.4 Match statement

```
fn Handle(Request r) Response{
  match r.Kind{
    "GET" Read(r.Path)
    "POST" Write(r.Body)
    else Response.Err 405 "method not allowed"
  }
}
```
```csharp
static Response Handle(Request r)
    => r.Kind switch
    {
        "GET"  => Read(r.Path),
        "POST" => Write(r.Body),
        _      => Response.Err(405, "method not allowed"),
    };
```

### 7.5 Optional / if-let

```
fn FindEmail(User u) String?{u.Email}
```
```csharp
static string? FindEmail(User u)
    => u.Email;
```

### 7.6 Loop with mutation (core forms, no sugar)

```
fn SumAll(List<Int> xs) Int{
  var acc=0
  for x in xs{acc+=x}
  acc
}
```
```csharp
static int SumAll(List<int> xs)
{
    var acc = 0;
    foreach (var x in xs)
        acc += x;
    return acc;
}
```

### 7.7 String template

```
fn Greet(User u) String{"Hello, {u.Name}! You have {u.PendingCount} tasks."}
```
```csharp
static string Greet(User u)
    => $"Hello, {u.Name}! You have {u.PendingCount} tasks.";
```

### 7.8 Module / exports (JS-facing)

```
@1 @ns Acme.Users @use Acme.Domain

export Users = Users
```
```javascript
import { Users } from "../Domain/index.js";

export const Users = Users;
```

### 7.9 Try / catch / finally

```
async fn Load(Int id) User?{
  try await Repo.Get id
  catch NotFound return null
  catch Error e throw e
  finally Log("done")
}
```
```csharp
static async Task<User?> Load(int id)
{
    try { return await Repo.Get(id); }
    catch (NotFound) { return null; }
    catch (Error e) { throw e; }
    finally { Log("done"); }
}
```

### 7.10 Version directive (the first form of every file)

```
@1 @ns Acme.Orders
```

---

## 8. Frozen decisions (formerly "Open Decisions")

All v0 §8 open decisions are now measured (`bench/results.md` route section
+ probe log, this corpus). Spelling ties broken by the readability floor
(§5). Decision order followed PLAN.md: arrow/bools/nullable → statics →
async annotations.

| # | Decision | Verdict | Basis |
|---|---|---|---|
| 1 | Signature shape `[…]`, arrow `->` vs `=>` | `fn Name(params) Ret{}`, no arrow | s-expr sig blocks falsified (fixtures A→D deltas) |
| 2 | Bool spellings | `true false` | `T F` tie → floor |
| 3 | Nullable | `Type?` | 2 tokens both vocabs; `~User` tie + ambiguous; `(Opt T)` falsified |
| 4 | Statics | `Type.Member(args?)` | `'.'` fuses both vocabs; `/` never fuses |
| 5 | Async | `async fn` modifier, bare ret type | `afn`, `^async` ties → floor; no `Promise<T>` in sigs |
| 6 | Generic containers | `List<T>` | target parity; `List Int` won llama by 1 but prefix types break C#/TS fidelity |
| 7 | Pipe `\|>` vs method chains | method chains | `'.'` fuses, `\|>` = standalone 2 tokens per hop (measured 2–3/2–3) |
| 8 | `case` keyword in match | dropped | 2 tok/arm pure overhead |
| 9 | Comments | `//`, dropped on round-trip | probe tie with `;;`, matches both targets |
| 10 | interface | `type I interface{` | `interface I{` wins by 1/1 on probes (116 vs 117 whole-fixture) — but `type` keeps ONE decl header keyword in the grammar; difference is per-file cost, not per-construct structure. Re-rollable if reader ergonomics prefer bare `interface`. |
| 11 | const / readonly | default immutable; `const` word; no `readonly` word | default covers readonly; `const Limit=27` 4/6 |
| 12 | Properties | `Type name=init` auto; `Type name => expr` computed | 3/4 + 6/8 vs `{get}=name` 6/6 + `{get} =>` 9/11 |
| 13 | Primary ctor + base list | `type Name(params) : Base{` | unambiguous; `s): ICollection` fuses (8/10) |
| 14 | `lock` / `while` | core statements, braceless bodies | `lock lockObject{` 4/4; `while number>0{` 5/5 |
| 15 | Local functions | `fn` inside a body = local | no syntax added; both targets native |
| 16 | Ranges | `..` half-open in `for` | `0..len{` 8/8 vs `0..<len{` 8/9 |
| 17 | Spread | `...expr` | 11/11 vs `Object.assign` 6/8 — actually costs same, kept for target-native shape |
| 18 | out/ref args | arg modifier `out` | `(k, out var v)` 6/7; `out` = 1 tok |
| 19 | ctor | `ctor(params){}` | `this(…)` 5/6 wins tokens but reader-ambiguity risk; `ctor` correctness-first |
| 20 | `is`-patterns | plain infix, no `is`/`pattern` grammar | `unity>1&&unity<5` 7/7 beats `is > 1 and < 5` 9/9 |
| 21 | overloads / type predicates / `as const` / union-template types | fidelity budget (§6): canonicalize or opaque — no surface syntax | overload sets canonicalized (§6.4); predicates printer-reconstructed |

Note 19 (ctor): `this(params)` was 1 token cheaper on o200k (5/6 vs 5/7)
but `this` is already meaningful in expressions; the reader would need
paren-lookahead disambiguation on every member line. `ctor` is unambiguous
at the cost of 1 token in files that have constructors — correctness-first.

---

## 9. Verification Plan for the Design Phase

- [x] Every decision in §1/§2 re-scored against `o200k_base` + llama vocab;
      results recorded in `bench/results.md` (variant progression A→D +
      probe log).
- [x] Golden examples in §7 hand-scored: 89.7% / 90.1% whole-file.
- [x] Real-world corpus (16 files, Humanizer + cheerio, MIT) hand-rendered
      in variant D and measured (0.4/0.5): **46.1% o200k / 46.2% llama**
      after the reconcile pass — **0.8 GO** recorded in results.md.
- [x] Reconcile pass (2026-10-03): constructs the 0.5 hand-renders needed
      that v1 lacked were each probe-measured and either admitted (rows
      10–20) or assigned to the fidelity budget (rows 21+); grammar above
      updated in the same pass.
- [ ] Golden examples machine-checked: valid reader parse → valid expansion
      into core forms → round-trip losslessly per §6 acceptance criteria
      (Phase 1, after grammar freeze).

---

## 10. Post-mortem: what v0 got wrong (kept as calibration)

1. **S-expr prefix parens can't beat BPE-fused infix.** The v0 hypothesis
   `(. recv Member)` / `(static Type Member)` assumed the paren-deliminated
   atoms are dense; probes showed each such form trades one fused infix
   token (`'.Find'`) for 2–3 unfused ones (`'.' '('`).
2. **Whitespace is the #1 token cost** — BPE fuses glue spaces into
   neighbors; every mandatory space between atoms is one token.
3. **Keyword abbreviations don't pay.** LLM vocabularies carry full English
   words as single tokens; single-letter keywords don't fuse and score
   identically to abbreviations, so the floor holds for free.
4. **Interpolation is already minimal** — `{u.Name}` inside a string costs
   the same either spelling; no second string-token type justified.
5. **The 30% claim will not come from syntax alone.** A syntax at ~90% of
   baseline leaves only ~10% to syntax; the remaining ~20% must come from
   *structural* savings (directives, imports, boilerplate classes) on real
   corpora — the go/no-go (0.8) must measure that, not re-litigate the
   surface syntax.