#!/usr/bin/env python3
"""J1 (PLAN v4 Phase 3) — behavior harness: emitted JS must BEHAVE like the
original sources, not merely parse.

For each subject: emit IR → JS (ircc to-js), transform the original with tsc
(`import type` disappears — js_01/js_08 originals run standalone), then run a
rig that calls both sides with the same inputs and compares outputs deeply
(JSON). Expected values are hand-derived from the original semantics and act
as a third check (emit==orig but != expected flags rendition drift).
"""

import json
import os
import shutil
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[2]
BUILD = HERE / "build"
IRCC = REPO / "target" / "debug" / "ircc"

# subject config: stub modules the emitted side-effect imports must resolve;
# behavior cases [[fn, args, expected]] (deep JSON compare); optional extra
# raw JS assertions for non-data cases (callbacks etc.)
SUBJECTS = [
    {
        "id": "js_01_util",
        "bind_extra": ["domEach"],
        "stubs": ["domhandler", "cheerio"],
        # round-trip canon aliases `./cheerio.js` → last-segment `js`
        "rt_stubs": ["domhandler", "js"],
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
        # round-trip canon keeps the full import's last segments
        "rt_stubs": ["cheerioselect", "domserializer", "domhandler", "htmlparser2", "parse5", "parse5htmlparser2treeadapter"],
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
    {
        "id": "linked_modules",
        "tasks_lib": "lib",
        "entry": "feature",
        "helpers": [("Helpers", "Fixtures/Helpers.js")],
        "cases": [
            ["Report", [3], 15],
            ["Report", [0], 0],
            ["Report", [10], 50],
            ["ReportMany", [[1, 2, 4]], [2, 4, 8]],
            ["ReportMany", [[]], []],
        ],
        "extra": "",
    },
]


RT_MODE = __import__("os").environ.get("RT_MODE") == "1"

def run(cmd, **kw):
    return subprocess.run(cmd, capture_output=True, text=True, **kw)


def prepare(subject):
    work = BUILD / subject["id"]
    if work.exists():
        shutil.rmtree(work)
    work.mkdir(parents=True)
    (work / "package.json").write_text('{"type":"module"}\n')
    stub_key = "rt_stubs" if os.environ.get("RT_MODE") == "1" else "stubs"
    for stub in subject.get(stub_key, subject.get("stubs", [])):
        sp = work / f"{stub}.js"
        sp.parent.mkdir(parents=True, exist_ok=True)
        sp.write_text("// behavior-harness stub: satisfies v0 side-effect import\n")

    def ir_path(name):
        # RT_MODE=1: run the harness against 4.4 round-trip canon
        # (bench/corpus/real/rt/) instead of the hand renditions (J1 mode)
        base = "rt" if os.environ.get("RT_MODE") == "1" else "ir"
        if subject.get("tasks_lib"):
            return HERE / subject["tasks_lib"] / f"{name}.ir"
        return REPO / "bench" / "corpus" / "real" / base / f"{name}.ir"

    def emit(name):
        src = ir_path(name)
        proc = run([str(IRCC), "to-js", "--map", str(src)])
        out = src.parent / f"{name}.js"
        if not out.exists():
            return None, (proc.stdout + proc.stderr).strip()[:300]
        return out.read_text(), (src.parent / f"{name}.map.json").read_text()

    if subject.get("tasks_lib"):
        for helper, dest in subject.get("helpers", []):
            js, _ = emit(helper)
            if js is None:
                return None, f"helper {helper} emission failed"
            dest_path = work / dest
            dest_path.parent.mkdir(parents=True, exist_ok=True)
            dest_path.write_text(js)
        js, mp = emit(subject["entry"])
        if js is None:
            return None, "entry emission failed"
        (work / "feature.js").write_text(js)
        (work / "feature.map.json").write_text(mp)
        return work, ""

    ir = ir_path(subject["id"])
    js, mp = emit(subject["id"])
    if js is None or mp is None:
        return None, "emission failed"
    (work / "feature.js").write_text(js)
    (work / "feature.map.json").write_text(mp)
    # original → standalone JS (tsc transform erases `import type`)
    src = REPO / "bench" / "corpus" / "real" / "src" / f"{subject['id']}.ts"
    t = run(["npx", "-y", "-p", "typescript", "tsc", "--target", "es2022",
             "--module", "esnext", "--outDir", str(work / "orig"),
             "--skipLibCheck", str(src)], cwd=str(REPO))
    orig = work / "orig" / f"{subject['id']}.js"
    if not orig.exists():
        return None, "tsc original transform produced no output: " + (t.stdout + t.stderr)[:300]
    shutil.copy(orig, work / "orig.js")
    return work, ""


def rig_js(subject):
    """The corpus-emitted modules expose no exports (renditions use bare
    `fn`), so the E-side binds local decls by CONCATENATING feature.js with
    the runner — the emitted artifact itself stays untouched. Linked-module
    subjects (tasks_lib) emit real exports; import both sides."""
    extra = subject.get("extra", "")
    cases = json.dumps(subject["cases"], indent=1)
    bound = sorted({c[0] for c in subject["cases"]} | set(subject.get("bind_extra", [])))
    if subject.get("tasks_lib"):
        tpl = """import * as E from "./__MODULE__.js";
const cases = __CASES__;
let fail = 0;
for (const [fn, args, expected] of cases) {
  const shown = args.map(a => JSON.stringify(a)).join(", ");
  let ge = "?", ere = null;
  try { ge = JSON.stringify(E[fn](...args)); } catch (e) { ere = String(e); }
  if (ere || ge === "?") {
    fail++;
    console.error(`THROW ${fn}(${shown}) emitted=${ere || ge}`);
  } else if (ge !== JSON.stringify(expected)) {
    fail++;
    console.error(`DIFF ${fn}(${shown}) emitted=${ge} expected=${JSON.stringify(expected)}`);
  } else {
    console.log(`PASS ${fn}(${shown}) -> ${ge}`);
  }
}
__EXTRA__
if (fail > 0) { console.error(`RESULTS: ${fail} failures`); process.exit(1); }
console.log("RESULTS: ALL MATCH");
"""
        module = subject.get("entry_module") or subject["entry"]
        return (tpl.replace("__MODULE__", module)
                   .replace("__CASES__", cases)
                   .replace("__EXTRA__", extra))
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