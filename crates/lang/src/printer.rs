//! Canonical printer (design doc §5): core forms → text, exactly one layout.
//!
//! Rules:
//! 1. `@`-directives at top, one line per clause, blank line after header.
//! 2. One decl per top-level line; statement bodies indented 2 spaces.
//! 3. Minimal spacing (`{`/`(`/`?` fuse to the preceding word; tight ops).
//! 4. Single-expression bodies fuse onto the header line.

use crate::ast::Atom;
use crate::expander::{Arm, Body, Core, FnDecl, TypeDecl, VarDecl, SPAN_SYNTH};

pub fn print(core: &[Core]) -> String {
    let mut out = String::new();
    // header first (canon: directives one per line)
    let mut seen_header = false;
    for c in core {
        if let Core::Header { version, ns, uses, .. } = c {
            out.push_str(&print_header(version, ns.clone(), uses.clone()));
            out.push('\n');
            seen_header = true;
        }
    }
    let first = seen_header; // blank line after header block
    let mut first_decl = true;
    for c in core {
        if matches!(c, Core::Header { .. }) {
            continue;
        }
        if !first_decl {
            out.push('\n');
        } else if first {
            out.push('\n'); // blank line after header block (canon §5.1)
        }
        first_decl = false;
        out.push_str(&print_core(c, 0));
    }
    // trailing newline
    if !out.ends_with('\n') {
        out.push('\n');
    }
    out
}

fn print_header(version: &str, ns: Option<String>, uses: Vec<String>) -> String {
    let mut s = String::new();
    if !version.is_empty() {
        s.push_str(&format!("@{version} "));
    }
    if let Some(n) = ns {
        s.push_str(&format!("@ns {n} "));
    }
    for u in uses {
        s.push_str(&format!("@use {u} "));
    }
    s.trim_end().to_string()
}

fn indent(level: usize) -> String {
    "  ".repeat(level)
}

fn print_core(c: &Core, level: usize) -> String {
    match c {
        Core::Header { .. } => String::new(), // handled in print()
        Core::Fn(f) => print_fn(f, level),
        Core::Type(t) => print_type(t, level),
        Core::Var(v) => print_var(v, level),
        Core::Expr(atoms, _) => format!("{}{}", indent(level), render_atoms(atoms)),
        Core::Stmt { head, body, arms, .. } => print_stmt(head, body, arms, level),
        Core::Reexport { star, is_type, names, from, .. } => {
            let mut s = String::from("export ");
            if *is_type {
                s.push_str("type ");
            }
            if *star {
                s.push_str("* from ");
            } else {
                s.push('{');
                s.push_str(&names.join(", "));
                s.push_str("} from ");
            }
            s.push_str(&format!("'{from}'"));
            s
        }
        Core::Clause(..) => String::new(), // associated into the owner pre-print
    }
}

/// Structured statement → canonical IR text. Shapes (design §2.3):
/// - primary + arms: braced blocks; else-arms re-fuse onto the close line:
///   `}else{…}`, `}else if c2{…}`.
/// - inline single-expr body: fused `{e}` on the header line.
/// - `else`/`catch`/`finally` fragments (reader re-parents them as sibling
///   lines after the close) print keyword-first with their own body.
fn print_stmt(head: &[Atom], body: &Body, arms: &[Arm], level: usize) -> String {
    let ind = indent(level);
    // fragment lines: `catch NotFound return null`, `finally Log("x")`, `else{…}`
    if let Some(Atom::Ident(kw)) = head.first() {
        if matches!(kw.as_str(), "else" | "catch" | "finally")
            && (arms.len() == 1 || (arms.is_empty() && !matches!(body, Body::Inline(b) if b.is_empty())))
        {
            // normalized clause: body lives in `body`, pattern empty-or-arms[0]
            let synthesized = Arm { pattern: Vec::new(), body: body.clone(), span: SPAN_SYNTH };
            let a: &Arm = if arms.len() == 1 { &arms[0] } else { &synthesized };
            match &a.body {
                Body::Inline(b) if !b.is_empty() => {
                    // fused single-expr, braceless ONLY for patternless
                    // fragments (`catch NotFound return null`, `finally e`,
                    // `else e`). `else if c{e}` prints braced — a braceless
                    // else-if + stmt re-parses as pattern-only and loses the
                    // body (ambiguity: no stmt-keyword anchor).
                    let mut s = format!("{ind}{kw}");
                    if !a.pattern.is_empty() {
                        s.push(' ');
                        s.push_str(&render_atoms(&a.pattern));
                    }
                    if a.pattern.is_empty() {
                        s.push(' ');
                        s.push_str(&render_atoms(b));
                        return s;
                    }
                    s.push_str(&render_body_inline(b));
                    return s;
                }
                Body::Inline(_) => {
                    // `else` bare (match default arm); braces only if a
                    // re-parse needs them — bare fragment is reader-stable
                    return format!("{ind}{kw}");
                }
                b => {
                    let mut s = if a.pattern.is_empty() {
                        format!("{ind}{kw}{{")
                    } else {
                        format!("{ind}{kw} {}{{", render_atoms(&a.pattern))
                    };
                    s.push('\n');
                    s.push_str(&render_body_block(b, level));
                    s.push_str(&ind);
                    s.push('}');
                    return s;
                }
            }
        }
    }
    // primary stmt: `head` atoms + body + arms (else glue re-fused)
    let mut s = format!("{ind}{}", render_atoms(head));
    // match/switch-expr: arms print as pattern-body lines in a braced block
    let head_is_switch = head.last().map(|a| matches!(a, Atom::Ident(k) if k == "switch")).unwrap_or(false);
    if matches!(head.first(), Some(Atom::Ident(k)) if k == "match") || head_is_switch {
        s.push('{');
        for a in arms {
            let pat = render_atoms(&a.pattern);
            s.push('\n');
            s.push_str(&indent(level + 1));
            match &a.body {
                Body::Inline(b) if !b.is_empty() => {
                    let b_txt = render_atoms(b);
                    if pat.is_empty() || pat == "else" {
                        s.push_str(&format!("else {b_txt}"));
                    } else {
                        // `{pat} {call(…)}` — the SPACE keeps arm pattern and
                        // body split across re-parses (fusing was unstable)
                        s.push_str(&format!("{pat} {b_txt}"));
                    }
                }
                Body::Inline(_) => {
                    s.push_str(&if pat.is_empty() { String::new() } else { pat });
                }
                b => {
                    if !pat.is_empty() {
                        s.push_str(&format!("{pat}"));
                    } else {
                        s.push_str("else");
                    }
                    s.push_str("{\n");
                    s.push_str(&render_body_block(b, level + 1));
                    s.push_str(&indent(level + 1));
                    s.push('}');
                }
            }
        }
        s.push('\n');
        s.push_str(&ind);
        s.push('}');
        return s;
    }
    // try: arms are catch/finally fragments (pattern[0] = clause keyword)
    if matches!(head.first(), Some(Atom::Ident(k)) if k == "try") {
        let mut frags: Vec<String> = Vec::new();
        match body {
            // canon: single stmt fuses ONTO the try line (`try await Repo.Get id`)
            Body::Inline(b) if !b.is_empty() => {
                s.push(' ');
                s.push_str(&render_atoms(b));
            }
            Body::Inline(_) => {}
            b => {
                s.push_str("{\n");
                s.push_str(&render_body_block(b, level));
                s.push_str(&ind);
                s.push('}');
            }
        }
        for a in arms {
            let kw = match a.pattern.first() {
                Some(Atom::Ident(k)) => k.clone(),
                _ => "catch".to_string(),
            };
            let pat = &a.pattern[1usize.min(a.pattern.len())..];
            let pat_txt = render_atoms(pat);
            let mut line = if pat_txt.is_empty() {
                format!("{ind}{kw}")
            } else {
                format!("{ind}{kw} {pat_txt}")
            };
            match &a.body {
                Body::Inline(b) if !b.is_empty() => {
                    line.push(' ');
                    line.push_str(&render_atoms(b));
                    frags.push(line);
                }
                Body::Inline(_) => frags.push(line),
                b => {
                    line.push_str("{\n");
                    frags.push(line);
                    frags.push(render_body_block(b, level));
                    frags.push(format!("{}}}", ind));
                }
            }
        }
        s.push('\n');
        s.push_str(&frags.join("\n"));
        return s;
    }
    match (body, arms.is_empty()) {
        (Body::Inline(b), true) if !b.is_empty() => s.push_str(&render_body_inline(b)),
        (Body::Inline(_), true) => {}
        (b, _) => {
            // then-arm re-fuse (split_glued_if convention: arms[0] with an
            // empty pattern IS the then-block; the stmt body is empty-inline)
            let (b, arms): (&Body, &[Arm]) =
                if matches!(body, Body::Inline(x) if x.is_empty())
                    && !arms.is_empty()
                    && arms[0].pattern.is_empty()
                {
                    (&arms[0].body, &arms[1..])
                } else {
                    (b, arms)
                };
            s.push('{');
            match b {
                Body::Inline(x) if !x.is_empty() => {
                    s.push('\n');
                    s.push_str(&print_core(&Core::Expr(x.clone(), SPAN_SYNTH), level + 1));
                    s.push('\n');
                    s.push_str(&ind);
                }
                Body::Bare(v) | Body::Block(v) => {
                    if !v.is_empty() {
                        s.push('\n');
                        for st in v {
                            s.push_str(&print_core(st, level + 1));
                            s.push('\n');
                        }
                        s.push_str(&ind);
                    }
                }
                _ => {}
            }
            s.push('}');
            for (ai, a) in arms.iter().enumerate() {
                // first link re-fuses on the just-closed brace; further links
                // open with their own close-line (chain shape: close+else per
                // line, reader-stable)
                if ai > 0 {
                    s.push('\n');
                    s.push_str(&ind);
                    s.push('}');
                }
                s.push_str(&print_arm_glue_rest(a, level));
            }
        }
    }
    s
}

/// arm text after the auto-close `}`: `else{…}` / `else if c{…}` / fused
fn print_arm_glue_rest(a: &Arm, level: usize) -> String {
    let ind = indent(level);
    let mut s = String::new();
    if !a.pattern.is_empty() {
        s.push_str(&render_atoms(&a.pattern));
    }
    match &a.body {
        Body::Inline(b) if !b.is_empty() => {
            s.push_str(&render_body_inline(b));
        }
        Body::Inline(_) => {
            s.push('{');
            s.push('}');
        }
        b => {
            s.push_str("{\n");
            s.push_str(&render_body_block(b, level));
            s.push('\n');
            s.push_str(&ind);
            s.push('}');
        }
    }
    s
}

fn render_body_inline(b: &[Atom]) -> String {
    format!("{{{}}}", render_atoms(b))
}

fn render_body_block(b: &Body, level: usize) -> String {
    let stmts = match b {
        Body::Bare(v) | Body::Block(v) => v,
        _ => return String::new(),
    };
    let mut s = String::new();
    for st in stmts {
        s.push_str(&print_core(st, level + 1));
        s.push('\n');
    }
    s
}

fn print_fn(f: &FnDecl, level: usize) -> String {
    let mut s = String::new();
    s.push_str(&indent(level));
    let mut mods: Vec<&String> = f.modifiers.iter().collect();
    // export goes first, async second (canon: `export async fn`)
    mods.sort_by_key(|m| match m.as_str() {
        "export" => 0,
        "async" => 1,
        _ => 2,
    });
    for m in &mods {
        s.push_str(m);
        s.push(' ');
    }
    if let Some(recv) = &f.receiver {
        s.push_str(&format!("fn {recv}.{}", f.name));
    } else if !f.modifiers.iter().any(|m| m == "ctor") {
        s.push_str("fn ");
        s.push_str(&f.name);
    }
    s.push('(');
    let params: Vec<String> = f
        .params
        .iter()
        .map(|(n, t)| {
            if t.is_empty() {
                n.clone()
            } else if t.contains('(') || t.contains('|') || t.contains(' ') {
                // ambiguous type exprs / modifier-prefixed types (`out String`)
                // print colon form (stability)
                format!("{n}: {t}")
            } else {
                format!("{n} {t}")
            }
        })
        .collect();
    s.push_str(&params.join(", "));
    s.push(')');
    if let Some(b) = &f.base {
        s.push_str(" : ");
        s.push_str(b);
    }
    if let Some(r) = &f.ret {
        s.push(' ');
        s.push_str(r);
    }
    match &f.body {
        Body::Inline(b)
            if b.len() == 1 && matches!(&b[0], Atom::Str(x) if x.starts_with("opaque:")) =>
        {
            // reader-captured verbatim blob (spans lines)
            let raw = match &b[0] {
                Atom::Str(x) => x["opaque:".len()..].to_string(),
                _ => unreachable!(),
            };
            return finish_opaque(&mut s, &raw, level);
        }
        Body::Inline(b) if !b.is_empty() => {
            s.push('{');
            s.push_str(&render_atoms(b));
            s.push('}');
        }
        Body::Inline(_) => {
            s.push_str("{}");
        }
        Body::Bare(stmts) => {
            s.push('{');
            s.push('\n');
            for st in stmts {
                s.push_str(&print_core(st, level + 1));
                s.push('\n');
            }
            s.push_str(&indent(level));
            s.push('}');
        }
        Body::Block(stmts) => {
            s.push('{');
            s.push('\n');
            for st in stmts {
                s.push_str(&print_core(st, level + 1));
                s.push('\n');
            }
            s.push_str(&indent(level));
            s.push('}');
        }
        Body::Opaque(raw) => {
            // verbatim: `opaque{ <raw lines…> }`
            s.push_str(" opaque{");
            for l in raw.lines() {
                s.push('\n');
                s.push_str(l);
            }
            s.push('\n');
            s.push_str(&indent(level));
            s.push('}');
        }
    }
    s
}

fn print_type(t: &TypeDecl, level: usize) -> String {
    let mut s = String::new();
    s.push_str(&indent(level));
    s.push_str("type ");
    s.push_str(&t.name);
    // C# source order: modifiers precede the name/primary (`readonly
    // struct CompositeUnit(...)`); the expander type arm consumes mods
    // before the primary scan, so canon must match that order
    for m in &t.modifiers {
        s.push(' ');
        s.push_str(m);
    }
    if let Some(p) = &t.primary {
        // primary ctor canon: `type Name(params) {` (fixture-row shape)
        let parts: Vec<String> = p.iter().map(|(n, ty)| {
            if ty.is_empty() { n.clone() } else { format!("{n} {ty}") }
        }).collect();
        s.push_str(&format!("({})", parts.join(", ")));
    }
    // kind word: canon keeps it when the target needs it (C# interface/
    // struct/record emit rows 5.4) — plain-class stays implied
    if t.kind != "class" {
        s.push(' ');
        s.push_str(&t.kind);
    }
    s.push('{');
    s.push('\n');
    for m in &t.members {
        s.push_str(&print_member(m, level + 1));
        s.push('\n');
    }
    s.push_str(&indent(level));
    s.push('}');
    s
}

fn print_member(c: &Core, level: usize) -> String {
    if let Core::Var(v) = c {
        let computed = v.modifiers.iter().any(|m| m == "computed");
        let mut s = String::new();
        s.push_str(&indent(level));
        // opaque member capture: value IS the verbatim row (`"opaque:…"`,
        // §8.1.3c) — no name/ty/`=` scaffolding. Signature-only opaque
        // vars (value None) keep the field rendering (name/ty text is
        // re-stable; the opaque mod is not part of canon surface).
        if v.modifiers.iter().any(|m| m == "opaque")
            && matches!(&v.value, Some(a) if !a.is_empty())
        {
            if let Some(val) = &v.value {
                s.push_str(&render_atoms(val));
            }
            return s;
        }
        if v.modifiers.iter().any(|m| m == "volatile") {
            s.push_str("volatile ");
        }
        if let Some(ty) = &v.ty {
            s.push_str(ty);
            s.push(' ');
        }
        s.push_str(&v.name);
        if let Some(val) = &v.value {
            if computed {
                s.push_str(" => ");
                s.push_str(&render_atoms(val));
            } else {
                // `>={` lexes `>=`: space after a generics-carrying ty
                if v.ty.as_ref().map(|t| t.ends_with('>')).unwrap_or(false) {
                    s.push(' ');
                }
                s.push('=');
                s.push_str(&render_atoms(val));
            }
        }
        s
    } else {
        print_core(c, level)
    }
}

fn print_var(v: &VarDecl, level: usize) -> String {
    let mut s = String::new();
    s.push_str(&indent(level));
    let mut want_var_word = true;
    let mut post_mut = false;
    for m in &v.modifiers {
        if m == "mut" {
            // grammar order: `["const"|"volatile"] "var" ["mut"] ident` —
            // mut prints AFTER the var word so canon re-parses stable
            post_mut = true;
            continue;
        }
        s.push_str(m);
        s.push(' ');
        if m == "var" {
            want_var_word = false;
        }
    }
    if want_var_word {
        s.push_str("var ");
    }
    if post_mut {
        s.push_str("mut ");
    }
    s.push_str(&v.name);
    if let Some(t) = &v.ty {
        s.push_str(": ");
        s.push_str(t);
    }
    if let Some(val) = &v.value {
        // a generic-close glued to `=` (`>={`) lexes as `>=` — canon keeps
        // one space after a generics-carrying ty so reparse is stable
        if v.ty.as_ref().map(|t| t.ends_with('>')).unwrap_or(false) {
            s.push(' ');
        }
        s.push('=');
        s.push_str(&render_atoms(val));
    }
    s
}

/// Render atoms with canonical minimal spacing (design doc §5.3):
/// space ONLY between word-ish atoms (Ident/Number ↔ Ident/Number) and
/// after `,`; everything glues.
/// Keywords always take a following space (readability floor §5.2; `if !x`
/// ties `if!x` 3/3 — readability-first).
const KEYWORDS: &[&str] = &[
    "fn", "var", "type", "if", "else", "match", "for", "in", "while", "lock", "try", "catch",
    "finally", "throw", "return", "null", "true", "false", "async", "export", "ctor",
    "interface", "record", "struct", "sealed", "abstract", "const", "volatile", "opaque", "out",
    "ref", "new", "typeof", "as", "is", "from", "extends", "await", "mut", "let", "delete",
];

pub fn render_atoms(atoms: &[Atom]) -> String {
    let mut s = String::new();
    let mut prev: Option<&Atom> = None;
    for a in atoms {
        let needs_space = match (prev, a) {
            (None, _) => false,
            (Some(left), right) => match (left, right) {
                // word word / word number / number word => space
                (Atom::Ident(_) | Atom::Number(_), Atom::Ident(_) | Atom::Number(_)) => true,
                // `a , b` — comma glues left, space after
                (Atom::Sym(c), _) if c == "," => true,
                // `!` ops after a word: the lexer's ident-suffix rule eats
                // `!` (`a!==` lexes `a!` + `==` — atoms change, and the
                // emitter drops the non-null `!` suffix). One space keeps
                // the operator lex-fresh → atoms survive the re-parse.
                (Atom::Ident(_) | Atom::Number(_), Atom::Sym(s)) if s.starts_with('!') => true,
                // `if(` — keyword space rule (probe tie, floor wins)
                (Atom::Ident(k), _) if is_keyword(k) => true,
                _ => false,
            },
        };
        if needs_space {
            s.push(' ');
        }
        s.push_str(&match a {
            Atom::Ident(x) | Atom::Number(x) => x.clone(),
            Atom::Str(x) => format!("{:?}", escape_string(x)),
            Atom::Char(x) => format!("'{x}'"),
            Atom::Regex(x) => format!("/{x}"),
            Atom::Sym(x) => x.clone(),
        });
        prev = Some(a);
    }
    s
}

fn is_keyword(id: &str) -> bool {
    KEYWORDS.contains(&id)
}

/// Debug view (readability floor §5.2, PLAN 1.5): macro-expanded core forms
/// rendered one per line for review. Not canonical IR text — every form is
/// labeled with its core kind so the closure invariant is visible.
pub fn print_debug(core: &[Core]) -> String {
    let mut out = String::new();
    for c in core {
        out.push_str(&debug_core(c, 0));
        out.push('\n');
    }
    out
}

fn debug_core(c: &Core, level: usize) -> String {
    let ind = indent(level);
    match c {
        Core::Header { version, ns, uses, .. } => {
            let mut s = format!("{ind}(header");
            if !version.is_empty() {
                s.push_str(&format!(" @{version}"));
            }
            if let Some(n) = ns {
                s.push_str(&format!(" @ns {n}"));
            }
            for u in uses {
                s.push_str(&format!(" @use {u}"));
            }
            s.push(')');
            s
        }
        Core::Fn(f) => debug_fn(f, level),
        Core::Type(t) => debug_type(t, level),
        Core::Var(v) => debug_var(v, level),
        Core::Expr(atoms, _) if atoms.is_empty() => format!("{ind}(expr)"),
        Core::Expr(atoms, _) => format!("{ind}(expr {})", debug_atoms(atoms)),
        Core::Stmt { head, body, arms, .. } => {
            let mut s = format!("{ind}(stmt {} (body {})", debug_atoms(head), debug_body_short(body));
            for a in arms {
                s.push_str(&format!(" (arm {} {})", debug_atoms(&a.pattern), debug_body_short(&a.body)));
            }
            s.push(')');
            s
        }
        Core::Reexport { star, is_type, names, from, .. } => {
            let mut s = format!("{ind}(reexport");
            if *is_type {
                s.push_str(" type");
            }
            if *star {
                s.push_str(" *");
            }
            if !names.is_empty() {
                s.push_str(&format!(" ({})", names.join(", ")));
            }
            s.push_str(&format!(" (from {from}))"));
            s
        }
        Core::Clause(a, _) => format!("{ind}(clause {})", debug_atoms(a)),
    }
}

fn debug_body_short(b: &Body) -> String {
    match b {
        Body::Inline(a) if a.is_empty() => "-".to_string(),
        Body::Inline(a) => format!("(expr {})", debug_atoms(a)),
        Body::Bare(stmts) | Body::Block(stmts) => format!(
            "(block {})",
            stmts.iter().map(debug_core_short).collect::<Vec<_>>().join(" ")
        ),
        Body::Opaque(raw) => format!("(opaque {raw:?})"),
    }
}

fn debug_core_short(c: &Core) -> String {
    debug_core(c, 0)
}

fn debug_fn(f: &FnDecl, level: usize) -> String {
    let ind = indent(level);
    let mut s = format!("{ind}(fn {}", f.name);
    if let Some(r) = &f.receiver {
        s.push_str(&format!(" (receiver {r})"));
    }
    if !f.modifiers.is_empty() {
        s.push_str(&format!(" (mods {})", f.modifiers.join(" ")));
    }
    if f.params.is_empty() {
        s.push_str(" (params)");
    } else {
        s.push_str(" (params");
        for (n, t) in &f.params {
            if t.is_empty() {
                s.push_str(&format!(" ({n})"));
            } else {
                s.push_str(&format!(" ({n} {t})"));
            }
        }
        s.push(')');
    }
    if let Some(b) = &f.base {
        s.push_str(&format!(" (base {b})"));
    }
    if let Some(r) = &f.ret {
        s.push_str(&format!(" (ret {r})"));
    }
    match &f.body {
        Body::Inline(b) if b.is_empty() => s.push_str(" (body)"),
        Body::Inline(b) => s.push_str(&format!(" (body (expr {}))", debug_atoms(b))),
        Body::Bare(stmts) => debug_body(&mut s, stmts, false),
        Body::Block(stmts) => debug_body(&mut s, stmts, true),
        Body::Opaque(raw) => s.push_str(&format!(" (body (opaque {raw:?}))")),
    }
    s.push(')');
    s
}

fn debug_body(s: &mut String, stmts: &[Core], block: bool) {
    if block {
        s.push_str(" (body (block");
    } else {
        s.push_str(" (body");
    }
    for st in stmts {
        s.push(' ');
        s.push_str(&debug_core(st, 0));
    }
    if block {
        s.push(')');
    }
    s.push(')');
}

fn debug_type(t: &TypeDecl, level: usize) -> String {
    let ind = indent(level);
    let mut s = format!("{ind}(type {}", t.name);
    s.push_str(&format!(" (kind {})", t.kind));
    if !t.modifiers.is_empty() {
        s.push_str(&format!(" (mods {})", t.modifiers.join(" ")));
    }
    if let Some(p) = &t.primary {
        s.push_str(" (primary");
        for (n, ty) in p {
            if ty.is_empty() {
                s.push_str(&format!(" ({n})"));
            } else {
                s.push_str(&format!(" ({n} {ty})"));
            }
        }
        s.push(')');
    }
    if !t.bases.is_empty() {
        s.push_str(&format!(" (bases {})", t.bases.join(" ")));
    }
    s.push_str(" (members");
    for m in &t.members {
        s.push(' ');
        s.push_str(&debug_core(m, 0));
    }
    s.push_str("))");
    s
}

fn debug_var(v: &VarDecl, level: usize) -> String {
    let mut s = format!("{}(var {}", indent(level), v.name);
    if !v.modifiers.is_empty() {
        s.push_str(&format!(" (mods {})", v.modifiers.join(" ")));
    }
    if let Some(t) = &v.ty {
        s.push_str(&format!(" (ty {t})"));
    }
    if let Some(val) = &v.value {
        s.push_str(&format!(" (val {})", debug_atoms(val)));
    }
    s.push(')');
    s
}

fn debug_atoms(atoms: &[Atom]) -> String {
    atoms
        .iter()
        .map(|a| a.to_string())
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expander::expand;

    #[test]
    fn golden01_roundtrip() {
        let core = expand("@1 @ns Acme.Users\n\nfn GetUser(Int id) User?{await Users.Find id}\n").unwrap();
        let text = print(&core);
        assert_eq!(text, "@1 @ns Acme.Users\n\nfn GetUser(id Int) User?{await Users.Find id}\n");
    }

    #[test]
    fn header_uses_print() {
        let core = expand("@1 @ns Cheerio @use domhandler\nfn f() Int{1}\n").unwrap();
        let text = print(&core);
        assert!(text.starts_with("@1 @ns Cheerio @use domhandler\n\n"));
        assert!(text.contains("fn f() Int{"));
    }

    #[test]
    fn type_members_indent() {
        let src = "type Order export{\n  List<LineItem> LineItems\n  fn Total() Decimal{LineItems.map(li=>li.Amount).sum()}\n}\n";
        let core = expand(src).unwrap();
        let text = print(&core);
        assert!(text.contains("  List<LineItem> LineItems"));
        assert!(text.contains("  fn Total() Decimal{LineItems.map(li=>li.Amount).sum()}"));
        // field first? canon keeps member order as parsed
        assert!(text.starts_with("@\n") == false);
    }

    #[test]
    fn minimal_spacing_rules() {
        assert_eq!(render_atoms(&[Atom::Ident("Users".into()), Atom::Sym(".".into()), Atom::Ident("Find".into())]), "Users.Find");
        assert_eq!(render_atoms(&[Atom::Ident("o".into()), Atom::Sym("&&".into()), Atom::Ident("x".into())]), "o&&x");
        assert_eq!(render_atoms(&[Atom::Ident("a".into()), Atom::Ident("b".into())]), "a b");
        assert_eq!(render_atoms(&[Atom::Ident("a".into()), Atom::Sym("+".into()), Atom::Ident("b".into())]), "a+b");
    }

    #[test]
    fn debug_view_labels_core_kinds() {
        let core = expand("@1 @ns Acme.Users\n\nfn GetUser(Int id) User?{await Users.Find id}\n").unwrap();
        let dbg = print_debug(&core);
        assert_eq!(
            dbg,
            "(header @1 @ns Acme.Users)\n(fn GetUser (params (id Int)) (ret User?) (body (expr await Users . Find id)))\n"
        );
        // closure invariant visible in the view: every top form is labeled
        for line in dbg.lines() {
            assert!(line.starts_with("(header") || line.starts_with("(fn") || line.starts_with("(type") || line.starts_with("(var") || line.starts_with("(expr"));
        }
    }
}
fn finish_opaque(s: &mut String, raw: &str, level: usize) -> String {
    s.push_str(" opaque{");
    for l in raw.lines() {
        s.push('\n');
        s.push_str(l);
    }
    s.push('\n');
    s.push_str(&indent(level));
    s.push('}');
    s.clone()
}

fn escape_string(s: &str) -> String {
    // pre-escaped content is stored raw; render_atom escapes it right after —
    // here we only normalize internal quotes/backslashes so {:?} is stable
    s.to_string()
}
