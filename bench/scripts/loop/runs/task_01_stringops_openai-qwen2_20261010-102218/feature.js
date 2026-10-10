
export function CompactLabel(s) {
  const trimmed = s.trim();
  if ((trimmed.length==0)) { return "-"; }
  const words = trimmed.split(" ").filter(w=>w.length>0);
  const initials = words.map(w=>w[0].toUpperCase()).join("");
  return initials.length>6?initials.slice(0, 6): initials;
}
