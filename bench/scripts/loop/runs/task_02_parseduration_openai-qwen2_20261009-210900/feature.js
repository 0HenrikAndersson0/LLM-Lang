
export function ParseDuration(s) {
  if (s.length==0) { return 0; }
  let total = 0;
  let i = 0;
  while (i<s.length) {
    let digits = "";
    while (i<s.length&&s[i]>="0"&&s[i]<="9") {
      digits+=s[i];
      i+=1;
    }
    if (digits.length==0) { return 0; }
    const unit = s[i];
    i+=1;
    if (unit!="h"&&unit!="m"&&unit!="s") { return 0; }
    const value = parseInt(digits);
    if (unit=="h") { total+=value*3600; }
    if (i>=s.length) { break; }
  }
  return total;
}
