//! PLAN 5.6 — C# reader (round-trip leg): tree-sitter-c-sharp CST → Core.
//!
//! Mirrors `js_read.rs`. Mapping per cs-target.md §1/§6 + the hand canon
//! (ground truth): type words canonicalize (`string`→`String`…), comments
//! and `using` directives drop by contract, `where` constraints drop, XML
//! docs are comments, `public`→`export`, `readonly` drops (immutability is
//! canon default), `const`/`volatile` keep, statics drop (static class is
//! implied by receiver fns), nullable `T?` rides the type, interpolation →
//! canon `"x{e}y"`, lambda/default-param shapes per expander conventions.

use tree_sitter::{Language, Parser};

use crate::ast::{Atom, Span};
use crate::expander::{Arm, Body, Core, FnDecl, TypeDecl, VarDecl, SPAN_SYNTH};

pub fn cs_language() -> Language {
    tree_sitter_c_sharp::LANGUAGE.into()
}

fn parser() -> Parser {
    let mut p = Parser::new();
    p.set_language(&cs_language()).expect("grammar load");
    p
}

/// C# source → core forms; None tree on hard failure.
pub fn read_cs(src: &str) -> Result<Vec<Core>, CsReadError> {
    let tree = parser()
        .parse(src, None)
        .ok_or(CsReadError { msg: "parse failed".into() })?;
    let mut r = CsReader { src, uses: Vec::new(), ns: None };
    let mut out = vec![];
    let mut c = tree.root_node().walk();
    let has_children = c.goto_first_child();
    if has_children {
        loop {
            let child = c.node();
            if child.kind() != "comment" {
                let forms = r.decl(child);
                out.extend(forms);
            }
            if !c.goto_next_sibling() {
                break;
            }
        }
    }
    // mut-overlabel post-pass: canon keeps only export/const/volatile —
    // fields whose names are ASSIGNED anywhere in the file are mutable for
    // the C# emit (readonly + assignment = CS0191); `mut` rides the Var
    // row (canon row `… var mut X` re-parses; the JS reader does the same
    // over-label for let-declared locals)
    let assigned = collect_assigned(&out);
    overlabel_fields(&mut out, &assigned);
    out.insert(0, r.header_core());
    Ok(out)
}

/// assignment-target names anywhere in the core tree
fn collect_assigned(core: &[Core]) -> Vec<String> {
    let mut out = vec![];
    for c in core {
        assigned_in_core(c, &mut out);
    }
    out.dedup();
    out
}

fn assigned_in_core(c: &Core, out: &mut Vec<String>) {
    let atoms: Vec<Atom> = match c {
        Core::Expr(atoms, _) => atoms.clone(),
        Core::Stmt { head, body, arms, .. } => {
            let mut hs = head.clone();
            match body {
                Body::Bare(v) | Body::Block(v) => {
                    for st in v { assigned_in_core(st, out); }
                }
                Body::Inline(a) => hs.extend(a.clone()),
                Body::Opaque(_) => {}
            }
            for a in arms {
                if let Body::Bare(v) = &a.body {
                    for st in v { assigned_in_core(st, out); }
                }
            }
            assigned_in_core(&Core::Expr(hs, crate::expander::SPAN_SYNTH), out);
            return;
        }
        Core::Var(v) => match &v.value { Some(a) => a.clone(), None => vec![] },
        Core::Fn(f) => { assigned_in_core(&Core::Expr(Vec::new(), crate::expander::SPAN_SYNTH), out); match &f.body { Body::Bare(v) | Body::Block(v) => { for st in v { assigned_in_core(st, out); } }, Body::Inline(a) => { return assigned_in_atoms_in_place(a, out); }, _ => {} }; return; }
        Core::Type(t) => {
            for m in &t.members { assigned_in_core(m, out); }
            return;
        }
        _ => return,
    };
    assigned_in_atoms_in_place(&atoms, out);
}

fn assigned_in_atoms_in_place(atoms: &[Atom], out: &mut Vec<String>) {
    for (i, a) in atoms.iter().enumerate() {
        if let Atom::Ident(name) = a {
            if let Some(op) = atoms.get(i + 1) {
                if matches!(op, Atom::Sym(sop) if matches!(sop.as_str(), "=" | "+=" | "-=" | "*=" | "/=" | "%=" | "??=")) {
                    out.push(name.clone());
                }
            }
            // assignments at run-start or after structural prefixes
            let _ = i;
        }
    }
}

fn overlabel_fields(core: &mut [Core], assigned: &[String]) {
    for c in core.iter_mut() {
        match c {
            Core::Var(v) => {
                if !v.modifiers.iter().any(|m| m == "mut")
                    && assigned.iter().any(|x| x == &v.name)
                {
                    v.modifiers.push("mut".into());
                }
            }
            Core::Type(t) => overlabel_fields(&mut t.members, assigned),
            Core::Fn(f) => {
                if let Body::Bare(v) | Body::Block(v) = &mut f.body {
                    overlabel_fields(v, assigned);
                }
            }
            Core::Stmt { body, arms, .. } => {
                if let Body::Bare(v) | Body::Block(v) = body {
                    overlabel_fields(v, assigned);
                }
                for a in arms {
                    if let Body::Bare(v) | Body::Block(v) = &mut a.body {
                        overlabel_fields(v, assigned);
                    }
                }
            }
            _ => {}
        }
    }
}

pub struct CsReadError {
    pub msg: String,
}

impl std::fmt::Display for CsReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.msg)
    }
}

impl std::fmt::Debug for CsReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CsReadError").field("msg", &self.msg).finish()
    }
}

fn text<'a>(node: tree_sitter::Node<'a>, src: &'a str) -> &'a str {
    node.utf8_text(src.as_bytes()).unwrap_or("")
}

struct CsReader<'a> {
    src: &'a str,
    uses: Vec<String>,
    ns: Option<String>,
}

impl<'a> CsReader<'a> {
    fn header_core(&self) -> Core {
        Core::Header {
            version: "1".into(),
            ns: self.ns.clone(),
            uses: self.uses.clone(),
            span: SPAN_SYNTH,
        }
    }

    fn span(&self, node: tree_sitter::Node) -> Span {
        Span::point(node.start_position().row + 1)
    }

    // ------------------------------------------------------------------
    // types (§6 word map; lowercase C# → canon capital)

    fn type_text(&self, node: tree_sitter::Node) -> String {
        match node.kind() {
            "identifier" | "predefined_type" | "generic_name" | "nullable_type" => {
                self.type_words(node)
            }
            "array_type" => {
                let el = node
                    .child(0)
                    .map(|n| self.type_text(n))
                    .unwrap_or_default();
                if el.is_empty() { String::new() } else { format!("Array<{el}>") }
            }
            _ => self.type_words(node),
        }
    }

    /// canonicalize the type TEXT of a node: identifiers/keywords via the word
    /// map, generics tight, `?` riding the trailing ident.
    fn type_words(&self, node: tree_sitter::Node) -> String {
        match node.kind() {
            "nullable_type" => {
                let inner = node.child(0).map(|n| self.type_words(n)).unwrap_or_default();
                format!("{inner}?")
            }
            "generic_name" => {
                let name = node.child(0).map(|n| self.type_words(n)).unwrap_or_default();
                let args = node
                    .child(1)
                    .map(|a| {
                        let mut v = vec![];
                        let mut c = a.walk();
                        if c.goto_first_child() {
                            loop {
                                let n = c.node();
                                if !matches!(n.kind(), "<" | ">" | "," | "comma") {
                                    let t = self.type_text(n);
                                    if !t.is_empty() {
                                        v.push(t);
                                    }
                                }
                                if !c.goto_next_sibling() {
                                    break;
                                }
                            }
                        }
                        v.join(",")
                    })
                    .unwrap_or_default();
                if args.is_empty() { name } else { format!("{name}<{args}>") }
            }
            "identifier" | "predefined_type" => canon_word(text(node, self.src)),
            _ => text(node, self.src).to_string(),
        }
    }

    // ------------------------------------------------------------------
    // interpolation payload: `x{e}y` (canon braces)

    fn interp_payload(&self, node: tree_sitter::Node) -> String {
        let mut s = String::new();
        let mut c = node.walk();
        if c.goto_first_child() {
            loop {
                let n = c.node();
                match n.kind() {
                    "string_content" | "escape_sequence" => s.push_str(text(n, self.src)),
                    "interpolation" => {
                        s.push('{');
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

    fn str_payload(&self, node: tree_sitter::Node) -> String {
        // content children (string_literal_content); verbatim string child(0)
        // is just the quote char
        let mut t = String::new();
        let mut c = node.walk();
        if c.goto_first_child() {
            loop {
                let n = c.node();
                match n.kind() {
                    "string_literal_content" | "escape_sequence" => t.push_str(text(n, self.src)),
                    _ => {}
                }
                if !c.goto_next_sibling() { break; }
            }
        }
        if t.is_empty() {
            let direct = text(node, self.src);
            t = direct.trim_matches(|c| c == '"' || c == '@').to_string();
        }
        // data-braces in plain literals: canon-escape `{{…}}` (the interp
        // canon `x{e}y` is only produced by $-prefixed interpolated strings —
        // the read distinguishes; the emit restores literals)
        if t.contains('{') {
            t = t.replace('{', "{{").replace('}', "}}");
        }
        t
    }

    // ------------------------------------------------------------------
    // expression → atom run (verbatim-first)

    /// atoms for a bare switch-expression value (subject + `switch` marker);
    /// the arm rows live in the enclosing value run's children (var hijack).
    fn v_switch_expr(&self, node: tree_sitter::Node) -> Vec<Atom> {
        let mut v = vec![];
        if let Some(subj) = node.child(0) {
            v.extend(self.atomize(subj));
        }
        v.push(Atom::Ident("switch".into()));
        v
    }

    fn atomize(&self, node: tree_sitter::Node) -> Vec<Atom> {
        match node.kind() {
            "identifier" => vec![Atom::Ident(text(node, self.src).to_string())],
            "this" | "base" => vec![Atom::Ident(text(node, self.src).to_string())],
            "generic_name" => {
                // single-canon-text generics glue one Ident; MULTI-ARG
                // generics split into atoms — the IR reader re-prints
                // comma-args spaced (render_atoms `,` rule), so the read
                // side must atomize the same way (canon identity gate)
                let txt = self.type_words(node);
                if txt.contains(',') {
                    let mut v: Vec<Atom> = vec![];
                    let mut depth = 0usize;
                    let cur = String::new();
                    let name_ged = txt.split('<').next().unwrap_or_default().to_string();
                    v.push(Atom::Ident(name_ged.clone()));
                    let inner: Vec<String> = {
                        let inner_txt = &txt[txt.find('<').unwrap_or(0) + 1..txt.rfind('>').unwrap_or(txt.len())];
                        inner_txt.split(',').map(|x| x.trim().to_string()).filter(|x| !x.is_empty()).collect()
                    };
                    v.push(Atom::Sym("<".into()));
                    for (k, a) in inner.iter().enumerate() {
                        if k > 0 {
                            v.push(Atom::Sym(",".into()));
                        }
                        v.push(Atom::Ident(a.clone()));
                    }
                    v.push(Atom::Sym(">".into()));
                    let _ = (depth, cur);
                    v
                } else {
                    vec![Atom::Ident(txt)]
                }
            }
            "invocation_expression" => {
                let mut v = vec![];
                if let Some(f) = node.child_by_field_name("function") {
                    v.extend(self.atomize(f));
                }
                v.push(Atom::Sym("(".into()));
                if let Some(a) = node.child_by_field_name("arguments") {
                    v.extend(self.args_atoms(a));
                }
                v.push(Atom::Sym(")".into()));
                v
            }
            "argument_list" => self.args_atoms(node),
            "member_access_expression" => {
                let obj = node.child_by_field_name("expression").map(|n| self.atomize(n)).unwrap_or_default();
                let _ = obj;
                let mut v = vec![];
                if let Some(e) = node.child_by_field_name("expression") {
                    v.extend(self.atomize(e));
                }
                v.push(Atom::Sym(".".into()));
                if let Some(n) = node.child_by_field_name("name") {
                    v.push(Atom::Ident(text(n, self.src).to_string()));
                }
                v
            }
            "element_access_expression" => {
                let mut v = vec![];
                if let Some(e) = node.child_by_field_name("expression") {
                    v.extend(self.atomize(e));
                }
                if let Some(a) = node.child_by_field_name("subscript") {
                    if let Some(inner) = a.child(1) {
                        v.push(Atom::Sym("[".into()));
                        v.extend(self.atomize(inner));
                        v.push(Atom::Sym("]".into()));
                    }
                }
                v
            }
            "binary_expression" | "assignment_expression" | "coalescing_expression"
            | "conditional_expression" | "is_pattern_expression2" => {
                // children-scan: operators are anonymous nodes (`&&`, `??`, `=`)
                let mut v = vec![];
                let mut c = node.walk();
                if c.goto_first_child() {
                    loop {
                        let n = c.node();
                        if n.kind() != "comment" {
                            let k = n.kind();
                            if k.len() == 2 && k.starts_with('&') || k == "||" || k == "??" || k == "=>" {
                                v.push(Atom::Sym(text(n, self.src).to_string()));
                            } else {
                                v.extend(self.atomize(n));
                            }
                        }
                        if !c.goto_next_sibling() { break; }
                    }
                }
                v
            }
            "unary_expression" => {
                let op = node.child(0).map(|n| text(n, self.src).to_string()).unwrap_or_default();
                let mut v = match op.as_str() {
                    "!" | "-" | "+" | "~" => vec![Atom::Sym(op)],
                    "await" => vec![Atom::Ident("await".into())],
                    _ => vec![Atom::Ident(op)],
                };
                if let Some(a) = node.child(1) { v.extend(self.atomize(a)); }
                v
            }
            "postfix_unary_expression" => {
                // `x!` null-forgiving (canon rides the ident — lex-stable);
                // `x++` suffix verbatim (value-position ++ rare in corpus)
                let mut v = node.child(0).map(|n| self.atomize(n)).unwrap_or_default();
                if let Some(op) = node.child(1) {
                    let op_txt = text(op, self.src);
                    if op_txt == "!" {
                        match v.last_mut() {
                            Some(Atom::Ident(last)) => {
                                let owned = format!("{last}!");
                                v.pop();
                                v.push(Atom::Ident(owned));
                            }
                            _ => v.push(Atom::Sym("!".into())),
                        }
                    } else {
                        v.push(Atom::Sym(op_txt.to_string()));
                    }
                }
                v
            }
            "assignment_expression2" => unreachable!(),
            "ternary_expression" => {
                let mut v = vec![];
                if let Some(c) = node.child_by_field_name("condition") { v.extend(self.atomize(c)); }
                v.push(Atom::Sym("?".into()));
                if let Some(t) = node.child_by_field_name("consequence") { v.extend(self.atomize(t)); }
                v.push(Atom::Sym(":".into()));
                if let Some(a) = node.child_by_field_name("alternative") { v.extend(self.atomize(a)); }
                v
            }
            "parenthesized_expression" => {
                let mut v = vec![Atom::Sym("(".into())];
                if let Some(inner) = node.child(1) { v.extend(self.atomize(inner)); }
                v.push(Atom::Sym(")".into()));
                v
            }
            "lambda_expression" | "anonymous_method_expression" => {
                let mut v = vec![];
                // params: the parameter_list child, or an implicit singleton
                // identifier before the `=>`
                let mut params_node: Option<tree_sitter::Node> = None;
                {
                    let mut lc = node.walk();
                    if lc.goto_first_child() {
                        loop {
                            if lc.node().kind() == "=>" {
                                break;
                            }
                            if matches!(lc.node().kind(), "parameter_list" | "identifier" | "parameter" | "implicit_parameter") {
                                params_node = Some(lc.node());
                                break;
                            }
                            if !lc.goto_next_sibling() { break; }
                        }
                    }
                }
                if let Some(p) = params_node {
                    if p.kind() == "parameter_list" {
                        v.extend(self.lambda_params_atoms(p));
                    } else {
                        v.extend(self.atomize(p));
                    }
                }
                v.push(Atom::Sym("=>".into()));
                if let Some(b) = node.child_by_field_name("body") {
                    match b.kind() {
                        "block" => {
                            v.push(Atom::Sym("{".into()));
                            let mut c = b.walk();
                            if c.goto_first_child() {
                                loop {
                                    let n = c.node();
                                    {
                                        if n.kind() != "{" && n.kind() != "}" && n.kind() != "comment" {
                                            v.extend(self.stmt_atoms(n));
                                        }
                                    }
                                    if !c.goto_next_sibling() { break; }
                                }
                            }
                            v.push(Atom::Sym("}".into()));
                        }
                        _ => v.extend(self.atomize(b)),
                    }
                }
                v
            }
            "switch_expression" => {
                // `subject switch {arms}` — Stmt path via trailing-switch
                self.v_switch_expr(node)
            }
            "interpolated_string_expression" => vec![Atom::Str(self.interp_payload(node))],
            "string_literal" => vec![Atom::Str(self.str_payload(node))],
            "character_literal" => vec![Atom::Str(node
                .child_by_field_name("content")
                .or_else(|| node.child_by_field_name("character_literal_content"))
                .map(|n| text(n, self.src).to_string())
                .unwrap_or_else(|| text(node, self.src).trim_matches(|c| c == '\'').replace("\\'", "'")))],
            "true" | "true_literal" | "Bool" => vec![Atom::Ident("true".into())],
            "false" | "false_literal" => vec![Atom::Ident("false".into())],
            "null" | "null_literal" => vec![Atom::Ident("null".into())],
            "object_creation_expression" => {
                let mut v = vec![Atom::Ident("new".into())];
                if let Some(ty) = node.child_by_field_name("type") { v.extend(self.atomize(ty)); }
                if let Some(args) = node.child_by_field_name("arguments") {
                    v.push(Atom::Sym("(".into()));
                    v.extend(self.args_atoms(args));
                    v.push(Atom::Sym(")".into()));
                }
                // `new T { { key, value }, … }` dict-initializer entries —
                // verbatim descent (round-initializer braces are data in
                // canon; the emit re-renders them verbatim)
                if let Some(init) = node.child_by_field_name("initializer") {
                    // verbatim carries its own brace pair (initializer braces)
                    let mut iv = vec![];
                    self.verbatim_atoms(init, &mut iv);
                    v.extend(iv);
                }
                v
            }
            "implicit_object_creation_expression" => {
                // `new(args)` — the argument_list child carries no field name
                let mut v = vec![Atom::Ident("new".into())];
                let mut nc = node.walk();
                if nc.goto_first_child() {
                    loop {
                        if nc.node().kind() == "argument_list" {
                            v.push(Atom::Sym("(".into()));
                            v.extend(self.args_atoms(nc.node()));
                            v.push(Atom::Sym(")".into()));
                            break;
                        }
                        if !nc.goto_next_sibling() { break; }
                    }
                }
                if !v.last().map(|a| matches!(a, Atom::Sym(sp) if sp == ")")).unwrap_or(false) {
                    v.push(Atom::Sym("(".into()));
                    v.push(Atom::Sym(")".into()));
                }
                v
            }
            "argument" => {
                // arg wrapper: name? + value — DESCEND to every child (out
                // args carry [out, var, name]; named args `name: value`)
                let mut v = vec![];
                let mut c = node.walk();
                if c.goto_first_child() {
                    loop {
                        let n = c.node();
                        if n.kind() != ";" && n.kind() != "," {
                            v.extend(self.atomize(n));
                        }
                        if !c.goto_next_sibling() { break; }
                    }
                }
                v
            }
            "is_pattern_expression" => {
                // verbatim: [X, is, [not], pattern atoms…]
                let mut c = node.walk();
                let mut v = vec![];
                if c.goto_first_child() {
                    loop {
                        let n = c.node();
                        match n.kind() {
                            "comment" | "is" | "not" => {
                                if n.kind() == "is" { v.push(Atom::Ident("is".into())); }
                                if n.kind() == "not" { v.push(Atom::Ident("not".into())); }
                            }
                            _ => v.extend(self.pattern_atoms(n)),
                        }
                        if !c.goto_next_sibling() { break; }
                    }
                }
                v
            }
            "cast_expression" => {
                let mut v = vec![Atom::Sym("(".into())];
                if let Some(t) = node.child_by_field_name("type") {
                    v.push(Atom::Ident(self.type_text(t)));
                }
                v.push(Atom::Sym(")".into()));
                // operand field: c-sharp grammar names it `left` here
                let operand = node.child_by_field_name("expression")
                    .or_else(|| node.child_by_field_name("left"))
                    .or_else(|| node.child_by_field_name("value"));
                if let Some(e) = operand {
                    v.extend(self.atomize(e));
                }
                v
            }
            _ => {
                // generic: verbatim via a breadth walk of leaf words/syms
                let mut v = vec![];
                self.verbatim_atoms(node, &mut v);
                if v.is_empty() {
                    vec![Atom::Str(format!("opaque:{}", text(node, self.src).trim()))]
                } else {
                    v
                }
            }
            "collection_expression" => {
                let mut v = vec![Atom::Sym("[".into())];
                let mut c = node.walk();
                let mut items = 0usize;
                if c.goto_first_child() {
                    loop {
                        let n = c.node();
                        if !matches!(n.kind(), "[" | "]") {
                            let part = self.atomize(n);
                            if items > 0 && !v.is_empty() {
                                v.push(Atom::Sym(",".into()));
                            }
                            v.extend(part);
                            items += 1;
                        }
                        if !c.goto_next_sibling() { break; }
                    }
                }
                v.push(Atom::Sym("]".into()));
                v
            }
        }
    }

    /// `X switch { arms }` → Core::Stmt (arms from switch_expression_arm
    /// children; patterns: constant/relational/type/discard). Value-position
    /// switches canon as trailing-switch rows (`X switch{arm…}`) — the
    /// reader keeps arm structure; atomizing loses it (canon is symmetric,
    /// which is why the identity gate missed this until the emit gate).
    fn switch_core(&self, node: tree_sitter::Node, span: Span) -> Core {
        let mut head: Vec<Atom> = vec![];
        if let Some(subj) = node.child(0) {
            head.extend(self.cond_atoms(subj));
        }
        head.push(Atom::Ident("switch".into()));
        let mut arms = vec![];
        let mut c = node.walk();
        if c.goto_first_child() {
            loop {
                let n = c.node();
                if n.kind() == "switch_expression_arm" {
                    let mut pat = vec![];
                    let mut body_atoms: Vec<Atom> = vec![];
                    let mut in_body = false;
                    let mut ac = n.walk();
                    if ac.goto_first_child() {
                        loop {
                            let an = ac.node();
                            if an.kind() == "=>" {
                                in_body = true;
                            } else if !in_body && an.kind() != "comment" {
                                pat.extend(self.pattern_atoms(an));
                            } else if in_body && an.kind() != "comment" {
                                body_atoms.extend(self.atomize(an));
                            }
                            if !ac.goto_next_sibling() { break; }
                        }
                    }
                    arms.push(Arm {
                        pattern: if pattern_empty_else(&pat) { vec![Atom::Sym("else".into())] } else { pat },
                        body: Body::Inline(body_atoms),
                        span: self.span(n),
                    });
                }
                if !c.goto_next_sibling() { break; }
            }
        }
        Core::Stmt { head, body: Body::Inline(vec![]), arms, span }
    }

    /// lambda parameter list → atoms: `(a, b)` (only the first parameter was
    /// taken via child(1) — multi-param lambdas lost the rest; the expander
    /// canon splits at emit)
    fn lambda_params_atoms(&self, params_node: tree_sitter::Node) -> Vec<Atom> {
        let mut v = vec![];
        let mut c = params_node.walk();
        let mut pc = 0usize;
        v.push(Atom::Sym("(".into()));
        if c.goto_first_child() {
            loop {
                let n = c.node();
                if !matches!(n.kind(), "(" | ")" | "," | "comment") {
                    if pc > 0 && !v.is_empty() {
                        v.push(Atom::Sym(",".into()));
                    }
                    if n.kind() == "parameter" {
                        // named params: the name ident
                        if let Some(name) = n.child_by_field_name("name") {
                            v.push(Atom::Ident(text(name, self.src).to_string()));
                        } else {
                            v.extend(self.atomize(n));
                        }
                    } else {
                        v.extend(self.atomize(n));
                    }
                    pc += 1;
                }
                if !c.goto_next_sibling() { break; }
            }
        }
        v.push(Atom::Sym(")".into()));
        v
    }

    /// pattern nodes → atoms (`{}-binder, relational, type patterns, `not`)
    fn pattern_atoms(&self, node: tree_sitter::Node) -> Vec<Atom> {
        match node.kind() {
            "recursive_pattern" | "property_pattern" => {
                // `{ }` / `{ prop: pat, … } [binder]` — verbatim leaf descent
                // (the clause nests `{ }` + named subpattern pairs one level
                // deep; the identifier-dropping arm lost them entirely)
                let mut v = vec![];
                self.verbatim_atoms(node, &mut v);
                v
            }
            "constant_pattern" => node
                .child(0)
                .map(|n| self.atomize(n))
                .unwrap_or_default(),
            "discard" => vec![Atom::Ident("_".into())],
            "relational_pattern" => {
                let mut v = vec![];
                let mut c = node.walk();
                if c.goto_first_child() {
                    loop {
                        let n = c.node();
                        if n.kind() != "comment" {
                            v.extend(self.atomize(n));
                        }
                        if !c.goto_next_sibling() { break; }
                    }
                }
                v
            }
            _ => {
                let mut v = vec![];
                self.verbatim_atoms(node, &mut v);
                v
            }
        }
    }

    /// `cond` may arrive parenthesized or direct depending on the grammar
    fn cond_atoms(&self, node: tree_sitter::Node) -> Vec<Atom> {
        if node.kind() == "parenthesized_expression" {
            if let Some(inner) = node.child(1) {
                return self.atomize(inner);
            }
            return vec![];
        }
        self.atomize(node)
    }

    /// breadth leaf-walk fallback: every terminal word/sym atom
    fn verbatim_atoms(&self, node: tree_sitter::Node, v: &mut Vec<Atom>) {
        // character literals: wrapper nodes with quote-char children — canon
        // is the Str form (the descent into quote-syms produces `'a'` rows
        // that re-lex differently on round 2)
        if node.kind() == "character_literal" {
            let t = text(node, self.src);
            v.push(Atom::Str(t.trim_matches(|c| c == '\'').replace("\\'", "'").to_string()));
            return;
        }
        if node.kind() == "string_literal" {
            // canon as Str (the quote-char leaves would glue `""…""` rows)
            let sub = node.walk();
            let sub_node = sub.node();
            let t = if sub_node.child_count() > 2 {
                let mut acc = String::new();
                let mut cc = node.walk();
                if cc.goto_first_child() {
                    loop {
                        if cc.node().kind() == "string_literal_content" {
                            acc.push_str(text(cc.node(), self.src));
                        }
                        if !cc.goto_next_sibling() { break; }
                    }
                }
                acc
            } else {
                text(node, self.src).trim_matches('"').to_string()
            };
            v.push(Atom::Str(t));
            return;
        }
        let mut c = node.walk();
        if !c.goto_first_child() {
            let k = node.kind();
            let t = text(node, self.src);
            match k {
                "identifier" | "predefined_type" | "var" | "this" | "base"
                | "out" | "in" | "ref" | "await" | "is" | "as" | "and" | "or" | "not"
                | "new" | "lock" | "switch" | "case" | "default" | "throw" | "return"
                | "typeof" | "nameof" | "when" | "params" | "yield" => {
                    v.push(Atom::Ident(t.to_string()));
                }
                // keyword LITERAL texts arrive as Sym rows through various
                // grammar kinds (csharp `null_literal`/args wrappers) — canon
                // needs the Ident (keyword-space glue rules)
                _ if matches!(t, "true" | "false" | "null" | "this" | "base" | "var" | "new") => {
                    v.push(Atom::Ident(t.to_string()));
                }
                "integer_literal" | "real_literal" | "decimal_literal" => {
                    v.push(Atom::Number(t.to_string()));
                }
                "character_literal" => {
                    // canon chars are Str atoms (double-quoted print; the
                    // verbatim Sym would re-lex as a Str on round 2 anyway)
                    eprintln!("CHARARM t={t:?}");
                    v.push(Atom::Str(t.trim_matches(|c| c == '\'').replace("\\'", "'").to_string()));
                }
                ";" => {
                    v.push(Atom::Sym(String::new()));
                }
                _ => {
                    v.push(Atom::Sym(t.to_string()));
                }
            }
            return;
        }
        loop {
            let n = c.node();
            if n.kind() != "comment" {
                self.verbatim_atoms(n, v);
            }
            if !c.goto_next_sibling() {
                break;
            }
        }
        c.goto_parent();
    }

    fn args_atoms(&self, args: tree_sitter::Node) -> Vec<Atom> {
        let mut v = vec![];
        let mut c = args.walk();
        let mut argc = 0usize;
        if c.goto_first_child() {
            loop {
                let n = c.node();
                if !matches!(n.kind(), "(" | ")" | "," | "comment") {
                    let mut part = self.atomize(n);
                    if argc > 0 && !v.is_empty() {
                        v.push(Atom::Sym(",".into()));
                    }
                    if !part.is_empty() {
                        argc += 1;
                        v.append(&mut part);
                    }
                }
                if !c.goto_next_sibling() { break; }
            }
        }
        v
    }

    /// statement → run pieces for lambda blocks
    fn stmt_atoms(&self, node: tree_sitter::Node) -> Vec<Atom> {
        match node.kind() {
            "local_declaration_statement" | "expression_statement" => node
                .child(0)
                .map(|n| self.atomize(n))
                .unwrap_or_default(),
            "return_statement" => {
                let mut v = vec![Atom::Ident("return".into())];
                if let Some(e) = node.child_by_field_name("expression") {
                    v.extend(self.atomize(e));
                }
                v
            }
            "throw_statement" => {
                let mut v = vec![Atom::Ident("throw".into())];
                if let Some(e) = node.child_by_field_name("expression") {
                    v.extend(self.atomize(e));
                }
                v
            }
            _ => self.atomize(node),
        }
    }

    // ------------------------------------------------------------------
    // declarations

    pub fn decl(&mut self, node: tree_sitter::Node<'a>) -> Vec<Core> {
        match node.kind() {
            "file_scoped_namespace_declaration" | "namespace_declaration" => {
                // patch the header (inserted later at [0])
                if let Some(name) = node.child_by_field_name("name") {
                    self.ns = Some(text(name, self.src).to_string());
                }
                vec![]
            }
            "using_directive" => {
                // contract drop: BCL using boilerplate (recorded row)
                vec![]
            }
            "comment" => vec![],
            "class_declaration" | "struct_declaration" | "record_declaration" | "record_struct_declaration" => {
                vec![self.type_decl(node, "class")]
            }
            "interface_declaration" => vec![self.type_decl(node, "interface")],
            "enum_declaration" => {
                vec![Core::Type(TypeDecl {
                    name: node.child_by_field_name("name")
                        .map(|n| {
                            let mut nm = text(n, self.src).to_string();
                            if let Some(tp) = node.child_by_field_name("type_parameters") {
                                nm = format!("{nm}{}", text(tp, self.src).replace(" ", ""));
                            }
                            nm
                        })
                        .unwrap_or_default(),
                    kind: self.kind_word(node),
                    primary: None,
                    bases: vec![],
                    modifiers: self.modifiers_export(node),
                    members: vec![],
                    span: self.span(node),
                })]
            }
            "global_attribute_list" | "attribute_list" => vec![Core::Expr(
                vec![Atom::Str(format!("opaque:{}", text(node, self.src).trim()))],
                self.span(node),
            )],
            _ => {
                // statements at file level (v0: opaque)
                vec![Core::Expr(
                    vec![Atom::Str(format!("opaque:{}", text(node, self.src).trim()))],
                    self.span(node),
                )]
            }
        }
    }

    fn modifiers_export_member(&self, node: tree_sitter::Node) -> Vec<String> {
        // canon member rows carry no export (type-level `export` covers the
        // surface; hand canon cs_04 `fn Humanize(...)` plain)
        vec![]
    }

    fn modifiers_export(&self, node: tree_sitter::Node) -> Vec<String> {
        let mut mods = vec![];
        let mut c = node.walk();
        if c.goto_first_child() {
            loop {
                if c.node().kind() == "modifier" {
                    match text(c.node(), self.src) {
                        "public" => mods.push("export".into()),
                        "sealed" => mods.push("sealed".into()),
                        "abstract" => mods.push("abstract".into()),
                        _ => {}
                    }
                }
                if !c.goto_next_sibling() { break; }
            }
        }
        mods
    }

    fn kind_word(&self, node: tree_sitter::Node) -> String {
        {
            let mut c = node.walk();
            if c.goto_first_child() {
                loop {
                    match c.node().kind() {
                        "struct" => return "struct".into(),
                        "record" => return "record".into(),
                        "interface" => return "interface".into(),
                        _ => {}
                    }
                    if !c.goto_next_sibling() { break; }
                }
            }
        }
        "class".into()
    }

    fn type_decl(&mut self, node: tree_sitter::Node<'a>, kind_hint: &str) -> Core {
        let mut name = node
            .child_by_field_name("name")
            .map(|n| text(n, self.src).to_string())
            .unwrap_or_default();
        if let Some(tp) = node.child_by_field_name("type_parameters") {
            name = format!("{name}{}", text(tp, self.src).replace(' ', ""));
        }
        // primary constructor: `type X(params)` — the parameter_list child
        // that is NOT a method's (type-level: sits directly on the declaration)
        let mut primary = None;
        {
            let mut pc = node.walk();
            if pc.goto_first_child() {
                loop {
                    if pc.node().kind() == "parameter_list" {
                        primary = Some(self.param_pairs(pc.node()));
                        break;
                    }
                    if !pc.goto_next_sibling() { break; }
                }
            }
        }
        let mut bases = vec![];
        if let Some(bl) = node.child_by_field_name("bases") {
            let mut c = bl.walk();
            if c.goto_first_child() {
                loop {
                    let n = c.node();
                    if n.kind() != ":" && n.kind() != "comment" {
                        let b = self.type_text(n);
                        if !b.is_empty() {
                            bases.push(b);
                        }
                    }
                    if !c.goto_next_sibling() { break; }
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
                        "method_declaration" => vec![self.method_decl(n, false)],
                        "constructor_declaration" => vec![self.ctor_decl(n)],
                        "field_declaration" => self.field_decl(n),
                        "property_declaration" => self.property_decl(n),
                        "operator_declaration" | "conversion_operator_declaration" => vec![Core::Expr(
                            vec![Atom::Str(format!("opaque:{}", text(n, self.src).trim()))],
                            self.span(n),
                        )],
                        "enum_declaration" | "event_declaration" | "event_field_declaration" | "delegate_declaration" => vec![Core::Expr(
                            vec![Atom::Str(format!("opaque:{}", text(n, self.src).trim()))],
                            self.span(n),
                        )],
                        "interface_declaration" | "class_declaration" | "struct_declaration"
                        | "record_declaration" => vec![self.type_decl(n, "class")],
                        "comment" => vec![],
                        _ => vec![],
                    };
                    members.append(&mut forms);
                    if !c.goto_next_sibling() { break; }
                }
            }
        }
        Core::Type(TypeDecl {
            name,
            kind: kind_hint.into(),
            primary,
            bases,
            modifiers: self.modifiers_export(node),
            members,
            span: self.span(node),
        })
    }

    fn method_decl(&mut self, node: tree_sitter::Node<'a>, _in_type: bool) -> Core {
        let name = node.child_by_field_name("name").map(|n| text(n, self.src).to_string()).unwrap_or_default();
        let params = node.child_by_field_name("parameters").map(|p| self.param_pairs(p)).unwrap_or_default();
        let ret = node
            .child_by_field_name("returns")
            .or_else(|| node.child_by_field_name("type"))
            .map(|r| self.type_text(r))
            .filter(|t| !t.is_empty() && !t.eq_ignore_ascii_case("void"));
        let mut modifiers: Vec<String> = vec![];
        for m in self.modifiers_raw(node) {
            if m == "async" {
                modifiers.push("async".into());
            }
        }
        // extension: `this Type name` param 0 → receiver fn
        let mut receiver = None;
        if let Some((_, ty)) = params.first() {
            if ty.starts_with("this ") {
                receiver = Some(ty["this ".len()..].trim().to_string());
            }
        }
        let body = node
            .child_by_field_name("body")
            .or_else(|| node.child_by_field_name("arrow_expression_clause_body"))
            .map(|b| self.body_core(b))
            .unwrap_or(Body::Inline(vec![]));
        Core::Fn(FnDecl {
            name,
            receiver,
            params,
            ret,
            modifiers,
            base: None,
            body,
            span: self.span(node),
        })
    }

    fn modifiers_raw(&self, node: tree_sitter::Node) -> Vec<String> {
        let mut mods = vec![];
        let mut c = node.walk();
        if c.goto_first_child() {
            loop {
                if c.node().kind() == "modifier" {
                    mods.push(text(c.node(), self.src).to_string());
                }
                if !c.goto_next_sibling() { break; }
            }
        }
        mods
    }

    fn ctor_decl(&mut self, node: tree_sitter::Node<'a>) -> Core {
        let params = node.child_by_field_name("parameters").map(|p| self.param_pairs(p)).unwrap_or_default();
        let mut base = None;
        if let Some(ci) = node.child_by_field_name("constructor_initializer") {
            let t = text(ci, self.src).trim_start_matches(':').trim();
            base = Some(t.to_string());
        }
        let body = node.child_by_field_name("body").map(|b| self.body_core(b)).unwrap_or(Body::Inline(vec![]));
        Core::Fn(FnDecl {
            name: "_ctor".into(), // name filled by the type emitter context
            receiver: None,
            params,
            ret: None,
            modifiers: vec!["ctor".into()],
            base,
            body,
            span: self.span(node),
        })
    }

    fn param_pairs(&self, p: tree_sitter::Node) -> Vec<(String, String)> {
        let mut pairs = vec![];
        let mut c = p.walk();
        if c.goto_first_child() {
            loop {
                let n = c.node();
                match n.kind() {
                    "parameter" => {
                        // children: [modifier?, type, name] + optional default
                        let mut mods = String::new();
                        let mut ty = String::new();
                        let mut name = String::new();
                        let mut default_atoms: Vec<String> = vec![];
                        let mut pc = n.walk();
                        let mut past_name = false;
                        if pc.goto_first_child() {
                            loop {
                                let m = pc.node();
                                match m.kind() {
                                    "parameter_modifier" | "modifier" => {
                                        mods = text(m, self.src).to_string();
                                    }
                                    "identifier" | "predefined_type" | "generic_name"
                                    | "array_type" | "nullable_type" | "pointer_type" => {
                                        if ty.is_empty() {
                                            ty = self.type_text(m);
                                        } else if name.is_empty() && !past_name {
                                            name = text(m, self.src).to_string();
                                        }
                                    }
                                    "=" => {
                                        past_name = true;
                                    }
                                    ";" | "," | "(" | ")" | "[" | "]" | "comment" => {}
                                    _ => {
                                        if past_name {
                                            // default expression → `T = d` (expander canon)
                                            let mut d = vec![];
                                            self.verbatim_atoms(m, &mut d);
                                            if d.is_empty() {
                                                d = self.atomize(m);
                                            }
                                            default_atoms.push(crate::printer::render_atoms(&d));
                                        } else if ty.is_empty() {
                                            ty = self.type_text(m);
                                        }
                                    }
                                }
                                if !pc.goto_next_sibling() { break; }
                            }
                        }
                        // expander convention: default rides the ty as `T = x`
                        let mut full_ty = if mods.is_empty() {
                            ty
                        } else {
                            format!("{mods} {ty}")
                        };
                        if !default_atoms.is_empty() {
                            full_ty = format!("{full_ty} = {}", default_atoms.join(", "));
                        }
                        pairs.push((name, full_ty));
                    }
                    "comment" | "," | "(" | ")" | ";" => {}
                    _ => {}
                }
                if !c.goto_next_sibling() { break; }
            }
        }
        pairs
    }

    fn field_decl(&mut self, node: tree_sitter::Node) -> Vec<Core> {
        let mut rows = vec![];
        let mods = self.modifiers_raw(node);
        // the c-sharp grammar names no field for the variable_declaration
        let decl = node.child_by_field_name("declaration").or_else(|| {
            let mut fc = node.walk();
            if fc.goto_first_child() {
                loop {
                    if fc.node().kind() == "variable_declaration" {
                        return Some(fc.node());
                    }
                    if !fc.goto_next_sibling() { break; }
                }
            }
            None
        });
        if let Some(vd) = decl {
            let ty = vd.child(0).map(|t| self.type_text(t)).unwrap_or_default();
            let mut c = vd.walk();
            if c.goto_first_child() {
                loop {
                    if c.node().kind() == "variable_declarator" {
                        let name = c.node().child(0).map(|x| text(x, self.src).to_string()).unwrap_or_default();
                        let value = c.node().child(2).map(|v| self.atomize(v));
                        rows.push(Core::Var(VarDecl {
                            name,
                            ty: Some(ty.clone()),
                            value,
                            modifiers: canon_modifiers(&mods),
                            objlit: None,
                            span: self.span(c.node()),
                        }));
                    }
                    if !c.goto_next_sibling() { break; }
                }
            }
        }
        rows
    }

    fn property_decl(&mut self, node: tree_sitter::Node) -> Vec<Core> {
        let ty = node.child_by_field_name("type").map(|t| self.type_text(t)).unwrap_or_default();
        let name = node.child_by_field_name("name").map(|n| text(n, self.src).to_string()).unwrap_or_default();
        let mods = self.modifiers_raw(node);
        let span = self.span(node);
        // computed `Type Name => expr` — the `value` field is the arrow
        // clause; descend to its expression (atomizing the clause itself
        // Verbatim-glues `=>`)
        if let Some(arrow) = node.child_by_field_name("value") {
            let is_arrow = arrow.kind() == "arrow_expression_clause";
            let atoms = if is_arrow {
                arrow.child(1).map(|e| self.atomize(e)).unwrap_or_default()
            } else {
                self.atomize(arrow)
            };
            let mut mods = canon_modifiers(&mods);
            if is_arrow {
                mods.push("computed".into());
            }
            mods.dedup();
            return vec![Core::Var(VarDecl {
                name,
                ty: Some(ty),
                value: Some(atoms),
                modifiers: mods,
                objlit: None,
                span,
            })];
        }
        // `Type Name { get; ... } [ = init ]`
        let value: Option<tree_sitter::Node> = {
            // `= init` after the accessor list sits between `}` and `;`
            let mut c = node.walk();
            let mut found: Option<tree_sitter::Node> = None;
            if c.goto_first_child() {
                loop {
                    if c.node().kind() == "=" {
                        if let Some(sib) = c.goto_next_sibling().then(|| c.node()) {
                            if sib.kind() != ";" {
                                found = Some(sib);
                            }
                        }
                        break;
                    }
                    if !c.goto_next_sibling() { break; }
                }
            }
            found
        };
        let value: Option<Vec<Atom>> = value.map(|v| self.atomize(v));
        match value {
            Some(atoms) => vec![Core::Var(VarDecl {
                name,
                ty: Some(ty),
                value: Some(atoms),
                modifiers: canon_modifiers(&mods),
                objlit: None,
                span,
            })],
            None => {
                // accessor-only: signature var (canon `Type Name`)
                vec![Core::Var(VarDecl {
                    name,
                    ty: Some(ty),
                    value: None,
                    modifiers: canon_modifiers(&mods),
                    objlit: None,
                    span,
                })]
            }
        }
    }

    // ------------------------------------------------------------------
    // bodies & statements

    fn body_core(&mut self, body: tree_sitter::Node<'a>) -> Body {
        // arrow-expression clause: `=> expr` — a switch value keeps arm
        // structure (canon row `fn ...{X switch{arms}}`)
        if body.kind() == "arrow_expression_clause" {
            if let Some(e) = body.child(1) {
                if e.kind() == "switch_expression" {
                    let sp = self.span(e);
                    return Body::Bare(vec![self.switch_core(e, sp)]);
                }
                return Body::Inline(self.atomize(e));
            }
            return Body::Inline(vec![]);
        }
        if body.kind() != "block" {
            // braceless single statement: statements canon as braced Bare
            // rows (`atomize(return…) falls to the Sym fallback otherwise)
            return match body.kind() {
                "local_declaration_statement" => {
                    if let Some(vd) = body.child(0) {
                        Body::Bare(vec![self.local_or_expr(vd, self.span(body))])
                    } else {
                        Body::Bare(vec![])
                    }
                }
                _ => Body::Bare(vec![self.stmt(body).unwrap_or(Core::Expr(vec![], self.span(body)))]),
            };
        }
        let mut stmts: Vec<Core> = vec![];
        let mut c = body.walk();
        if c.goto_first_child() {
            loop {
                let n = c.node();
                if n.kind() != "{" && n.kind() != "}" && n.kind() != "comment" {
                    let mut forms = match n.kind() {
                        "local_function_statement" => vec![self.method_decl(n, true)],
                        _ => match self.stmt(n) {
                            Some(st) => vec![st],
                            None => vec![],
                        },
                    };
                    stmts.append(&mut forms);
                }
                if !c.goto_next_sibling() { break; }
            }
        }
        if stmts.is_empty() {
            Body::Inline(vec![])
        } else {
            Body::Bare(stmts)
        }
    }

    fn stmt(&mut self, node: tree_sitter::Node<'a>) -> Option<Core> {
        let span = self.span(node);
        // fn-tail statement values that are switch expressions carry arms
        if node.kind() == "expression_statement" {
            if node.child(0).map(|inner| inner.kind() == "switch_expression").unwrap_or(false) {
                return Some(self.switch_core(node.child(0).unwrap(), span));
            }
        }
        Some(match node.kind() {
            "if_statement" => {
                let mut head: Vec<Atom> = vec![Atom::Ident("if".into())];
                if let Some(c) = node.child_by_field_name("condition") {
                    head.extend(self.cond_atoms(c));
                }
                let body = node.child_by_field_name("consequence").map(|b| self.body_core(b)).unwrap_or(Body::Inline(vec![]));
                let mut arms = vec![];
                if let Some(alt) = node.child_by_field_name("alternative") {
                    // the alternative field may hold the else_clause wrapper
                    // or the block body directly (c-sharp grammar)
                    let body_n = if alt.kind() == "else_clause" {
                        alt.child(1).unwrap_or(alt)
                    } else {
                        alt
                    };
                    match body_n.kind() {
                        "if_statement" => {
                            let mut p: Vec<Atom> = vec![Atom::Ident("else".into()), Atom::Ident("if".into())];
                            if let Some(c) = body_n.child_by_field_name("condition") {
                                p.extend(self.cond_atoms(c));
                            }
                            arms.push(Arm {
                                pattern: p,
                                body: body_n.child_by_field_name("consequence").map(|b| self.body_core(b)).unwrap_or(Body::Bare(vec![])),
                                span: self.span(alt),
                            });
                        }
                        _ => arms.push(Arm {
                            pattern: vec![Atom::Ident("else".into())],
                            body: self.body_core(body_n),
                            span: self.span(alt),
                        }),
                    }
                }
                Core::Stmt { head, body, arms, span }
            }
            "while_statement" => {
                let mut head: Vec<Atom> = vec![Atom::Ident("while".into())];
                if let Some(c) = node.child_by_field_name("condition") {
                    head.extend(self.cond_atoms(c));
                }
                let body = node.child_by_field_name("body").map(|b| self.body_core(b)).unwrap_or(Body::Inline(vec![]));
                Core::Stmt { head, body, arms: vec![], span }
            }
            "do_statement" => {
                let mut head: Vec<Atom> = vec![Atom::Ident("while".into())];
                if let Some(c) = node.child_by_field_name("condition") {
                    head.extend(self.cond_atoms(c));
                }
                let body = node.child_by_field_name("body").map(|b| self.body_core(b)).unwrap_or(Body::Inline(vec![]));
                Core::Stmt { head, body, arms: vec![], span }
            }
            "for_statement" => {
                // C-style head verbatim: `for (init; cond; step)`
                let mut head: Vec<Atom> = vec![Atom::Ident("for".into())];
                for part in ["initializer", "condition", "update"] {
                    if let Some(p) = node.child_by_field_name(part) {
                        // parts arrive direct (binary_expression cond, var
                        // decl init) — cond_atoms handles both
                        head.extend(self.cond_atoms(p));
                        head.push(Atom::Ident(" ".into()));
                        head.pop();
                    }
                    head.push(Atom::Sym(";".into()));
                }
                if head.last() == Some(&Atom::Sym(";".into())) {
                    head.pop();
                }
                let body = node.child_by_field_name("body").map(|b| self.body_core(b)).unwrap_or(Body::Inline(vec![]));
                Core::Stmt { head, body, arms: vec![], span }
            }
            "foreach_statement" => {
                let mut head: Vec<Atom> = vec![Atom::Ident("for".into())];
                if let Some(l) = node.child_by_field_name("left") {
                    // `left` = the identifier itself (kind "identifier")
                    head.push(Atom::Ident(text(l, self.src).to_string()));
                }
                head.push(Atom::Ident("in".into()));
                if let Some(r) = node.child_by_field_name("right") {
                    head.extend(self.atomize(r));
                }
                let body = node.child_by_field_name("body").map(|b| self.body_core(b)).unwrap_or(Body::Inline(vec![]));
                Core::Stmt { head, body, arms: vec![], span }
            }
            "lock_statement" => {
                let mut head: Vec<Atom> = vec![Atom::Ident("lock".into())];
                // subject sits between the `lock` kw and the body block,
                // either as a parenthesized node or raw anon children
                let mut lc = node.walk();
                if lc.goto_first_child() {
                    loop {
                        let n = lc.node();
                        if n.kind() == "block" || n.kind() == "comment" {
                            if n.kind() == "block" { break; }
                        } else if n.kind() == "parenthesized_expression" {
                            if let Some(inner) = n.child(1) {
                                head.extend(self.atomize(inner));
                            }
                            break;
                        } else if !matches!(n.kind(), "lock" | "(" | ")") {
                            head.extend(self.atomize(n));
                        }
                        if !lc.goto_next_sibling() { break; }
                    }
                }
                let body: Body = {
                    let mut blk = None;
                    let mut lc = node.walk();
                    if lc.goto_first_child() {
                        loop {
                            if lc.node().kind() == "block" {
                                blk = Some(lc.node());
                                break;
                            }
                            if !lc.goto_next_sibling() { break; }
                        }
                    }
                    blk.map(|b| self.body_core(b)).unwrap_or(Body::Inline(vec![]))
                };
                Core::Stmt { head, body, arms: vec![], span }
            }
            "switch_statement" => {
                let mut head: Vec<Atom> = vec![Atom::Ident("match".into())];
                if let Some(e) = node.child_by_field_name("expression") {
                    head.extend(self.cond_atoms(e));
                }
                let mut arms = vec![];
                if let Some(body) = node.child_by_field_name("body") {
                    let mut c = body.walk();
                    if c.goto_first_child() {
                        loop {
                            let n = c.node();
                            if n.kind() == "switch_section" {
                                arms.push(self.switch_arm(n));
                            }
                            if !c.goto_next_sibling() { break; }
                        }
                    }
                }
                Core::Stmt { head, body: Body::Inline(vec![]), arms, span }
            }
            "try_statement" => {
                let body = node.child_by_field_name("body").map(|b| self.body_core(b)).unwrap_or(Body::Inline(vec![]));
                let mut arms = vec![];
                let mut c = node.walk();
                if c.goto_first_child() {
                    loop {
                        let n = c.node();
                        if n.kind() == "catch_clause" {
                            let mut pat: Vec<Atom> = vec![Atom::Ident("catch".into())];
                            if let Some(cd) = n.child_by_field_name("declaration") {
                                // `(Type [name])`
                                let mut cc = cd.walk();
                                if cc.goto_first_child() {
                                    loop {
                                        let x = cc.node();
                                        if !matches!(x.kind(), "(" | ")") {
                                            pat.push(Atom::Ident(text(x, self.src).to_string()));
                                        }
                                        if !cc.goto_next_sibling() { break; }
                                    }
                                }
                            } else {
                                pat.push(Atom::Sym("_".into()));
                            }
                            let cbody = n.child_by_field_name("body").map(|b| self.body_core(b)).unwrap_or(Body::Inline(vec![]));
                            arms.push(Arm { pattern: pat, body: cbody, span: self.span(n) });
                        }
                        if n.kind() == "finally_clause" {
                            let cbody = n.child_by_field_name("body").map(|b| self.body_core(b)).unwrap_or(Body::Inline(vec![]));
                            arms.push(Arm { pattern: vec![Atom::Ident("finally".into())], body: cbody, span: self.span(n) });
                        }
                        if !c.goto_next_sibling() { break; }
                    }
                }
                Core::Stmt {
                    head: vec![Atom::Ident("try".into())],
                    body,
                    arms,
                    span,
                }
            }
            "return_statement" | "throw_statement" => {
                let kw = if node.kind() == "return_statement" { "return" } else { "throw" };
                let mut head: Vec<Atom> = vec![Atom::Ident(kw.into())];
                let mut arms = vec![];
                let mut c = node.walk();
                if c.goto_first_child() {
                    loop {
                        let n = c.node();
                        if matches!(n.kind(), "return" | "throw" | ";" | "comment") {
                            if !c.goto_next_sibling() { break; }
                            continue;
                        }
                        if n.kind() == "switch_expression" {
                            if let Core::Stmt { head: sw_head, arms: sw_arms, .. } = {
                                let sp = self.span(n);
                                self.switch_core(n, sp)
                            } {
                                head.extend(sw_head);
                                arms = sw_arms;
                            }
                            break;
                        }
                        head.extend(self.atomize(n));
                        if !c.goto_next_sibling() { break; }
                    }
                }
                Core::Stmt { head, body: Body::Inline(vec![]), arms, span }
            }
            "local_declaration_statement" | "expression_statement" => {
                node.child(0).map(|n| self.local_or_expr(n, span)).unwrap_or(Core::Expr(vec![], span))
            }
            "break_statement" | "continue_statement" => {
                let kw = if node.kind() == "break_statement" { "break" } else { "continue" };
                Core::Stmt { head: vec![Atom::Ident(kw.into())], body: Body::Inline(vec![]), arms: vec![], span }
            }
            "yield_statement" => Core::Stmt {
                head: vec![Atom::Ident("yield".into())],
                body: Body::Inline(vec![]),
                arms: vec![],
                span,
            },
            other => {
                let t = text(node, self.src).trim().to_string();
                if t.is_empty() {
                    return None;
                }
                Core::Expr(vec![Atom::Str(format!("opaque:{t}"))], span)
            }
        })
    }

    fn local_or_expr(&mut self, n: tree_sitter::Node<'a>, span: Span) -> Core {
        match n.kind() {
            "variable_declaration" => {
                // `var name = e` / typed locals — decl type drops (contract)
                let ty = n.child(0).map(|t| self.type_text(t)).unwrap_or_default();
                let mut c = n.walk();
                let mut rows: Vec<Core> = vec![];
                if c.goto_first_child() {
                    loop {
                        if c.node().kind() == "variable_declarator" {
                            let name = c.node().child(0).map(|x| text(x, self.src).to_string()).unwrap_or_default().clone();
                            let value = declarator_value(c.node(), self);
                            rows.push(Core::Var(VarDecl {
                                name,
                                ty: if ty == "var" { None } else { Some(ty.clone()) },
                                value,
                                modifiers: vec![],
                                objlit: None,
                                span: self.span(c.node()),
                            }));
                        }
                        if !c.goto_next_sibling() { break; }
                    }
                }
                if rows.len() == 1 { rows.remove(0) } else { Core::Expr(vec![Atom::Str(format!("opaque:{}", text(n, self.src)))], span) }
            }
            _ => Core::Expr(self.atomize(n), span),
        }
    }

    fn switch_arm(&mut self, section: tree_sitter::Node<'a>) -> Arm {
        let span = self.span(section);
        let mut pattern: Vec<Atom> = vec![];
        let mut body_atoms: Vec<Atom> = vec![];
        let mut c = section.walk();
        if c.goto_first_child() {
            loop {
                let n = c.node();
                match n.kind() {
                    "case_switch_label" => {
                        // `case PAT:` — atoms of the constant
                        if let Some(cp) = n.child(1) {
                            pattern.extend(self.atomize(cp));
                        }
                    }
                    "case_pattern_switch_label" => {
                        // `case <pattern> [when c]: [decl]`
                        let mut cc = n.walk();
                        if cc.goto_first_child() {
                            loop {
                                let x = cc.node();
                                if matches!(x.kind(), "constant_pattern" | "recursive_pattern" | "relational_pattern" | "declaration_pattern" | "var_pattern" | "discard") {
                                    pattern.extend(self.pattern_atoms(x));
                                } else if x.kind() == "when_clause" {
                                    // verbatim when-guard atom blob
                                    let mut vv = vec![];
                                    self.verbatim_atoms(x, &mut vv);
                                    let wtxt = crate::printer::render_atoms(&vv);
                                    pattern.push(Atom::Ident(wtxt));
                                }
                                if !cc.goto_next_sibling() { break; }
                            }
                        }
                    }
                    "default_switch_label" => pattern.push(Atom::Ident("else".into())),
                    _ => {
                        // statements of the section
                        let st = c.node();
                        if st.kind() != "comment" && !matches!(st.kind(), ":" | ";") {
                            body_atoms.extend(self.stmt_atoms(st));
                        }
                    }
                }
                if !c.goto_next_sibling() { break; }
            }
        }
        let body = if body_atoms.is_empty() {
            Body::Inline(vec![])
        } else {
            Body::Inline(body_atoms)
        };
        Arm { pattern, body, span }
    }
}

/// a `variable_declarator`'s value: the atoms after the `=`-kind child
fn declarator_value<'b>(d: tree_sitter::Node, r: &CsReader) -> Option<Vec<Atom>> {
    let mut c = d.walk();
    let mut seen_eq = false;
    let mut v: Vec<Atom> = vec![];
    if c.goto_first_child() {
        loop {
            let n = c.node();
            if n.kind() == "=" {
                seen_eq = true;
                if !c.goto_next_sibling() {
                    break;
                }
                continue;
            }
            if seen_eq && n.kind() != ";" && n.kind() != "comment" {
                v.extend(r.atomize(n));
            }
            if !c.goto_next_sibling() { break; }
        }
    }
    if seen_eq && !v.is_empty() { Some(v) } else { None }
}

fn pattern_empty_else(pat: &[Atom]) -> bool {
    matches!(pat.first(), Some(Atom::Sym(sp)) if sp == "else")
        || matches!(pat.first(), Some(Atom::Ident(x)) if x == "else" || x == "_")
}

/// canon modifiers for member rows: public→export; const/volatile keep;
/// others (readonly/static/etc) drop (canon field shapes)
fn canon_modifiers(mods: &[String]) -> Vec<String> {
    let mut out = vec![];
    for m in mods {
        match m.as_str() {
            "public" => out.push("export".into()),
            "const" => out.push("const".into()),
            "volatile" => out.push("volatile".into()),
            "mut" => out.push("mut".into()),
            _ => {}
        }
    }
    out
}

/// phase-0 type-word canonicalization (cs-target §6, inverted): lowercase
/// C# keywords → canon capitals; anything else verbatim
fn canon_word(w: &str) -> String {
    match w {
        "string" => "String".into(),
        "int" => "Int".into(),
        "bool" => "Bool".into(),
        "double" => "Float".into(),
        "long" => "Int64".into(),
        "decimal" => "Decimal".into(),
        "object" => "Object".into(),
        "char" => "Char".into(),
        "byte" => "Byte".into(),
        "sbyte" => "SByte".into(),
        "short" => "Short".into(),
        "ushort" => "UShort".into(),
        "uint" => "UInt".into(),
        "ulong" => "Int64".into(),
        "float" => "Float".into(),
        other => other.to_string(),
    }
}
#[cfg(test)]
mod roundtrip_tests {
    use super::*;
    use crate::printer;

    fn canon_identity(src: &str, label: &str) {
        let core = read_cs(src).unwrap_or_else(|e| panic!("{label}: read_cs failed: {e}"));
        let canon = printer::print(&core);
        let core2 = crate::expander::expand(&canon)
            .unwrap_or_else(|e| panic!("{label}: canon failed to re-parse:\n{e}\n--- canon ---\n{canon}"));
        let canon2 = printer::print(&core2);
        assert_eq!(canon, canon2, "{label}: canon NOT stable across IR re-parse");
    }

    #[test]
    fn cs_corpus_canon_identity() {
        canon_identity(include_str!("../../../bench/corpus/real/src/cs_01_interface.cs"), "cs_01");
        canon_identity(include_str!("../../../bench/corpus/real/src/cs_02_generic_registry.cs"), "cs_02");
        canon_identity(include_str!("../../../bench/corpus/real/src/cs_03_class_methods.cs"), "cs_03");
        canon_identity(include_str!("../../../bench/corpus/real/src/cs_04_strategy.cs"), "cs_04");
        canon_identity(include_str!("../../../bench/corpus/real/src/cs_05_extension_static.cs"), "cs_05");
        canon_identity(include_str!("../../../bench/corpus/real/src/cs_06_extension_generics.cs"), "cs_06");
        canon_identity(include_str!("../../../bench/corpus/real/src/cs_08_formatter.cs"), "cs_08");
        canon_identity(include_str!("../../../bench/corpus/real/src/cs_09_detector.cs"), "cs_09");
        canon_identity(include_str!("../../../bench/corpus/real/src/cs_10_detector2.cs"), "cs_10");
    }

    #[test]
    fn cs_construct_smoke() {
        canon_identity("interface I { a?: string; }", "interface");
        canon_identity(
            "class A : B { readonly string X { get; } = \"v\"; string Y { get; set; } string Z => \"c\"; const int L = 27; volatile bool v = true; }",
            "class-members",
        );
        canon_identity(
            "class A { public A(int s) : this(s.ToString()) { } public A(string s) { } }",
            "ctors",
        );
        canon_identity(
            "class A { public static string Ext(this string input) => input.Trim(); }",
            "extension",
        );
        canon_identity("class A { string M(List<int> xs, string s = \"d\") { lock (xs) { foreach (var x in xs) { } } return 0; } }", "lock-foreach");
        canon_identity("class A { string M(string s) { var r = s switch { \"a\" => 1, _ => 2 }; return r; } }", "switch-expr");
        canon_identity("class A { string M(string s) { if (s is not { } b) return \"z\"; return b; } }", "is-binder");
        canon_identity("class A { string M(int s) { try { return s.ToString(); } catch (Exception e) { throw e; } finally { Log(); } } }", "try-catch");
        canon_identity("class A { string M(int s) { return $\"v {s} w\"; } }", "interp");
        canon_identity("class A { string M(int s) { for (var i = 0; i < s; i++) { s++; } return s; } }", "cfor");
        canon_identity("using System.Collections.Generic;\nnamespace X;\nclass A { }", "ns-uses");
        canon_identity("namespace X;\nclass A<T> : IAlpha where T : class { }", "generics-where");
        canon_identity("class A { string M(string[] xs, params int[] ys) => xs[0]; }", "arrays-params");
        canon_identity("class A { string M(string x) { var p = new Dictionary<char, int> { ['k'] new(\"yotta\") }; return p != null ? \"a\" : \"b\"; } }", "dict-init");
        canon_identity("class A { string M() { var xs = new[] { 1, 2, 3 }; return xs[1]; } }", "array-init");
        canon_identity("class A { string M(Func<int, bool> f) { int x = 0; x += 1; return x.ToString(x.GetHashCode()); } }", "casts-assign");
    }
}
