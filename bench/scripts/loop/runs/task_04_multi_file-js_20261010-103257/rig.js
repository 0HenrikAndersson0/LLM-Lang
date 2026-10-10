import { CompactLabel, ParseDuration, Tidy } from "./feature.js";
const cases = [["CompactLabel", ["hello world"], "HW"], ["CompactLabel", ["hello brave new world"], "HBNW"], ["CompactLabel", [""], "-"], ["CompactLabel", ["   "], "-"], ["CompactLabel", ["Hello world"], "HW"], ["CompactLabel", ["a b c d e f g h i"], "ABCDEF"], ["CompactLabel", ["  Multiple   Spaces   Here  "], "MSH"], ["ParseDuration", ["90s"], 90], ["ParseDuration", ["1m"], 60], ["ParseDuration", ["1m30s"], 90], ["ParseDuration", ["2h"], 7200], ["ParseDuration", ["1h30m"], 5400], ["ParseDuration", ["1h0m30s"], 3630], ["ParseDuration", [""], 0], ["ParseDuration", ["1x"], 0], ["ParseDuration", ["s"], 0], ["ParseDuration", ["10"], 0], ["ParseDuration", ["1h"], 3600], ["Tidy", [["Foo Bar", "qux", "", "  Baz Qux  ", "Hello World Again", "one"]], ["fooBar", "qux", "bazQux"]], ["Tidy", [[]], []], ["Tidy", [["x"]], ["x"]], ["Tidy", [["A B", "c d", "e f", "g h"]], ["aB", "cD", "eF"]], ["Tidy", [["   ", "  Single Word  "]], ["singleWord"]]];
const FNS = {CompactLabel: CompactLabel, ParseDuration: ParseDuration, Tidy: Tidy};
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
