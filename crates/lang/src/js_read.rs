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

/// Parse every file in `dir` (or each given `.ts`/`.js` file), histogram the
/// kinds, report parse failures explicitly.
pub fn corpus_kinds(dir: &Path) -> Result<(BTreeMap<String, usize>, Vec<String>), String> {
    let mut hist = BTreeMap::new();
    let mut failures = Vec::new();
    let files: Vec<_> = if dir.is_file() {
        vec![dir.to_path_buf()]
    } else {
        std::fs::read_dir(dir)
            .map_err(|e| e.to_string())?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.is_file())
            .collect()
    };
    for path in files {
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
// ============================================================================
// PLAN 4.4 — JS reader (round-trip leg): tree-sitter CST → Core forms
//
// Classification follows js-target.md §8.2 (the 4.2 audit is the checklist):
// core/verbatim constructs → core forms; templates/`++`/fn-exprs → canon
// sugar; type surface → signatures (erased); union/template type aliases →
// `opaque type` verbatim capture; unclassifiable members → `opaque:`-atom
// verbatim capture (js-target §8.1). Never wrong: every member maps.

use crate::ast::{Atom, Span};
use crate::expander::{Arm, Body, Core, FnDecl, TypeDecl, VarDecl, SPAN_SYNTH};

pub struct JsReadError {
    pub msg: String,
}

impl std::fmt::Debug for JsReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JsReadError").field("msg", &self.msg).finish()
    }
}

impl std::fmt::Display for JsReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.msg)
    }
}

/// JS/TS source → core forms. `uses` accumulate `@use` aliases (module path
/// last segment, J4 convention).
pub fn read_js(src: &str) -> Result<Vec<Core>, JsReadError> {
    let tree = parse_src(src).ok_or(JsReadError { msg: "parse failed".into() })?;
    let mut r = JsReader { src, uses: Vec::new() };
    let mut out = vec![];
    let root = tree.root_node();
    let mut cursor = root.walk();
    let program = cursor.goto_first_child();
    if !program {
        return Ok(vec![header_core(&r.uses)]);
    }
    loop {
        let child = cursor.node();
        if child.kind() != "comment" {
            let forms = r.decl(child);
            out.extend(forms);
        }
        if !cursor.goto_next_sibling() {
            break;
        }
    }
    out.insert(0, header_core(&r.uses));
    Ok(out)
}

fn header_core(uses: &[String]) -> Core {
    Core::Header {
        version: "1".into(),
        ns: None,
        uses: uses.to_vec(),
        span: SPAN_SYNTH,
    }
}

struct JsReader<'a> {
    src: &'a str,
    uses: Vec<String>,
}

/// text of a node (verbatim, untrimmed)
fn text<'a>(node: tree_sitter::Node<'a>, src: &'a str) -> &'a str {
    node.utf8_text(src.as_bytes()).unwrap_or("")
}

impl<'a> JsReader<'a> {
    fn span(&self, node: tree_sitter::Node) -> Span {
        Span::point(node.start_position().row + 1)
    }

    fn add_use(&mut self, path: &str) {
        let alias = path
            .trim_matches(|c| c == '\'' || c == '"')
            .rsplit(['/', '.'])
            .next()
            .unwrap_or("m")
            .replace('-', "");
        if !self.uses.iter().any(|u| u == &alias) {
            self.uses.push(alias);
        }
    }

    // ------------------------------------------------------------------
    // type surface → text (erased to signatures; IR conventions: ` | ` in
    // unions per hand renders, tight generics `List<T>`)

    /// the type half of a `type_annotation` (its first child is the `:`).
    fn annotation_type(&self, ta: Option<tree_sitter::Node>) -> String {
        let Some(ta) = ta else { return String::new() };
        let mut c = ta.walk();
        if !c.goto_first_child() {
            return String::new();
        }
        loop {
            let n = c.node();
            let is_colon = n.kind() == ":";
            let empty = n.utf8_text(self.src.as_bytes()).unwrap_or("").trim().is_empty();
            if !is_colon && !empty {
                return canon_ty_text(&self.type_text(n));
            }
            if !c.goto_next_sibling() {
                return String::new();
            }
        }
    }

    fn type_text(&self, node: tree_sitter::Node) -> String {
        match node.kind() {
            "identifier" | "type_identifier" | "unknown" | "this" => text(node, self.src).to_string(),
            // TS type words → IR signature spellings (§8.2; hand canon)
            "predefined_type" | "void" => match text(node, self.src) {
                "string" => "String".into(),
                "number" => "Int".into(),
                "boolean" => "Bool".into(),
                "unknown" | "any" => "Any".into(),
                other => other.to_string(),
            },
            "generic_type" => {
                let name = node
                    .child_by_field_name("name")
                    .map(|n| self.type_text(n))
                    .unwrap_or_default();
                let args = node
                    .child_by_field_name("type_arguments")
                    .map(|a| self.type_args(a))
                    .unwrap_or_default();
                if args.is_empty() {
                    name
                } else {
                    format!("{name}<{args}>")
                }
            }
            "type_arguments" => self.type_args(node),
            "union_type" => {
                // canon unions are TIGHT (`a|b`) — the IR reader lexes ` | `
                // space forms and re-prints tight, so read-side unions spell
                // tight from the start (identity gate)
                let mut c = node.walk();
                let mut parts = vec![];
                if c.goto_first_child() {
                    loop {
                        // `|` separator tokens are children too — skip them
                        if c.node().kind() != "|" {
                            let t = self.type_text(c.node());
                            if !t.is_empty() {
                                parts.push(t);
                            }
                        }
                        if !c.goto_next_sibling() {
                            break;
                        }
                    }
                }
                parts.join("|")
            }
            "array_type" => {
                let el = node.child(0).map(|n| self.type_text(n)).unwrap_or_default();
                if el.is_empty() {
                    String::new()
                } else {
                    format!("Array<{el}>")
                }
            }
            "function_type" => {
                // erased signature: function-typed params/rets drop the
                // annotation (ellipsis spellings would need a grammar change;
                // types erase at emit anyway — fidelity note, §8.2)
                String::new()
            }
            "parenthesized_type" => {
                let inner = node.child(1).map(|n| self.type_text(n)).unwrap_or_default();
                if inner.is_empty() {
                    String::new()
                } else {
                    format!("({inner})")
                }
            }
            "literal_type" => {
                node.child(0).map(|n| match n.kind() {
                    "string" | "number" | "true" | "false" | "null" => text(n, self.src).to_string(),
                    other => self.type_text(n),
                })
                .unwrap_or_default()
            }
            "lookup_type" => {
                // `SelectOptions['quirksMode']` → member access (js_08.ir:24)
                let mut c = node.walk();
                let mut obj = String::new();
                let mut idx = String::new();
                if c.goto_first_child() {
                    loop {
                        let n = c.node();
                        let k = n.kind();
                        if obj.is_empty()
                            && matches!(
                                k,
                                "identifier" | "type_identifier" | "predefined_type"
                                    | "generic_type" | "parenthesized_type"
                            )
                        {
                            obj = self.type_text(n);
                        } else if !obj.is_empty()
                            && idx.is_empty()
                            && matches!(k, "string" | "number" | "identifier")
                        {
                            idx = text(n, self.src)
                                .trim_matches(|c| c == '\'' || c == '"')
                                .to_string();
                        }
                        if !c.goto_next_sibling() {
                            break;
                        }
                    }
                }
                format!("{obj}.{idx}")
            }
            "template_literal_type" | "template_type" => text(node, self.src).to_string(),
            "object_type" => {
                // interface-shaped object-type annotation erases
                String::new()
            }
            _ => text(node, self.src).to_string(),
        }
    }

    fn type_args(&self, node: tree_sitter::Node) -> String {
        let mut c = node.walk();
        let mut parts = vec![];
        if c.goto_first_child() {
            loop {
                let kind = c.node().kind();
                if kind != "," && kind != "<" && kind != ">" && kind != "type" {
                    parts.push(self.type_text(c.node()));
                }
                if !c.goto_next_sibling() {
                    break;
                }
            }
        }
        parts.join(",")
    }

    // ------------------------------------------------------------------
    // string / template payloads

    fn str_payload(&self, node: tree_sitter::Node) -> String {
        let t = text(node, self.src);
        t.trim_matches(|c| c == '\'' || c == '"').to_string()
    }

    fn template_payload(&self, node: tree_sitter::Node) -> String {
        let mut c = node.walk();
        let mut s = String::new();
        if c.goto_first_child() {
            loop {
                let n = c.node();
                match n.kind() {
                    "string_fragment" | "escape_sequence" => s.push_str(text(n, self.src)),
                    "template_substitution" => {
                        s.push('{');
                        // the expr → IR atoms → canon text (tight spacing)
                        if let Some(e) = n.child(1) {
                            s.push_str(&crate::printer::render_atoms(&self.atomize(e)));
                        }
                        s.push('}');
                    }
                    _ => {}
                }
                if !c.goto_next_sibling() {
                    break;
                }
            }
        }
        s
    }

    // ------------------------------------------------------------------
    // expression → atom run (§8.2 core/verbatim; sugar handled here)

    fn atomize(&self, node: tree_sitter::Node) -> Vec<Atom> {
        match node.kind() {
            "identifier" | "property_identifier" | "shorthand_property_identifier" => {
                vec![Atom::Ident(text(node, self.src).to_string())]
            }
            "this" => vec![Atom::Ident("this".into())],
            "number" => vec![Atom::Number(text(node, self.src).to_string())],
            "true" => vec![Atom::Ident("true".into())],
            "false" => vec![Atom::Ident("false".into())],
            "null" => vec![Atom::Ident("null".into())],
            "undefined" => vec![Atom::Ident("undefined".into())],
            "string" => vec![Atom::Str(self.str_payload(node))],
            "template_string" => vec![Atom::Str(self.template_payload(node))],
            "regex" => {
                let pat = node.child_by_field_name("pattern")
                    .or_else(|| node.child(1))
                    .map(|n| text(n, self.src))
                    .unwrap_or("");
                let flags = node.child_by_field_name("flags")
                    .map(|n| text(n, self.src))
                    .unwrap_or("");
                vec![Atom::Regex(format!("{pat}/{flags}"))]
            }
            "parenthesized_expression" => {
                let inner = node.child(1).map(|n| self.atomize(n)).unwrap_or_default();
                let mut v = vec![Atom::Sym("(".into())];
                v.extend(inner);
                v.push(Atom::Sym(")".into()));
                v
            }
            "call_expression" => {
                let mut v = vec![];
                if let Some(f) = node.child_by_field_name("function") {
                    v.extend(self.atomize_callee(f));
                }
                v.push(Atom::Sym("(".into()));
                if let Some(args) = node.child_by_field_name("arguments") {
                    let arg_atoms = self.args_atoms(args);
                    v.extend(arg_atoms);
                }
                v.push(Atom::Sym(")".into()));
                v
            }
            "arguments" => {
                let mut v = vec![Atom::Sym("(".into())];
                v.extend(self.args_atoms(node));
                v.push(Atom::Sym(")".into()));
                v
            }
            "member_expression" => {
                let obj = node.child_by_field_name("object")
                    .map(|n| self.atomize(n))
                    .unwrap_or_default();
                let prop = node.child_by_field_name("property")
                    .map(|n| text(n, self.src).to_string())
                    .unwrap_or_default();
                let mut v = obj;
                // `x?.y`: an inline `optional_chain` child carries the `?.`
                // (including its dot) → canon `?.`; plain chains add `.`
                let mut opt = false;
                {
                    let mut c = node.walk();
                    if c.goto_first_child() {
                        loop {
                            if c.node().kind() == "optional_chain" {
                                opt = true;
                                break;
                            }
                            if !c.goto_next_sibling() {
                                break;
                            }
                        }
                    }
                }
                if opt {
                    v.push(Atom::Sym("?".into()));
                    v.push(Atom::Sym(".".into()));
                } else {
                    v.push(Atom::Sym(".".into()));
                }
                v.push(Atom::Ident(prop));
                v
            }
            "subscript_expression" => {
                let obj = node.child_by_field_name("object")
                    .map(|n| self.atomize(n))
                    .unwrap_or_default();
                // `x?.[i]` optional subscript: `optional_chain` child marker
                let mut opt = false;
                {
                    let mut c = node.walk();
                    if c.goto_first_child() {
                        loop {
                            if c.node().kind() == "optional_chain" {
                                opt = true;
                                break;
                            }
                            if !c.goto_next_sibling() {
                                break;
                            }
                        }
                    }
                }
                let idx = node.child_by_field_name("index")
                    .map(|n| self.atomize(n))
                    .unwrap_or_default();
                let mut v = obj;
                if opt {
                    v.push(Atom::Sym("?".into()));
                }
                v.push(Atom::Sym("[".into()));
                v.extend(idx);
                v.push(Atom::Sym("]".into()));
                v
            }
            "binary_expression" | "logical_expression" => {
                let op = node.child_by_field_name("operator")
                    .map(|n| text(n, self.src).to_string())
                    .unwrap_or("+".into());
                let l = node.child_by_field_name("left").map(|n| self.atomize(n)).unwrap_or_default();
                let r = node.child_by_field_name("right").map(|n| self.atomize(n)).unwrap_or_default();
                let mut v = l;
                v.push(Atom::Sym(op));
                v.extend(r);
                v
            }
            "unary_expression" => {
                let op = node.child_by_field_name("operator")
                    .map(|n| text(n, self.src).to_string())
                    .unwrap_or("!".into());
                let arg = node.child_by_field_name("argument").map(|n| self.atomize(n)).unwrap_or_default();
                let mut v = if op.chars().all(|c| c.is_ascii_alphabetic()) {
                    vec![Atom::Ident(op)]
                } else {
                    vec![Atom::Sym(op)]
                };
                // prefix operators glue tight (`!options`, `await e` spaces)
                match v.first() {
                    Some(Atom::Ident(_)) => v.extend(arg),
                    _ => v.extend(arg),
                }
                v
            }
            "update_expression" => {
                // `i++` → `i+=1` canon (js-target §8.2 sugar)
                let arg = node.child_by_field_name("argument").map(|n| self.atomize(n)).unwrap_or_default();
                let op = node.child_by_field_name("operator")
                    .map(|n| text(n, self.src).to_string())
                    .unwrap_or("++".into());
                let before = node.child_by_field_name("argument")
                    .map(|n| n.start_position() < node.child_by_field_name("operator").map(|o| o.start_position()).unwrap_or(n.start_position()))
                    .unwrap_or(true);
                let mut v = arg;
                if before {
                    v.push(Atom::Sym(if op == "++" { "+=".into() } else { "-=".into() }));
                    v.push(Atom::Number("1".into()));
                } else {
                    // postfix — same canon (statement tail; behavior-preserving
                    // when the value is discarded; value-using forms are rare)
                    v.push(Atom::Sym(if op == "++" { "+=".into() } else { "-=".into() }));
                    v.push(Atom::Number("1".into()));
                }
                v
            }
            "assignment_expression" | "augmented_assignment_expression" => {
                let l = node.child_by_field_name("left").map(|n| self.atomize(n)).unwrap_or_default();
                let op = node.child_by_field_name("operator")
                    .map(|n| text(n, self.src).to_string())
                    .unwrap_or("=".into());
                let r = node.child_by_field_name("right").map(|n| self.atomize(n)).unwrap_or_default();
                let mut v = l;
                v.push(Atom::Sym(op));
                v.extend(r);
                v
            }
            "ternary_expression" => {
                let c = node.child_by_field_name("condition").map(|n| self.atomize(n)).unwrap_or_default();
                let t = node.child_by_field_name("consequence").map(|n| self.atomize(n)).unwrap_or_default();
                let a = node.child_by_field_name("alternative").map(|n| self.atomize(n)).unwrap_or_default();
                let mut v = c;
                v.push(Atom::Sym("?".into()));
                v.extend(t);
                v.push(Atom::Sym(":".into()));
                v.extend(a);
                v
            }
            "spread_element" => {
                let mut v = vec![Atom::Sym("...".into())];
                v.extend(node.child(1).map(|n| self.atomize(n)).unwrap_or_default());
                v
            }
            "object" => {
                let mut v = vec![Atom::Sym("{".into())];
                let mut c = node.walk();
                let mut pairs = 0usize;
                if c.goto_first_child() {
                    loop {
                        let n = c.node();
                        match n.kind() {
                            "pair" | "shorthand_property_identifier" | "spread_element"
                            | "method_definition" => {
                                let mut part = self.atomize(n);
                                if pairs > 0 && !v.is_empty() {
                                    v.push(Atom::Sym(",".into()));
                                }
                                if !part.is_empty() {
                                    pairs += 1;
                                    v.append(&mut part);
                                }
                            }
                            _ => {}
                        }
                        if !c.goto_next_sibling() {
                            break;
                        }
                    }
                }
                v.push(Atom::Sym("}".into()));
                v
            }
            "pair" => {
                let key = node.child_by_field_name("key").map(|n| self.atomize(n)).unwrap_or_default();
                let val = node.child_by_field_name("value").map(|n| self.atomize(n)).unwrap_or_default();
                let mut v = key;
                v.push(Atom::Sym(":".into()));
                v.extend(val);
                v
            }
            "array" => {
                let mut v = vec![Atom::Sym("[".into())];
                let mut c = node.walk();
                let mut items = 0usize;
                if c.goto_first_child() {
                    loop {
                        let n = c.node();
                        if !matches!(n.kind(), "comment" | "," | "[" | "]") {
                            let mut part = self.atomize(n);
                            if items > 0 && !v.is_empty() {
                                v.push(Atom::Sym(",".into()));
                            }
                            if !part.is_empty() {
                                items += 1;
                                v.append(&mut part);
                            }
                        }
                        if !c.goto_next_sibling() {
                            break;
                        }
                    }
                }
                v.push(Atom::Sym("]".into()));
                v
            }
            "new_expression" => {
                let callee = node.child_by_field_name("constructor")
                    .or_else(|| node.child(1))
                    .map(|n| self.atomize(n))
                    .unwrap_or_default();
                let args = node.child_by_field_name("arguments")
                    .map(|n| self.args_atoms(n))
                    .unwrap_or_default();
                let mut v = vec![Atom::Ident("new".into())];
                v.extend(callee);
                v.push(Atom::Sym("(".into()));
                v.extend(args);
                v.push(Atom::Sym(")".into()));
                v
            }
            "arrow_function" | "function_expression" | "function" => self.lambda_atoms(node),
            "as_expression" => {
                // `x as T` — operand + `as` + erased type atoms; union parts
                // split (`|` as Sym) so canon print round-1 == round-2 glue
                let l = node.child(0).map(|n| self.atomize(n)).unwrap_or_default();
                let mut v = l;
                v.push(Atom::Ident("as".into()));
                if let Some(ty) = node.child(2) {
                    v.extend(self.alias_body_atoms(ty));
                }
                v
            }
            "optional_chain" => {
                // `x?.cheerio` → x ? . cheerio (canon glue renders `?.`)
                let obj = node.child(0).map(|n| self.atomize(n)).unwrap_or_default();
                let prop = node.child(2).map(|n| text(n, self.src).to_string()).unwrap_or_default();
                let mut v = obj;
                v.push(Atom::Sym("?".into()));
                v.push(Atom::Sym(".".into()));
                v.push(Atom::Ident(prop));
                v
            }
            "non_null_expression" => {
                let mut v = node.child(0).map(|n| self.atomize(n)).unwrap_or_default();
                v.push(Atom::Sym("!".into()));
                v
            }
            _ => vec![Atom::Str(format!(
                "opaque:{}", text(node, self.src).trim()
            ))],
        }
    }

    /// callee atoms — with callee-generics erased (`map<T,..>(` → `map(`)
    fn atomize_callee(&self, node: tree_sitter::Node) -> Vec<Atom> {
        match node.kind() {
            "generic_function" | "call_expression" if node.child_by_field_name("type_arguments").is_some() => {
                let mut v = node
                    .child_by_field_name("function")
                    .map(|n| self.atomize(n))
                    .unwrap_or_default();
                // generics erased at emit (J3) — reader emits them too? canon
                // keeps callee generics in the surface (erasure happens at
                // emit); keep verbatim atoms
                if node.kind() == "generic_function" {
                    if let Some(ta) = node.child_by_field_name("type_arguments") {
                        let t = text(ta, self.src);
                        let mut generics: Vec<Atom> = vec![Atom::Sym("<".into())];
                        for (i, part) in t
                            .trim_start_matches('<')
                            .trim_end_matches('>')
                            .split(',')
                            .filter(|s| !s.trim().is_empty())
                            .enumerate()
                        {
                            if i > 0 {
                                generics.push(Atom::Sym(",".into()));
                            }
                            generics.push(Atom::Ident(part.trim().to_string()));
                        }
                        generics.push(Atom::Sym(">".into()));
                        v.extend(generics);
                    }
                }
                v
            }
            _ => self.atomize(node),
        }
    }

    fn args_atoms(&self, args_node: tree_sitter::Node) -> Vec<Atom> {
        let mut v: Vec<Atom> = vec![];
        let mut c = args_node.walk();
        let mut argc = 0usize;
        if c.goto_first_child() {
            loop {
                let n = c.node();
                // separators & parens: rendered by the canon, never atomized
                if matches!(n.kind(), "comment" | "(" | ")" | "," | "=>") {
                    if !c.goto_next_sibling() {
                        break;
                    }
                    continue;
                }
                let mut part = self.atomize(n);
                if argc > 0 && !v.is_empty() {
                    v.push(Atom::Sym(",".into()));
                }
                if !part.is_empty() {
                    argc += 1;
                    v.append(&mut part);
                }
                if !c.goto_next_sibling() {
                    break;
                }
            }
        }
        v
    }

    fn lambda_atoms(&self, node: tree_sitter::Node) -> Vec<Atom> {
        let mut v: Vec<Atom> = vec![];
        // params
        if let Some(p) = node.child_by_field_name("parameters") {
            v.extend(self.lambda_params(p));
        } else {
            // `x => e` — parameter is the first identifier child
            let mut c = node.walk();
            if c.goto_first_child() {
                loop {
                    let n = c.node();
                    if matches!(n.kind(), "identifier" | "this") {
                        v.push(Atom::Ident(text(n, self.src).to_string()));
                        break;
                    }
                    if !c.goto_next_sibling() {
                        break;
                    }
                }
            }
        }
        v.push(Atom::Sym("=>".into()));
        if let Some(body) = node.child_by_field_name("body") {
            match body.kind() {
                "expression" | "parenthesized_expression" | "call_expression"
                | "member_expression" | "binary_expression" | "logical_expression"
                | "identifier" | "string" | "template_string" | "number"
                | "new_expression" | "ternary_expression" | "array" | "object"
                | "await_expression" | "as_expression" => v.extend(self.atomize(body)),
                // `{ … }` block lambda: braces as atoms; statements glue as
                // keyword-led runs (split_stmt_runs classifies at emit)
                "statement_block" => {
                    v.push(Atom::Sym("{".into()));
                    let mut c = body.walk();
                    if c.goto_first_child() {
                        loop {
                            let n = c.node();
                            if n.kind() != "{" && n.kind() != "}" && n.kind() != "comment" {
                                v.extend(self.stmt_atoms(n));
                            }
                            if !c.goto_next_sibling() {
                                break;
                            }
                        }
                    }
                    v.push(Atom::Sym("}".into()));
                }
                _ => v.extend(self.stmt_atoms(body)),
            }
        }
        v
    }

    /// destructure pattern → raw atoms (`[, q]`, `{id}`, `...rest`): syms
    /// verbatim, identifiers at the leaves, type annotations erased
    fn pattern_atoms(&self, node: tree_sitter::Node) -> Vec<Atom> {
        match node.kind() {
            "identifier" | "this" => vec![Atom::Ident(text(node, self.src).to_string())],
            "comment" | "type_annotation" => vec![],
            _ => {
                let mut v: Vec<Atom> = vec![];
                let mut c = node.walk();
                if c.goto_first_child() {
                    loop {
                        let n = c.node();
                        if matches!(n.kind(), "[" | "]" | "{" | "}" | "," | "..." | "=") {
                            v.push(Atom::Sym(n.kind().into()));
                        } else if n.kind() != "comment" && n.kind() != "type_annotation" {
                            v.extend(self.pattern_atoms(n));
                        }
                        if !c.goto_next_sibling() { break; }
                    }
                }
                v
            }
        }
    }

    fn lambda_params(&self, p: tree_sitter::Node) -> Vec<Atom> {
        let mut v: Vec<Atom> = vec![Atom::Sym("(".into())];
        let mut c = p.walk();
        let mut pc = 0usize;
        if c.goto_first_child() {
            loop {
                let n = c.node();
                if n.kind() != "comment" && n.kind() != "(" && n.kind() != ")" && n.kind() != ","
                    && n.kind() != ":" && n.kind() != "type_annotation"
                {
                    if matches!(
                        n.kind(),
                        "identifier" | "this" | "rest_pattern" | "object_pattern" | "array_pattern"
                        | "required_parameter" | "optional_parameter" | "formal_parameter"
                    ) {
                        let is_destructure = matches!(n.kind(), "array_pattern" | "object_pattern" | "rest_pattern");
                        let part = if is_destructure {
                            // destructure patterns carry as RAW atoms
                            // (`([, q]) => …`) — opaque strings re-emit as
                            // invalid quoted destructuring targets
                            self.pattern_atoms(n)
                        } else if let Some(p) = n.child_by_field_name("pattern") {
                            if matches!(p.kind(), "array_pattern" | "object_pattern" | "rest_pattern") {
                                self.pattern_atoms(p)
                            } else {
                                self.atomize(p)
                            }
                        } else {
                            self.atomize(n)
                        };
                        if pc > 0 && !v.is_empty() {
                            v.push(Atom::Sym(",".into()));
                        }
                        pc += 1;
                        v.extend(part);
                    }
                }
                if !c.goto_next_sibling() {
                    break;
                }
            }
        }
        v.push(Atom::Sym(")".into()));
        v
    }

    /// bare `kw e` run pieces: leading ident + value atoms (`;` never atomizes)
    fn kw_stmt_atoms(&self, node: tree_sitter::Node, kw: &str) -> Vec<Atom> {
        let mut v: Vec<Atom> = vec![Atom::Ident(kw.into())];
        let mut c = node.walk();
        if c.goto_first_child() {
            loop {
                let n = c.node();
                if !matches!(n.kind(), ";" | "comment") && n.kind() != kw {
                    v.extend(self.atomize(n));
                }
                if !c.goto_next_sibling() {
                    break;
                }
            }
        }
        v
    }

    /// statement → its atom run (for lambda-block / run pieces)
    fn stmt_atoms(&self, node: tree_sitter::Node) -> Vec<Atom> {
        match node.kind() {
            "lexical_declaration" | "variable_declaration" => {
                let mut v: Vec<Atom> = vec![];
                let mut c = node.walk();
                if c.goto_first_child() {
                    loop {
                        let n = c.node();
                        if n.kind() == "variable_declarator" {
                            if let Some(name) = n.child_by_field_name("name") {
                                v.extend(self.atomize(name));
                            }
                            if let Some(val) = n.child_by_field_name("value") {
                                v.push(Atom::Sym("=".into()));
                                v.extend(self.atomize(val));
                            }
                        }
                        if !c.goto_next_sibling() {
                            break;
                        }
                    }
                }
                v
            }
            "return_statement" => self.kw_stmt_atoms(node, "return"),
            "throw_statement" => self.kw_stmt_atoms(node, "throw"),
            "expression_statement" => node
                .child(0)
                .map(|n| self.atomize(n))
                .unwrap_or_default(),
            "if_statement" => {
                let cond = node.child_by_field_name("condition").map(|n| {
                    let mut a = vec![Atom::Sym("(".into())];
                    if let Some(inner) = n.child(1) {
                        a.extend(self.atomize(inner));
                    }
                    a.push(Atom::Sym(")".into()));
                    a
                });
                let _ = cond;
                // statement-shaped if inside a run: `if c{…}else{…}`
                self.if_atoms(node)
            }
            _ => self.atomize(node),
        }
    }

    /// `if cond{…}else{…}` as an atom run (statement-shaped, brace glued)
    fn if_atoms(&self, node: tree_sitter::Node) -> Vec<Atom> {
        let mut v: Vec<Atom> = vec![Atom::Ident("if".into())];
        if let Some(cond) = node.child_by_field_name("condition") {
            // cond node includes its parens; inner expr only (canon: bare)
            if let Some(inner) = cond.child(1) {
                v.extend(self.atomize(inner));
            }
        }
        v.push(Atom::Sym("{".into()));
        // then-arm: `consequence` field, else arm via else_clause
        if let Some(then) = node.child_by_field_name("consequence") {
            if then.kind() == "statement_block" {
                let mut c = then.walk();
                if c.goto_first_child() {
                    loop {
                        let n = c.node();
                        if n.kind() != "{" && n.kind() != "}" && n.kind() != "comment" {
                            v.extend(self.stmt_atoms(n));
                        }
                        if !c.goto_next_sibling() {
                            break;
                        }
                    }
                }
            } else {
                // braceless arm — the node IS the statement
                v.extend(self.stmt_atoms(then));
            }
        }
        v.push(Atom::Sym("}".into()));
        if let Some(else_c) = node.child_by_field_name("alternative") {
            match else_c.kind() {
                "else_clause" => {
                    v.push(Atom::Ident("else".into()));
                    let body = else_c.child(1).unwrap_or(else_c);
                    match body.kind() {
                        "if_statement" => v.extend(self.if_atoms(body)),
                        "statement_block" => {
                            v.push(Atom::Sym("{".into()));
                            let mut c = body.walk();
                            if c.goto_first_child() {
                                loop {
                                    let n = c.node();
                                    if n.kind() != "{" && n.kind() != "}" && n.kind() != "comment" {
                                        v.extend(self.stmt_atoms(n));
                                    }
                                    if !c.goto_next_sibling() {
                                        break;
                                    }
                                }
                            }
                            v.push(Atom::Sym("}".into()));
                        }
                        _ => v.extend(self.stmt_atoms(body)),
                    }
                }
                _ => {}
            }
        }
        v
    }
}

// ---------------------------------------------------------------------------
// declarations & statements (module level; Stage B+C)

impl<'a> JsReader<'a> {
    /// a module-level statement → one or more Core forms
    pub fn decl(&mut self, node: tree_sitter::Node<'a>) -> Vec<Core> {
        match node.kind() {
            "import_statement" => {
                if let Some(src_n) = node.child_by_field_name("source") {
                    self.add_use(text(src_n, self.src));
                }
                vec![]
            }
            "export_statement" => {
                // three shapes: `export <decl>` (declaration child),
                // `export {…} from m` (export_clause), `export * from m`
                // (star child, no clause). Keyword/string children under an
                // export are NEVER module content — pre-scan decides.
                let mut has_clause = false;
                let mut has_decl = false;
                let mut has_star = false;
                {
                    let mut c = node.walk();
                    if c.goto_first_child() {
                        loop {
                            match c.node().kind() {
                                "export_clause" | "export_specifier" => has_clause = true,
                                "declaration" | "lexical_declaration"
                                | "function_declaration" | "class_declaration"
                                | "interface_declaration" | "type_alias_declaration"
                                | "statement_block"
                                | "function_signature" | "abstract_method_signature"
                                | "method_signature" => has_decl = true,
                                "*" => has_star = true,
                                _ => {}
                            }
                            if !c.goto_next_sibling() {
                                break;
                            }
                        }
                    }
                }
                if has_clause {
                    vec![self.reexport(node)]
                } else if has_star {
                    vec![self.reexport(node)]
                } else if has_decl {
                    let mut forms = vec![];
                    let mut c = node.walk();
                    if c.goto_first_child() {
                        loop {
                            let n = c.node();
                            if matches!(
                                n.kind(),
                                "declaration" | "lexical_declaration"
                                | "function_declaration" | "class_declaration"
                                | "interface_declaration" | "type_alias_declaration"
                                | "statement_block" | "function_signature"
                                | "abstract_method_signature" | "method_signature"
                            ) {
                                let mut f = self.decl(n);
                                for form in f.iter_mut() {
                                    match form {
                                        Core::Fn(d) => {
                                            if !d.modifiers.iter().any(|m| m == "export") {
                                                d.modifiers.push("export".into());
                                            }
                                        }
                                        Core::Type(t) => {
                                            if !t.modifiers.iter().any(|m| m == "export") {
                                                t.modifiers.push("export".into());
                                            }
                                        }
                                        Core::Var(v) => {
                                            if !v.modifiers.iter().any(|m| m == "export") {
                                                v.modifiers.insert(0, "export".into());
                                            }
                                        }
                                        _ => {}
                                    }
                                }
                                forms.extend(f);
                            }
                            if !c.goto_next_sibling() {
                                break;
                            }
                        }
                    }
                    forms
                } else if node.child_count() > 1 {
                    vec![self.reexport(node)]
                } else {
                    vec![]
                }
            }
            "function_declaration" => vec![self.fn_decl(node, false)],
            // overload signatures (declaration-only fns) drop — IR has no
            // overload set; the implementation fn carries the signature
            // (hand canon js_05.ir:3-5, fidelity item 5)
            "function_signature" => vec![],
            "abstract_method_signature" | "method_signature" => vec![self.fn_decl(node, true)],
            "lexical_declaration" | "variable_declaration" => {
                self.top_vars(node)
            }
            "expression_statement" => {
                let inner = node.child(0).unwrap_or(node);
                if inner.kind() == "call_expression" {
                    vec![Core::Expr(self.atomize(inner), self.span(node))]
                } else {
                    vec![Core::Expr(self.atomize(inner), self.span(node))]
                }
            }
            "if_statement" => vec![self.stmt(node)],
            "for_statement" | "for_in_statement" | "while_statement" => {
                vec![self.stmt(node)]
            }
            "return_statement" | "throw_statement" => vec![self.stmt(node)],
            "class_declaration" => vec![self.class_decl(node)],
            "interface_declaration" => vec![self.interface_decl(node)],
            "type_alias_declaration" => vec![self.alias_decl(node)],
            "comment" => vec![],
            _ => vec![Core::Expr(
                vec![Atom::Str(format!("opaque:{}", text(node, self.src).trim()))],
                self.span(node),
            )],
        }
    }

    /// named re-export / star re-export → Core::Reexport
    fn reexport(&mut self, node: tree_sitter::Node) -> Core {
        let mut names = vec![];
        let mut star = false;
        let mut is_type = false;
        let mut from = String::new();
        let mut c = node.walk();
        if c.goto_first_child() {
            loop {
                let n = c.node();
                match n.kind() {
                    "export_clause" => {
                        let mut c2 = n.walk();
                        if c2.goto_first_child() {
                            loop {
                                let spec = c2.node();
                                if spec.kind() == "export_specifier" {
                                    let name = spec.child_by_field_name("name")
                                        .map(|x| text(x, self.src).to_string())
                                        .unwrap_or_default();
                                    if !name.is_empty() {
                                        names.push(name);
                                    }
                                } else if spec.kind() == "identifier" {
                                    names.push(text(spec, self.src).to_string());
                                }
                                if !c2.goto_next_sibling() {
                                    break;
                                }
                            }
                        }
                    }
                    "export" => {}
                    "type" => is_type = true,
                    "*" => star = true,
                    "import_from_statement" | "string" => {
                        if n.kind() == "string" {
                            from = self.str_payload(n);
                            self.add_use(&from);
                        }
                    }
                    "from" => {
                        // source follows
                    }
                    _ => {
                        // the module path string may appear as a direct child
                        if n.kind() == "string" {
                            from = self.str_payload(n);
                            self.add_use(&from);
                        }
                    }
                }
                if !c.goto_next_sibling() {
                    break;
                }
            }
        }
        if let Some(src_n) = node.child_by_field_name("source") {
            from = self.str_payload(src_n);
            self.add_use(&from);
        }
        Core::Reexport {
            star,
            is_type,
            names,
            from: from.trim().to_string(),
            span: self.span(node),
        }
    }

    fn fn_decl(&mut self, node: tree_sitter::Node<'a>, is_method: bool) -> Core {
        let name = node.child_by_field_name("name")
            .map(|n| text(n, self.src).to_string())
            .unwrap_or("<fn>".into());
        let params = node
            .child_by_field_name("parameters")
            .map(|p| self.param_pairs(p))
            .unwrap_or_default();
        let ret = node
            .child_by_field_name("return_type")
            .and_then(|n| self.ret_text(n))
            .filter(|t| !t.is_empty());
        let mut modifiers: Vec<String> = vec![];
        if is_async(node) {
            modifiers.push("async".into());
        }
        let body = node
            .child_by_field_name("body")
            .map(|b| self.body_core(b))
            .unwrap_or_else(|| {
                // signature-only (interface/method_signature): empty inline
                // (Body::Bare([]) canon prints `{`+newline+`}` — unstable)
                Body::Inline(vec![])
            });
        let _ = is_method;
        Core::Fn(FnDecl {
            name,
            receiver: None,
            params,
            ret,
            modifiers,
            base: None,
            body,
            span: self.span(node),
        })
    }

    /// fn return type surface → signature text (fn type params dropped:
    /// the expander skips `<…>` on re-parse, so name stays bare — canon
    /// identity). Type predicates (`x is T`) erase to `Bool` (hand canon
    /// js_01.ir:5).
    fn ret_text(&self, node: tree_sitter::Node) -> Option<String> {
        fn has_predicate(node: tree_sitter::Node<'_>) -> bool {
            let mut c = node.walk();
            'walk: loop {
                if c.node().kind() == "type_predicate" {
                    return true;
                }
                if c.goto_first_child() {
                    continue;
                }
                loop {
                    if c.goto_next_sibling() {
                        continue 'walk;
                    }
                    if !c.goto_parent() {
                        return false;
                    }
                    if c.node() == node {
                        return false;
                    }
                }
            }
        }
        if has_predicate(node) {
            return Some("Bool".into());
        }
        let mut c = node.walk();
        let inner = if c.goto_first_child() {
            loop {
                let n = c.node();
                let empty = n.utf8_text(self.src.as_bytes()).unwrap_or("").trim().is_empty();
                if n.kind() != ":" && !empty {
                    break;
                }
                if !c.goto_next_sibling() {
                    break;
                }
            }
            self.type_text(c.node())
        } else {
            String::new()
        };
        (!inner.is_empty() && inner != "void").then_some(inner)
    }

    /// params: canon `name Type` pairs; types erased to text; `p?` → `Type?`
    fn param_pairs(&mut self, p: tree_sitter::Node) -> Vec<(String, String)> {
        let mut pairs = vec![];
        let mut c = p.walk();
        if c.goto_first_child() {
            loop {
                let n = c.node();
                let mut name = String::new();
                let mut ty = String::new();
                let mut optional = false;
                match n.kind() {
                    "required_parameter" | "optional_parameter" | "formal_parameter" => {
                        optional = n.kind() == "optional_parameter";
                        if let Some(pat) = n.child_by_field_name("pattern") {
                            name = pattern_name(pat, self.src);
                            if pattern_is_destructured(pat, self.src) {
                                // rest/object patterns → v0 opaque params
                                name = format!("$pattern");
                                ty = String::new();
                                pairs.push((name, ty));
                                if !c.goto_next_sibling() {
                                    break;
                                }
                                continue;
                            }
                        }
                        if let Some(ta) = n.child_by_field_name("type") {
                            ty = self.annotation_type(Some(ta));
                        }
                        if optional {
                            ty = format!("{ty}?");
                        }
                        pairs.push((name, ty));
                    }
                    "comment" | "," | "(" | ")" => {}
                    _ => {
                        // shorthand `(x, y)` — identifier params, untyped
                        if n.kind() == "identifier" || n.kind() == "this" {
                            pairs.push((text(n, self.src).to_string(), String::new()));
                        }
                    }
                }
                if !c.goto_next_sibling() {
                    break;
                }
            }
        }
        pairs
    }

    /// fn body → Body (braceless canon: single statement inline, else bare)
    fn body_core(&mut self, block: tree_sitter::Node<'a>) -> Body {
        let mut stmts: Vec<Core> = vec![];
        let mut c = block.walk();
        if c.goto_first_child() {
            loop {
                let n = c.node();
                if n.kind() != "{" && n.kind() != "}" && n.kind() != "comment" {
                    let mut forms = match n.kind() {
                        "lexical_declaration" | "variable_declaration" => self.top_vars(n),
                        "function_declaration" => vec![self.fn_decl(n, false)],
                        _ => vec![self.stmt(n)],
                    };
                    stmts.append(&mut forms);
                }
                if !c.goto_next_sibling() {
                    break;
                }
            }
        }
        if stmts.is_empty() {
            Body::Inline(vec![])
        } else if stmts.len() == 1 {
            // single-statement body — check for a plain expr to fuse inline
            match &stmts[0] {
                Core::Expr(atoms, _) if !atoms.is_empty() => {
                    Body::Inline(atoms.clone())
                }
                _ => Body::Bare(stmts),
            }
        } else {
            Body::Bare(stmts)
        }
    }

    /// module/function-level variable declarations → Core::Var
    fn top_vars(&mut self, node: tree_sitter::Node) -> Vec<Core> {
        // mutability analysis: `let x` re-assigned anywhere → `var mut`
        let mut decls = vec![];
        let mut c = node.walk();
        if c.goto_first_child() {
            loop {
                let n = c.node();
                if n.kind() != "variable_declarator" {
                    if !c.goto_next_sibling() {
                        break;
                    }
                    continue;
                }
                let kw = node.child(0).map(|k| text(k, self.src).to_string()).unwrap_or("const".into());
                let name = n.child_by_field_name("name")
                    .map(|x| pattern_name(x, self.src))
                    .unwrap_or_default();
                let destructured = n
                    .child_by_field_name("name")
                    .map(|x| pattern_is_destructured(x, self.src))
                    .unwrap_or(false);
                let ty = n
                    .child_by_field_name("type")
                    .map(|t| self.annotation_type(Some(t)))
                    .filter(|t| !t.is_empty());
                let value = n
                    .child_by_field_name("value")
                    .map(|v| self.atomize(v));
                let mut modifiers: Vec<String> = vec![];
                if kw == "let" {
                    // re-assignment anywhere in the module → `var mut` —
                    // over-labeling is compile-safe (emits let, never const)
                    modifiers.push("mut".into());
                }
                if destructured {
                    // v0: opaque capture (pattern binding)
                    decls.push(Core::Expr(
                        vec![Atom::Str(format!(
                            "opaque:{}", text(node, self.src).trim()
                        ))],
                        self.span(node),
                    ));
                    if !c.goto_next_sibling() {
                        break;
                    }
                    continue;
                }
                decls.push(Core::Var(VarDecl {
                    name,
                    ty,
                    value,
                    modifiers,
                    objlit: None,
                    span: self.span(n),
                }));
                if !c.goto_next_sibling() {
                    break;
                }
            }
        }
        decls
    }

    /// statement → Core::Stmt / Expr / etc. (module & fn-body level)
    fn stmt(&mut self, node: tree_sitter::Node<'a>) -> Core {
        match node.kind() {
            "if_statement" => {
                let mut head: Vec<Atom> = vec![Atom::Ident("if".into())];
                if let Some(cond) = node.child_by_field_name("condition") {
                    if let Some(inner) = cond.child(1) {
                        head.extend(self.atomize(inner));
                    }
                }
                let body = node
                    .child_by_field_name("consequence")
                    .map(|b| self.block_body(b))
                    .unwrap_or(Body::Bare(vec![]));
                let mut arms = vec![];
                if let Some(alt) = node.child_by_field_name("alternative") {
                    if alt.kind() == "else_clause" {
                        let body_n = alt.child(1).unwrap_or(alt);
                        let arm_pattern = match body_n.kind() {
                            "if_statement" => {
                                // else-if arm: pattern `else if <cond>`
                                let mut p: Vec<Atom> = vec![Atom::Ident("else".into()), Atom::Ident("if".into())];
                                if let Some(c) = body_n.child_by_field_name("condition") {
                                    if let Some(inner) = c.child(1) {
                                        p.extend(self.atomize(inner));
                                    }
                                }
                               Arm { pattern: p, body: body_n.child_by_field_name("consequence").map(|b| self.block_body(b)).unwrap_or(Body::Bare(vec![])), span: self.span(alt) }
                            }
                            _ => Arm {
                                pattern: vec![Atom::Ident("else".into())],
                                body: self.block_body(body_n),
                                span: self.span(alt),
                            },
                        };
                        arms.push(arm_pattern);
                    }
                }
                Core::Stmt { head, body, arms, span: self.span(node) }
            }
            "while_statement" => {
                let mut head: Vec<Atom> = vec![Atom::Ident("while".into())];
                if let Some(cond) = node.child_by_field_name("condition") {
                    if let Some(inner) = cond.child(1) {
                        head.extend(self.atomize(inner));
                    }
                }
                let body = node
                    .child_by_field_name("body")
                    .map(|b| self.block_body(b))
                    .unwrap_or(Body::Bare(vec![]));
                Core::Stmt { head, body, arms: vec![], span: self.span(node) }
            }
            "for_in_statement" => {
                // `for (const x of xs)` → `for x in xs`
                let mut head: Vec<Atom> = vec![Atom::Ident("for".into())];
                if let Some(l) = node.child_by_field_name("left") {
                    head.push(Atom::Ident(pattern_name(l.child(1).unwrap_or(l), self.src)));
                }
                head.push(Atom::Ident("in".into()));
                if let Some(r) = node.child_by_field_name("right") {
                    head.extend(self.atomize(r));
                }
                let body = node
                    .child_by_field_name("body")
                    .map(|b| self.block_body(b))
                    .unwrap_or(Body::Bare(vec![]));
                Core::Stmt { head, body, arms: vec![], span: self.span(node) }
            }
            "for_statement" => {
                // C-style: prefer the corpus canon (range loop) when the
                // init/cond/step shape matches `let i = 0; i < N; i++`;
                // else C-style head verbatim (js-target §4 additive row)
        let init = node.child_by_field_name("initializer").or_else(|| node.child_by_field_name("init"));
        let cond = node.child_by_field_name("condition");
        let step = node.child_by_field_name("increment")
            .or_else(|| node.child_by_field_name("update"));
                let range = self.cstyle_as_range(init, cond, step);
                let head = match range {
                    Some((var, hi)) => {
                        vec![
                            Atom::Ident("for".into()),
                            Atom::Ident(var),
                            Atom::Ident("in".into()),
                            Atom::Number("0".into()),
                            Atom::Sym("..".into()),
                        ]
                        .into_iter()
                        .chain(self.atomize(hi))
                        .collect::<Vec<Atom>>()
                    }
                    None => {
                        // verbatim head atoms: `for (init; cond; step)`
                        let mut head: Vec<Atom> = vec![Atom::Ident("for".into())];
                        let mut c = node.walk();
                        if c.goto_first_child() {
                            loop {
                                let n = c.node();
                                match n.kind() {
                                    // `for` keyword child: head already
                                    // starts with the Ident — don't repeat
                                    "(" | ")" | ";" => head.push(Atom::Sym(text(n, self.src).to_string())),
                                    // NOTE: expression_statement children are
                                    // ALWAYS the loop body (update/increment
                                    // position is `increment`) — never head
                                    "variable_declaration" | "lexical_declaration"
                                    | "empty_statement"
                                    | "assignment_expression" | "binary_expression"
                                    | "augmented_assignment_expression"
                                    | "update_expression" => head.extend(self.stmt_atoms(n)),
                                    _ => {}
                                }
                                if !c.goto_next_sibling() {
                                    break;
                                }
                            }
                        }
                        head
                    }
                };
                let body = node
                    .child_by_field_name("body")
                    .map(|b| self.block_body(b))
                    .unwrap_or(Body::Bare(vec![]));
                Core::Stmt { head, body, arms: vec![], span: self.span(node) }
            }
            "return_statement" | "throw_statement" => {
                let kw = if node.kind() == "return_statement" { "return" } else { "throw" };
                let mut head: Vec<Atom> = vec![Atom::Ident(kw.into())];
                // value = any non-keyword, non-`;` child
                let mut c = node.walk();
                if c.goto_first_child() {
                    loop {
                        let n = c.node();
                        if !matches!(n.kind(), ";" | "comment") && n.kind() != kw {
                            head.extend(self.atomize(n));
                        }
                        if !c.goto_next_sibling() {
                            break;
                        }
                    }
                }
                Core::Stmt { head, body: Body::Inline(vec![]), arms: vec![], span: self.span(node) }
            }
            "expression_statement" => {
                let inner = node.child(0).unwrap_or(node);
                Core::Expr(self.atomize(inner), self.span(node))
            }
            _ => Core::Expr(
                vec![Atom::Str(format!("opaque:{}", text(node, self.src).trim()))],
                self.span(node),
            ),
        }
    }

    /// `for (let i = 0; i < arr.length; i++)` → `(i, arr.length)` range canon
    fn cstyle_as_range(
        &mut self,
        init: Option<tree_sitter::Node<'a>>,
        cond: Option<tree_sitter::Node<'a>>,
        step: Option<tree_sitter::Node<'a>>,
    ) -> Option<(String, tree_sitter::Node<'a>)> {
        let init = init?;
        // init: `let i = 0` → variable_declarator with number 0
        let declr = find_kind(init, "variable_declarator")?;
        let name = declr.child_by_field_name("name")?;
        if pattern_is_destructured(name, self.src) {
            return None;
        }
        let val = declr.child_by_field_name("value")?;
        if text(val, self.src).trim() != "0" {
            return None;
        }
        let cond = cond?;
        // cond: `i < N`
        let l = cond.child_by_field_name("left")?;
        let r = cond.child_by_field_name("right")?;
        if text(l, self.src).trim() != text(name, self.src) {
            return None;
        }
        if text(cond.child_by_field_name("operator")?, self.src) != "<" {
            return None;
        }
        // step: `i++` (postfix on the same var)
        let step = step?;
        if !matches!(step.kind(), "update_expression") {
            return None;
        }
        let arg = step.child_by_field_name("argument")?;
        if text(arg, self.src).trim() != text(name, self.src) {
            return None;
        }
        Some((text(name, self.src).to_string(), r))
    }

    /// statement-block body → Body (fused single-statement, else Bare)
    fn block_body(&mut self, node: tree_sitter::Node<'a>) -> Body {
        if node.kind() != "statement_block" {
            // bare single statement (braceless): expression-like bodies fuse
            // inline onto the head canon (`for i in 0..len{fn(a, i)}` — hand
            // canon js_01.ir:20; a standalone `fn (...)` call LINE would
            // re-parse as a fn declaration)
            return match node.kind() {
                "lexical_declaration" | "variable_declaration" => {
                    let vs = self.top_vars(node);
                    Body::Bare(vs)
                }
                _ => Body::Inline(self.stmt_atoms(node)),
            };
        }
        self.body_core(node)
    }

    // ------------------------------------------------------------------
    // class / interface / alias declarations

    fn class_decl(&mut self, node: tree_sitter::Node<'a>) -> Core {
        let name = node.child_by_field_name("name")
            .map(|n| text(n, self.src).to_string())
            .unwrap_or_default();
        let mut modifiers: Vec<String> = vec![];
        if is_async(node) {
            modifiers.push("async".into());
        }
        // heritage: `class A extends B` — but NOT typed heritage clauses
        let mut bases = vec![];
        if let Some(h) = node.child_by_field_name("heritage") {
            for ch in children(h) {
                if ch.kind() == "extends" {
                    continue;
                }
                if ch.kind() == "identifier" || ch.kind() == "type_identifier" {
                    bases.push(text(ch, self.src).to_string());
                }
            }
        }
        let mut members = vec![];
        if let Some(body) = node.child_by_field_name("body") {
            let mut c = body.walk();
            if c.goto_first_child() {
                loop {
                    let n = c.node();
                    let mut forms = match n.kind() {
                        "method_definition" => vec![self.fn_decl(n, true)],
                        "property_definition" | "public_field_definition" => self.member_field(n),
                        "abstract_method_signature" | "method_signature" => {
                            vec![self.fn_decl(n, true)]
                        }
                        _ => vec![],
                    };
                    members.append(&mut forms);
                    if !c.goto_next_sibling() {
                        break;
                    }
                }
            }
        }
        Core::Type(TypeDecl {
            name,
            kind: "class".into(),
            primary: None,
            bases,
            constraints: vec![],
            modifiers,
            members,
            span: self.span(node),
        })
    }

    fn member_field(&mut self, node: tree_sitter::Node) -> Vec<Core> {
        let name_raw = node.child_by_field_name("name")
            .map(|n| text(n, self.src).to_string())
            .unwrap_or_default();
        let span = self.span(node);
        // private member (`#p`): the IR name model lexes no `#` — capture
        // the whole member opaque (§8.1.3c; bounded loss, never wrong)
        if name_raw.starts_with('#') && node.parent().map(|p| p.kind() == "class_body").unwrap_or(false) {
            return vec![Core::Expr(
                vec![Atom::Str(format!("opaque:{}", text(node, self.src).trim()))],
                span,
            )];
        }
        let name = name_raw;
        let ty_opt = self.annotation_type(node.child_by_field_name("type"));
        let has_optional = node.child(0).map(|c| c.kind() == "?").unwrap_or(false);
        let ty = if ty_opt.is_empty() {
            None
        } else {
            Some(if has_optional { format!("{ty_opt}?") } else { ty_opt })
        };
        let is_property_signature = node.kind() == "property_signature";
        let value = node
            .child_by_field_name("value")
            .filter(|_| !is_property_signature)
            .filter(|v| !matches!(v.kind(), "arrow_function" | "function_expression" | "function"))
            .map(|v| self.atomize(v));
        let mut modifiers: Vec<String> = vec![];
        if is_async(node) {
            modifiers.push("async".into());
        }
        let _ = &modifiers;
        match value {
            Some(atoms) if !atoms.is_empty() => {
                vec![Core::Var(VarDecl {
                    name,
                    ty,
                    value: Some(atoms),
                    modifiers,
                    objlit: None,
                    span,
                })]
            }
            None if is_property_signature => {
                // property signature: type half only (hand canon js_08.ir:5
                // `Bool? xmlMode`), never opaque — signature surface §8.2
                vec![Core::Var(VarDecl {
                    name,
                    ty,
                    value: None,
                    modifiers: vec![],
                    objlit: None,
                    span,
                })]
            }
            _ => vec![Core::Expr(
                vec![Atom::Str(format!(
                    "opaque:{}",
                    text(node, self.src).trim()
                ))],
                span,
            )],
        }
    }

    fn interface_decl(&mut self, node: tree_sitter::Node<'a>) -> Core {
        let name = node.child_by_field_name("name")
            .map(|n| text(n, self.src).to_string())
            .unwrap_or_default();
        // `interface I extends A, B` → bases
        let mut bases = vec![];
        if let Some(h) = node.child_by_field_name("heritage") {
            for ch in children(h) {
                if ch.kind() == "extends" {
                    continue;
                }
                if matches!(ch.kind(), "extends_type_clause" | "type_identifier" | "generic_type") {
                    let t = self.type_text(ch);
                    bases.push(t);
                }
            }
        }
        let mut members = vec![];
        if let Some(body) = node.child_by_field_name("body") {
            let mut c = body.walk();
            if c.goto_first_child() {
                loop {
                    let n = c.node();
                    let mut forms = match n.kind() {
                        "property_signature" => self.member_field(n),
                        "method_signature" | "function_signature" => vec![self.fn_decl(n, true)],
                        _ => vec![],
                    };
                    members.append(&mut forms);
                    if !c.goto_next_sibling() {
                        break;
                    }
                }
            }
        }
        Core::Type(TypeDecl {
            name,
            kind: "interface".into(),
            primary: None,
            bases,
            constraints: vec![],
            modifiers: vec![],
            members,
            span: self.span(node),
        })
    }

    /// `type X = <shape>` — interface-shaped → `type X interface{}`;
    /// union/template machinery → `opaque type X = …` (verbatim canon row)
    fn alias_decl(&mut self, node: tree_sitter::Node<'a>) -> Core {
        let name = node.child_by_field_name("name")
            .map(|n| text(n, self.src).to_string())
            .unwrap_or_default();
        let value = node.child_by_field_name("value");
        let interface_shaped = value.map(|v| matches!(v.kind(), "object_type" | "object")).unwrap_or(false);
        if interface_shaped {
            let mut members = vec![];
            if let Some(v) = value {
                let mut c = v.walk();
                if c.goto_first_child() {
                    loop {
                        let n = c.node();
                        let mut forms = match n.kind() {
                            "property_signature" => self.member_field(n),
                            "method_signature" | "function_signature" => vec![self.fn_decl(n, true)],
                            _ => vec![],
                        };
                        members.append(&mut forms);
                        if !c.goto_next_sibling() {
                            break;
                        }
                    }
                }
            }
            return Core::Type(TypeDecl {
                name,
                kind: "interface".into(),
                primary: None,
                bases: vec![],
                constraints: vec![],
                modifiers: vec![],
                members,
                span: self.span(node),
            });
        }
        // opaque type: canon atom run `opaque type Name = <atoms>`
        let mut atoms: Vec<Atom> = vec![
            Atom::Ident("opaque".into()),
            Atom::Ident("type".into()),
            Atom::Ident(name.clone()),
            Atom::Sym("=".into()),
        ];
        if let Some(v) = value {
            atoms.extend(self.alias_body_atoms(v));
        }
        Core::Expr(atoms, self.span(node))
    }

    /// alias body atoms: unions → ` | `, literals → strings, template types
    /// → their payload text, generics tight
    fn alias_body_atoms(&self, node: tree_sitter::Node) -> Vec<Atom> {
        match node.kind() {
            "union_type" => {
                let mut v = vec![];
                let mut c = node.walk();
                if c.goto_first_child() {
                    loop {
                        let n = c.node();
                        if n.kind() == "|" {
                            v.push(Atom::Sym("|".into()));
                        } else if n.kind() != "comment" {
                            v.extend(self.alias_body_atoms(n));
                        }
                        if !c.goto_next_sibling() {
                            break;
                        }
                    }
                }
                v
            }
            "generic_type" | "type_identifier" | "predefined_type" | "identifier"
            | "array_type" | "lookup_type" | "object_type" => {
                // signature-surface leaves spell through type_text (tight
                // unions; Array<T>; lookup → member; object_type → empty)
                let t = self.type_text(node);
                if t.is_empty() { vec![] } else { vec![Atom::Ident(t)] }
            }
            "type_arguments" => {
                let mut v = vec![Atom::Sym("<".into())];
                let mut c = node.walk();
                let mut first = true;
                if c.goto_first_child() {
                    loop {
                        let n = c.node();
                        if matches!(n.kind(), "type_arguments") && false {
                            // nested — recurse
                        }
                        if n.kind() == "," && first {
                            v.push(Atom::Sym(",".into()));
                        }
                        if !matches!(n.kind(), "<" | ">" | ",") {
                            if !first {
                                v.push(Atom::Sym(",".into()));
                                first = true;
                            }
                            v.extend(self.alias_body_atoms(n));
                        }
                        if !c.goto_next_sibling() {
                            break;
                        }
                    }
                }
                v.push(Atom::Sym(">".into()));
                v
            }
            "parenthesized_type" => node
                .child(1)
                .map(|n| self.alias_body_atoms(n))
                .unwrap_or_default(),
            "template_literal_type" | "template_type" => {
                let mut v = vec![Atom::Str(text(node, self.src).to_string())];
                v
            }
            "literal_type" => {
                let inner = node.child(0).unwrap_or(node);
                match inner.kind() {
                    "string" => vec![Atom::Str(self.str_payload(inner))],
                    "number" => vec![Atom::Number(text(inner, self.src).to_string())],
                    "true" | "false" | "null" => vec![Atom::Ident(text(inner, self.src).to_string())],
                    "undefined" | "void" | "unknown" => vec![Atom::Ident(text(inner, self.src).to_string())],
                    _ => vec![Atom::Ident(text(inner, self.src).to_string())],
                }
            }
            "string" => vec![Atom::Str(self.str_payload(node))],
            "number" => vec![Atom::Number(text(node, self.src).to_string())],
            // `as const` assertion: the type operand is a bare `const` leaf
            "true" | "false" | "null" | "undefined" | "void" | "unknown" | "const" => {
                vec![Atom::Ident(text(node, self.src).to_string())]
            }
            _ => vec![Atom::Str(format!(
                "opaque:{}", text(node, self.src).trim()
            ))],
        }
    }

}

// ---------------------------------------------------------------------------
// helpers

fn is_async(node: tree_sitter::Node) -> bool {
    node.child(0).map(|c| c.kind() == "async").unwrap_or(false)
}

fn children<'t, 'a>(node: tree_sitter::Node<'t>) -> impl Iterator<Item = tree_sitter::Node<'t>> {
    let mut c = node.walk();
    std::iter::from_fn(move || {
        if c.node() == node {
            if !c.goto_first_child() {
                return None;
            }
        } else if !c.goto_next_sibling() {
            return None;
        }
        Some(c.node())
    })
}

/// brace payloads inside ty strings (`readonly {a: T; b: U}[]`) carry raw
/// source spacing no atom rebuild (`parse_params` `spaced`) can reproduce —
/// ty text canonizes to the spaced spelling so print→print identity holds
/// from the first print (fresh-project canon gate). Non-brace ty texts are
/// already spaced-stable (`readonly Staff[]`, `T?`, `A|B`, generics).
fn canon_ty_text(s: &str) -> String {
    if !s.contains('{') {
        return s.to_string();
    }
    match crate::lexer::lex(s) {
        Ok(l) => {
            let atoms: Vec<crate::ast::Atom> = l
                .tokens
                .into_iter()
                .filter(|(t, _, _)| !matches!(t, crate::lexer::Tok::Eof))
                .map(|(t, _, _)| match t {
                    crate::lexer::Tok::Ident(x) => crate::ast::Atom::Ident(x),
                    crate::lexer::Tok::Number(x) => crate::ast::Atom::Number(x),
                    crate::lexer::Tok::Sym(x) => crate::ast::Atom::Sym(x),
                    other => crate::ast::Atom::Ident(format!("{other:?}")),
                })
                .collect();
            crate::expander::spaced(&atoms)
        }
        Err(_) => s.to_string(),
    }
}

fn pattern_name<'a>(node: tree_sitter::Node<'a>, src: &'a str) -> String {
    match node.kind() {
        "identifier" | "shorthand_property_identifier" | "property_identifier"
        | "this" | "rest_pattern" | "object_pattern" | "array_pattern" => {
            let t = text(node, src);
            if node.kind() == "rest_pattern" {
                node.child(1).map(|n| text(n, src).to_string()).unwrap_or_else(|| t.to_string())
            } else if node.kind() == "array_pattern" {
                "(array pattern)".into()
            } else {
                t.to_string()
            }
        }
        _ => node
            .child(0)
            .map(|n| pattern_name(n, src))
            .unwrap_or_default(),
    }
}

fn pattern_is_destructured(node: tree_sitter::Node, _src: &str) -> bool {
    matches!(node.kind(), "object_pattern" | "array_pattern")
}


/// depth-first search for the first node of `kind` in `node`'s subtree
fn find_kind<'t>(
    node: tree_sitter::Node<'t>,
    kind: &str,
) -> Option<tree_sitter::Node<'t>> {
    let mut c = node.walk();
    'walk: loop {
        if c.node().kind() == kind {
            return Some(c.node());
        }
        if cursor_down(&mut c) {
            continue;
        }
        loop {
            if c.goto_next_sibling() {
                continue 'walk;
            }
            if !c.goto_parent() || c.node() == node {
                return None;
            }
        }
    }
}

/// descend into first child if any
fn cursor_down(c: &mut tree_sitter::TreeCursor) -> bool {
    c.goto_first_child()
}

#[cfg(test)]
mod roundtrip_tests {
    use super::*;
    use crate::printer;

    /// canon identity: JS → core → canon text → IR reader → expand → print
    /// == canon. This is the 4.4 gate — the canon surface the reader emits
    /// must be re-stable under the Phase-1 reader/printer pair.
    fn canon_identity(src: &str, label: &str) {
        let core = read_js(src).unwrap_or_else(|e| panic!("{label}: read_js failed: {e}"));
        let canon = printer::print(&core);
        let core2 = crate::expander::expand(&canon)
            .unwrap_or_else(|e| panic!("{label}: canon failed to re-parse:\n{e}\n--- canon ---\n{canon}"));
        let canon2 = printer::print(&core2);
        assert_eq!(canon, canon2, "{label}: canon NOT stable across IR re-parse");
    }

    #[test]
    fn js_01_util_canon_identity() {
        canon_identity(include_str!("../../../bench/corpus/real/src/js_01_util.ts"), "js_01");
    }

    #[test]
    fn js_02_types_canon_identity() {
        canon_identity(include_str!("../../../bench/corpus/real/src/js_02_types.ts"), "js_02");
    }

    #[test]
    fn js_03_parse_canon_identity() {
        canon_identity(include_str!("../../../bench/corpus/real/src/js_03_parse.ts"), "js_03");
    }

    #[test]
    fn js_04_forms_canon_identity() {
        canon_identity(include_str!("../../../bench/corpus/real/src/js_04_forms.ts"), "js_04");
    }

    #[test]
    fn js_05_css_canon_identity() {
        canon_identity(include_str!("../../../bench/corpus/real/src/js_05_css.ts"), "js_05");
    }

    #[test]
    fn js_06_slim_canon_identity() {
        canon_identity(include_str!("../../../bench/corpus/real/src/js_06_slim.ts"), "js_06");
    }

    #[test]
    fn js_08_options_canon_identity() {
        canon_identity(include_str!("../../../bench/corpus/real/src/js_08_options.ts"), "js_08");
    }

    #[test]
    fn reader_construct_smoke() {
        // the §8.2 mapping rows the corpus exercises, one per canon shape
        canon_identity("const x = 1;", "var");
        canon_identity("let x = 1; x = 2;", "var-mut");
        canon_identity("const {a, b} = obj;", "destructured-object");
        canon_identity("function f(a, b=1, ...rest){ return a; }", "fn-defaults");
        canon_identity(
            "class A extends B { static x = 1; #p = 2; m(){ return this.#p; } get g(){ return 1 } }",
            "class",
        );
        canon_identity("interface I { a?: string; b: number; m(x: number): void }", "interface");
        canon_identity("type T = { a: string; b?: number };", "type-object");
        canon_identity("type U = 'a' | 'b';", "opaque-union");
        canon_identity("if (a) { b(); } else if (c) { d(); } else { e(); }", "if-elseif-else");
        canon_identity("while (a) { b(); }", "while");
        canon_identity("for (const k in obj){ f(obj[k]); }", "for-in");
        canon_identity("for (let i = 0; i < n; i++) { f(i); }", "for-range");
        canon_identity("for (let x = xs.length - 1; x >= 0; x--) { g(x); }", "for-cstyle");
        canon_identity("return;", "bare-return");
        canon_identity("throw new Error('oops');", "throw-new");
        canon_identity("try { a(); } catch (e) { b(e); } finally { c(); }", "try-catch");
        canon_identity("switch (x) { case 1: a(); break; default: b(); }", "switch");
        canon_identity("const f = (a) => a * 2;", "arrow");
        canon_identity("const g = (a) => { const b = a + 1; return b; };", "arrow-block");
        canon_identity("`t ${v}`;", "template");
        canon_identity("new Foo(a, b);", "new-expr");
        canon_identity("x?.y?.z ?? d;", "optional-chain");
        canon_identity("a ? b : c;", "ternary");
        canon_identity("typeof x === 'string';", "typeof");
        canon_identity("delete m[k];", "delete");
        canon_identity("`[a-z]` && /[a-z]+/g;", "regex");
        canon_identity("import { a, b } from './mod.js'; f(a);", "import-named");
        canon_identity("import * as M from './mod.js'; M.f();", "import-star");
        canon_identity("export { a, b } from './mod.js';", "reexport");
        canon_identity("export * from './mod.js';", "reexport-star");
    }
}
