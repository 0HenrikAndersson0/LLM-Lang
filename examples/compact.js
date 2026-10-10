
export function CompactLabel(s) {
  const t = s.trim();
  if (t.length==0) { return "-"; }
  return t.split(" ").filter(w=>w.length>0).map(w=>w[0].toUpperCase()).join("").slice(0, 6);
}

export function Take(xs, n) {
  return xs.slice(0, n);
}
