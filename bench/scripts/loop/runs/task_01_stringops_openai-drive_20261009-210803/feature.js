
export function CompactLabel(s) {
  s=s.trim();
  if (s.length==0) { return "-"; }
  const words = s.split(" ").filter(word=>word.length>0);
  const initials = words.map(word=>word[0].toUpperCase()).join("");
  if (initials.length>6) { return initials.slice(0, 6); }
  return initials;
}
