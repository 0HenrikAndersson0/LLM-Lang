# Task 5.4 — C# emit vs hand renditions (diff report)

> Phase-5 gate (PLAN 5.4): real corpus (9 Humanizer files) emitted by
> `irc to-cs`, compiled under `dotnet build`; diff vs hand renditions.
> Binary = `target/debug/irc` (5.3+, commit aabee86+); dotnet 10.0.400.
> Gates measured 2026-10-10. Gate rig mirrors 2.4: the corpus is 9
> fragments of the real Humanizer library, so semantic resolution runs
> against a **stub world** (35 generated types + project-level BCL
> usings) — the C# analog of 2.4's `tsc --noResolve`.

## Gates

| gate | result |
|---|---|
| golden corpus `dotnet build` (5.3, per-file projects + stub rigs) | **8/9** (03_pipeline excluded — proven upstream: the golden fragment itself fails identically; `.Select(o=>o.Total)` vs declared `List<Order>`) |
| real corpus, parse/structure phase (`dotnet build` syntax classes) | **9/9 clean** after fixes below |
| real corpus, semantic phase vs stub world | member-level diagnostics remain **by gate design** (see below) |
| `cargo test` (48 lib incl. all-canon identity + 7-fixture identity) | green after every fix |

## Emitter fixes shipped en route (each additive; identity re-verified)

| row | fix |
|---|---|
| `bracket_slice` depth tracking — inner `(…)` pairs inflated depth; only the close char decremented (`Units[(Int)i]` → `]]`) | decrement on any `)]}`, close match checked separately |
| cast operand off-by-one (`(Int)x` never fired) | operand at `atoms[j]` (bracket_slice leaves j past the close) |
| C#-canon interpolation — payload carried the leading `$` (`$"...")` → `$$"..."`) | `$`-atom + payload pairing (`$` lexes `Ident("$")`); `strip_prefix('$')` on payloads |
| mid-run pattern keywords glued (`b.M is` → `b.Mis` — the canon glues; the ident branch only spaced AFTER keywords) | space BEFORE keyword when the output ends word-ish |
| `}`-close glue before a binder (`is not {} multiple` → `{}multiple`) | `}` + word → `} ` (group-close path included) |
| `else` arm in switch: the splitter folds it to `Sym("else")` — emit checked only Ident → C# got `else =>` | `arm_is_else` accepts Sym form; both switch builders |
| pattern designators dropped at expansion (`ICollection<T> items` → `ICollection<T>`) — expander `split_switch_arms` binder skip | designator stays in the pattern (canon text unchanged — text was dropped; now reproduces the fixture row; full-corpus identity green) |
| C-style `for` heads (cs_02 `for c=culture; !…; c=c.Parent`) had no lowering row | head atoms carry `init;cond;step` — emit verbatim |
| return-less fns emitted without a type | `void` (non-ctor) |
| extension fns overwrote their params with the receiver (cs_06 `Humanize(format)` lost `format` → duplicate-signature CS0111) | receiver PREPENDED for receiver-less canon rows; canon `this`-param rows (JS-style) reuse the row's own receiver |
| `static` duplicated on extension fns (CS1004 ×46) | single `static` discipline across `mods`/extension path |
| `const` marked static (CS0504 — C# consts are implicitly static) | static suppressed for const rows |
| ctor single-statement bodies wrapped in `return new(…)` (CS0127) | tail-wrap requires `ret.is_some()` |
| `new Type` with no args/initializer in the run (canon dropped a multi-line `{…}` initializer — reader-level row) | `new T()` fallback |
| expander var-branch dropped run-end `{…}` payloads after a real value (J1's hijack covered empty-value/`=>` only) | capture braces inline into value atoms (identity green) |
| cast operands dropped (`(long)b` — the c-sharp grammar's field is `left`, not `expression`) | `expression`/`left`/`value` fallback chain |
| void rets case-sensitive filter (`void` lowercase passed) → void fn tail `return` (CS0127) | `eq_ignore_ascii_case` reader+emit |
| else-arm bodies truncated at the first row (`alt.child(1)` sliced into the block — the c-sharp grammar's `alternative` field holds the block DIRECTLY) | wrapper-vs-direct check |
| else-if cond atoms (`c.child(1)` again → operator-only `>=`) | `cond_atoms` |
| `internal` on LOCAL functions (CS1003/8124) ×~300 | `fn_sig(f, type_ctx, in_static)` split |
| local fns' ret field (`type`, not `returns`) → `void outside(...)` + CS0201 | ret fallback |
| static-host members not static (CS0708) | static push for all non-ctor members of static hosts |
| `$`/`{count}` data-braces vs interpolation indistinguishable at canon | **C#-reader distinguishes** (interpolated_string vs plain literal); plain literals' braces canon-escape `{{…}}` and the emit restores C# literal braces |
| `struct UnitPrefix(string name…)` primary-ctor params dropped → field-inits unresolvable | primary capture (reader) + canon print (fixture-row shape — the expander already parses `type X(params)`) |
| property computed rows (`LongScaleWord => …`) | value-field arrow detection |
| opaque fragment statements emitted as string literals (CS0201) | drop with `/* opaque */` flag in stmt/arm/inline paths |

## Fidelity findings recorded (upstream / design limits — NOT emitter bugs)

| row | tag | evidence |
|---|---|---|
| hand-canon elision rows `<rest-omitted>` in cs_08 (2) — the hand rendition skipped long bodies | ✍️ | cs_08_formatter.ir:108,290; gates delete them for structure compilation — bodies remain incomplete by construction |
| multi-line `new T{dict-initializer}` rows lose the rows at the reader level (R3 runs them together; canon prints the argless ctor) — `new Dictionary<Char,UnitPrefix>` + 17 rows | 📐⚙️ | cs_05 canon row 11-28 vs canon print; CS1526 fix emits `new T()`; honest initializer capture needs reader work (queued) |
| `var EnglishFallback=new(CultureInfo.GetCultureInfo("en"))` — target-typed new whose type the canon lost → `dynamic` + `new ()` | 📐 | CS8752 ×2; canon-type loss |
| switch-arm type-pattern designators (fixed above); value-position `switch` tails `return …switch{…}` work | ✅ after fix | cs_03/cs_05/cs_08 rows |
| field-vs-property nuance: canon members render private-readonly fields; originals mix `{ get; }` properties | 📐 | cs_02 vs cs_03 originals (printed-quality diff; compile-neutral) |
| overload sets collapse (cs_03 `Humanize` ×4 — different canon param spellings keep them distinct; two identical-shape rows would CS0111) | ✅/📐 | interface surface carries the set |

## Stub-world boundary (by design)

The corpus references the real Humanizer API surface (`ByteSize.FromBits`,
`ConditionalWeakTable.GetValue`, `CultureInfo.CurrentCulture` — real BCL),
`MetricNumeralFormats.HasValue`…). The stub world declares **types**
(35) but not their members, so `CS0117/CS1061/CS1501/CS1955` diagnostics
mark exactly the API calls the canon carries — by construction, the
emitted call sites are verbatim canon rows (the same rows the originals
compile against inside the library). The member-level semantic gate
therefore reduces to the real-library context, which type- and
member-collides with redeclared emitted types — the same reason 2.4 used
`--noResolve` for tsc. Full-library semantic compile is queued as a 5.4
follow-up behind a redeclaration-aware rig.

## 5.6 leg-1 round-trip gate (first pass, same session)

`cs_read.rs` + `irc from-cs`; round-trip artifacts `bench/corpus/real/rt_cs/`:

- **canon identity: all 9 cs_*.cs + 17-row construct smoke green**
  (`cargo test --lib cs_read`).
- **re-emit leg: syntax-clean** after 9 + 13 further reader/emitter fixes
  (second-pass queue below); the re-emit gate closes round 1 with
  only the design-limit + stub-boundary rows remaining. Original fixes:
  emitter fixes (arg-wrapper descent over all children incl. out-args;
  string-payload content children; verbatim-structural syms restored;
  keyword-text→Ident map; C#-for part fields `initializer/condition/update`;
  switch arms — `switch_core` arms + interface signature-row emit   interface signature-row emit + the
  interface-kind canon print (now `type X interface{`, reparse-stable);
  trailing-switch subject strips a leading `return`; pattern designator
  split excludes statement keywords; ret-tail void filter; `?`-suffix
  ternary split at emit; opaque-fragment statement drop).
- **Remaining diagnostics (215 unique) root-caused**: 207 = the stub-world
  **member** gap (`ByteSize.FromBits`, `CultureInfo.CurrentCulture`,
  `LocalizedInflectionCatalog.TrySelectCategory` — types resolve + are
  enum-shaped, members don't) — by gate design, the corpus compiles inside
  the real Humanizer library (emitted cross-file calls now carry the
  internal-default and resolve like the originals). 8 = the **char-vs-string
  canon row** (C# `'0'`/chars canon to Str atoms; C#-emit restores strings —
  `new string("0", …)` CS1503, `c=="0"` CS0019, `chars[i]="0"` CS0029) —
  a design-limit row for `docs/fidelity.md` (the IR has no char atom; a
  core-form extension is out of scope for additive-only v0). All syntax
  classes (CS1001/2/3/1525/1519/0708/0127/0201/0122…) now emit clean.

## 5.6 leg-2 — behavior rig (boundary documented)

The C# analog of the J1 behavior harness was built incrementally; the
emitted corpus cannot RUN at v0, and the boundary decomposes precisely:

- **char-vs-string design rows (55 diagnostics / ~10 rows)** — `Symbols`
  (`List<char>`, char-key dict entries), char comparisons
  (`chars[i]=="0"`), `PadLeft(n, "0")`, `IndexOf(char)`-args, and
  `ToString(char…)` overloads all canon as Str atoms. The C# emit restores
  strings in char positions → compile-level type errors inside the
  emitted data rows. Fidelity row 17; unblocking needs either a Char
  core-form atom (frozen-grammar change → Phase-0 re-run) or a
  reader-level char-annotation convention — Phase-6 design decisions,
  deliberately not rushed here.
- **stub-member rows (~195 diagnostics)** — the library API surface
  (`ByteSize.FromBits`, the phrase-table catalogs, cross-file
  `DataUnitHumanizeExact` — that member IS carried by the emitted file
  now, with the internal default). The rig closes the gap only with the
  real Humanizer library compiled alongside, which redeclares the
  corpus's own types — the same shape 2.4 chose `--noResolve` for.

What leg-2 established positively: the emitted corpus structure-compiles
end to end for all files except the two char-row files (cs_05/cs_06 —
every non-char row in them compiles, verified by the diagnostic
decomposition), canon identity is stable across all 9 files, and the
emit-side field-mutability analysis (`readonly` + assignment → CS0191)
keeps round-tripped mutable state mutable.

**Behavior identity for the C# round trip: deferred to the char-atom
decision (Phase-6 fidelity work).** The JS leg's behavior gate (3/3
subjects) remains the strongest verified behavior evidence in the repo.

## Verdict — Phase 5 status

- 5.3 golden gate: **8/9** (1 proven-upstream: the golden fragment itself
  does not compile in its own world).
- 5.4 real-corpus emit: **syntax-clean after the row-queue passes**;
  every semantic diagnostic attributed (stub boundary / char design row).
- 5.6 leg-1 round-trip: canon identity 9/9 + re-emit syntax-clean.
- 5.6 leg-2 behavior: boundary documented above; the corpus is
  library-context dependent.
- **Remaining: 5.5 (LLM loop test for C#, `--lang cs`) + the Phase-6
  char-atom decision.**
