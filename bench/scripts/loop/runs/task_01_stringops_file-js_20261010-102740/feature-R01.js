export function CompactLabel(s) {
  s = s.trim();
  if (s.length === 0) return "-";
  const words = s.split(" ").filter(w => w.length > 0);
  let label = words.map(w => w[0].toUpperCase()).join("");
  if (label.length > 6) label = label.slice(0, 6);
  return label;
}
