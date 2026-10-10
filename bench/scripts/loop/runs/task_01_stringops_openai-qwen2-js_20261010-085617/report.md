# Loop run — task_01_stringops (js baseline world)

- lang/world: `js` — no guide, no transpile, no span map (J6 baseline)
- adapter: `openai`
- task: CompactLabel — trim/split/initials string ops
- rounds: 1 (cap 6); started 2026-10-10T08:57:50+00:00
- verdict: **PASS**

| round | outcome | detail | src chars | s |
|---|---|---|---|---|
| 1 | ALL PASS | 8 rig lines | 632 | 92.4 |

Artifacts: `prompt-R*.md` (transcripts), `reply-R*.md` (model replies), `feature-R*.js`, `feature.js` (last), `feature.js` (last emit/run), `rig.js` (cases) — no map in the js world.

## Final JS

```js
export function CompactLabel(s) {
  // Trim surrounding whitespace
  s = s.trim();

  // If the result is empty, return '-'
  if (s === '') {
    return '-';
  }

  // Split on single spaces and keep only nonempty words
  const words = s.split(/\s+/).filter(word => word.length > 0);

  // Take the first character of each word, uppercased
  const initials = words.map(word => word[0].toUpperCase());

  // Concatenate those characters
  let result = initials.join('');

  // If the result is longer than 6 characters, return its first 6 characters
  if (result.length > 6) {
    result = result.slice(0, 6);
  }

  return result;
}
```

