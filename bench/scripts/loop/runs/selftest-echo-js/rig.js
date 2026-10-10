import { CompactLabel } from "./feature.js";
const cases = [["CompactLabel", ["hello world"], "HW"], ["CompactLabel", ["hello brave new world"], "HBNW"], ["CompactLabel", [""], "-"], ["CompactLabel", ["   "], "-"], ["CompactLabel", ["Hello world"], "HW"], ["CompactLabel", ["a b c d e f g h i"], "ABCDEF"], ["CompactLabel", ["  Multiple   Spaces   Here  "], "MSH"]];
const FNS = {CompactLabel: CompactLabel};
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
