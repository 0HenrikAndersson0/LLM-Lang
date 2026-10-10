# JS Target Design (2.1 AST + printer, 2.2 lowering table)

> Phase 2. Design-first: this note freezes the JS emission model before
> `crates/lang/src/js.rs` gets written. Downstream of language-design.md v1.1
> (frozen grammar + core forms).

## 0. Scope

IR → plain modern ES modules (`.js`). The JS target has **no type system**:
type surface is erased (fidelity §6.3 — locals untyped; §3 — types only in
signatures, which JS doesn't carry). Exit gate: golden corpus + `js_*` real
corpus pass `node --check` / `tsc --noEmit` (PLAN 2.4); LLM repair loop
(PLAN 2.7).

## 1. Expander finding (blocks 2.3): statement bodies lost in catch-all

Round-trip *text* identity (Phase 1 exit) holds, but the expander's
catch-all in `expand_line` maps every non-def keyword line
(`match`, `for`, `while`, `if`, `try`, `lock`, bare statements) to
`Core::Expr(atoms)` and **drops `children`/`inline`** — match arms, loop
bodies, if-else bodies are gone from the core-form layer. Golden fixture 04's
`match r.Kind{…}` expands to a bare header; fixture 06's `for x in xs{…}`
keeps only the inline atoms.

Fix (additive, per core-form-closure agreement): keep `Core::Expr` for
body-less statements, add one core form carrying structured control flow:

```
Core::Stmt { head: Vec<Atom>, body: Body, arms: Vec<(Vec<Atom>, Body)> }
```

- `body`  → the structured body (`for/while/if/lock/try` + bare-fused
  single-expr bodies); `Body::Bare` = braceless line children,
  `Body::Inline` = `{…}` fused on the header line, `Block` = both.
- `arms`  → match/catch/else clauses: each arm's pattern atoms + its body
  (`catch NotFound return null` = arm `["NotFound"]` + inline return).
- Every emitted `Stmt` keeps `is_closed` true: `core::kind()` returns
  `"stmt"`, which joins `CORE_KINDS`.

The IR **surface text is unchanged** by this — the reader already parses
these lines; only the expander's classification improves. The Phase 0 bench
measures surface text token counts, so classification cannot move the
headline number; noted in PLAN 2.2 as the working-agreement check rather
than a full re-run. (Re-run before shipping 4.x round-trip anyway.)

## 2. JS AST (estree-style, minimal — not full ESTree)

Why not the full ESTree spec: the IR's expression layer is already
JS-shaped infix (Route 1 chose it for exactly this), so expressions stay
**verbatim atom runs**; structure is needed only where JS grammar demands
nesting (decls, control flow, catch clauses, template literals). A full
expr tree would add a whole re-parser for zero emission benefit in v0.
A later phase that needs JS→IR (Phase 4) will need tree-sitter-JS anyway
(PLAN 4.1); re-using spans from that parser is the plan of record.

```rust
enum JsNode {
    Program { header: Header, items: Vec<JsNode> },   // header → import/export lines
    Import  { names: Vec<String>, from: String },      // @use path → ESM import
    Export  { what: Box<JsNode> },                     // export fn/var/class; export {…}
    Class   { name, kind, super: Option<String>, members: Vec<JsNode> },
    Method  { async, name, params: Vec<String>, body: Body<JsNode> },
    Field   { name, init: Option<String>, computed: Option<String> },
    Ctor    { params, super_call: Option<String>, body },
    Fn      { async, name, params, body },             // free + local functions
    Var     { name, mutable, init: Option<String> },   // const (default) / let (mut)
    Stmt    { head: String, head_binders: Vec<String>, children: Vec<JsNode> },
            // for(…of…)/while/if/lock-omitted with a block child or fused inline
    Switch  { disc: String, arms: Vec<(String, String)>, default: Option<String> },
    Try     { body, catches: Vec<(binder, body)>, finally: Option<String> },
    Expr    { text: String },                          // verbatim lowered atom run
    Raw     { text: String },                          // opaque passthrough only
}
```

`Stmt.head` carries the already-lowered header text (`for (const x of xs)`)
plus declared binders (`x`) for span work in 2.6. `Expr.text` is emitted
with no re-spacing — it is produced by the Atom→JS rules (§3), which keep
canonical tight spacing; JS accepts all of it (`o.Paid&&o.Total>100` is
valid JS).

## 3. Atom → JS mapping (shared by all lowering)

| IR atom | JS | note |
|---|---|---|
| `100m`, `1.5d` | `100`, `1.5` | Decimal suffix dropped (fidelity §3: no Decimal in JS) |
| `null true false` | as-is | JS-native |
| `"Hello, {u.Name}!"` | `` `Hello, ${u.Name}!` `` | interpolation → template literal (§4 sugar) |
| `/re/flags` | `/re/flags` | verbatim |
| `?`, `??`, `?.`, `!` | `?`, `??`, `?.`, dropped | `!` is C# null-forgiving — erase (no TS emitted) |
| `&& \|\| == != < > <= >= + - * / %` | as-is | tight spacing valid JS |
| `=>` (lambda) | `=>` | as-is |
| `...x` | `...x` | spread native |
| `is {}` binder pattern | `instanceof`/destructure — **not v0** | js corpus uses plain exprs; C#-corpus patterns stay C#-side (Phase 3) |

## 4. Lowering table (2.2): core form → JS

| Core form | JS emission |
|---|---|
| `Header{version,ns,uses}` | version: dropped (no JS equivalent — tooling metadata only); `ns`: dropped (ESM files are module-scoped); `@use P` → `import … from "./P.js"`; `@use a.b` → `"./a/b.js"`. Exact names of imported symbols are unresolved in v0 → emit `import * as a? from` — **no**: emit `import "./P.js"` side-effect form when names are unknown, named imports only when the `@use` carries them (`@use` paths are bare in v1; side-effect import is the honest v0 shape; PLAN 2.4 diff-vs-hand-renditions will show the gap) |
| `Fn` w/ `export` | `export function f(params) {…}`; `async` → `async function`; inline body `\{e\}` → `function f(params){ return e; }` when the fn has a ret type or the body is an expression; statement bodies → verbatim stmt lines |
| `Fn` w/ `receiver T.m` | JS has no extension methods → emit as free function `m(t, …)` with receiver as first param, **and** record a lowering note; C# target (Phase 3) is the native one |
| `Fn` w/ `ctor` | `constructor(params) { super(args)?; body }` |
| `Type{kind}` | `class Name extends Base {…}`; `interface` → TS-only — emit `/** @interface */` comment + skip when targeting pure JS (real-corpus js_02/j s_types stays under tsc later; v0 golden corpus has interface files → emit empty class + JSDoc); `record`/`struct` → class with fields |
| `Type{primary}` | primary-ctor params become `constructor(params){ assign each }` with `this.X = x` per param |
| `Var` top/member, `computed` | `const name = e` (mutable: `let`); member `computed` (`Type name => e`) → `get name(){ return e; }`; member field with init → class field `name = e`; member field w/o init → declaration only when typed, else `constructor` assignment is the C# convention — JS: emit bare field name (TC39 class-field semantics) → **emit `name;` decl only, no value** |
| `Var` w/ `opaque` | dropped unless init present (opaque blob stays for round-trip only) |
| `Expr` (atom run) | verbatim §3 mapping, one line per expr |
| `Stmt{for x in xs}` | `for (const x of xs) {…}` |
| `Stmt{for i in a..b}` | `for (let i = a; i < b; i++) {…}` (half-open, design §2.2) |
| `Stmt{while c}` | `while (c) {…}` |
| `Stmt{if c{a}else b}` | `if (c) {…} else {…}`; expr-arm form `if c a else b` → ternary only in expr position — v0: emit statement form |
| `Stmt{lock e}` | dropped body → `{…}` (single-threaded; JS has no lock — lowering note) |
| `Stmt{match e{arms}}` | `switch (e) { case p: … break; default: … }`; arm exprs wrap with `return`? no — arms are statements in fn bodies, and expr-matches in the corpus are `x? y : z` ternaries already. v0: `case "GET": Read(r.Path); break;` |
| `try s / catch P b s / finally s` | `try { s } catch (e) { if (e instanceof P) {…} else { throw e; } } finally {…}` — pattern + binder → `instanceof` guard chain; pattern-only → guard without binder name |
| `throw e` / `return e` / `await e` | verbatim (JS-native) |
| spread/out-args | `out` → unsupported in JS: lowering error at emit time (must be resolved by sugar before this stage); real js_* corpus contains none |
| string interpolation | template literal (§3) |
| `.toList()` | `[...receiver]`; `.map/.filter/.sort/.reduce` | native, verbatim |
| `.sum()` | expansion: `.reduce((a,b)=>a+b,0)` |
| `List<T> Map<K,V> Set<T> Array<T>` | erased (no type surface); `Array<T>` fields in JS corpus are plain arrays |
| `for init;cond;step{…}` (C-style) | `for (init; cond; step) {…}` verbatim; bare `x = e` init upgraded to `let x = e` (ESM strict mode) — additive row, added with 2.6 to restore cs_02 emission |
| `[.. x]` array spread | `[...x]` (corpus hand-form `..` tolerated; grammar `...` preferred) |

## 5. Canonical JS layout (printer, 2.3)

1. imports first, one per line, blank line after.
2. One decl per statement; 2-space indent; `{` glued to preceding word
   (JS can't glue class-brace to keyword the way IR does — still minimal:
   `class Order{` is valid JS).
3. Tight operators (§3) — identical to IR because it's legal JS.
4. Bodies: single-expression → fused inline `{ return e; }`; multi-statement
   → one stmt per line.
5. `/** @type … */` JSDoc for member fields that had IR types (keeps
   `tsc --noEmit` meaningful on the real corpus without emitting TS) —
   **cut to keep v0 emission pure-JS; revisit at 2.4 diff** (span-fidelity
   concern: JSDoc lines shift line numbers for 2.6 span mapping).

## 6. Verification for this phase

- Every golden fixture → JS → `node --check` (2.3/2.4). Re-verified 26/26
  after the 2.6 layout fixes (`node --check` gate re-run over corpus).
- `js_*` real corpus → JS → `tsc --noEmit` with `allowJs` (2.4).
- Lowering notes recorded per gap (receiver fns, lock, imports) — these
  feed the diff-vs-hand-renditions report (PLAN 2.4 footnote [2]).

## 7. Span tracking (PLAN 2.6) — IR ↔ JS line mapping

Every core form carries a line-granular `Span{lo,hi}` (1-based, inclusive;
`lo` = header line, the stack-trace anchor; `0` = synthetic, e.g. primary-
ctor assignments). Thread: reader forms → expander `Core` (Header/Fn/Type/
Var/Expr/Stmt/Clause + `Arm`) → emitter.

Emission is marker-based and side-effect-free: each emitted form's text is
wrapped in invisible control-char markers (`\u{1}~irLo..irHi\u{1}` …
`\u{2}`), stripped by a single post-pass that also records nesting-accurate
JS line ranges. Result:

- `js::emit` — plain JS, **byte-identical** to pre-span output.
- `js::emit_with_map` — `(String, js::SourceMap)`; same text, plus:
- `SourceMap.js_to_ir(js_line) -> Option<Span>` — innermost entry wins;
  unmapped lines (`}`-only, blank) fall back to the nearest preceding
  entry (stack-trace anchors). `ir_to_js(ir_line) -> Option<(js_lo,js_hi)>`
  covers arms too (each if/match arm is marked with its `Arm.span`).
- `SourceMap.to_json(source)` — `{"source":…,"mappings":[[jsLo,jsHi,irLo,irHi],…]}`.

CLI: `ircc to-js --map f.ir` writes `f.js` + `f.map.json` (harness interface
for 2.7). Lowering failures carry their IR line: `js: <msg> (IR line N)`.

Known limits (accepted for v0): line-level only (columns not tracked —
stack traces carry columns but line fidelity is what the repair loop
needs); catch-guard inline bodies map to the try line, not each catch arm;
synthetic emission (primary-ctor field assigns) maps to `0`. The corpus
gate (`tests/spans.rs`) asserts every non-blank JS line maps and no
markers leak.

Emitter gaps recorded at the 2.4 gate (bench/results-js-diff.md), status
after J2/J3 (v4 Phase 3): RESOLVED — `export {…} from` re-exports (additive
`Core::Reexport`: canon prints the re-export line; JS drops the `type`
word), inline statement bodies in arrow-block runs (statement-ified at
emit: `var/const/let` decls, `if/else` pieces, `return`/`throw`, bare
terminal expr → callback return value; `split_stmt_runs` + `stmtify_piece`),
callee-generics erasure (`map<T,…>(` → `map(`; type-ish heuristic guards
comparisons), bare-`let` rendition drift canon-stabilized to `var mut`,
typed-var initializers (the ty-branch consumed the `=` — latent since 2.3),
empty-objlit `= {}` inits, and bool/null literal match arms. Canon rule
added: one space between a generics-carrying ty and `=` (`Record<String,String>`
+ `={}` lexes `>=` and would reparse wrong).

Still open (accepted v0): value-position `match` arms emit statements —
models write explicit `return` per the guide; statement-shaped content
that is NOT keyword-led inside inline runs (multi-assignment pieces without
`var`) stays verbatim; `tree-sitter` (Phase 4) is the plan of record for
full inline-run classification.
## 8. Round trip: JS → IR (PLAN 4.3 mapping)

Direction contract (language-design §6): the reader (JS → IR) is explicitly
lossy; the writer back is lossless *relative to the IR*. The 4.2 audit
(bench/results-coverage.md) classified every tree-sitter CST kind in the js
corpus; this section is the construct→form mapping the 4.4 reader implements,
plus the `(opaque …)` capture design that bounds the loss.

### 8.1 The `(opaque …)` capture design

Opaque capture bounds worst-case fidelity loss at zero per member: anything
the reader cannot map is captured **verbatim (spans + original text)** and
re-emitted byte-identically. Rules:

1. **Capture unit: the whole declaration or member — never a partial
   expression.** A captured class member, statement, or top-level type
   re-emits exactly as read (including its original formatting).
2. **Storage:** the existing `Body::Opaque(String)` + the `opaque` modifier
   word (expander §63/90) hold the captured text; canon prints it as-is —
   captured text participates in print→print identity verbatim.
3. **Capture sites (exhaustive):**
   - `opaque type Name = …` — type aliases whose bodies are union/template/
     literal machinery (§6.6, reconcile row 21; js_02.ir:5-9 are the canon
     precedent). Interface-shaped aliases (`type X = {a: B}`) are NOT
     captured — they map to `type X interface{`.
   - any statement the reader cannot classify into the §2 core closure
     (the expander's expr-statement fallback generalizes: a genuinely
     unclassifiable *line* becomes a `fn … opaque{}` body or an
     `opaque:`-prefixed atom — never dropped).
   - grammar-level parse failures (the ts fallback below).
4. **When it triggers (from the audit):** verdict `opaquetype` (8 nodes,
   js_02 aliases) and the one `grammar` node (`export type *` is NOT opaque
   — it maps to Core::Reexport; the audit's ERROR is the ts grammar's gap).
5. **Double-trip acceptance:** JS → IR(±opaque) → JS must compile and
   behave identically — opaque members guarantee this by construction.

### 8.2 Mapping table (by audit bucket)

**Core / verbatim** (reader emits the corresponding core form; names and
literals pass through by floor #1):

| JS construct | IR surface |
|---|---|
| import decl (incl. named imports) | `@use Path` (+ named form, J4) |
| export stmt / `export {a,b} from m` / `export type *` | `export` modifier / `Core::Reexport` |
| function declaration | `fn Name(params) Ret{…}` |
| arrow function / function expr | `x=>e` / `x=>{…}` (block lambda) |
| lexical decl + declarator | `var name=expr` (let/const → `var mut` canon) |
| expression statement | statement run / `expr-statement` fallback |
| if / else, for, for-in, while | `if c{…}else{…}` / `for i in a..b` / `for x in xs` / `while c{…}` |
| return / throw / await / yield* | `return e` / `throw e` / `await e` |
| call + member chain | `recv.Member(args)` — args incl. lambdas |
| binary / unary / ternary / assignment (+=` augmented) | infix atom runs / `!x` / `c ? a : b` / `x=e`, `x+=e` |
| subscript `a[i]` | `a[i]` |
| `typeof x`, `delete x.k`, `x as T`, `?.` | verbatim (hand renders: js_05.ir:26-28, js_01.ir:6) |
| `new T(args)` | `new T(args)` verbatim (cs fixtures) |
| object literal + pairs + spread | `{ k: v, …spread }` (J1 objlit) |
| array literal | inline literal run |
| template string + substitutions | interpolated `"…{e}…"` (§4 sugar) |
| regex literal + flags | regex literal (lexer 1.2) |
| params (+ optional `p?`) | `(name? Type …)` — `?` erased at emit |
| optional chain | `x?.y` verbatim |

**Sugar (canonicalize, §6.4):** template strings → interpolation; function
expressions → arrows; `i++` → compound assign (`i+=1`).

**Signature surface (erase to types-in-signatures, §3):** type annotations,
type arguments/parameters, predefined/generic/union/function/array/object/
parenthesized types, literal & lookup types (`SelectOptions['quirksMode']` →
member access on the alias, js_08.ir:24), `Boolean/unknown/void` type words,
generic constraints & param defaults (**not encoded** — printer emits
conventional defaults), `as`-predicates (`x is T` → §6.6 printer
reconstruction).

**Opaque capture:** type aliases with union/template machinery
(`opaque type`, §8.1.3a).

**Dropped (by design, not opaque):** comments (§6.1 — 25–40% of real
sources); formatting normalized to canon.

**Reader-side reclassification (2.4-era accepted-v0 rows):** value-position
`match` arms stay statement-shaped (guide teaches explicit `return`);
non-keyword-led inline statement pieces in atom runs — the 4.4 reader
classifies these from the tree-sitter CST instead of emission-time
heuristics (the J2/J3 `split_stmt_runs`/`stmtify_piece` classification is
the emission-side mirror of exactly this table).

### 8.3 ts-grammar fallback

tree-sitter-typescript 0.23.2 reports ERROR nodes where it lacks a construct
(`export type *` — known; mapped to `Core::Reexport`, not opaque). Rule: an
ERROR node whose source line matches a known-representable construct gets
its mapping; otherwise the enclosing member is captured opaque (§8.1.3c).
The 4.2 audit histogram is the checklist: any future corpus kind landing as
ERROR gets classified before the reader is extended.

### 8.4 Reader canon shapes (4.4, verified)

Decisions locked while bringing `read_js` + canon print through the identity
gate (`cargo test --lib roundtrip_tests`: all 7 corpus files + 30 construct
rows stable). These are canon-shape truths, not design drift:

- **fn type parameters are dropped** (`isCheerio<T>` → `isCheerio`) — the
  expander skips `<…>` on fn re-parse, so keeping them breaks identity.
  Type-ARGUMENTS in call positions stay verbatim and erase at emit (J3).
- **Signatures erase tight**: unions spell `a|b` in canon (the printer
  re-renders spaced only where the frozen keyword-space rule requires, e.g.
  `null`), function types erase entirely (param keeps its bare name), empty
  generics drop (`Array<AnyNode>` keeps contents; `Array<{…}>` → bare).
  Type words: string/number/boolean/unknown → String/Int/Bool/Any; void
  drops. Optional member `?` rides the type (`Bool? xmlMode`).
- **Overload signatures drop** (fidelity budget item 5, hand js_05.ir:3).
- **Type predicates** (`x is T` return types) erase to `Bool`.
- **Private members (`#p`) capture opaque** — the IR name model lexes no
  `#` (§8.1.3c member row `"opaque:…"`, print_member verbatim).
- **`cstyle-for → range` canon** uses the field name `increment` (not
  `update`); non-range C-style heads print verbatim.
- **Block-lambda bodies glue as statement-pieced runs** (`=>{stmt stmt}`),
  matching the J2 emission-side classification; bare if/for bodies fuse
  inline onto the head (`for i in 0..len{fn(array[i], i)}`).
- **`x?.y` reads the member's inline `optional_chain` child** and canon
  spells `?.` as two syms (lex-stable).
- **Alias/`as` union leaves spell through `type_text`** (array/lookup/object
  leaves included) — an as-union like `AnyNode | AnyNode[] | Document` stays
  representable text instead of capturing (4.5 tsc-gate fix).
- **`!==` needs a space before the operator in canon** — the frozen ident
  suffix rule (`x!`) makes glued `a!==` lex `a!` + `==`, which then drops
  the `!` at emit. `render_atoms` spaces before `!`-prefixed syms
  (hand canon's ` !==` spacing reproduces correct emit `str!=="string"`).

**Pipeline holes closed additively to make the round trip stable** (canon
identity gates were already text-only so these were silent before the reader
exercised them at scale): `expand_line`'s expr fallback splices run-end
`{…}` inline bodies back into the atom run (tail block-lambdas were dropped
on re-parse); the var-branch obj-lit hijack extends to `=>`-terminated
values; `expand_member` accepts the `"opaque:…"` verbatim member row;
`print_member` renders captured members verbatim (signature-only opaque
vars keep field rendering).
