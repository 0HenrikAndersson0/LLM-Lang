#!/usr/bin/env python3
"""J1 (PLAN v4 Phase 3) — behavior harness: emitted JS must BEHAVE like the
original sources, not merely parse.

For each subject: emit IR → JS (irc to-js), transform the original with tsc
(`import type` disappears — js_01/js_08 originals run standalone), then run a
rig that calls both sides with the same inputs and compares outputs deeply
(JSON). Expected values are hand-derived from the original semantics and act
as a third check (emit==orig but != expected flags rendition drift).
"""

import json
import shutil
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[2]
BUILD = HERE / "build"
IRC = REPO / "target" / "debug" / "irc"

# subject config: stub modules the emitted side-effect imports must resolve;
# behavior cases [[fn, args, expected]] (deep JSON compare); optional extra
# raw JS assertions for non-data cases (callbacks etc.)
SUBJECTS = [
    {
        "id": "js_01_util",
        "bind_extra": ["domEach"],
        "stubs": ["domhandler", "cheerio"],
        "cases": [
            ["camelCase", ["data-bar"], "dataBar"],
            ["camelCase", ["foo_bar_baz"], "fooBarBaz"],
            ["camelCase", ["foo"], "foo"],
            ["camelCase", ["foo-"], "foo"],
            ["camelCase", ["abc.def"], "abcDef"],
            ["cssCase", ["fooBar"], "foo-bar"],
            ["cssCase", ["already"], "already"],
            ["cssCase", ["Foo"], "-foo"],
            ["cssCase", ["aB"], "a-b"],
            ["isHtml", ["<div>"], True],
            ["isHtml", ["<a>"], True],
            ["isHtml", ["abc"], False],
            ["isHtml", ["<"], False],
            ["isHtml", ["<a"], False],
            ["isHtml", ["<3 heart"], False],
            ["isHtml", ["<!doctype html>"], True],
            ["isHtml", ["a < b > c"], False],
            ["isCheerio", [{"cheerio": "[cheerio object]"}], True],
            ["isCheerio", [{"cheerio": "no"}], False],
            ["isCheerio", [None], False],
            ["isCheerio", [{}], False],
        ],
        "extra": """
// domEach: fn(array[i], i) for every index; returns the SAME array
{
  const seen = [];
  const arr = ["a", "b", "c"];
  const ret = E.domEach(arr, (el, i) => seen.push([el, i]));
  const okSame = ret === arr;
  const okCalls = JSON.stringify(seen) === JSON.stringify([["a",0],["b",1],["c",2]]);
  if (okSame && okCalls) console.log("PASS domEach(side effects + identity)");
  else { fail++; console.error(`FAIL domEach seen=${JSON.stringify(seen)} same=${okSame}`); }
}
""",
    },
    {
        "id": "js_08_options",
        "stubs": ["cheerioselect", "domserializer", "domhandler", "htmlparser2", "parse5", "parse5treeadapter"],
        "cases": [
            ["flattenOptions", [None, {"a": 1}], {"a": 1}],
            ["flattenOptions", [{}, None], {"_useHtmlParser2": False}],
            ["flattenOptions", [{"xmlMode": True}, None], {"_useHtmlParser2": True, "xmlMode": True}],
            ["flattenOptions", [{"xml": True}, None], {"_useHtmlParser2": True, "xml": True, "xmlMode": True}],
            ["flattenOptions", [{"xml": {"q": 1}}, None], {"_useHtmlParser2": True, "xml": {"q": 1}, "xmlMode": True, "q": 1}],
            ["flattenOptions", [{"xml": True, "x": 5}, None], {"_useHtmlParser2": True, "xml": True, "x": 5, "xmlMode": True}],
            ["flattenOptions", [{"xmlMode": False}, None], {"_useHtmlParser2": False, "xmlMode": False}],
            ["flattenOptions", [None, None], {"_useHtmlParser2": False}],
        ],
        "extra": "",
    },
]


def run(cmd, **kw):
    return subprocess.run(cmd, capture_output=True, text=True, **kw)


def prepare(subject):
    work = BUILD / subject["id"]
    if work.exists():
        shutil.rmtree(work)
    work.mkdir(parents=True)
    (work / "package.json").write_text('{"type":"module"}\n')
    for stub in subject["stubs"]:
        sp = work / f"{stub}.js"
        sp.parent.mkdir(parents=True, exist_ok=True)
        sp.write_text("// behavior-harness stub: satisfies v0 side-effect import\n")
    ir = REPO / "bench" / "corpus" / "real" / "ir" / f"{subject['id']}.ir"
    proc = run([str(IRC), "to-js", "--map", str(ir)])
    if not (ir.parents[0] / (ir.stem + ".js")).exists():
        return None, (proc.stdout + proc.stderr).strip()[:200]
    shutil.copy(ir.parent / f"{ir.stem}.js", work / "feature.js")
    shutil.copy(ir.parent / f"{ir.stem}.map.json", work / "feature.map.json")
    # original → standalone JS (tsc transform erases `import type`)
    src = REPO / "bench" / "corpus" / "real" / "src" / f"{subject['id']}.ts"
    t = run(["npx", "-y", "-p", "typescript", "tsc", "--target", "es2022",
             "--module", "esnext", "--outDir", str(work / "orig"),
             "--skipLibCheck", str(src)], cwd=str(REPO))
    orig = work / "orig" / f"{ir.stem}.js"
    if not orig.exists():
        return None, "tsc original transform produced no output: " + (t.stdout + t.stderr)[:300]
    shutil.copy(orig, work / "orig.js")
    return work, ""


def rig_js(subject):
    """The emitted module exposes no exports (renditions use bare `fn`), so
    the E-side binds local decls by CONCATENATING feature.js with the runner —
    the emitted artifact itself stays untouched."""
    extra = subject.get("extra", "")
    cases = json.dumps(subject["cases"], indent=1)
    bound = sorted({c[0] for c in subject["cases"]} | set(subject.get("bind_extra", [])))
    tpl = """import * as O from "./orig.js";
const E = { __BINDS__ };
const cases = __CASES__;
let fail = 0;
for (const [fn, args, expected] of cases) {
  const shown = args.map(a => JSON.stringify(a)).join(", ");
  let ge = "?", go = "?", ere = null, ore = null;
  try { ge = JSON.stringify(E[fn](...args)); } catch (e) { ere = String(e); }
  try { go = JSON.stringify(O[fn](...args)); } catch (e) { ore = String(e); }
  if (ere || ore) {
    fail++;
    console.error(`THROW ${fn}(${shown}) emitted=${ere || ge} orig=${ore || go}`);
  } else if (ge !== go) {
    fail++;
    console.error(`DIFF ${fn}(${shown}) emitted=${ge} orig=${go}`);
  } else if (ge !== JSON.stringify(expected)) {
    fail++;
    console.error(`NOTE ${fn}(${shown}) emitted==orig==${ge} expected=${JSON.stringify(expected)}`);
  } else {
    console.log(`PASS ${fn}(${shown}) -> ${ge}`);
  }
}
__EXTRA__
if (fail > 0) { console.error(`RESULTS: ${fail} failures`); process.exit(1); }
console.log("RESULTS: ALL MATCH");
"""
    return tpl.replace("__BINDS__", ", ".join(bound)) \
              .replace("__CASES__", cases) \
              .replace("__EXTRA__", extra)


def main():
    rows = []
    clean = True
    for subject in SUBJECTS:
        work, err = prepare(subject)
        if work is None:
            rows.append((subject["id"], "SETUP-FAIL", err.strip()[:200]))
            clean = False
            continue
        (work / "rig.js").write_text(
            (work / "feature.js").read_text() + "\n" + rig_js(subject)
        )
        proc = run(["node", "rig.js"], cwd=str(work))
        status = "BEHAVIOR-MATCH" if proc.returncode == 0 else "BEHAVIOR-FAIL"
        clean &= status == "BEHAVIOR-MATCH"
        rows.append((subject["id"], status, "\n".join((proc.stdout + proc.stderr).splitlines())))
    rep = ["# J1 behavior gate — emitted vs original (same inputs)", ""]
    for rid, status, detail in rows:
        rep.append(f"## {rid} — **{status}**\n\n```")
        rep.append(detail if detail else "(no output)")
        rep.append("```")
    report = "\n".join(rep) + "\n"
    (HERE / "report.md").write_text(report)
    print(report)
    sys.exit(0 if clean else 1)


if __name__ == "__main__":
    main()