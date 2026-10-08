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

- Every golden fixture → JS → `node --check` (2.3/2.4).
- `js_*` real corpus → JS → `tsc --noEmit` with `allowJs` (2.4).
- Lowering notes recorded per gap (receiver fns, lock, imports) — these
  feed the diff-vs-hand-renditions report (PLAN 2.4 footnote [2]).