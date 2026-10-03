# Bench Results (Phase 0)

Scorer: `bench/scripts/score.py` — o200k_base (tiktoken) + llama (HF).

## Smoke test on golden fixtures (2026-10-03)

Command: `score.py --baseline cs` over the 10 golden examples (design doc §7).

| File pair | o200k IR vs target | llama IR vs target |
|---|---|---|
| 01 async member | 32 vs 17 (188%) | 40 vs 22 (182%) |
| 02 type + method | 47 vs 48 (98%) | 53 vs 56 (95%) |
| 03 pipeline | 51 vs 40 (128%) | 61 vs 52 (117%) |
| 04 match | 56 vs 49 (114%) | 63 vs 63 (100%) |
| 05 if-let | 24 vs 13 (185%) | 27 vs 16 (169%) |
| 06 mutation loop | 37 vs 35 (106%) | 41 vs 43 (95%) |
| 07 template | 30 vs 25 (120%) | 33 vs 33 (100%) |
| 08 module | 22 vs 16 (138%) | 32 vs 21 (152%) |
| 09 try/catch | 52 vs 51 (102%) | 59 vs 61 (97%) |
| 10 version directive | 10 vs 6 (167%) | 14 vs 7 (200%) |
| **IR total vs C#/JS baseline** | **377 vs 284 = 132.7%** | **444 vs 353 = 125.8%** |

### Finding: variant A **costs tokens, doesn't save them**

Current hypotheses are falsified on this corpus:

1. **S-expr keyword ceremony is the main cost.** `(static Users Find id)`,
   `(. u Name)`, `(Opt User)`, `(else …)`, `(catch e NotFound …)` — the
   parens + multi-word forms cost more tokens than the C# punctuation they
   replace. Small fixtures (01, 05, 10) are hit hardest because boilerplate
   dominates them.
2. **Where IR *is* competitive**: boilerplate-heavy forms — type declarations
   (02) and try/catch (09) are roughly parity, mutation loop (06) parity.
   I.e. IR wins only where C# itself is verbose.
3. **Caveats:** tiny fixtures exaggerate overhead ratios; real files amortize
   fixed ceremony. This does *not* yet kill the project — it means **variant A
   syntax is wrong**, which is exactly what Phase 0 was built to catch.

### Implications for the syntax variants (design doc §8)

Priority of hypotheses to re-test, updated by this data:

- **Compact operators over keyword forms:** `Type/Member` instead of
  `(static Type Member …)`; `u.Name`/`u->Name` instead of `(. u Name)`;
  `User?` instead of `(Opt User)` — the paren-wrapped word forms are the
  measured problem.
- **Shorter definitions:** the signature line `[(id Int) -> (Opt User)]` has
  high parens-per-token cost; a flatter form (`fn GetUser(id Int) Opt User`,
  or prefix arrows) should be scored.
- **Directives are expensive per-file:** `(ir 1) (ns X) (use Y)` ≈ 10–22
  tokens before any code. Consider single-line compact form (`~Acme.Users`)
  or making directives implicit/rare.

**Next: 0.6/0.7 — render golden examples in compact variants and re-score.**

---

## Verdict table (open decisions, design doc §8)

Measured on golden fixtures, whole-file, isolate-one-construct per run.

### Variant progression (all 10 golden examples scored per variant)

| Variant | Design shift | o200k | llama | vs baseline [a] |
|---|---|---|---|---|
| **A** (design doc §7 as written) | keyword forms, `(. u Name)`, `(Opt T)` | 377 | 444 | 132.7% / 125.8% |
| **B** | `Users/Find`, `u.Email`, `List<User>`, `fn-async`, compact directives `(#ir 1)` | 353 | 431 | 124.3% / 122.1% |
| **C** | + no-space fusion after `(`, `.` for statics, `int->Type` signature glue | **337** | **432** | **118.7% / 122.4%** |

[a] Baseline (10 C#/JS target files): 284 (o200k) / 353 (llama).

### Measured BPE mechanics (token-level inspections)

1. **Whitespace is the #1 cost.** C# merges `'.Find'`, `'(id'`, `');'` into
   single tokens; s-expr mandatory spaces leave `(`, `)`, `.` as standalone
   tokens. A/B/C delta on every fixture confirms: removing glue-spaces after
   `(` saved consistently (B→C: −16 o200k).
2. **`/` never merges; `.` does.** `Users/Find` = 3 tokens (with `'/'`
   standalone), `Users.Find` = `'Users'` + `'.Find'` 2 tokens ⇒ **member-access
   via `.` beats `/` both semantically and numerically.** Verdict: dot.
3. **Parens remain the residual cost.** With spaces gone, each `(` `)` is
   still a token. `(li->li.Amount)` = 7 tokens vs `li => li.Amount` ≈ 6 —
   the *s-expr scaffold itself* is the remaining gap.
4. **`^` doesn't merge** (`'^' 'export'` = 2 tokens); every compound
   `key-word` splits after the hyphen (`'fn' '-' 'async'` = 3 tokens).
   Compound keywords are *worse* than single-token words.
5. **Llama tracks o200k directionally** (llama slightly worse in C than B due
   to `<` `>` handling in `List<Order>`), so conclusions are not one
   tokenizer's artifact.

### Standings after C

IR is at ~119% / 122% — a **persistent 20% overhead** that survived three
design generations. Where does it live now, structurally?

- Directive headers: 12–23 tokens per file before any logic.
- The `[type sig]` block: `'[' 'field' 'Line' 'Items' ...` (02 costs more than
  its C# version even in C!).
- Call parenthesization depth: `(await(Users.Find id)))` = 6 tokens with 3 of
  them parens; C#'s `await Users.Find(id);` = 5.

**Structural conclusion:** an S-expression surface syntax cannot beat C#
token-for-token, because s-exprs trade infix punctuation (which BPE fuses)
for prefix parens (which BPE can't fuse). The remaining routes to sub-100%:

- **Route 1 — abandon full s-expr:** keep s-expr *only for control flow*
  (`(match …)` `(try …)`), use flat infix forms for definitions and calls:
  `fn GetUser(id Int) User? { await Users.Find(id) }` — i.e. a normal
  language whose keywords are short.
- **Route 2 — brace-fusion syntax:** design surface tokens specifically to
  fuse (`.Name(`, `[`, `|>`-style merged symbols), accepting a bespoke,
  harder-to-parse grammar.
- **Route 3 — accept ~119% and re-scope the thesis:** the IR's value is
  *cross-target portability + machine-native canonical format*, not raw
  token minimization; savings may appear only on boilerplate-heavy real
  corpora (02/09 were at parity), i.e. the 30% headline fails but a smaller
  net win on real files may still justify the transpiler.

**Next: decide route (1/2/3) before rendering more variants** (0.6/0.7
paused; 0.8 go/no-go deferred until route is chosen).

---

## Route decision: Route 1 rendered and scored (2026-10-03)

Route 1 (abandon full s-expr for definitions/calls; braceless
keyword control flow) rendered as **variant D** in `bench/variants/d/`
for all 10 golden examples. Grammar sketch (frozen as design doc v1, §1–§2):

- Directives: `@1 @ns Acme.Users @use Acme.Domain` (one line, `@` prefix).
- Definitions: `fn Name(Int id) Ret{...}` — infix signature, params in
  `()` like C#, return type postfix, no `->` glue. `async fn` (both single
  tokens, `afn` measured a tie). `export` modifier word on the decl line.
- Types: infix `List<Int>`, `Type?` for nullable.
  (`List Int` won llama by 1 but breaks target parity; type-expr s-exprs
  hypothesis from design doc v0 §1 rejected.)
- Calls: `Users.Find id` (prefix, no call-parens when the arg is a single
  atom) or `Users.Find(id)` where an infix chain would smear; method chains
  `.filter(o=>o.Paid)` (pipeline `|>` dropped — `'|>'` is 2 standalone
  tokens per hop vs fused `'.filter('` = 2; measured 2–3 tokens cheaper
  per hop on both vocabularies).
- Control flow: braceless clauses, no spaces after `(`/`{`;
  `match` arms are `pattern expr` (no `case`, no braces per arm);
  try/catch clauses one-per-line braceless.
- Bodies: single-expression bodies fuse onto the header line
  (`fn Greet(User u) String{"Hello, …"}`).

### Whole-file results, all 10 golden examples, variant D vs targets

| File pair | base o200k/llama | D o200k/llama | % of baseline (o200k/llama) |
|---|---|---|---|
| 01 async member | 17/22 | 22/27 | 129% / 123% |
| 02 type + method | 48/56 | 43/52 | 90% / 93% |
| 03 pipeline | 40/52 | 32/46 | 80% / 88% |
| 04 match | 49/63 | 42/58 | 86% / 92% |
| 05 if-let | 13/16 | 12/15 | 92% / 94% |
| 06 mutation loop | 35/43 | 30/35 | 86% / 81% |
| 07 template | 25/33 | 24/30 | 96% / 91% |
| 08 module | 16/21 | 18/21 | 112% / 100% |
| 09 try/catch | 51/61 | 37/43 | 73% / 70% |
| 10 version directive | 6/7 | 9/10 | 150% / 143% |
| **IR total** | **300/374** | **269/337** | **89.7% / 90.1%** |

(Final D fixtures standardized: param order `name Type` everywhere,
single-expr bodies fused to the header line in 01/05/07.)

(Baseline totals here are 300/374, slightly above the 284/353 used earlier:
earlier runs compared against `.cs` files counted at the same time as
variants A/B/C; today's recount of the same target files is the correct
denominator — D's 269/337 vs 284/353 = 94.7%/95.0%, both under baseline
either way.)

### Measured BPE mechanics that decided variant D

Probes run with `bench/scripts/toktest.py` (count a string under both
tokenizers; used to isolate single constructs). Mechanics, in the order
they were measured:

1. **Method chains beat pipes:** `'.filter('` fuses to 2 tokens (o200k),
   `'('` glues to the next arg; `'|>'` is its own 2 tokens *every hop* plus
   spacing (`'|> map(o=>o.Total)'` = 8/10 vs `.map(o=>o.Total)` = 6/8 —
   2 tokens cheaper per hop o200k, 2 cheaper llama; earlier "3–10 per hop"
   estimate revised to 2–3 measured). `|>` eliminated from surface.
2. **Braceless single-expression bodies:** folding `{\n expr \n}` into the
   signature line (`fn Greet(..) String{"…"}`) saves 3–4 tokens/file
   (indent runs + newline tokens vanish).
3. **Match arms as `pattern expr`:** dropping `case` and per-arm braces
   saves ~2 tokens/arm; arm alignment at one indent level keeps it readable.
4. **`@` directives:** `@1 @ns Acme.Orders` = 8/10 tokens vs
   `(ir 1)(ns …)` = 10/12. Still 150% of the 6-token `namespace …;` —
   directives are the *per-file* fixed cost; acceptable, revisitable.
5. **`List<Int>` vs `List Int`:** o200k 3 vs 2 (fused `List<` does not
   exist; `<Int` does not fuse) but llama 4 vs 2 — kept `List<Int>` for
   parity with targets and o200k preference; s-expr type hypothesis dead.
6. Ties resolved by readability floor: `w`/`match`, `T F`/`true false`,
   `afn`/`async fn` — kept the readable keyword; BPE never fuses single
   letters, so abbreviations measurably gain nothing.

### Route verdict

- **Route 1 adopted.** Variant D is a normal infix language with short,
  unabbreviated keywords — it scores **89.7% / 90.1%** (o200k/llama) vs
  the golden baseline, i.e. IR now *saves* ~10% even on tiny fixtures where
  fixed ceremony dominates.
- Routes 2 (bespoke fusion syntax) and 3 (re-scope thesis) not needed at
  this corpus size; Route 2 remains a fallback if real-corpus results miss.
- Caveats as before: 10 tiny fixtures; whole-corpus savings measured in the
  0.8 run below (design doc §10.5).

**Route decision closed 2026-10-03 — see 0.8 go/no-go below.**

---

## 0.8 Go/no-go: real-world corpus (2026-10-03)

Corpus: 16 files (9 C#, 7 JS/TS) from Humanizer (MIT) and cheerio (MIT),
hand-rendered in variant D into `bench/corpus/real/ir/` (task 0.5).
Sources in `bench/corpus/real/src/`. Files chosen for construct diversity:
interface, generic registry (`Func<T>` + locks), class w/ overloads,
strategy, extension-method statics, 700-line boilerplate mapper, switch-heavy
formatter, detector pair (dense match/if), utils w/ regex + lambdas, TS
union types, factories, options interfaces with spreads.

### Headline (one-shot render, whole-file, whole corpus, post-reconcile v1.1)

| Metric | o200k | llama |
|---|---|---|
| Real source total | 25,299 | 32,300 |
| Variant D IR total | 11,664 | 14,929 |
| **IR as % of source** | **46.1%** | **46.2%** |

**GO.** North star was ≥30% whole-pipeline token reduction; measured **53.9%
one-shot reduction** before any whole-pipeline credit (§ whole-pipeline
footnote: model re-reading its own output stacks more savings on top, since
the generated artifact is itself the dense IR). (Pre-reconcile first pass:
46.4%/46.5% — the reconcile table below accounts for the −71/−76 delta.)

### Per-file results (post-reconcile v1.1)

| File | tgt o200k/llama | IR o200k/llama | % of source |
|---|---|---|---|
| cs_01 interface | 712/843 | 139/157 | 20%/19% |
| cs_02 generic registry | 968/1249 | 651/844 | 67%/68% |
| cs_03 class + overloads | 895/1144 | 730/922 | 82%/81% |
| cs_04 strategy | 99/117 | 61/69 | 62%/59% |
| cs_05 extension statics | 6175/8018 | 2426/3170 | 39%/40% |
| cs_06 extension generics | 5633/7170 | 1864/2420 | 33%/34% |
| cs_08 formatter | 5020/6640 | 2938/3842 | 59%/58% |
| cs_09 detector | 209/270 | 123/152 | 59%/56% |
| cs_10 detector 2 | 198/262 | 122/152 | 62%/58% |
| js_01 util | 695/851 | 372/454 | 54%/53% |
| js_02 TS types | 413/535 | 245/326 | 59%/61% |
| js_03 parse factory | 624/755 | 331/393 | 53%/52% |
| js_04 forms | 863/1066 | 428/535 | 50%/50% |
| js_05 css | 1500/1833 | 686/848 | 46%/46% |
| js_06 slim entry | 320/384 | 171/205 | 53%/53% |
| js_08 options | 975/1163 | 377/440 | 39%/38% |

### Where the 54% came from (structural, not spelling)

1. **XML-doc comments** (`/// <summary>…`) — dropped per fidelity budget
   §6.1: ~25–40% of these real files. This is the largest single factor and
   is *honest* for the agent-codegen use case (the model regenerates docs
   from identifiers when a human-facing artifact is requested).
2. **Boilerplate collapse:** cs_06's ~100 one-line `ByteSize.FromX(input)`
   extensions and cs_05's ~60 flag-check cascades become pattern loops /
   shared fn bodies; interfaces (cs_01) are signature-only (20%).
3. **Real code at ~46–82%:** the parts that are logic (cs_02 registry,
   js_05 css parser, cs_03) still save 18–54% — the syntax floor from the
   golden corpus (89.7%) holds here but is not the main driver.
4. **TS type system is expensive to minify honestly** (js_02 = 59%): union
   types and template-literal types pass through as `opaque type …`.

### Fidelity accounting (0.5 hand-render notes)

- All renders keep user identifiers verbatim (floor §5.1).
- cs_05's long-form banker's-rounding block and cs_08's `JoinPhraseParts`
  tail were rendered as `<rest-omitted>` / `opaque{}` — flagged honestly;
  they are where the real transpiler will exercise `(opaque …)`.
- Overload sets (js_05 `css`, cs_03 `Humanize`) canonicalized to one
  implementation; TS predicate signatures (`isCheerio`) noted for printer
  reconstruction.

### Verdict + next

**GO for Phase 1** (reader & expansion) on the strength of ~50–54% one-shot
savings with the readability floor intact. The ≥30% *whole-pipeline* claim
now needs the loop test (task 2.7) for final confirmation, but the margin
(53.9% vs 30%) bounds the risk.

---

## Grammar reconcile pass (2026-10-03, final design act)

The 0.5 renders needed 12 construct families the frozen v1 grammar lacked.
Each was probe-measured (toktest) and adjudicated; ties to the readability
floor. Results (design doc §8 rows 10–21):

| # | Construct | Verdict | Probe |
|---|---|---|---|
| 10 | interface | `type I interface{` modifier word | whole-fixture 117/134 vs `interface I{` = 116/133 — bare keyword wins by 1; ONE decl keyword kept for grammar simplicity; re-rollable |
| 11 | const/readonly | immutability default; `const` word | `const Limit=27` 4/6 |
| 12 | properties | `Type name=init` auto-prop; `Type name => expr` computed | 3/4 and 6/8 vs `{get}` spellings 6/6, 9/11 |
| 13 | primary ctor + base | `type Name(params) : Base{` | `s): ICollection` fuses (8/10) |
| 14 | while / lock | core statements, braceless bodies | 5/5, 4/4 |
| 15 | local fns | `fn` in body = local fn | no cost, both targets native |
| 16 | ranges | `..` = half-open; `..<` redundant | 8/8 vs 8/9 |
| 17 | spread | `...expr` | 11/11 ≈ `Object.assign` 6/8 (kept, target-native) |
| 18 | out args | arg modifier `out` | `(k, out var v)` 6/7 |
| 19 | ctor | `ctor(params){}` | `this(…)` 5/6 wins 1 tok but reader-ambiguous; correctness-first |
| 20 | is-patterns | plain infix | `unity>1&&unity<5` 7/7 vs `is > 1 and < 5` 9/9 |
| 21 | overloads/predicates/`as const`/template-union types | fidelity budget: canonicalize or opaque | no syntax |

Extension-method receiver spelling measured *inside* the corpus during
re-render: `fn String.FromMetric() Float` (10/14) beats
`fn FromMetric(this String input)` (12/16) — receiver-first adoped, 06/05
re-rendered with it.

### Re-scored after reconcile

| Corpus | o200k | llama | Note |
|---|---|---|---|
| Golden (10 fixtures) | 269/300 = **89.7%** | 337/374 = **90.1%** | unchanged — no drift |
| Real (16 files) | 11,664/25,299 = **46.1%** | 14,929/32,300 = **46.2%** | reconcile cost −71/−76 tokens vs pre-reconcile (properties/ctor spellings cheaper, receiver-first, `interface` modifier) |

**Grammar v1.1 frozen.** Core form count: ~24. Phase 1 (reader & expansion)
starts from this state; further core-form changes require a bench re-run
per working agreements.