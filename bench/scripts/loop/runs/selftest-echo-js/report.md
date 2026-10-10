# Loop run — task_01_stringops (js baseline world)

- lang/world: `js` — no guide, no transpile, no span map (J6 baseline)
- adapter: `echo`
- task: CompactLabel — trim/split/initials string ops
- rounds: 2 (cap 4); started 2026-10-10T10:29:09+00:00
- verdict: **PASS**

| round | outcome | detail | src chars | s |
|---|---|---|---|---|
| 1 | runtime-fail | ERROR CompactLabel("hello world"): TypeError: w.toFirstUpper is not a function | 255 | 0.0 |
| 2 | ALL PASS | 8 rig lines | 257 | 0.0 |

Artifacts: `prompt-R*.md` (transcripts), `reply-R*.md` (model replies), `feature-R*.js`, `feature.js` (last), `feature.js` (last emit/run), `rig.js` (cases) — no map in the js world.

## Final JS

```js
export function CompactLabel(s) {
  const t = s.trim();
  if (t.length === 0) return "-";
  const ws = t.split(" ").filter(w => w.length > 0);
  const label = ws.map(w => w[0].toUpperCase()).join("");
  return label.length > 6 ? label.slice(0, 6) : label;
}
```

