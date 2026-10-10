//! PLAN 4.1 — tree-sitter reader setup for the JS → IR round trip.
//!
//! The round-trip contract (Phase 4): existing JS → lossy IR → back, with
//! the double double-trip acceptance gate. Everything here is read-side
//! scaffolding: parse the JS/TS source corpus into tree-sitter CSTs and
//! expose the node-kind histogram that grounds the 4.2 coverage audit.

use std::collections::BTreeMap;
use std::path::Path;

use tree_sitter::{Language, Parser};

/// The TS grammar parses the js_*.ts corpus (superset of JS); TS-specific
/// nodes surface as `type_annotation` / `ambient_*` kinds and get skipped
/// by the JS-only mapping later.
pub fn ts_language() -> Language {
    tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()
}

pub fn js_language() -> Language {
    tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()
}

fn parser_for(lang: Language) -> Parser {
    let mut p = Parser::new();
    p.set_language(&lang).expect("grammar load");
    p
}

/// Parse a source string; None on syntax error (round trip is lossy-bounded,
/// never wrong).
pub fn parse_src(src: &str) -> Option<tree_sitter::Tree> {
    let mut p = parser_for(ts_language());
    p.parse(src, None)
}

/// Depth-first node-kind histogram (4.2 groundwork): which constructs the
/// corpus actually contains, at every tree depth.
pub fn kind_histogram(tree: &tree_sitter::Tree) -> BTreeMap<String, usize> {
    let mut out = BTreeMap::new();
    let mut cursor = tree.walk();
    'walk: loop {
        let kind = cursor.node().kind().to_string();
        *out.entry(kind).or_insert(0) += 1;
        // descend
        if cursor.goto_first_child() {
            continue;
        }
        // ascend to next sibling
        loop {
            if cursor.goto_next_sibling() {
                continue 'walk;
            }
            if !cursor.goto_parent() {
                break 'walk;
            }
        }
    }
    out
}

/// Parse every file in `dir` (rglob `.ts`/`.js`), histogram the kinds,
/// report parse failures explicitly.
pub fn corpus_kinds(dir: &Path) -> Result<(BTreeMap<String, usize>, Vec<String>), String> {
    let mut hist = BTreeMap::new();
    let mut failures = Vec::new();
    for entry in std::fs::read_dir(dir).map_err(|e| e.to_string())? {
        let path = entry.map_err(|e| e.to_string())?.path();
        if !path.is_file() {
            continue;
        }
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if ext != "ts" && ext != "js" {
            continue;
        }
        let src = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
        match parse_src(&src) {
            Some(tree) if tree.root_node().has_error() => {
                failures.push(format!("{}: syntax errors in CST", path.display()));
                for (k, v) in kind_histogram(&tree) {
                    *hist.entry(k).or_insert(0) += v;
                }
            }
            Some(tree) => {
                for (k, v) in kind_histogram(&tree) {
                    *hist.entry(k).or_insert(0) += v;
                }
            }
            None => failures.push(format!("{}: parse returned None", path.display())),
        }
    }
    Ok((hist, failures))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_ts_corpus_modules() {
        // representative fixtures for each shape the corpus carries
        let srcs = [
            "export function f(x: number): number { return x + 1; }",
            "export interface I { a?: string; b: number }",
            "const o = { a: 1, [k]: 2, ...rest };",
            "for (let i = 0; i < n; i++) fn(arr[i], i);",
            "class A extends B { #x = 1; static m() {} get g() { return 1 } }",
            "try { f(); } catch (e) { log(e); } finally { cleanup(); }",
            "x?.y?.z ?? d; a ? b : c; `(t ${v})`;",
        ];
        for src in srcs {
            let tree = parse_src(src).expect("parse");
            assert!(!tree.root_node().has_error(), "clean parse for: {src}");
        }
    }

    #[test]
    fn histogram_counts_kinds() {
        let tree = parse_src("export const a = 1;").expect("parse");
        let h = kind_histogram(&tree);
        assert!(h.get("program") >= Some(&1));
        assert!(h.get("lexical_declaration") >= Some(&1));
        assert!(h.get("export_statement") >= Some(&1));
    }
}