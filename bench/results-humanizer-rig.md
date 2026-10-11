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

## Remaining (queued)

- **The library's own test suite against the rig assembly** — the
  strongest behavior evidence. Environment-bound: the vendor pins SDK
  11 (`global.json`, rollForward latestFeature) and the Microsoft
  Testing Platform runner refuses VSTest targets on SDK 10
  ("opt-in to the new dotnet test experience"); the produced test-exec
  also failed to bind Humanizer at module-init locally. Needs either
  `sdk install` of the pinned SDK or `-p:RunAnalyzers=false` +
  runner resolution — scoped but not closed this pass.
- **6.2 loop at scale** (model time).