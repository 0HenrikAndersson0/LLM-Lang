Filter first (entry dropped when its trimmed form is empty), then camelize per entry: split the trimmed string on single spaces, lowercase the first word entirely, and capitalize only the initial character of every following word. Entries reaching the map always yield at least one word, so indexing `words[0]` is safe; joining with no separator after the first word gives the camelize. Finally keep at most the first 3 entries.

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