# Loop run — task_04_multi (js baseline world)

- lang/world: `js` — no guide, no transpile, no span map (J6 baseline)
- adapter: `file`
- task: Multi-feature module — CompactLabel + ParseDuration + Tidy (J6 session-scale)
- rounds: 1 (cap 4); started 2026-10-10T10:33:03+00:00
- verdict: **PASS**

| round | outcome | detail | src chars | s |
|---|---|---|---|---|
| 1 | ALL PASS | 24 rig lines | 1077 | 6.1 |

Artifacts: `prompt-R*.md` (transcripts), `reply-R*.md` (model replies), `feature-R*.js`, `feature.js` (last), `feature.js` (last emit/run), `rig.js` (cases) — no map in the js world.

## Final JS

```js
export function CompactLabel(s) {
  const t = s.trim();
  if (t.length === 0) return "-";
  const words = t.split(" ").filter(w => w.length > 0);
  const label = words.map(w => w[0].toUpperCase()).join("");
  return label.length > 6 ? label.slice(0, 6) : label;
}

export function ParseDuration(s) {
  let total = 0;
  let i = 0;
  while (i < s.length) {
    const numStart = i;
    while (i < s.length && s[i] >= "0" && s[i] <= "9") i += 1;
    if (i === numStart) return 0;
    const num = parseInt(s.slice(numStart, i));
    if (i >= s.length) return 0;
    if (s[i] === "h") total += num * 3600;
    else if (s[i] === "m") total += num * 60;
    else if (s[i] === "s") total += num;
    else return 0;
    i += 1;
  }
  return total;
}

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

