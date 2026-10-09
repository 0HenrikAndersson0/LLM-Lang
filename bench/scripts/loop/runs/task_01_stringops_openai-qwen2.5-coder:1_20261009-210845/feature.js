
export function CompactLabel(s) {
  const trimmed = s.trim();
  if (trimmed=="") { return "-"; }
  const words = trimmed.split(" ").filter(word=>word!="");
  const compacted = words.map(word=>word.charAt(0).toUpperCase()).join("");
  return compacted.length>6?compacted.substring(0, 6): compacted;
}
