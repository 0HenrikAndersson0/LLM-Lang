
export function CompactLabel(s) {
  const t = s.trim();
  if (t.length==0) { return "-"; }
  const ws = t.split(" ").filter(w=>w.length>0);
  const label = ws.map(w=>w[0].toUpperCase()).join("");
  return label.length>6?label.slice(0, 6): label;
}

export function ParseDuration(s) {
  let total = 0;
  let i = 0;
  while (i<s.length) {
    const numStart = i;
    while (i<s.length&&s[i]>="0"&&s[i]<="9") { i+=1; }
    if (i==numStart) { return 0; }
    const num = parseInt(s.slice(numStart, i));
    if (i>=s.length) { return 0; }
    switch (s[i]) {
      default:
      return 0;
      break;
      case "h":
      total+=num*3600;
      break;
      case "m":
      total+=num*60;
      break;
      case "s":
      total+=num;
      break;
    }
    i+=1;
  }
  return total;
}

export function Tidy(xs) {
  return xs.filter(s=>{ const t = s.trim(); return t.length>0; }).map(s=>{ const words = s.split(" ").filter(w=>w.length>0); const first = words[0].toLowerCase(); const rest = words.slice(1).map(w=>w[0].toUpperCase()+w.slice(1)); return first+rest.join(""); }).slice(0, 3);
}
