# 4.2 — IR-form coverage audit (js corpus)

tree-sitter-typescript CST node kinds over `bench/corpus/real/src/js_*.ts` (7 files), joined with the kind→IR verdict table (design doc §2/§3/§6; hand renditions are ground truth for verbatim/representable calls).

- total CST nodes: **4398** over 145 kinds
- constructs the reader must map (excluding trivia + dropped comments): **2233**
- **representable: 100.0%** — every kind classified

| bucket | nodes | kinds | reading |
|---|---:|---:|---|
| core | 1576 | 51 | direct core form / verbatim surface |
| sugar | 14 | 4 | canonicalizes into a core form |
| signature | 634 | 23 | erased to signatures or printer-reconstructed (§3/§6.6) |
| opaquetype | 8 | 1 | `opaque type` capture (§6.5, reconcile row 21) — bounded loss |
| grammar | 1 | 1 | tree-sitter gap; construct IS representable (Core::Reexport) |
| unmapped | 0 | 0 | AUDIT INCOMPLETE — kind has no verdict |
| dropped | 62 | 1 | comments, §6.1 by design |
| trivia | 2103 | 63 | syntax atoms, no IR counterpart needed |

## Evidence for non-core buckets

### sugar

- `function_expression` ×1 (js_03_parse.ts ×1): canonicalized to arrow
- `template_string` ×3 (js_04_forms.ts ×1, js_05_css.ts ×2): string interpolation `"…{e}…"` (§4)
- `template_substitution` ×7 (js_04_forms.ts ×2, js_05_css.ts ×5): interpolated expr
- `update_expression` ×3 (js_01_util.ts ×1, js_03_parse.ts ×1, js_05_css.ts ×1): `i++` → `i+=1` canon

### opaquetype

- `type_alias_declaration` ×8 (js_02_types.ts ×8): union/template aliases → `opaque type` (js_02.ir:5-9, reconcile row 21); interface-shaped → `type I interface{}`

### grammar

- `ERROR` ×1 (js_06_slim.ts ×1): tree-sitter-typescript 0.23.2 lacks `export type * from`; the construct is Core::Reexport (J3)

