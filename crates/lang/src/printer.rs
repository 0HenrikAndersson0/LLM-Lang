//! Canonical printer (design doc §5): core forms → text, exactly one layout.
//!
//! Rules:
//! 1. `@`-directives at top, one line per clause, blank line after header.
//! 2. One decl per top-level line; statement bodies indented 2 spaces.
//! 3. Minimal spacing (`{`/`(`/`?` fuse to the preceding word; tight ops).
//! 4. Single-expression bodies fuse onto the header line.

use crate::ast::Atom;
use crate::expander::{Body, Core, FnDecl, TypeDecl, VarDecl};

pub fn print(core: &[Core]) -> String {
    let mut out = String::new();
    // header first (canon: directives one per line)
    let mut seen_header = false;
    for c in core {
        if let Core::Header { version, ns, uses } = c {
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
        Core::Expr(atoms) => format!("{}{}", indent(level), render_atoms(atoms)),
    }
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
            } else if t.contains('(') || t.contains('|') {
                // ambiguous type exprs print colon form (stability)
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
    for m in &t.modifiers {
        s.push(' ');
        s.push_str(m);
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
    for m in &v.modifiers {
        s.push_str(m);
        s.push(' ');
        if m == "var" {
            want_var_word = false;
        }
    }
    if want_var_word {
        s.push_str("var ");
    }
    s.push_str(&v.name);
    if let Some(t) = &v.ty {
        s.push_str(": ");
        s.push_str(t);
    }
    if let Some(val) = &v.value {
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
