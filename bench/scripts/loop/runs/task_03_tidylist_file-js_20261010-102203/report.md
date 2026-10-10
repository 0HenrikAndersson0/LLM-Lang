# Loop run — task_03_tidylist (js baseline world)

- lang/world: `js` — no guide, no transpile, no span map (J6 baseline)
- adapter: `file`
- task: Tidy — chained pipelines + block-bodied lambdas (J4/J5 idioms)
- rounds: 1 (cap 4); started 2026-10-10T10:22:08+00:00
- verdict: **PASS**

| round | outcome | detail | src chars | s |
|---|---|---|---|---|
| 1 | ALL PASS | 6 rig lines | 336 | 5.6 |

Artifacts: `prompt-R*.md` (transcripts), `reply-R*.md` (model replies), `feature-R*.js`, `feature.js` (last), `feature.js` (last emit/run), `rig.js` (cases) — no map in the js world.

## Final JS

```js
export function Tidy(xs) {
  return xs
    .filter(s => s.trim().length > 0)
    .map(s => {
      const words = s.split(" ").filter(w => w.length > 0);
      const first = words[0].toLowerCase();
      const rest = words.slice(1).map(w => w[0].toUpperCase() + w.slice(1));
      return first + rest.join("");
    })
    .slice(0, 3);
}
```

