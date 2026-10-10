# Versioning policy (6.4)

> v0.1 — frozen from this repo's working agreements and what actually
> happened across Phases 1–5. The rule set that lets core forms evolve
> without breaking agent files already in flight.

## 1. Core forms are additive-only

The ~24 core forms (language-design §2) never change semantics in place.
New capability = a **new atom or new row shape**, decided by measurement
(the 0.2 probe discipline) and admitted with:

1. A coverage audit row (the 4.2 audit class: representable / canon /
   signature / opaque),
2. A loop-test row (the construct appears in a repairable task),
3. The Phase-0 bench re-run if the grammar surface changes *textually*
   (what agents must re-learn).

Precedent set by the repo: `Core::Reexport` (J3), `Core::Stmt`'s arms
vocabulary (2.7), the `volatile ` member-var print (5.6 — additive canon
text), the data-brace canon-escape `{{…}}` (5.6 — canonicalized by the
C#-reader/emit pair; JS canon unchanged). The char-atom proposal
(fidelity row 17) is **the next candidate** and is gated on Phase 6's
fidelity work: it fixes 8 C#-emit rows but touches the frozen lexing
surface, so it needs the Phase-0 re-run discipline.

## 2. Opaque bounds fidelity, never blocks versioning

Unrepresentable constructs capture verbatim (`(opaque …)` / `opaque{ … }`
/ `opaque:` atom rows). The capture print is print-stable by construction,
so opaque rows *can't drift* with grammar changes: they round-trip as
raw source text. Adding real core forms later only *shrinks* the opaque
set — an agent file with opaque rows stays valid.

## 3. Canon text: layout is stable, semantics is frozen

The canonical layout (§5) is byte-stable by the identity gates:
`print(read(x)) ≡ print(expand(print(read(x))))` over every corpus file.
Canon-text changes happen only through:

- **Additive rows** (e.g. `type X interface{` kind words, the
  `volatile ` member modifier) — old files reprint unchanged because the
  absent marker is the default;
- **Reader normalization** — the printer may tighten/glue text if (and
  only if) the identity gate holds across the whole corpus; agents'
  existing files stay readable (the reader is the compatibility surface,
  not the printer).
- **Never** a rename of existing keywords or atoms (`fn`, `var`, `=>`,
  `..`, `?.` …): those are the tokens agents have in-context from
  the guide; renaming invalidates learned behavior, not just files.

## 4. Versions of what

- `@1` header carries the *file-format* version. Additive changes keep
  `@1`. A change that makes an old file **mean something different**
  (the char-atom flip `Str`→`Char` for single-quote texts, if adopted)
  bumps `@2` **with an automatic reader migration** (old text still
  reads; the new spell is printed going forward).
- The **writer** (`to-js`/`to-cs`) is versioned with the crates.io
  release; emitted target code carries no version (it's just code).
- The **loop harness** (`bench/scripts/loop`) is bench-only — no compat
  promise.

## 5. Deprecation

No deprecation before v1.0. The `opaque` escape hatch is the fallback:
if a canon shape proves broken, the reader re-classifies it (as the 4.4
C# reader did for the 2.4-era accepted-v0 rows) — agents' files keep
reading; the canon may tighten around them.

## 6. What absorbs growth

- **Macros/sugar at read time**: new source-language constructs land as
  reader sugar expanding into existing core forms (the mapping-table
  route, js-target §8.2 / cs-target §4) — no core-form growth.
- **Opaque capture**: the bounded-loss escape for unrepresentable
  constructs.
- **Target-side lowering**: `to-js`/`to-cs` absorb platform differences
  (the LINQ op map, the type-word map) without touching the IR grammar.