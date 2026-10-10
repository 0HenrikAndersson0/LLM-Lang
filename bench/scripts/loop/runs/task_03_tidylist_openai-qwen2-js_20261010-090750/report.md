# Loop run — task_03_tidylist (js baseline world)

- lang/world: `js` — no guide, no transpile, no span map (J6 baseline)
- adapter: `openai`
- task: Tidy — chained pipelines + block-bodied lambdas (J4/J5 idioms)
- rounds: 1 (cap 6); started 2026-10-10T09:14:15+00:00
- verdict: **PASS**

| round | outcome | detail | src chars | s |
|---|---|---|---|---|
| 1 | ALL PASS | 6 rig lines | 300 | 385.6 |

Artifacts: `prompt-R*.md` (transcripts), `reply-R*.md` (model replies), `feature-R*.js`, `feature.js` (last), `feature.js` (last emit/run), `rig.js` (cases) — no map in the js world.

## Final JS

```js
export function Tidy(xs) {
  return xs
    .map(s => s.trim())
    .filter(s => s.length > 0)
    .map(s => {
      const words = s.split(" ").filter(w => w.length > 0);
      return words.map((w, i) => i === 0 ? w.toLowerCase() : w[0].toUpperCase() + w.slice(1)).join('');
    })
    .slice(0, 3);
}
```

