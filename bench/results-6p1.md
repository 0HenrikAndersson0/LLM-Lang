# 6.1 — Bench rerun on machine canon (the Phase-0 headline re-measured)

> The Phase-0 headline (46.1%/46.2%, GO) was computed on **hand
> renditions**. Phase 4/5 shipped `from-js`/`from-cs` — the canon is now
> **machine-generated**. This rerun pairs each corpus file's `src/`
> original with its round-trip canon (`rt/` for js, `rt_cs/` for cs) and
> scores both with the Phase-0 tokenizers (o200k_base, llama).
> Measured 2026-10-10 (`bench/scripts/.venv`, tiktoken + tokenizers).

## JS canon vs original TS (`bench/corpus/real/rt`)

7/7 canon files vs their own sources. Note: js data rows restored in
5.6's JS leg work (dict/array initializers, primary ctors, the
mut-overlabel) — identical machinery to the C# leg's field restore.

| file | original | canon | % | llama % |
|---|---|---|---|---|
| js_01_util | 695 | 303 | 43.6% | 43.8% |
| js_02_types | 413 | 248 | 60.0% | 61.2% |
| js_03_parse | 624 | 254 | 40.7% | 42.1% |
| js_04_forms | 863 | 282 | 32.7% | 34.3% |
| js_05_css | 1500 | 560 | 37.3% | 39.4% |
| js_06_slim | 320 | 100 | 31.2% | 32.1% |
| js_08_options | 975 | 232 | 23.8% | 25.9% |
| **JS total** | **5390** | **1979** | **36.7%** | **37.7%** |

## C# canon vs original C# (`bench/corpus/real/rt_cs`)

| file | original | canon | % | llama % |
|---|---|---|---|---|
| cs_01_interface | 712 | 137 | 19.2% | 19.2% |
| cs_02_generic_registry | 968 | 630 | 65.1% | 66.1% |
| cs_03_class_methods | 895 | 680 | 76.0% | 76.0% |
| cs_04_strategy | 99 | 51 | 51.5% | 49.6% |
| cs_05_extension_static | 6175 | 3175 | 51.4% | 52.3% |
| cs_06_extension_generics | 5633 | 3250 | 57.7% | 59.2% |
| cs_08_formatter | 5020 | 4235 | 84.4% | 82.3% |
| cs_09_detector | 209 | 108 | 51.7% | 50.0% |
| cs_10_detector2 | 198 | 95 | 48.0% | 46.6% |
| **C# total** | **19909** | **12361** | **62.1%** | **62.5%** |

## Reading

1. **The headline holds and strengthens on machine canon.** JS canon =
   36.7% o200k / 37.7% llama — *well past* the ≥30% north star, and
   better than the hand-rendition Phase-0 number (46%) because the
   machine canon reads the real source (comments, formatting, and XML
   docs all drop mechanically) rather than a curated rendition.
2. **C# canon = 62.1% / 62.5%** — the C# corpus is *less* compressible
   than JS for structural reasons, exactly as the J6 denominator finding
   predicted: cs_08 (the largest file) is dense C# with few comments
   (84.4% — near the bare-runtime denominator), while signature/interface
   files compress 19–52%. The **weighted-both-languages** view: (1979 +
   12361) / (5390 + 19909) = **57.6% o200k** — the round-trip canon is
   42% cheaper than the sources *as files*, before any loop amortization.
3. **The char-family and char-free split explain most variance**: the
   char-heavy cs_05 sits at 51% because the IR's type-free canon drops
   the type ceremony; the comment-light cs_08 at 84% — the canon of
   comment-rich sources saves most (the fidelity contract's dominant
   saving, §6.1).
4. Whole-pipeline framing (J6): the static canon cost is one input; the
   loop cost is the other — J6's amortized 77–90% at small scale and the
   task_04 marginal-parity finding carry over unchanged, now anchored to
   a *machine-generated* canon that an agent can regenerate from any
   file.

**Verdict: 6.1 GO — the machine canon reproduces and beats the Phase-0
headline on JS and sits at 57.6% weighted across both languages; the
fidelity contract (docs/fidelity.md) is what the saving buys.**

---

*Method note: paired per-file scoring (same basename, src vs rt-canon);
no cross-file aggregation tricks. The scorer = the Phase-0 harness
(`bench/scripts/score.py`), run under `bench/scripts/.venv`.*

## 6.4 — versioning policy (same-session doc)

`docs/versioning.md` lands with the policy frozen from the working
agreements + what this repo's history actually did (additive-only core
forms; the canon-print interface fixes were additive; the `volatile `/
interface-kind prints were additive canon rows; the data-brace `{{…}}`
escape = a canon-text change absorbed by the reader→expander pair).