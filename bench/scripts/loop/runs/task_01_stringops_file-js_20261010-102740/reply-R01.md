Straight library ops: trim; if the trimmed string is empty return `-`; otherwise split on single spaces, drop empty words (runs of spaces collapse), take each word's first character uppercased, join; return the first 6 characters when the label is longer than 6.

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