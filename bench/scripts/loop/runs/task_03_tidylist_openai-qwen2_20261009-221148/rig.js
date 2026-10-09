import { Tidy } from "./feature.js";
const cases = [["Tidy", [["Foo Bar", "qux", "", "  Baz Qux  ", "Hello World Again", "one"]], ["fooBar", "qux", "bazQux"]], ["Tidy", [[]], []], ["Tidy", [["x"]], ["x"]], ["Tidy", [["A B", "c d", "e f", "g h"]], ["aB", "cD", "eF"]], ["Tidy", [["   ", "  Single Word  "]], ["singleWord"]]];
const FNS = {Tidy: Tidy};
let fail = 0;
for (const [fn, args, expected] of cases) {
  const shown = args.map(a => JSON.stringify(a)).join(", ");
  try {
    const got = FNS[fn](...args);
    const g = JSON.stringify(got), e = JSON.stringify(expected);
    if (g === e) console.log(`PASS ${fn}(${shown}) -> ${g}`);
    else { fail++; console.error(`FAIL ${fn}(${shown}) -> ${g} (expected ${e})`); }
  } catch (err) {
    fail++;
    console.error(`ERROR ${fn}(${shown}): ${err.stack || err}`);
  }
}
if (fail > 0) { console.error(`RESULT: ${fail} failing`); process.exit(1); }
console.log("RESULT: ALL PASS");
