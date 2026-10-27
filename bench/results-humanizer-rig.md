# Humanizer behavior rig — build boundary (5.6 leg-2, session 2)

> Goal: compile Humanizer's real source tree (all 203 sibling .cs files +
> 102 locale yml + source generator) **in the same assembly** as the
> round-tripped corpus, so behavior identity can run at library scale.
> Setup: `/tmp/humanizer_src` (scratch clone @ Oct-2026 main,
> the corpus's byte-identical generation), `src/HumanizerRT/` — vendor
> sources minus the 9 corpus-mirror files, plus the emitted rt_cs corpus.
> The corpus files are byte-identical to vendor main (verified:
> MetricNumeralExtensions.cs diff = 0), so this is the exact sibling
> generation.

## What works

- **The round-tripped corpus re-emits and compiles** inside the vendor
  tree: modifiers (`virtual`/`override`/`static`/`internal`/`protected`),
  generic type-params (`LocaliserRegistry<TLocaliser>`,
  `Humanize<T>`), primary ctors, computed properties, and data rows all
  survive canon and emit (committed 7140a7f).
- **The emitted corpus's own rows now compile against the real library
  shapes** — e.g. `BuildNumber` calls resolve, the metric `{{…}}` dict
  data rows (char keys) work, the primary-ctor `UnitPrefix` resolves.

## The remaining boundary (~834 diagnostics, decomposed)

1. **Member access modifiers drop in canon member rows** (~400):
   `protected CultureInfo Culture`, `protected virtual string …`, and
   ctor visibility (`protected LocaliserRegistry(Func<…>)`) — canon
   member rows carry only public/const/volatile; the emit defaults to
   internal → vendor code accessing the emitted corpus's protected
   members/ctors fails (CS0122 ×~400 rows).
   - **Fix path**: carry the access word in member canon rows
     (`protected CultureInfo Culture`) + ctor rows — additive canon
     rows same as the type-modifier row landed this session.
2. **`readonly struct CompositeUnit`'s readonly, `public readonly
   string LongScaleWord => …` fields** — same access/modifier family.
3. **char-atom residue**: the C# emit now emits `'X'`; the char-typed
   Members (`List<char> Symbols`) resolve ✓. (The 8 char rows fixed by
   the atom DID clear.)
4. **cs_02's generics**: `LocaliserRegistry<TLocaliser>` rows now canon
   with type-params ✓ — remaining TLocaliser rows reference the generic
   param inside member rows that the emitted class declares ✓ (these
   compile once access rows fix the ctor visibility).

## Honest verdict (2026-10-10, superseded same week)

The behavior rig *built incrementally* but cannot reach green without
the member-access canon rows — the same class as the type-modifier row
(just landed) but member-granular: ~1 session of work (reader member
rows + expander member parse + printer/emit arms + the re-gates) to
close, then behavior cases can run against Humanizer's own semantics
(the library's own test suite could then run against the round-tripped
assembly — the strongest possible behavior evidence).

## Member-row pass (2026-10-11): the boundary CLOSED — rig builds 0 errors

The queued row set landed (additive canon rows, the type-modifier-row
precedent):

| row | fix | where |
|---|---|---|
| member access words | `protected CultureInfo Culture`, `private protected LocalePhraseTable PhraseTable`, `export ctor` — canon member rows carry access words verbatim (compound pairs joined at emit); `canon_modifiers` widened to the fn-family set | cs_read `canon_modifiers` |
| ctor visibility + initializer chain | ctor rows carry access (`export ctor (…)`) AND the `: this(...)`/`: base(...)` initializer (unnamed `constructor_initializer` child — `child_by_field_name` missed) | cs_read `ctor_decl` |
| modifier-led member rows | expander drains mod words on member rows (fn rows delegate per the 7140a7f path; field/property/ctor rows carry the mods) | expander `expand_member` |
| canon print | `print_member` prints the mod words before the type; `print_type` prints `: bases` (spec §2 grammar had them all along) + kind-before-primary order | printer |
| emit | access words + family words on member var rows (const/readonly/static discipline kept), explicit-impl dotted names, static-class detection excludes explicit rows (CS0112 root) | cs `member_var`, `fn_sig` |
| record structs | `record struct` kind phrase folds through `record_declaration` (+ literal `struct` token child); emit restores `record struct` | cs_read dispatch, cs emit_type |
| static lambdas | `static locale => locale` carried as canon idents (was: dropped in the lambda path, glued into `staticlocale` in the verbatim path — the CS0103 row) | cs_read lambda arm + verbatim keyword-text arm |

**Result: the vendored rig (vendor tree minus the 9 corpus mirrors +
the emitted round-tripped corpus) compiles 834 → 0 errors** — canon
identity 9/9 (`ircc verify`) + all test suites (59 rows) green; JS
behavior 3/3 BEHAVIOR-MATCH re-verified; loop selftests ×4 green.

## Library test suite vs the rig (2026-10-27): 61,581 total — 61,578 ✓ / 2 ✗ / 1 skip

The strongest behavior evidence, now measured. Rig rebuilt from scratch
(`/tmp` was wiped; no script existed — recipe below, committed mirrors are
byte-identical to vendor main `4574b93` 2026-10-07, `cmp`-verified ×9) and
run on the repo's committed `rt_cs` emissions.

**Verdict: 99.997% test-level behavior identity at library scale** — the
vendor's own 61,581-test suite against the round-tripped assembly: 61,578
pass, 2 fail (both root-caused to canon rows below), 1 skip
(`Create_MixedDoubleSet_MeetsPerformanceGate` — vendor-disabled perf gate,
not a fidelity row). Every `Verify` snapshot passed except the public-API
approver; culture/locale/string formatters all green.

| # | failure | root row | corpus evidence |
|---|---------|----------|-----------------|
| 1 | `Approve_Public_Api` (API approver, 1 line each) | **get-only auto-property → field**: `protected CultureInfo Culture { get; }` canon'd as `protected CultureInfo Culture;` | cs_08 src line 15 vs rt line 8 |
| 2 | `Approve_Public_Api` (same test, second row) | **generic `where` constraint erased**: `where TLocaliser : class` (own line) dropped from the type row | cs_02 src lines 8–9 vs rt line 4 |
| 3 | `HumanizesNegativeCompositeValuesWithOneLeadingSign` (assert `"-1 EB"` vs `"--1 EB"`) | **unsigned erasure**: every `ulong` canon'd signed — `(ulong)(-(input.Bits+1))+1 : (ulong)input.Bits` → `(Int64)… : (long)…`, `void addPart(ulong …)` → `(long …)`, `(ulong)BytesInExabyte*…` → `(long)…` | cs_06 src 528–539 vs rt 172–198 |

Row 3 is the semantic star: the UB-free unchecked wrap at the EB boundary
(6.4e19 bits > ulong max) survives unsigned math but flips sign in signed
division → parts print `-1` *plus* the `isNegative` prefix → `--1 EB`.
It compiles 0-errors and passes canon identity; only the vendor's own
boundary test separates the two. This is the whole argument for running
the real library suite.

### Run recipe (SDK 10.0.403 — the pinned SDK 11 is NOT required)

1. `git clone --depth 50 Humanizr/Humanizer` → `fetch --unshallow`
   (Nerdbank.GitVersioning needs full history for version height).
2. Scratch only: `mv global.json global.json.vendor-pin` (the
   `11.0.100-rc.1` pin is bypassed, not honored).
3. Remove the 9 mirror files; copy `bench/corpus/real/rt_cs/cs_*.cs` in
   (placed under `src/Humanizer/CorpusRT/` for default globbing).
4. `dotnet build src/Humanizer/Humanizer.csproj -c Release
   -p:RunAnalyzers=false -p:TargetFrameworks=net10.0` — the global
   `TargetFrameworks` override is load-bearing (`-f net10.0` leaves a
   partial restore graph that poisons the P2P: test deps.json got
   `Humanizer` assets as `bin/placeholder` → module-init
   `FileNotFoundException: Humanizer, Version=4.0.0.0`).
5. Build tests the same way; if `Humanizer.Tests.deps.json` lacks a
   `Humanizer/…` entry, move it aside — MTP exe runs via app-dir probing
   (`./Humanizer.Tests` artifacts path: `artifacts/bin/Humanizer.Tests/release_net10.0/`).

net11.0/net48/netstandard2.0 TFMs unexercised; `dotnet test` (VSTest) path
unexercised — irrelevant for net10.0 gates.

### Run 2 after the row pass (2026-10-27, same day): **all green**

The 3 rows landed as an additive row pass (reader + expander + printer +
emit, one Canon row per row):

| row | fix | where |
|---|---|---|
| get-only auto-property (23) | accessor-only getter rows canon `get` mod word (`protected get CultureInfo Culture`); emit restores `… { get; }` (no trailing `;` without an initializer; `= init;` rows keep theirs) + spec ty list unchanged | cs_read `property_decl`/`is_getter_only`, expander `mod_words` + `get` drain, cs `member_var` |
| generic constraint carry (19) | `TypeDecl.constraints` (additive field, all construction sites); reader captures `type_parameter_constraints_clause` children normalized to the expander's `constraint_text` shape; canon prints ` type Name<T> … where T : class{` (re-parse stable); emit restores C# order after the base list | cs_read `type_decl`/`constraint_clause_text`, expander type arm + `constraint_text`, printer `print_type`, cs `emit_type` |
| unsigned type-words (24) | `ulong` canonizes capital `ULong` (spec §1 ty list extended, additive); emit maps `ULong→ulong`; the cast arm widened — single mapped Ident in parens is a cast (the `(ULong)(-x)` paren-operand shape previously rendered raw) | cs_read `canon_word`, cs `map_type` + cast arm, spec.md |

**Result: 61,581 tests — 61,580 pass / 0 fail / 1 vendor-disabled skip.**
Bench re-measured on the 6.1 basis: **C# canon 64.2% o200k / 64.1%
llama** (+2 tokens corpus-wide for all 3 rows — the honest cost), weighted
58.3% — unchanged. Gates re-run: canon identity 18/18 (`ircc verify` over
`ir/` + `rt_cs/`), 59 test rows, JS behavior 3/3 BEHAVIOR-MATCH, loop
selftests ×4 — all green.

Also this session: the JS behavior gate was found failing at HEAD
(`function isCheerio<T>` leaked into the rig — the member pass's fn
type-param re-attach reached canon but `to-js`'s generid erase missed the
re-attached name; the earlier "3/3 re-verified" did not hold on current
code). Fixed (generic-name erase in `js.rs`, top-level + member arms);
behavior re-verified 3/3 for real.

## Remaining

- **6.2 loop at scale** (model time).