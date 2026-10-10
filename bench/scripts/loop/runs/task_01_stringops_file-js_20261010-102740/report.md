# Loop run — task_01_stringops (js baseline world)

- lang/world: `js` — no guide, no transpile, no span map (J6 baseline)
- adapter: `file`
- task: CompactLabel — trim/split/initials string ops
- rounds: 1 (cap 4); started 2026-10-10T10:27:45+00:00
- verdict: **PASS**

| round | outcome | detail | src chars | s |
|---|---|---|---|---|
| 1 | ALL PASS | 8 rig lines | 267 | 4.6 |

Artifacts: `prompt-R*.md` (transcripts), `reply-R*.md` (model replies), `feature-R*.js`, `feature.js` (last), `feature.js` (last emit/run), `rig.js` (cases) — no map in the js world.

## Final JS

```js
export function CompactLabel(s) {
  s = s.trim();
  if (s.length === 0) return "-";
  const words = s.split(" ").filter(w => w.length > 0);
  let label = words.map(w => w[0].toUpperCase()).join("");
  if (label.length > 6) label = label.slice(0, 6);
  return label;
}
```

