
export function CompactLabel(s) {
  const trimmed = s.trim();
  if ((trimmed.length==0)return "-") {}
  const words = trimmed.split(" ").filter(w=>w.length>0);
  const initials = words.map(w=>w[0].toUpperCase()).join("");
  if ((initials.length>6)return initials.slice(0, 6)) {}
  return initials;
}
