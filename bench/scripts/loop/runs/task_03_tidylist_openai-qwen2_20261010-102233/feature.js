
export function Tidy(xs) {
  return xs.filter(s=>{ const t = s.trim(); return t.length>0; }).map(s=>{ const words = s.split(" "); return words.map(w=>{ return ((words.indexOf(w)==0) ? w.toLowerCase() : w[0].toUpperCase()+w.slice(1)); }).join(""); }).slice(0, 3);
}
