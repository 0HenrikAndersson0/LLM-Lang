```javascript
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