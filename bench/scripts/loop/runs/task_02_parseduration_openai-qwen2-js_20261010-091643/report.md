# Loop run — task_02_parseduration (js baseline world)

- lang/world: `js` — no guide, no transpile, no span map (J6 baseline)
- adapter: `openai`
- task: ParseDuration — compound durations (loop + match + parseInt)
- rounds: 1 (cap 6); started 2026-10-10T09:16:58+00:00
- verdict: **PASS**

| round | outcome | detail | src chars | s |
|---|---|---|---|---|
| 1 | ALL PASS | 12 rig lines | 1217 | 15.4 |

Artifacts: `prompt-R*.md` (transcripts), `reply-R*.md` (model replies), `feature-R*.js`, `feature.js` (last), `feature.js` (last emit/run), `rig.js` (cases) — no map in the js world.

## Final JS

```js
export function ParseDuration(s) {
    if (s.length === 0) return 0;

    let totalSeconds = 0;
    let i = 0;

    while (i < s.length) {
        // Find the end of the digit run
        let digitEnd = i;
        while (digitEnd < s.length && s[digitEnd] >= '0' && s[digitEnd] <= '9') {
            digitEnd++;
        }

        // Extract the digits and convert to an integer
        if (digitEnd > i) {
            let digits = s.slice(i, digitEnd);
            let value = parseInt(digits, 10);

            // Find the unit character
            if (digitEnd < s.length) {
                let unit = s[digitEnd];
                if (unit === 'h') {
                    totalSeconds += value * 3600;
                } else if (unit === 'm') {
                    totalSeconds += value * 60;
                } else if (unit === 's') {
                    totalSeconds += value;
                } else {
                    return 0; // Invalid unit
                }
            } else {
                return 0; // Missing unit
            }
        } else {
            return 0; // No digits in segment
        }

        // Move to the next segment
        i = digitEnd + 1;
    }

    return totalSeconds;
}
```

