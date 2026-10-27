# LLM-Lang language spec (reference)

The complete reference for LLM-Lang canon v1: syntax, core forms, types,
the two targets, and the invariants the toolchain enforces. Companion
documents: `docs/language-design.md` (design + measured decisions),
`docs/tutorial.md` (the teaching walkthrough), `docs/versioning.md`
(policy), `docs/fidelity.md` (round-trip budget).

## 1. Lexical structure

- **Files**: `@1` version directive first; `@ns Path.To.Module`;
  `@use Path.To.Module` repeats for imports. Then one declaration per
  top-level row; children indent two spaces.
- **Comments**: none (dropped by contract; the readability floor is the
  layout itself). Hand canon may carry `//` rows; machine canon drops.
- **Identifiers**: unabbreviated (the readability floor). Type words
  canonize capital: `String Int Bool Float Int64 ULong Decimal Char Byte Short
  UShort UInt SByte Object`. Target keywords stay lowercase keywords.
- **Atoms**: idents, numbers (with `m`/`d`/`f` suffixes pass through),
  strings `"…{interp}…"` (C#-reader also accepts literal-brace data
  `{{…}}`), regexes `/pattern/flags`, and structural syms
  `( ) { } [ ] < > , : ; . ? ! | & => .. ?? … etc`.

## 2. Grammar (canon v1.1, ~24 core forms)

```
file         := directives (decl)+ EOF
directives   := ('@1' | '@ns' PATH | '@use' PATH)*
decl         := fnDecl | typeDecl | varDecl | reexport | expr
fnDecl       := ['async'] ['export'] ['sealed'|'abstract'] 'fn'
                Name ['<' TP '>'] ['(' params ')' ['->' 'Base']]
                '(' params ')' [Ret] body
params       := '(' param (',' param)* ')'        // param := [mod] [Name] Type? ['=' dflt]?
typeDecl     := 'type' Name [params] [mod ('mod')*] [':' bases] ['{' member* '}']
varDecl      := ['const'|'volatile'|'export'] 'var' ['mut'] Name ['=' expr]
typeMember   := typedField | computed | method | ctor
typedField   := Type Name ['=' expr]
computed     := Type Name '=>' expr
ctor         := 'ctor(' params ')' [':' base] body
reexport     := 'export' ['type'] '{' NAMES '}' 'from' MOD
              | 'export' ['type'] '*' 'from' MOD
stmt         := ifS | forS | foreachS | whileS | lockS | matchS | tryS
              | retS | throwS | breakS | continueS | yieldS | varDecl | expr
ifS          := 'if' c body ['else' body | elseIfS]*
forS         := 'for' Name 'in' iterable body | 'for' '(init;cond;step)' body
matchS       := 'match' e '{' (PAT body)* '}'
tryS         := 'try' body ('catch' TYPE [binder] body)* ['finally' body]
expr         := verbatim atom run (expression surface = the target's own)
```

Statement bodies fuse inline when a single statement or expression:
`fn Cap(String s) String{...}`, `if c{r}`, `for i in 0..n{acc+=i}`; use
braces + indent for multi-statement bodies. A body's last bare expression
is a return when `Ret` is declared.

The **closure invariant** (enforced by `is_closed`): after macro
expansion every form is in the table above — nothing else remains.

## 3. Types

| canon | JS emit | C# emit |
|---|---|---|
| `String` | string | string |
| `Int` | number | int |
| `Bool` | boolean | bool |
| `Float` | number | double |
| `Int64` | number | long |
| `ULong` | number | ulong |
| `Decimal` | number | decimal |
| `T?` | erased | `T?` |
| `Array<T>` | array/Array | `T[]` |
| named types | verbatim | verbatim |

Type words carry no runtime weight on JS. Generics pass verbatim
(`List<T>`, `Func<A, B>`). `where` constraints drop (contract).

## 4. Expressions (verbatim surface)

The expression layer is the *target's own* surface, canon-tight:
calls/chains `a.b(c.d)`, binaries `a&&b c<d`, ternary `c?a:b`, null-ops
`x?.y x?[i] x??d`, casts `(Int64)x`, lambdas `(a,b)=>e` (block bodies
glue statements), collection literals `[a,b]`/`[..x]`, index-from-end
`[^i]`, interpolated strings `"x{e}y"`, `new(args)` / `new Type(args)`,
`is`-patterns (`is { } b`, `is < 0 or > 15`, `is null`), `typeof`,
`nameof`, `delete m[k]`, `await`, `out var x` args, regex literals.

**Canonization table** (read-side sugar): `i++`→`+=1`; template strings →
interpolation; function-expressions → arrows; callee generics erase at
emit; `.filter/.map/.sum/.toList/.sort` lower per target (JS verbatim;
C# LINQ `Where/Select/Sum/ToList/OrderBy`).

## 5. Statements

Row shapes (already §2 grammar) with canon bodies:

- `if c{…}else{…}` chains; braceless arms canon to braced
- `for i in a..b{…}` (range, half-open); `for x in xs{…}` (foreach);
  C-style `for(init;cond;step)` verbatim
- `while`, `lock e{…}`
- `match e{ PAT body }` — arms are literal patterns, bodies are
  statements (write `return` explicitly inside fns)
- `try stmt / catch TYPE b? stmt / finally stmt` — clause rows are the
  canon lines (`catch Error e throw e`)
- `return e?`, `throw e`, `break`, `continue`, `yield`

## 6. Members and types

```
type Name export{            // export = public; kind word = interface |
  Type name                  // struct | record; class implied
  Type name = init           // field with initializer
  Type name => expr          // computed property
  volatile var flag          // volatile member
  const var Limit=27         // const member
  ctor(params){…}            // constructor (base: `: this(...)` rows)
  fn Name(args) Ret{…}       // method
  fn Recv.Name(args) Ret{…}  // extension fn (receiver = this)
}
```

`export` = public. All other access collapses to target-internal by the
emit (C# `internal`, the fidelity doc row 18). Statics: receiver-less
members of a static-host canon rows emit `static` (C# static-class rule).
Primary constructor params canon-print (`type Name(params)`).

## 7. Modules and re-exports

`@ns` → JS: dropped (ESM files); C#: `namespace X;` (file-scoped).
`@use P` → JS: side-effect `import "./P.js"` (+named rows when known);
C#: `using P;` sorted after the ns. Re-export rows: JS passthrough;
C#: dropped with a flag (no file-level equivalent).

## 8. Printing (canonical layout)

`print(read(x)) ≡ print(expand(print(read(x))))` — enforced per corpus
file. Minimal spacing: `{`/`(`/`?` fuse to the preceding word; operators
tight; identifiers one space apart; keyword-space rule (a keyword takes
a following space; `null`/`true`/`false` space before most punct).
`!==` needs the leading space (the `x!` suffixed ident rule). Collections
canon tight (`[a,b]`); `T<A, B>` spaced-args in canon text where the
reader re-lexes.

## 9. Round trip

- Reader: tree-sitter (JS/TS, C#) → Core forms (additive-only; §2 table).
- Canon identity per file (all 26 corpus + 16 C# rt files green).
- Emit: `to-js` (2.3 table + J/J5 verifications) and `to-cs` (§5.2
  table + the 5.4 dotnet gates). Opaque rows emit verbatim with
  `/* opaque */` statement flags where fragments can't.
- Fidelity budget: `docs/fidelity.md` (JS rows 1–16; C# addendum 17–22).

## 10. Targets

- **JS** (`.js`, ESM) — the fully-verified path (behavior gates 3/3
  harness subjects; node --check 26/26; tsc 7/7).
- **C#** (`.cs`, .NET 10) — emit + reader verified; behavior rig
  boundary documented (row 17/18/21; the library context).
- **C#-lite**: versioned `@1`; additive-only growth per
  `docs/versioning.md`.