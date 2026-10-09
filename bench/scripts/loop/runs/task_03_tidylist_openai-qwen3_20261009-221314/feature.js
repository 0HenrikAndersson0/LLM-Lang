
export function Tidy(xs) {
  return xs.filter(s=>{ const t = s.trim(); return t.length>0; }).map(s=>{ const words = s.split(" "); const result = ""for iin 0..words.length{if i==0{result+=words[i].toLowerCase()}else {result+=words[i][0].toUpperCase()+words[i].slice(1)}}; return result; }).slice(0, 3);
}
