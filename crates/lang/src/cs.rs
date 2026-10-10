//! C# emitter (PLAN 5.3) — Core → C#. Design: docs/cs-target.md §2–§6.
//!
//! Mirrors the js.rs direct-emitter shape: render over `Core`, expressions
//! verbatim-first (the IR expression surface was C#-derived by Phase-0
//! measurement), with a small op-name map for the JS-ish collection ops
//! the grammar canonized (`.sum`/.filter/.map → LINQ) and type-word
//! mapping (§6). Roslyn-inspired OUTPUT conventions: 4-space indent,
//! file-scoped namespace, usings after the namespace, brace-new-line
//! bodies on type/method declarations, expression-bodied single statements.

use crate::ast::Atom;
use crate::expander::{Arm, Body, Core, FnDecl, TypeDecl, VarDecl};

pub struct CsLowerError {
    pub msg: String,
}

impl std::fmt::Display for CsLowerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.msg)
    }
}

const IND: &str = "    ";

// ---------------------------------------------------------------------------
// §6 type words + signatures

/// word-boundary type-word map (String→string, Float→double, …); anything
/// else passes verbatim (CultureInfo, TLocaliser, TimeUnit…).
fn map_type(ty: &str) -> String {
    const WORDS: &[(&str, &str)] = &[
        ("String", "string"),
        ("Int", "int"),
        ("Bool", "bool"),
        ("Float", "double"),
        ("Int64", "long"),
        ("Decimal", "decimal"),
        ("Object", "object"),
        ("Object?", "object?"),
        ("Void", "void"),
    ];
    let mut out = String::with_capacity(ty.len());
    let bytes: Vec<char> = ty.chars().collect();
    let mut i = 0usize;
    while i < bytes.len() {
        let c = bytes[i];
        if c.is_ascii_alphabetic() || c == '_' {
            let start = i;
            while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == '_') {
                i += 1;
            }
            let word: String = bytes[start..i].iter().collect();
            out.push_str(match WORDS.iter().find(|(w, _)| *w == word) {
                Some((_, cs)) => cs,
                None => &word,
            });
            if i < bytes.len() && bytes[i] == '?' {
                out.push('?');
                i += 1;
            }
            // `Array<T>` → `T[]`
            if out.ends_with("Array") && word == "Array" && bytes.get(i) == Some(&'<') {
                let mut depth = 0i32;
                let mut j = i;
                while j < bytes.len() {
                    match bytes[j] {
                        '<' => depth += 1,
                        '>' => {
                            depth -= 1;
                            if depth == 0 {
                                break;
                            }
                        }
                        _ => {}
                    }
                    j += 1;
                }
                if j < bytes.len() {
                    let inner: String = bytes[i + 1..j].iter().collect();
                    out.truncate(out.len() - "Array".len());
                    out.push_str(&map_type(&inner));
                    out.push_str("[]");
                    i = j + 1;
                }
            }
        } else {
            out.push(c);
            i += 1;
        }
    }
    out
}

/// canon member/local ty strings may carry the declaration keywords the
/// expander's field-parse treats as type idents (`const var Limit`, `var X`,
/// `volatile var f`) — strip them into flags, leaving the real type.
fn decl_flags(ty: &str) -> (bool, bool, Option<String>) {
    let mut is_const = false;
    let mut is_volatile = false;
    let mut rest = ty.trim().trim_start_matches("mut ").trim();
    loop {
        if let Some(r) = rest.strip_prefix("const ") {
            is_const = true;
            rest = r.trim();
        } else if let Some(r) = rest.strip_prefix("volatile ") {
            is_volatile = true;
            rest = r.trim();
        } else if rest == "const" {
            is_const = true;
            rest = "";
        } else if rest == "volatile" {
            is_volatile = true;
            rest = "";
        } else if rest == "var" {
            rest = "";
        } else if let Some(r) = rest.strip_prefix("var ").map(|x| x.trim()) {
            if r.is_empty() || r.chars().next().is_some_and(|c| c.is_uppercase()) || r.starts_with('(') {
                rest = r;
            } else {
                break;
            }
        } else {
            break;
        }
    }
    let real = if rest.is_empty() { None } else { Some(rest.to_string()) };
    (is_const, is_volatile, real)
}

/// split an IR param ty `Bool = false` into (real_ty, default_text)
fn split_default(ty: &str) -> (&str, Option<&str>) {
    match ty.find(" = ") {
        Some(i) if !ty[..i].ends_with("??") => (&ty[..i], Some(ty[i + 3..].trim())),
        _ => (ty, None),
    }
}

// ---------------------------------------------------------------------------
// expression rendering (§3): verbatim-run rewriter

fn is_csharp_keyword(id: &str) -> bool {
    matches!(
        id,
        "if" | "else" | "for" | "foreach" | "while" | "lock" | "try" | "catch" | "finally"
            | "throw" | "return" | "await" | "async" | "new" | "is" | "as" | "in" | "out"
            | "ref" | "var" | "const" | "static" | "public" | "private" | "internal" | "readonly"
            | "volatile" | "override" | "virtual" | "sealed" | "abstract" | "switch"
            | "when" | "typeof" | "nameof" | "this" | "base" | "null" | "true" | "false"
            | "match" | "and" | "or"
    )
}

/// IR collection ops (JS-ish lowercase, Phase-0 decision) → LINQ spellings
fn linq_op(id: &str) -> Option<&'static str> {
    match id {
        "filter" => Some("Where"),
        "map" => Some("Select"),
        "sum" => Some("Sum"),
        "toList" => Some("ToList"),
        "count" => Some("Count"),
        "first" => Some("First"),
        "max" => Some("Max"),
        "min" => Some("Min"),
        "any" => Some("Any"),
        "all" => Some("All"),
        "sort" => Some("OrderBy"), // `.sort()` → `.OrderBy(x=>x)` — argless sort
        _ => None,
    }
}

struct ExprCtx<'a> {
    atoms: &'a [Atom],
}

/// `await Users.Find id` → `await Users.Find(id)` — an identifier followed by
/// a bare operand collects its operand span into parens (§3 paren rule).
/// `else` default arm: the splitter folds it to `Sym("else")`; canon rows
/// may also carry the bare ident form.
fn arm_is_else(pat: &[Atom]) -> bool {
    matches!(pat.first(), Some(Atom::Sym(s2)) if s2 == "else")
        || matches!(pat.first(), Some(Atom::Ident(x)) if x == "else")
}

/// switch-arm pattern text: canon glue (`>items`) violates C# member lookup
/// after a generic-close — separate `>` from a following lowercase ident
/// (never `>=` or `>>`).
fn pattern_txt(atoms: &[Atom]) -> String {
    let txt = expr(atoms);
    let mut out = String::with_capacity(txt.len());
    let ch: Vec<char> = txt.chars().collect();
    for (k, c) in ch.iter().enumerate() {
        out.push(*c);
        if *c == '>' && k + 1 < ch.len() && ch[k + 1] != '=' && ch[k + 1] != '>'
            && ch[k + 1].is_ascii_lowercase()
        {
            out.push(' ');
        }
        if matches!(*c, ']' | ')') && k + 1 < ch.len() && ch[k + 1].is_ascii_lowercase()
            && k > 0 && ch[k - 1] != ' '
        {
            out.push(' ');
        }
    }
    out
}

fn is_bare_operand_start(a: &Atom) -> bool {
    match a {
        Atom::Ident(x) => !is_csharp_keyword(x),
        Atom::Number(_) | Atom::Str(_) | Atom::Regex(_) => true,
        _ => false,
    }
}

pub fn expr(atoms: &[Atom]) -> String {
    let ctx = ExprCtx { atoms };
    ctx.render()
}

impl<'a> ExprCtx<'a> {
    fn render(&self) -> String {
        let atoms = self.atoms;
        let n = atoms.len();
        let mut out = String::new();
        let mut i = 0usize;
        while i < n {
            match &atoms[i] {
                Atom::Ident(id) => {
                    if id == "new" {
                        // `new Type` with no args/initializer in the run
                        // (the canon dropped a multi-line `{…}` initializer —
                        // reader-level row, cs-target §7): emit `new T()`
                        let mut k = i + 1;
                        let mut type_txt = String::new();
                        if let Some(Atom::Ident(x2)) = atoms.get(k) {
                            type_txt = x2.clone();
                            k += 1;
                        }
                        if matches!(atoms.get(k), Some(Atom::Sym(sp)) if sp == "<") {
                            let mut d = 1i32;
                            type_txt.push('<');
                            k += 1;
                            while k < n {
                                if let Atom::Sym(sp2) = &atoms[k] {
                                    let sc = sp2.as_str();
                                    if sc == "<" { d += 1; }
                                    if sc == ">" {
                                        d -= 1;
                                        if d == 0 { type_txt.push('>'); k += 1; break; }
                                    }
                                    type_txt.push_str(sc);
                                } else if let Atom::Ident(x2) = &atoms[k] { type_txt.push_str(x2); }
                                k += 1;
                            }
                        }
                        let has_shape = matches!(atoms.get(k), Some(Atom::Sym(sp)) if sp == "(" || sp == "{")
                            || matches!(atoms.get(k), Some(Atom::Sym(sp)) if sp == ".");
                        if !has_shape && !type_txt.is_empty() {
                            out.push_str("new ");
                            out.push_str(&type_txt);
                            out.push_str("()");
                            i = k;
                            continue;
                        }
                    }
                    if id == "$" {
                        // canon `$"…"` interpolation prefix: `$` + payload → `$"…"`
                        if let Some(Atom::Str(k)) = atoms.get(i + 1) {
                            let esc = k.replace('\\', "\\\\").replace('"', "\\\"");
                            out.push_str(&format!("${esc:?}"));
                            i += 2;
                            continue;
                        }
                    }
                    // `match` keyword inside expr runs (C# switch expr emits it)
                    // mid-run pattern keywords need a space BEFORE when the
                    // previous token ended word-ish (`M is`, `x is not` — the
                    // canon glues these; C# requires the separation)
                    if is_csharp_keyword(id)
                        && out.ends_with(|c: char| c.is_alphanumeric() || c == ']')
                    {
                        out.push(' ');
                    }
                    out.push_str(id);
                    // bare-call: Ident followed by a bare operand — successive
                    // operands separate on `,` at top level (IR arg paren rule)
                    if !is_csharp_keyword(id)
                        && matches!(atoms.get(i + 1), Some(a) if is_bare_operand_start(a))
                        && !out.ends_with('.')
                    {
                        let mut args: Vec<Atom> = vec![];
                        let mut prev_val = false;
                        for a in &atoms[i + 1..] {
                            let starts = is_bare_operand_start(a);
                            if starts && prev_val {
                                args.push(Atom::Sym(",".into()));
                            }
                            args.push(a.clone());
                            prev_val = matches!(a, Atom::Ident(_) | Atom::Number(_) | Atom::Str(_))
                                || matches!(a, Atom::Sym(sp) if matches!(sp.as_str(), ")" | "]"));
                        }
                        out.push_str(&format!("({})", expr(&args)));
                        i = n;
                        continue;
                    }
                    if is_csharp_keyword(id) {
                        out.push(' ');
                        i += 1;
                        continue;
                    }
                    i += 1;
                }
                Atom::Number(k) => {
                    out.push_str(k);
                    i += 1;
                }
                Atom::Str(k) => {
                    // interpolation canon `x{e}y` → `$"x{e}y"`; plain → "…".
                    // C#-canon payloads carry the leading `$` ($"...") — strip
                    let k2 = k.strip_prefix('$').unwrap_or(k);
                    let interp = k2.contains('{') && k2.contains('}');
                    let esc = k2.replace('\\', "\\\\").replace('"', "\\\"");
                    // canon `$"…"` = `$` atom + payload — a `$` right before means the
                    // payload renders as a plain quoted interpolation arg
                    let rendered = if interp && !out.ends_with('$') {
                        format!("${esc:?}")
                    } else {
                        format!("{esc:?}")
                    };
                    out.push_str(&rendered);
                    i += 1;
                }
                Atom::Regex(k) => {
                    out.push_str(&format!("/{k}"));
                    i += 1;
                }
                Atom::Sym(s) => match s.as_str() {
                    // `}`-close followed by a word (`… is not {} multiple`):
                    // canon glues, C# needs the separation
                    "}" => {
                        if matches!(atoms.get(i + 1), Some(Atom::Ident(_)) | Some(Atom::Number(_))) {
                            out.push_str("} ");
                        } else {
                            out.push('}');
                        }
                        i += 1;
                    }
                    "(" | "[" => {
                        let c = if s == "(" { ')' } else { ']' };
                        let mut j = i + 1;
                        let inner = bracket_slice(atoms, &mut j, c);
                        // `(Float)x` cast → mapped keyword
                        if s == "(" && inner.len() == 1
                            && matches!(&inner[0], Atom::Ident(t) if map_type(&t.to_string()) != *t)
                            && matches!(atoms.get(j), Some(Atom::Ident(_)) | Some(Atom::Number(_)))
                        {
                            let t = match &inner[0] { Atom::Ident(t) => t.clone(), _ => unreachable!() };
                            out.push_str(&format!("({})", map_type(&t)));
                            i = j;
                            continue;
                        }
                        let close_txt = if c == '}'
                            && matches!(atoms.get(j), Some(Atom::Ident(_)) | Some(Atom::Number(_)))
                        {
                            format!("}} ")
                        } else {
                            c.to_string()
                        };
                        out.push_str(&format!("{s}{}{close_txt}", expr(&inner)));
                        i = j;
                    }
                    // `[^1]`, `[.. xs]`, `[a, b]` — verbatim (C# 12)
                    "." => {
                        // member op-name map: `.sum(` → `.Sum(`
                        if let Some(Atom::Ident(nx)) = atoms.get(i + 1) {
                            if let Some(mapped) = linq_op(nx) {
                                if matches!(atoms.get(i + 2), Some(Atom::Sym(sp)) if sp == "(") {
                                    out.push('.');
                                    out.push_str(mapped);
                                    i += 2;
                                    continue;
                                } else if matches!(atoms.get(i + 2), Some(a) if is_bare_operand_start(a)) {
                                    // argless `.sort()` → `.OrderBy(x=>x)`
                                    if mapped == "OrderBy" {
                                        out.push_str(".OrderBy(x=>x)");
                                        // consume the operand run (bare-call style)
                                        i += 2;
                                        while i < n {
                                            if matches!(&atoms[i], Atom::Sym(sp) if matches!(sp.as_str(), "(" | "[")) {
                                                let c = if matches!(&atoms[i], Atom::Sym(sp2) if sp2 == "(") { ')' } else { ']' };
                                                let mut j = i + 1;
                                                let inner = bracket_slice(atoms, &mut j, c);
                                                out.push_str(&format!("({})", expr(&inner)));
                                                i = j;
                                                break;
                                            }
                                            if matches!(&atoms[i], Atom::Ident(id2) if is_csharp_keyword(id2)) { break; }
                                            i += 1;
                                        }
                                        continue;
                                    }
                                    out.push('.');
                                    out.push_str(mapped);
                                    out.push_str(&format!("({})", expr(&atoms[i + 2..])));
                                    i = n;
                                    continue;
                                }
                            }
                        }
                        out.push('.');
                        i += 1;
                    }
                    "," => {
                        while out.ends_with(' ') {
                            out.pop();
                        }
                        out.push_str(", ");
                        i += 1;
                    }
                    ":" => {
                        out.push_str(": ");
                        i += 1;
                    }
                    "?" if matches!(atoms.get(i + 1), Some(Atom::Sym(d)) if d == ".") => {
                        out.push_str("?.");
                        i += 2;
                    }
                    "=>" => {
                        out.push_str(" => ");
                        i += 1;
                    }
                    _ => {
                        out.push_str(s);
                        i += 1;
                    }
                },
            }
        }
        out
    }
}

fn bracket_slice(atoms: &[Atom], i: &mut usize, close: char) -> Vec<Atom> {
    let mut depth = 1i32;
    let start = *i;
    while *i < atoms.len() {
        if let Atom::Sym(s) = &atoms[*i] {
            let sc = s.chars().next().unwrap_or(' ');
            match sc {
                '(' | '[' | '{' if s.len() == 1 => depth += 1,
                ')' | ']' | '}' if s.len() == 1 => {
                    depth -= 1;
                    if depth == 0 && s.chars().next().unwrap_or(' ') == close {
                        let inner = atoms[start..*i].to_vec();
                        *i += 1;
                        return inner;
                    }
                }
                _ => {}
            }
        }
        *i += 1;
    }
    atoms[start..].to_vec()
}

/// C# needs `async Task<Ret>` on awaiting methods (§4 async row): an IR fn
/// whose body awaits but whose canon carries no `async` upgrades the
/// signature (the JS target has no such wrapper).
fn async_upgrade(f: &mut FnDecl) {
    fn body_awaits(b: &Body) -> bool {
        match b {
            Body::Inline(a) => a.iter().any(|x| matches!(x, Atom::Ident(i) if i == "await")),
            Body::Bare(v) | Body::Block(v) => v.iter().any(|c| match c {
                Core::Expr(a, _) => a.iter().any(|x| matches!(x, Atom::Ident(i) if i == "await")),
                Core::Stmt { head, body, arms, .. } => {
                    head.iter().any(|x| matches!(x, Atom::Ident(i) if i == "await"))
                        || body_awaits(body)
                        || arms.iter().any(|a| body_awaits(&a.body))
                }
                Core::Var(v) => v.value.as_ref().map(|a| a.iter().any(|x| matches!(x, Atom::Ident(i) if i == "await"))).unwrap_or(false),
                Core::Fn(f2) => body_awaits(&f2.body),
                other => {
                    let _ = other;
                    false
                }
            }),
            _ => false,
        }
    }
    let awaited = body_awaits(&f.body);
    if !awaited {
        return;
    }
    let has_async = f.modifiers.iter().any(|m| m == "async");
    if !has_async {
        f.modifiers.push("async".into());
    }
    if let Some(r) = &f.ret {
        let (rt, _) = split_default(r);
        let wrapped = if rt == "void" { "Task".to_string() } else { format!("Task<{rt}>") };
        let _ = &wrapped;
        if !rt.starts_with("Task") {
            f.ret = Some(wrapped);
        }
    } else {
        f.ret = Some("Task".into());
    }
}

// ---------------------------------------------------------------------------
// statement/block rendering (§4)

fn stmts_csharp(sts: &[Core], level: usize, out: &mut String) -> Result<(), CsLowerError> {
    for st in sts {
        match st {
            Core::Expr(atoms, _) => {
                out.push_str(&IND.repeat(level + 1));
                out.push_str(&render_stmt_line(atoms));
                out.push_str(";\n");
            }
            Core::Var(v) => out.push_str(&local_var(v, level)),
            Core::Stmt { head, body, arms, .. } => {
                let st = emit_stmt(head, body, arms, level)?;
                out.push_str(&st);
                if !out.ends_with('\n') {
                    out.push('\n');
                }
            }
            Core::Fn(f) => {
                // local function
                out.push_str(&IND.repeat(level + 1));
                out.push_str(&fn_sig(f, false));
                out.push_str(&render_body(&f.body, level, &f.ret)?);
                out.push('\n');
            }
            other => return Err(CsLowerError { msg: format!("unexpected stmt core: {}", crate::expander::kind(other)) }),
        }
    }
    Ok(())
}

fn render_stmt_line(atoms: &[Atom]) -> String {
    let txt = expr(atoms);
    // `match …{…}` value tails come through Expr — kept verbatim
    txt
}

fn local_var(v: &VarDecl, level: usize) -> String {
    let (ty, def) = match &v.ty {
        Some(t) => {
            let (rt, d) = split_default(t);
            (map_type(rt), d.map(|x| x.to_string()))
        }
        None => (String::new(), None),
    };
    let mut s = String::new();
    let mut mods: Vec<&String> = v.modifiers.iter().collect();
    let is_mut = mods.iter().any(|m| m.as_str() == "mut");
    let is_const = mods.iter().any(|m| m.as_str() == "const");
    let is_volatile = mods.iter().any(|m| m.as_str() == "volatile");
    mods.retain(|m| !matches!(m.as_str(), "mut" | "const" | "volatile"));
    s.push_str(&IND.repeat(level + 1));
    for m in &mods {
        s.push_str(m);
        s.push(' ');
    }
    if is_const {
        // C# const requires an explicit type — infer from the canon ty or
        // default to int for numeric literals
        s.push_str("const ");
        s.push_str(if ty.is_empty() { "int" } else { &ty });
        s.push(' ');
    } else if is_volatile {
        s.push_str("volatile ");
        s.push_str(if ty.is_empty() { "bool" } else { &ty });
        s.push(' ');
    } else {
        s.push_str(if ty.is_empty() { "var" } else { &ty });
        s.push(' ');
    }
    let _ = is_mut;
    s.push_str(&v.name);
    if let Some(atoms) = &v.value {
        if !atoms.is_empty() {
            s.push_str(" = ");
            s.push_str(&expr(atoms));
        }
    }
    if let Some(d) = def {
        if v.value.is_none() {
            s.push_str(" = ");
            s.push_str(&map_type(&d.to_string()));
        }
    }
    s.push_str(";\n");
    s
}

fn emit_stmt(head: &[Atom], body: &Body, arms: &[Arm], level: usize) -> Result<String, CsLowerError> {
    let ind = IND.repeat(level + 1);
    let lead = match head.first() {
        Some(Atom::Ident(k)) => k.clone(),
        _ => String::new(),
    };
    // match → switch (value-position handled by caller fusing `return`)
    if head.len() >= 2 && head.last().map(|a| matches!(a, Atom::Ident(x) if x == "switch")).unwrap_or(false) {
        let subject = expr(&head[..head.len() - 1]);
        let mut s = format!("{ind}return {subject} switch\n{ind}{{");
        let rendered_arms: Vec<(String, String)> = arms.iter().filter_map(|a| {
            let pat = pattern_txt(&a.pattern);
            let pat_txt = if a.pattern.is_empty() || pat.is_empty() || arm_is_else(&a.pattern) { "_".to_string() } else { pat };
            let body_txt = match &a.body {
                Body::Inline(b) if !b.is_empty() => expr(b),
                Body::Inline(_) => "default".to_string(),
                _ => String::new(),
            };
            Some((pat_txt, body_txt))
        }).collect::<Vec<(String, String)>>();
        let (mut firsts, elses): (Vec<(String, String)>, Vec<(String, String)>) =
            rendered_arms.into_iter().partition(|(p, _)| p != "_");
        firsts.extend(elses);
        for (k, (pat, body)) in firsts.iter().enumerate() {
            s.push('\n');
            s.push_str(&IND.repeat(level + 2));
            s.push_str(&format!("{pat} => {body}"));
            s.push(if k + 1 < firsts.len() { ',' } else { '\n' });
        }
        s.push_str(&ind);
        s.push_str("};");
        return Ok(s);
    }
    if lead == "match" && head.len() > 1 {
        let subject = expr(&head[1..]);
        let mut s = format!("{ind}return {subject} switch\n{ind}{{");
        let rendered_arms: Vec<(String, String)> = arms.iter().filter_map(|a| {
            let pat = pattern_txt(&a.pattern);
            let is_else = a.pattern.is_empty() || pat.is_empty() || a.pattern.first().map(|x| matches!(x, Atom::Ident(e) if e == "else")).unwrap_or(false);
            let pat_txt = if arm_is_else(&a.pattern) || is_else { "_".to_string() } else { pat };
            let body_txt = match &a.body {
                Body::Inline(b) if !b.is_empty() => expr(b),
                Body::Inline(_) => "default".to_string(),
                b => {
                    let mut blk = String::new();
                    if let Body::Bare(v) | Body::Block(v) = b {
                        stmts_csharp(v, level + 2, &mut blk).ok()?;
                    }
                    blk.lines().map(|l| l.to_string()).collect::<Vec<_>>().join("\n")
                }
            };
            Some((pat_txt, body_txt))
        }).collect::<Vec<(String, String)>>();
        let (mut firsts, elses): (Vec<(String, String)>, Vec<(String, String)>) =
            rendered_arms.into_iter().partition(|(p, _)| p != "_");
        firsts.extend(elses);
        for (k, (pat, body)) in firsts.iter().enumerate() {
            s.push('\n');
            s.push_str(&IND.repeat(level + 2));
            s.push_str(&format!("{pat} => {body}"));
            s.push(if k + 1 < firsts.len() { ',' } else { '\n' });
        }
        s.push_str(&ind);
        s.push_str("};");
        return Ok(s);
    }
    if lead == "try" {
        let mut s = String::new();
        match body {
            Body::Inline(b) if !b.is_empty() => {
                s.push_str(&format!("{ind}try {{ return {}; }}", expr(b)));
            }
            Body::Inline(_) => s.push_str(&format!("{ind}try {{}}")),
            Body::Bare(v) | Body::Block(v) => {
                s.push_str(&format!("{ind}try\n{ind}{{\n"));
                stmts_csharp(v, level + 1, &mut s)?;
                s.push_str(&ind);
                s.push_str("}");
            }
            Body::Opaque(_) => {}
        }
        for a in arms {
            let kw = match a.pattern.first() {
                Some(Atom::Ident(k)) => k.clone(),
                _ => "catch".into(),
            };
            let is_finally = kw == "finally";
            let rest = &a.pattern[1usize.min(a.pattern.len())..];
            let binder = match rest.split_last() {
                Some((Atom::Ident(last), head_pats)) if !is_csharp_keyword(last) && last.chars().next().is_some_and(|c| c.is_lowercase()) => {
                    format!("{} {}", head_pats.iter().map(|x| expr(std::slice::from_ref(x))).collect::<Vec<_>>().join(" "), last)
                }
                _ => expr(rest),
            };
            let clause = if is_finally { "finally".to_string() } else { format!("{kw} ({binder})") };
            match &a.body {
                Body::Inline(b) if !b.is_empty() => {
                    let txt = if matches!(b.first(), Some(Atom::Ident(k2)) if k2 == "return" || k2 == "throw") || is_finally {
                        expr(b).trim_end().to_string()
                    } else {
                        format!("return {}", expr(b))
                    };
                    s.push_str(&format!("\n{ind}{clause} {{ {txt}; }}"));
                }
                Body::Inline(_) => s.push_str(&format!("\n{ind}{clause} {{}}")),
                b => {
                    s.push_str(&format!("\n{ind}{clause}\n{ind}{{\n"));
                    if let Body::Bare(v) | Body::Block(v) = b {
                        stmts_csharp(v, level + 1, &mut s)?;
                    }
                    s.push_str(&ind);
                    s.push('}');
                }
            }
        }
        return Ok(s);
    }
    if lead == "lock" && head.len() > 1 {
        let subj = expr(&head[1..]);
        let mut s = format!("{ind}lock ({subj})\n{ind}{{\n");
        match body {
            Body::Bare(v) | Body::Block(v) => stmts_csharp(v, level + 1, &mut s)?,
            Body::Inline(b) if !b.is_empty() => s.push_str(&format!("{}{};\n", IND.repeat(level + 2), expr(b))),
            _ => {}
        }
        s.push_str(&ind);
        s.push('}');
        return Ok(s);
    }
    if lead == "if" {
        // head: `if <cond>` (+ arms: then via body, else-if/else arms)
        let cond = expr(&head[1..]);
        let mut s = format!("{ind}if ({cond})\n{ind}{{\n");
        match body {
            Body::Inline(b) if !b.is_empty() => s.push_str(&format!("{}{};\n", IND.repeat(level + 2), render_stmt_line(b))),
            Body::Bare(v) | Body::Block(v) => stmts_csharp(v, level + 1, &mut s)?,
            Body::Inline(_) => {}
            Body::Opaque(_) => {}
        }
        s.push_str(&ind);
        s.push('}');
        let mut s = s;
        for a in arms {
            let pat_txt = expr(&a.pattern);
            let is_else = a.pattern.is_empty() || pat_txt.is_empty() || pat_txt == "else"
                || arm_is_else(&a.pattern);
            if let Some(else_if) = pat_txt.strip_prefix("else if ") {
                let cond2 = else_if.to_string();
                s.push_str(&format!("\n{0}else if ({cond2})\n{0}{{", IND.repeat(level + 1)));
                s = push_arm_body(&a.body, s, level + 1)?;
                s.push('\n');
                s.push_str(&IND.repeat(level + 1));
                s.push('}');
            } else if is_else {
                s.push_str(&format!("\n{0}else\n{0}{{", IND.repeat(level + 1)));
                s = push_arm_body(&a.body, s, level + 1)?;
                s.push('\n');
                s.push_str(&IND.repeat(level + 1));
                s.push('}');
            }
        }
        return Ok(s);
    }
    if lead == "for" && head.iter().any(|a| matches!(a, Atom::Sym(sp) if sp == ";")) {
        // C-style: `for (init; cond; step)` — head atoms carry the shape
        let inner = expr(&head[1..]);
        let mut s = format!("{ind}for ({inner})\n{ind}{{\n");
        match body {
            Body::Inline(b) if !b.is_empty() => s.push_str(&format!("{}{};\n", IND.repeat(level + 2), render_stmt_line(b))),
            Body::Bare(v) | Body::Block(v) => stmts_csharp(v, level + 1, &mut s)?,
            Body::Inline(_) => {}
            Body::Opaque(_) => {}
        }
        s.push_str(&ind);
        s.push('}');
        return Ok(s);
    }
    if lead == "for" {
        // `for x in coll` / `for i in 0..n`
        if head.iter().any(|a| matches!(a, Atom::Ident(x) if x == "in")) {
            let ini = head.iter().position(|a| matches!(a, Atom::Ident(x) if x == "in")).unwrap();
            let var_name = expr(&head[1..ini]);
            let coll = expr(&head[ini + 1..]);
            let mut s = format!("{ind}foreach (var {var_name} in {coll})\n{ind}{{\n");
            match body {
                Body::Inline(b) if !b.is_empty() => s.push_str(&format!("{}{};\n", IND.repeat(level + 2), render_stmt_line(b))),
                Body::Bare(v) | Body::Block(v) => stmts_csharp(v, level + 1, &mut s)?,
                _ => {}
            }
            s.push_str(&ind);
            s.push('}');
            return Ok(s);
        }
    }
    if lead == "while" {
        let cond = expr(&head[1..]);
        let mut s = format!("{ind}while ({cond})\n{ind}{{\n");
        match body {
            Body::Inline(b) if !b.is_empty() => s.push_str(&format!("{}{};\n", IND.repeat(level + 2), render_stmt_line(b))),
            Body::Bare(v) | Body::Block(v) => stmts_csharp(v, level + 1, &mut s)?,
            _ => {}
        }
        s.push_str(&ind);
        s.push('}');
        return Ok(s);
    }
    // generic head statement (return/throw/other)
    let txt = render_stmt_line(head);
    let mut s = format!("{ind}{txt}");
    match body {
        Body::Inline(b) if !b.is_empty() => s.push_str(&format!(" {}", render_stmt_line(b))),
        Body::Bare(v) | Body::Block(v) if !v.is_empty() => {
            s.push_str(&format!("\n{ind}{{\n"));
            stmts_csharp(v, level + 1, &mut s)?;
            s.push_str(&ind);
            s.push('}');
        }
        _ => {}
    }
    Ok(s)
}

fn push_arm_body(b: &Body, mut s: String, level: usize) -> Result<String, CsLowerError> {
    match b {
        Body::Inline(x) if !x.is_empty() => {
            s.push('\n');
            s.push_str(&IND.repeat(level + 1));
            s.push_str(&render_stmt_line(x));
            s.push(';');
        }
        Body::Bare(v) | Body::Block(v) => {
            stmts_csharp(v, level, &mut s)?;
        }
        _ => {}
    }
    Ok(s)
}

// ---------------------------------------------------------------------------
// fn / ctor / type emission (§4)

fn fn_sig(f: &FnDecl, in_static: bool) -> String {
    let mut mods: Vec<String> = vec![];
    let is_ctor = f.modifiers.iter().any(|m| m == "ctor");
    let is_async = f.modifiers.iter().any(|m| m == "async");
    if f.modifiers.iter().any(|m| m == "export") {
        mods.push("public".into());
    }
    for m in &f.modifiers {
        if matches!(m.as_str(), "sealed" | "abstract" | "override" | "virtual" | "static" | "extern") {
            mods.push(m.clone());
        }
    }
    if is_async {
        mods.push("async".into());
    }
    if in_static && !is_ctor && !mods.iter().any(|m| m == "static") {
        mods.push("static".into());
    }
    let mut s = String::new();
    for m in &mods {
        s.push_str(m);
        s.push(' ');
    }
    if is_ctor {
        s.push_str(&f.name);
    } else if let Some(recv) = &f.receiver {
        // extension fn: `fn String.FromMetric() Float` → static, `this string input`
        // param 0 (static already in mods when the host type is a static class)
        if !mods.iter().any(|m| m == "static") {
            s.push_str("static ");
        }
        if let Some(r) = &f.ret {
            s.push_str(&map_type(r));
            s.push(' ');
        }
        s.push_str(&f.name);
        let mut params = f.params.clone();
        if params.first().map(|(_, t)| t.starts_with("this ")).unwrap_or(false) {
            // canon carries the receiver as param 0 (`fn serialize(this Cheerio<T>)`)
            let recv_ty = params[0].1["this ".len()..].trim().to_string();
            let recv_name = params[0].0.clone();
            params[0] = (recv_name, format!("this {}", map_type(&recv_ty)));
        } else {
            params.insert(0, ("input".into(), format!("this {}", map_type(recv))));
        }
        s.push_str(&params_csharp(&params));
        return s;
    } else {
        if let Some(r) = &f.ret {
            let (rt, _) = split_default(r);
            s.push_str(&map_type(rt));
            s.push(' ');
        } else {
            s.push_str("void ");
        }
        s.push_str(&f.name);
    }
    s.push_str(&params_csharp(&f.params));
    s
}

fn params_csharp(params: &[(String, String)]) -> String {
    let mut out = String::from("(");
    for (i, (nm, ty)) in params.iter().enumerate() {
        if i > 0 {
            out.push_str(", ");
        }
        let (rt, def) = split_default(ty);
        if rt.is_empty() {
            out.push_str(nm);
        } else if let Some(rest) = rt.strip_prefix("out ").map(|x| x.trim()) {
            out.push_str(&format!("out {} {}", map_type(rest), nm));
        } else if rt == "out" {
            out.push_str(&format!("out {nm}"));
        } else {
            out.push_str(&map_type(rt));
            out.push(' ');
            out.push_str(nm);
        }
        if let Some(d) = def {
            out.push_str(" = ");
            // default rendering: plain atoms (literals)
            let def_atoms = crate::lexer::lex(d)
                .map(|l| l.tokens.iter().filter_map(|(t, _, _)| match t {
                    crate::lexer::Tok::Ident(x) => Some(Atom::Ident(x.clone())),
                    crate::lexer::Tok::Number(x) => Some(Atom::Number(x.clone())),
                    crate::lexer::Tok::Str(x) => Some(Atom::Str(x.clone())),
                    crate::lexer::Tok::Sym(x) => Some(Atom::Sym(x.clone())),
                    _ => None,
                }).collect::<Vec<_>>());
            if let Ok(da) = def_atoms {
                out.push_str(&expr(&da));
            } else {
                out.push_str(d);
            }
        }
    }
    out.push(')');
    out
}

fn render_body(b: &Body, level: usize, ret: &Option<String>) -> Result<String, CsLowerError> {
    let ind = IND.repeat(level);
    match b {
        Body::Inline(a) if a.len() == 1 && matches!(&a[0], Atom::Str(x) if x.starts_with("opaque:")) => {
            let raw = match &a[0] { Atom::Str(x) => x["opaque:".len()..].to_string(), _ => unreachable!() };
            // verbatim capture (§8.1.3c analog): emit lines as-is, wrapped
            let _ = raw;
            return Ok(" { /* opaque */ }".to_string());
        }
        Body::Inline(a) if !a.is_empty() => {
            // single fused statement: expression-bodied method when it is a
            // plain expr, else a block with the statement(s)
            let lead = a.first().map(|x| match x { Atom::Ident(k) => k.clone(), _ => String::new() }).unwrap_or_default();
            if matches!(lead.as_str(), "return" | "throw" | "if" | "for" | "while" | "lock" | "match" | "try" | "var")
                || a.iter().any(|x| matches!(x, Atom::Ident(kw) if kw == "await") && lead != "await")
            {
                let mut s = String::from("\n{");
                s.push('\n');
                stmts_from_inline(a, level + 1, &mut s, true)?;
                s.push_str(&ind);
                s.push('}');
                return Ok(s);
            }
            if lead == "await" {
                let txt = expr(a);
                if ret.is_some() {
                    return Ok(format!("\n{ind}{{\n{ind}    return {txt};\n{ind}}}"));
                }
                return Ok(format!("\n{ind}{{\n{ind}    {txt};\n{ind}}}"));
            }
            let txt = render_stmt_line(a);
            if let Some(r) = ret {
                let _ = r;
                return Ok(format!(" =>\n{ind}    {txt};"));
            }
            return Ok(format!("\n{ind}{{\n{ind}    {txt};\n{ind}}}"));
        }
        Body::Inline(_) => Ok(" { }".to_string()),
        Body::Bare(v) | Body::Block(v) => {
            if v.len() == 1 && ret.is_some() {
                if let Core::Expr(atoms, _) = &v[0] {
                    let lead = atoms.first().map(|x| match x { Atom::Ident(k) => k.clone(), _ => String::new() }).unwrap_or_default();
                    let flow = matches!(lead.as_str(), "return" | "throw" | "if" | "for" | "while" | "lock" | "match" | "try")
                        || atoms.iter().any(|x| matches!(x, Atom::Ident(kw) if kw == "await"));
                    match &v[0] {
                        Core::Stmt { .. } => {}
                        _ if !flow => {
                            let txt = render_stmt_line(atoms);
                            return Ok(format!("\n{ind}{{\n{ind}    return {txt};\n{ind}}}"));
                        }
                        _ => {}
                    }
                }
            }
            // ret-tail value rule: a non-flow bare-expr tail returns
            let mut s = format!("\n{0}{{\n", IND.repeat(level));
            let mut sts: &[Core] = v;
            let mut tail: Option<&Vec<Atom>> = None;
            if ret.is_some() {
                if let [Core::Expr(atoms, _)] = &v[v.len().saturating_sub(1)..] {
                    let lead = atoms.first().map(|x| match x { Atom::Ident(k) => k.clone(), _ => String::new() }).unwrap_or_default();
                    let flow = matches!(lead.as_str(), "return" | "throw" | "if" | "for" | "while"
                        | "lock" | "match" | "try" | "var" | "new");
                    let has_await = atoms.iter().any(|x| matches!(x, Atom::Ident(kw) if kw == "await"));
                    if !atoms.is_empty() && lead.is_empty() || (!flow && !has_await) {
                        if atoms.iter().any(|x| matches!(x, Atom::Sym(sp) if matches!(sp.as_str(), "="))) {
                            // assignment tail stays a statement
                        } else {
                            if v.len() > 1 {
                                sts = &v[..v.len() - 1];
                            }
                            tail = Some(atoms);
                        }
                    }
                }
            }
            stmts_csharp(sts, level, &mut s)?;
            if let Some(t) = tail {
                s.push_str(&IND.repeat(level + 1));
                s.push_str(&format!("return {};", expr(t)));
                s.push('\n');
            }
            s.push_str(&ind);
            s.push('}');
            Ok(s)
        }
        Body::Opaque(raw) => {
            // verbatim (§8.1.3c): emit captured lines untouched
            let mut s = String::from("\n{");
            for l in raw.lines() {
                s.push('\n');
                s.push_str(l);
            }
            s.push('\n');
            s.push_str(&ind);
            s.push('}');
            Ok(s)
        }
    }
}

fn stmts_from_inline(a: &[Atom], level: usize, out: &mut String, ret_gate: bool) -> Result<(), CsLowerError> {
    let ind = IND.repeat(level);
    let lead = a.first().map(|x| match x { Atom::Ident(k) => k.clone(), _ => String::new() }).unwrap_or_default();
    // kw-led inline runs: split at depth-0 statement keywords (J2 shape)
    if matches!(lead.as_str(), "return" | "throw") {
        out.push_str(&ind);
        out.push_str(&render_stmt_line(a));
        out.push_str(";\n");
        return Ok(());
    }
    let _ = ret_gate;
    out.push_str(&ind);
    out.push_str(&render_stmt_line(a));
    out.push_str(";\n");
    Ok(())
}

fn emit_type(t: &TypeDecl, level: usize, out: &mut String) -> Result<(), CsLowerError> {
    let ind = IND.repeat(level);
    // extension methods force a static class (C# rule)
    let is_static_class = t.members.iter().any(|m| matches!(m, Core::Fn(f) if f.receiver.is_some()))
        && (t.kind == "class" || t.kind == "struct");
    let mut mods: Vec<String> = vec![];
    if t.modifiers.iter().any(|m| m == "export") {
        mods.push("public".into());
    }
    for m in &t.modifiers {
        if matches!(m.as_str(), "sealed" | "abstract") {
            mods.push(m.clone());
        }
    }
    if is_static_class {
        mods.push("static".into());
    }
    out.push_str(&ind);
    for m in &mods {
        out.push_str(m);
        out.push(' ');
    }
    out.push_str(match t.kind.as_str() {
        "interface" => "interface",
        "record" => "record",
        "struct" => "struct",
        _ => "class",
    });
    out.push(' ');
    out.push_str(&t.name);
    if let Some(p) = &t.primary {
        out.push_str(&params_csharp(p));
    }
    if !t.bases.is_empty() {
        out.push_str(" : ");
        out.push_str(&t.bases.iter().map(|b| map_type(b)).collect::<Vec<_>>().join(", "));
    }
    out.push('\n');
    out.push_str(&ind);
    out.push_str("{\n");
    for m in &t.members {
        match m {
            Core::Fn(f) => {
                if f.modifiers.iter().any(|x| x == "ctor") {
                    let mut f2 = f.clone();
                    if !f2.name.is_empty() && f2.name != "{}" {
                        // ctor name == type name
                    }
                    f2.name = t.name.clone();
                    out.push_str(&IND.repeat(level + 1));
                    out.push_str(&fn_sig(&f2, false));
                    out.push_str(&render_body(&f2.body, level + 1, &None)?);
                } else {
                    let mut f2 = f.clone();
                    async_upgrade(&mut f2);
                    let body_ret = f2.ret.clone();
                    out.push_str(&IND.repeat(level + 1));
                    out.push_str(&fn_sig(&f2, is_static_class));
                    out.push_str(&render_body(&f.body, level + 1, &body_ret)?);
                }
                out.push('\n');
            }
            Core::Var(v) => {
                out.push_str(&member_var(v, t, level + 1)?);
            }
            Core::Type(inner) => emit_type(inner, level + 1, out)?,
            Core::Expr(atoms, _) if atoms.len() == 1 && matches!(&atoms[0], Atom::Str(x) if x.starts_with("opaque:")) => {
                // verbatim member capture (§8.1.3c)
                let raw = match &atoms[0] { Atom::Str(x) => x["opaque:".len()..].to_string(), _ => unreachable!() };
                let mut lines = raw.lines();
                let first = lines.next().unwrap_or("").to_string();
                out.push_str(&IND.repeat(level + 1));
                // de-indent capture: reader captured the raw member line(s)
                for (li, line) in raw.lines().enumerate() {
                    if li > 0 {
                        out.push('\n');
                        out.push_str(line);
                    }
                }
                let _ = first;
                out.push('\n');
            }
            _ => {}
        }
    }
    out.push_str(&ind);
    out.push_str("}\n");
    Ok(())
}

fn member_var(v: &VarDecl, t: &TypeDecl, level: usize) -> Result<String, CsLowerError> {
    let ind = IND.repeat(level);
    // sig-only member rows (ty call-shaped): `DateHumanize_Now() String` —
    // expander models ty="Name(params)" name="Ret" — INVERT (cs-target §4)
    let ty_call = v.ty.as_deref().map(|x| x.contains('(')).unwrap_or(false);
    if ty_call {
        let ty = v.ty.as_deref().unwrap_or_default();
        let open = ty.find('(').unwrap_or(0);
        let m_name = ty[..open].trim();
        let chars: Vec<char> = ty.chars().collect();
        let mut depth = 0i32;
        let mut close = chars.len();
        for j in open..chars.len() {
            match chars[j] {
                '(' => depth += 1,
                ')' => {
                    depth -= 1;
                    if depth == 0 {
                        close = j;
                        break;
                    }
                }
                _ => {}
            }
        }
        let balanced = close < chars.len();
        let ptext: String = chars[open + 1..close.min(chars.len())].iter().collect();
        let rt_word = v.name.trim();
        let ret_capped = rt_word.chars().next().is_some_and(|c| c.is_uppercase());
        if !balanced && !ret_capped && v.value.as_ref().map(|a| a.len() >= 2).unwrap_or(false) {
            // expander split `…,T p=d ) Ret` → ty ends mid-params, name=param,
            // value=[default…, Ret] — reconstruct the full signature
            let val: Vec<Atom> = v.value.as_ref().unwrap().iter()
                .filter(|a| !matches!(a, Atom::Sym(sp) if sp == ")" || sp == "("))
                .cloned().collect();
            let (def_atoms, ret_atom) = val.split_at(val.len() - 1);
            let def_txt = expr(def_atoms).trim_end().to_string();
            let ret = map_type(&match ret_atom.last() { Some(Atom::Ident(x)) => x.clone(), _ => String::new() });
            return Ok(format!(
                "{ind}{ret} {m_name}({} {} = {});\n",
                map_type(ptext.trim()), rt_word, def_txt
            ));
        }
        let rt = map_type(&v.name);
        return Ok(format!("{ind}{rt} {m_name}({});\n", map_type(ptext.trim())));
    }
    let is_static = is_static_class_ctx(t) && t.kind != "interface";
    let is_computed = v.modifiers.iter().any(|m| m == "computed");
    if is_computed {
        let mut s = String::new();
        s.push_str(&ind);
        if let Some(ty) = &v.ty {
            s.push_str(&map_type(ty));
            s.push(' ');
        }
        s.push_str(&v.name);
        match &v.value {
            Some(atoms) if !atoms.is_empty() => {
                s.push_str(" => ");
                s.push_str(&expr(atoms));
                s.push_str(";\n");
            }
            _ => s.push_str(" { get; set; }\n"),
        }
        return Ok(s);
    }
    // ty may carry the declaration keywords (`const var`/`volatile var`)
    let (is_const, is_volatile, ty_real) = decl_flags(v.ty.as_deref().unwrap_or(""));
    let is_mut = v.modifiers.iter().any(|m| m == "mut");
    let typed = ty_real.clone().or_else(|| {
        // no real type and no value → still emit as untyped row handled below
        v.ty.clone().filter(|t| !t.trim().is_empty() && t.trim() != "const var" && t.trim() != "var" && t.trim() != "volatile var")
    });
    // ty-None member field: infer from the leading literal, else dynamic
    if typed.is_none() {
        let ty_guess = v.value.as_ref().and_then(|a| v.value.as_ref().map(|b| b)).and_then(|a| a.first()).map(|x| match x {
            Atom::Number(k) if k.ends_with('m') => "decimal",
            Atom::Number(k) if k.contains('.') || k.ends_with('d') => "double",
            Atom::Number(_) => "int",
            Atom::Str(_) => "string",
            Atom::Ident(k) if k == "Math" => "double",
            _ => "",
        }).unwrap_or("");
        let mut s = String::new();
        s.push_str(&ind);
        if v.modifiers.iter().any(|m| m == "export") {
            s.push_str("public ");
        }
        if (is_static || v.modifiers.iter().any(|m| m == "static")) && !is_const {
            s.push_str("static ");
        }
        if is_const {
            s.push_str("const int ");
        } else {
            if !is_mut && t.kind != "interface" {
                s.push_str("readonly ");
            }
            if is_volatile {
                s.push_str("volatile ");
            }
            s.push_str(if ty_guess.is_empty() { "dynamic" } else { ty_guess });
            s.push(' ');
        }
        s.push_str(&v.name);
        if let Some(atoms) = &v.value {
            if !atoms.is_empty() {
                s.push_str(" = ");
                s.push_str(&expr(atoms));
            }
        }
        if let Some(rows) = &v.objlit {
            s.push_str(" { ");
            for row in rows {
                if let Core::Expr(atoms, _) = row {
                    s.push_str(&expr(atoms));
                }
                s.push_str(", ");
            }
            s.push_str("}");
        }
        s.push_str(";\n");
        return Ok(s);
    }
    let ty = typed.unwrap();
    let mut s = String::new();
    s.push_str(&ind);
    if v.modifiers.iter().any(|m| m == "export") {
        s.push_str("public ");
    }
    if is_static || v.modifiers.iter().any(|m| m == "static") {
        s.push_str("static ");
    }
    if ty.starts_with("out ") {
        s.push_str(&ty);
        s.push(' ');
    } else if is_const {
        s.push_str("const ");
        let mt = if ty.is_empty() { "int".to_string() } else { map_type(&ty) };
        s.push_str(&mt);
        s.push(' ');
    } else if is_volatile {
        s.push_str("volatile ");
        let mt = if ty.is_empty() { "bool".to_string() } else { map_type(&ty) };
        s.push_str(&mt);
        s.push(' ');
    } else if t.kind == "interface" {
        s.push_str(&map_type(&ty));
        s.push(' ');
        s.push_str(&v.name);
        s.push_str(" { get; set; }\n");
        return Ok(s);
    } else if !is_mut {
        s.push_str("readonly ");
        let mt = if ty.is_empty() { "var".to_string() } else { map_type(&ty) };
        s.push_str(&mt);
        s.push(' ');
    } else if ty.is_empty() {
        s.push_str("var ");
    } else {
        s.push_str(&map_type(&ty));
        s.push(' ');
    }
    s.push_str(&v.name);
    if let Some(atoms) = &v.value {
        if !atoms.is_empty() {
            s.push_str(" = ");
            s.push_str(&expr(atoms));
        }
    }
    s.push_str(";\n");
    return Ok(s);
}

fn is_static_class_ctx(t: &TypeDecl) -> bool {
    t.members.iter().any(|m| matches!(m, Core::Fn(f) if f.receiver.is_some()))
}

fn is_static_ctx_for_computed(t: &TypeDecl) -> bool {
    is_static_class_ctx(t)
}

// ---------------------------------------------------------------------------
// file emission

/// top-level Core forms → C# file (docs/cs-target.md §5 layout)
pub fn emit(core: &[Core]) -> Result<String, CsLowerError> {
    let mut out = String::new();
    let mut ns: Option<String> = None;
    let mut uses: Vec<String> = vec![];
    for c in core {
        if let Core::Header { ns: n, uses: u, .. } = c {
            ns = n.clone();
            for u in u {
                if !uses.contains(u) {
                    uses.push(u.clone());
                }
            }
        }
    }
    if let Some(n) = &ns {
        out.push_str(&format!("namespace {n};\n"));
    }
    uses.sort();
    uses.dedup();
    for u in &uses {
        out.push_str(&format!("using {u};\n"));
    }
    if !out.is_empty() {
        out.push('\n');
    }
    // split: type decls in place; top-level fn/var/expr → synthetic static
    // class (C# needs a containing type; cs-target §4)
    let mut synth: Vec<Core> = vec![];
    for c in core {
        match c {
            Core::Header { .. } => {}
            Core::Type(t) => {
                emit_type(t, 0, &mut out)?;
            }
            other => synth.push(other.clone()),
        }
    }
    if !synth.is_empty() {
        let cls = "ModuleTop".to_string();
        out.push_str(&format!("internal static partial class {cls}\n{{\n"));
        for c in &synth {
            match c {
                Core::Fn(f) => {
                    let ret = f.ret.clone();
                    let mut f2 = f.clone();
                    if !f2.modifiers.iter().any(|m| m == "static") {
                        f2.modifiers.push("static".into());
                    }
                    async_upgrade(&mut f2);
                    out.push_str(&IND.repeat(1));
                    out.push_str(&fn_sig(&f2, true));
                    out.push_str(&render_body(&f2.body, 1, &ret)?);
                    out.push('\n');
                }
                Core::Var(v) => {
                    // module re-export shape (`export X = X`): no C#
                    // equivalent — drop with a fidelity flag (cs-target §4)
                    if v.value.as_ref().and_then(|a| a.first()).map(|a| matches!(a, Atom::Ident(x) if *x == v.name)).unwrap_or(false) {
                        out.push_str(&IND.repeat(1));
                        out.push_str("// @use re-export row (module semantics) — dropped: no C# file-level equivalent\n");
                        continue;
                    }
                    let mut v2 = v.clone();
                    if !v2.modifiers.iter().any(|m| m == "export") {
                        v2.modifiers.push("export".into());
                    }
                    // top-level var → static ModuleTop field
                    let mut tv = TypeDecl {
                        name: cls.clone(),
                        kind: "class".into(),
                        primary: None,
                        bases: vec![],
                        modifiers: vec!["export".into()],
                        members: vec![],
                        span: crate::expander::SPAN_SYNTH,
                    };
                    let _ = &mut tv;
                    out.push_str(&IND.repeat(1));
                    // emit as static field of the synthetic class
                    let sig = member_field_top(&v2)?;
                    out.push_str(&sig);
                    out.push('\n');
                }
                Core::Expr(atoms, _) => {
                    out.push_str(&IND.repeat(1));
                    out.push_str(&render_stmt_line(atoms));
                    out.push_str(";\n");
                }
                Core::Stmt { head, body, arms, .. } => {
                    out.push_str(&emit_stmt(head, body, arms, 0)?);
                    out.push('\n');
                }
                _ => {}
            }
        }
        out.push_str("}\n");
    }
    Ok(out)
}

fn member_field_top(v: &VarDecl) -> Result<String, CsLowerError> {
    // static field shape (`public static T Name = e;`), ty from the canon row
    let mut s = String::from("public ");
    if let Some(ty) = &v.ty {
        let (rt, _) = split_default(ty);
        if rt.is_empty() {
            s.push_str("var ");
        } else {
            s.push_str(&map_type(rt));
            s.push(' ');
        }
    } else {
        s.push_str("var ");
    }
    s.push_str(&v.name);
    if let Some(atoms) = &v.value {
        if !atoms.is_empty() {
            s.push_str(" = ");
            s.push_str(&expr(atoms));
        }
    }
    s.push(';');
    Ok(s)
}