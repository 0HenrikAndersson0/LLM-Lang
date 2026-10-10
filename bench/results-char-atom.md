# Char-atom probe (Phase-0 re-run for the @2 admission)

> The named @1→@2 candidate in `docs/versioning.md`: admit a **Char
> atom** to the canon (single-quote char literals survive round-trip as
> chars, not stringified). The working agreements require the Phase-0
> bench re-run before any frozen-grammar change. This is that
> measurement, plus the decision framing. Probe:
> `bench/scripts/charatom_probe.py`; measured 2026-10-10
> (tiktoken/tokenizers, paired rows, no aggregation tricks).

## The problem the atom solves

The IR canon has no char atom: C# `'k'` chars canon to Str atoms
(`"k"`). The C# emitter restores strings in char positions — 8
compile-diagnostics inside the round-tripped cs_05/cs_06 files
(`new string("0", n)` CS1503, `chars[i]=="0"` CS0019,
`chars[i]="0"` CS0029, dict-key CS1950/CS0021 — fidelity row 17). The
*behavior* leg of the C# round trip is gated on exactly these rows.

**The strongest argument is not tokens — it's ambiguity**: cs_05
holds the payloads `k`, `M`, `G` as BOTH char literals (the Symbols
arrays, the dict keys) AND string literals (the `.ToString(nfi)+"k"`
suffix). Canon text `"k"` cannot distinguish them; only the reader
remembers. A Char atom makes the canon text express what the reader
already knows. This is also why the naive text-swap is wrong: a
payload-blind swap corrupts the 3 suffix rows (the probe's first draft
did exactly that — the row-level precision below is the honest number).

## Token measurement

| probe | proposed (Char) | current (Str) | delta |
|---|---|---|---|
| per-literal canon context rows (5 rows × 2 tokenizers) | 19 o200k / 21 llama | 19 / 21 | **±0** — the quoted pair `{"Y"` vs `{'Y'` tokenize identically |
| cs_05 whole-file canon (36 char positions swapped; string-suffix rows stay) | 3174 / 4193 | 3175 / 4192 | **−1 o200k / +1 llama** over 36 swaps |

**Token cost of the atom: zero.** The quoted-pair structure is
identical (`{'Y'` vs `{"Y"` — both a punct + 1-char payload + punct);
the BPE tokenizes them the same. 36 swaps netted −1/+1 tokens across
files: rounding noise. Extrapolated corpus-wide (47 source occurrences),
the canon delta is ±2 tokens — **no measurable token economy change**.

## Side benefits (measured elsewhere, unchanged here)

- The 8 char-family emit diagnostics clear (fidelity row 17 — the
  behavior rig's primary blocker).
- The emit-side "restore string in char position" heuristic (row 17's
  current workaround) dies; the emit becomes literal-shape-driven.
- The 6.1 weighted headline stays at 57.6% (the atom is token-neutral).

## Cost

| cost | size |
|---|---|
| Grammar: a Char atom in the reader/printer/both readers | one arms row each (`character_literal` already read; the JS side never produces it) |
| Canon text: `{'Y'`-single-quote rows are new canon surface | the identity test suite gains the row shapes |
| `@1 → @2` migration per versioning.md | reader accepts both; emits single-quote going forward; `@`-header bump |
| Phase-0-style bench re-run | this document (token-neutral, measured) |
| JS target | **unaffected** (C#-only rows; JS `to-js` continues to emit strings for chars — actually emits the char *text*; JS has no char literal, matching today's JS behavior) |
| The probe's own false-positive lesson | the naive payload-swap corrupted the 3 suffix rows — the implementation must be tree-position-based (the reader knows; the canon text can't) |

## Recommendation

**Admit the Char atom as `@2`** per `docs/versioning.md`:

1. Reader arms (`cs_read` character_literal → Char atom; `js_read`
   never produces one), printer arm (`'X'` canon), C# emitter restore
   (the 8 rows), opaque-row treatment unchanged.
2. `@`-header bump + the migration test: old `@1` files still read (the
   reader stays permissive); new canon prints single-quote going
   forward.
3. Identity re-run across both corpora (the gate).
4. The Phase-0 token verdict: **token-neutral**, so the bench block of
   the working agreements is satisfied by this document.

The decision call (bump the grammar or stay at v0.1 with row 17
documented) belongs to the human — cost is ~1 day of reader/emitter
arms + test updates against a token-neutral background.