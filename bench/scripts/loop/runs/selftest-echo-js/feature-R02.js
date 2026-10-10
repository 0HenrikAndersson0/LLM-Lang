export function CompactLabel(s) {
  const t = s.trim();
  if (t.length === 0) return "-";
  const ws = t.split(" ").filter(w => w.length > 0);
  const label = ws.map(w => w[0].toUpperCase()).join("");
  return label.length > 6 ? label.slice(0, 6) : label;
}
