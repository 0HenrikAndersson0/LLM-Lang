
export function Tidy(xs) {
  return xs.filter(s=>{ const t = s.trim(); return t.length>0; }).map(s=>{ const words = s.split(" ").filter(w=>w.length>0); const first = words[0].toLowerCase(); const rest = words.slice(1).map(w=>w[0].toUpperCase()+w.slice(1)); return first+rest.join(""); }).slice(0, 3);
}
