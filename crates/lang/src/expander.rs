//! Expander: reader output → core forms (design doc §2 table).
//!
//! The core table is CLOSED: `Core::kind()` enumerates every legal core form.
//! `expander::expand` returns `Err` if a construct classifies to nothing —
// the expander never emits a non-core form.

use crate::ast::{Atom, Form, Span};

#[derive(Debug, Clone, PartialEq)]
pub enum Core {
    /// file header: version, ns path, uses
    Header { version: String, ns: Option<String>, uses: Vec<String>, span: Span },
    /// fn or async fn decl
    Fn(FnDecl),
    /// type/class/interface/record/struct decl
    Type(TypeDecl),
    /// top-level or member `var name=expr` / `const var …` / `volatile var …`
    Var(VarDecl),
    /// a plain expression statement
    Expr(Vec<Atom>, Span),
    /// structured statement: control-flow head + body and/or arms.
    /// head atoms (e.g. `for x in xs`, `if p`); `else`/`elseif`/`catch`
    /// fragments carry their keyword as head[0].
    Stmt { head: Vec<Atom>, body: Body, arms: Vec<Arm>, span: Span },
    /// sibling association marker (internal: consumed pre-print)
    #[doc(hidden)]
    Clause(Vec<Atom>, Span),
}

/// one control-flow arm: pattern atoms (`"GET"`, `NotFound`, `Error e`,
/// `else`, `else if c`, empty for the primary body) + its body.
#[derive(Debug, Clone, PartialEq)]
pub struct Arm {
    pub pattern: Vec<Atom>,
    pub body: Body,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FnDecl {
    pub name: String,
    pub receiver: Option<String>, // `fn Type.name()` extension receiver
    pub params: Vec<(String, String)>,
    pub ret: Option<String>,
    pub modifiers: Vec<String>, // async, export, ctor
    pub base: Option<String>,   // ctor delegating base call
    pub body: Body,
    /// header-run span (decl line, incl. param continuations)
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Body {
    /// single expression fused to the header
    Inline(Vec<Atom>),
    /// braceless statement-per-line
    Bare(Vec<Core>),
    /// braced block
    Block(Vec<Core>),
    /// verbatim passthrough (fn … opaque{})
    Opaque(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypeDecl {
    pub name: String,
    pub kind: String,        // class | interface | record | struct (canon: words)
    pub primary: Option<Vec<(String, String)>>, // (name, type)
    pub bases: Vec<String>,
    pub modifiers: Vec<String>, // export, sealed, abstract
    pub members: Vec<Core>,
    /// header-run span (decl line)
    pub span: Span,
}

impl TypeDecl {
    pub fn is_signature_only(&self) -> bool {
        self.kind == "interface" || self.members.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct VarDecl {
    pub name: String,
    pub ty: Option<String>, // member fields may be typed
    pub value: Option<Vec<Atom>>,
    pub modifiers: Vec<String>, // const, volatile, mut, export, opaque, computed
    /// obj-literal body: `var x: T = {` members `}` — the reader models the
    /// `{`…`}` span as block children; empty value + children ⇒ obj-lit
    pub objlit: Option<Vec<Core>>,
    /// header-run span (decl line)
    pub span: Span,
}

/// zero span (synthetic form) — printers ignore spans; the map only needs
/// spans read off source
pub const SPAN_SYNTH: Span = Span { lo: 0, hi: 0 };

/// Core-form classification table (the closed set).
pub fn kind(c: &Core) -> &'static str {
    match c {
        Core::Header { .. } => "header",
        Core::Fn(_) => "fn",
        Core::Type(_) => "type",
        Core::Var(_) => "var",
        Core::Expr(..) => "expr",
        Core::Stmt { .. } => "stmt",
        Core::Clause(..) => "clause",
    }
}

/// Invariant (design doc §2): after expansion, no form is outside the table.
pub fn is_closed(forms: &[Core]) -> bool {
    forms.iter().all(|f| CORE_KINDS.contains(&kind(f)))
}

pub const CORE_KINDS: &[&str] = &["header", "fn", "type", "var", "expr", "stmt", "clause"];

#[derive(Debug, Clone)]
pub struct ExpandError {
    pub msg: String,
}

impl std::fmt::Display for ExpandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "expand: {}", self.msg)
    }
}

pub fn expand(src: &str) -> Result<Vec<Core>, ExpandError> {
    let forms = crate::reader::read(src).map_err(|e| ExpandError { msg: e.to_string() })?;
    let core: Vec<Core> = forms.iter().map(expand_form).collect::<Result<Vec<_>, _>>()?;
    Ok(associate(core))
}

/// Sibling association pass: reader models clause lines (catch/finally/else)
/// and match arms as siblings/children; fold them into their owning Stmt.
pub fn associate(core: Vec<Core>) -> Vec<Core> {
    let mut out: Vec<Core> = Vec::with_capacity(core.len());
    for c in core {
        match c {
            Core::Stmt { head, body, arms, span } => {
                let kw = head.first().map(|a| match a {
                    Atom::Ident(x) => x.clone(),
                    _ => String::new(),
                }).unwrap_or_default();
                let body = assoc_body(body);
                if kw == "try" {
                    out.push(Core::Stmt { head, body, arms, span });
                } else if kw == "match" {
                    let stmts = match &body {
                        Body::Bare(v) | Body::Block(v) => v.clone(),
                        Body::Inline(v) if !v.is_empty() => vec![Core::Expr(v.clone(), span)],
                        _ => vec![],
                    };
                    let mut all: Vec<Arm> = arms;
                    for st in stmts {
                        match st {
                            Core::Expr(a, sp) => {
                                if let Some(arm) = split_match_arm(&a, sp) {
                                    all.push(arm);
                                    continue;
                                }
                                all.push(Arm { pattern: Vec::new(), body: Body::Inline(a), span: sp });
                            }
                            Core::Stmt { head: h2, body: b2, arms: a2, span: sp } => {
                                if matches!(&h2[0], Atom::Ident(x) if x == "else") {
                                    if let Some(a3) = a2.first() {
                                        all.push(Arm { pattern: a3.pattern.clone(), body: a3.body.clone(), span: sp });
                                    } else {
                                        all.push(Arm { pattern: h2[1..].to_vec(), body: b2, span: sp });
                                    }
                                } else {
                                    all.push(Arm { pattern: Vec::new(), body: Body::Inline(h2), span: sp });
                                }
                            }
                            _ => {}
                        }
                    }
                    out.push(Core::Stmt { head, body: Body::Inline(Vec::new()), arms: all, span });
                } else {
                    out.push(Core::Stmt { head, body, arms, span });
                }
            }
            Core::Fn(mut f) => {
                f.body = assoc_body(f.body);
                out.push(Core::Fn(f));
            }
            Core::Type(mut t) => {
                t.members = associate(t.members);
                out.push(Core::Type(t));
            }
            other => out.push(other),
        }
    }
    fold_clauses(&out)
}

fn split_match_arm(a: &[Atom], span: Span) -> Option<Arm> {
    // corpus arm shapes: `"LIT" stmt…` (pattern = leading literal run)
    if matches!(&a[0], Atom::Str(_) | Atom::Number(_)) {
        return Some(Arm { pattern: a[..1].to_vec(), body: Body::Inline(a[1..].to_vec()), span });
    }
    None
}

fn assoc_body(b: Body) -> Body {
    match b {
        Body::Bare(v) => Body::Bare(assoc_list(associate(v))),
        Body::Block(v) => Body::Block(assoc_list(associate(v))),
        other => other,
    }
}

/// fold trailing `else`-fragment Stmts into the preceding `if` Stmt (the
/// reader splits `}else{` onto a sibling line)
fn assoc_list(core: Vec<Core>) -> Vec<Core> {
    let mut out: Vec<Core> = Vec::with_capacity(core.len());
    for c in core {
        let frag = match &c {
            Core::Stmt { head, .. } => {
                head.first().map(|a| matches!(a, Atom::Ident(x) if x == "else")).unwrap_or(false)
            }
            _ => false,
        };
        if let (true, Some(Core::Stmt { head: ih, arms: if_arms, .. })) =
            (frag, out.last_mut())
        {
            let is_if = ih.first().map(|a| matches!(a, Atom::Ident(x) if x == "if")).unwrap_or(false);
            if is_if {
                if let Core::Stmt { body: fb, arms: f_arms, .. } = &c {
                    let (pat, body) = match f_arms.first() {
                        Some(a3) if !a3.pattern.is_empty() => (a3.pattern.clone(), a3.body.clone()),
                        _ => (Vec::new(), fb.clone()),
                    };
                    let mut p = vec![Atom::Ident("else".into())];
                    p.extend(pat);
                    let fspan = match &c {
                        Core::Stmt { span, .. } => *span,
                        _ => SPAN_SYNTH,
                    };
                    if_arms.push(Arm { pattern: p, body, span: fspan });
                }
                continue;
            }
        }
        out.push(c);
    }
    out
}

/// fold sibling catch/finally Stmts that FOLLOW a try Stmt into its arms
fn fold_clauses(core: &[Core]) -> Vec<Core> {
    let mut out: Vec<Core> = Vec::with_capacity(core.len());
    let mut i = 0usize;
    while i < core.len() {
        let c = &core[i];
        if let Core::Stmt { head, body, arms, span } = c {
            let is_try = head.first().map(|a| matches!(a, Atom::Ident(x) if x == "try")).unwrap_or(false);
            if is_try {
                let mut all: Vec<Arm> = arms.clone();
                let mut j = i + 1;
                while j < core.len() {
                    let frag_kw = match &core[j] {
                        Core::Stmt { head: h, .. } => h.first().map(|a| match a {
                            Atom::Ident(x) => x.clone(),
                            _ => String::new(),
                        }).unwrap_or_default(),
                        _ => String::new(),
                    };
                    if matches!(frag_kw.as_str(), "catch" | "finally") {
                        if let Core::Stmt { head: h2, body: b2, arms: a2, span: fspan } = &core[j] {
                            // keep the clause keyword as pattern head so the
                            // printers can re-emit `catch P` / `finally`
                            if a2.is_empty() {
                                all.push(Arm { pattern: h2.clone(), body: b2.clone(), span: *fspan });
                            } else {
                                for mut arm in a2.iter().cloned() {
                                    let mut pat = h2.clone();
                                    pat.extend(arm.pattern.clone());
                                    arm.pattern = pat;
                                    all.push(arm);
                                }
                            }
                            j += 1;
                            continue;
                        }
                    }
                    break;
                }
                out.push(Core::Stmt { head: head.clone(), body: body.clone(), arms: all, span: *span });
                i = j;
                continue;
            }
        }
        out.push(c.clone());
        i += 1;
    }
    out
}

fn expand_form(f: &Form) -> Result<Core, ExpandError> {
    match f {
        Form::Directive(..) => {
            let atoms = match f {
                Form::Directive(a, _) => a.clone(),
                _ => unreachable!(),
            };
            let span = match f {
                Form::Directive(_, sp) => *sp,
                _ => unreachable!(),
            };
            Ok(expand_directives(&atoms, span)?)
        }
        Form::Line { .. } => expand_line(f),
    }
}

/// Split a directive atom run: `1 @ ns X [ @ use Y ]`
fn expand_directives(atoms: &[Atom], span: Span) -> Result<Core, ExpandError> {
    let mut version = String::new();
    let mut ns = None;
    let mut uses = Vec::new();
    let mut i = 0;
    while i < atoms.len() {
        let start = i;
        while i < atoms.len() && !matches!(&atoms[i], Atom::Sym(s) if s == "@") {
            i += 1;
        }
        let clause = &atoms[start..i];
        if clause.is_empty() {
            i += 1;
            continue;
        }
        match &clause[0] {
            Atom::Number(n) => {
                version = n.clone();
                i += 1;
            }
            Atom::Ident(kw) if kw == "ns" => {
                ns = Some(dotted(&clause[1..]));
                i += 1;
            }
            Atom::Ident(kw) if kw == "use" => {
                uses.push(dotted(&clause[1..]));
                i += 1;
            }
            other => {
                return Err(ExpandError {
                    msg: format!("unknown directive clause {other}"),
                })
            }
        }
    }
    Ok(Core::Header { version, ns, uses, span })
}

fn dotted(atoms: &[Atom]) -> String {
    let mut s = String::new();
    for a in atoms {
        match a {
            Atom::Ident(x) | Atom::Number(x) => s.push_str(x),
            Atom::Sym(x) => s.push_str(x),
            Atom::Str(x) => s.push_str(x),
            Atom::Regex(x) => {
                s.push('/');
                s.push_str(x)
            }
        }
    }
    s
}

/// dotted join, but Ident↔Ident get a single space (`out String` stays two
/// words; syms glue: `List<Int>`, `A.B`)
pub fn spaced(atoms: &[Atom]) -> String {
    let mut s = String::new();
    let mut prev_ident = false;
    for a in atoms {
        let cur_ident = matches!(a, Atom::Ident(_));
        if prev_ident && cur_ident {
            s.push(' ');
        }
        match a {
            Atom::Ident(x) | Atom::Number(x) => s.push_str(x),
            Atom::Sym(x) => s.push_str(x),
            Atom::Str(x) => s.push_str(x),
            Atom::Regex(x) => {
                s.push('/');
                s.push_str(x)
            }
        }
        prev_ident = cur_ident;
    }
    s
}

fn expand_line(f: &Form) -> Result<Core, ExpandError> {
    let (atoms, inline, children, span) = match f {
        Form::Line { atoms, inline_body, children, span } =>
            (atoms.clone(), inline_body.clone(), children.clone(), *span),
        _ => unreachable!(),
    };
    if atoms.is_empty() {
        return Ok(Core::Expr(Vec::new(), span));
    }
    let kw = match &atoms[0] {
        Atom::Ident(id) => id.clone(),
        Atom::Sym(s) if s == "@" => {
            return Ok(expand_directives(&atoms, span)?);
        }
        // `}else…` close-line: strip the leading `}` and classify the rest
        // as the else fragment (the reader linked close+else on one canon
        // line; the `}` belongs to the enclosing block, re-emitted canon-side)
        Atom::Sym(sx) if sx == "}" => {
            let rest = &atoms[1..];
            if rest.first().map(|a| matches!(a, Atom::Ident(x) if x == "else")).unwrap_or(false) {
                let inner_atoms: Vec<Atom> = rest.to_vec();
                return expand_stmt(&inner_atoms, inline, children, span);
            }
            return Ok(Core::Expr(atoms, span));
        }
        other => return Ok(Core::Expr({ let _ = other; atoms }, span)),
    };
    match kw.as_str() {
        "fn" | "async" | "export"
            if !(kw == "export"
                && !matches!(atoms.get(1), Some(Atom::Ident(k)) if k == "fn" || k == "async")) =>
        {
            // `fn …` / `async fn …` / canon `export fn …` (+ `export async fn`)
            // `export <other>` stays in the var branch (export type/const/…)
            let mut mods = Vec::new();
            let mut i = 0usize;
            if kw == "export" {
                mods.push("export".into());
                i += 1;
                if matches!(atoms.get(i), Some(Atom::Ident(k)) if k == "async") {
                    mods.push("async".into());
                    i += 1;
                }
            }
            // `async fn` (async-led) — handled above only via kw==async
            if kw == "async" {
                mods.push("async".into());
                i += 1;
            }
            if !matches!(atoms.get(i), Some(Atom::Ident(id)) if id == "fn") {
                return Err(ExpandError { msg: format!("fn missing fn token {i}") });
            }
            i += 1;
            let mut name = match atoms.get(i) {
                Some(Atom::Ident(n)) => {
                    i += 1;
                    n.clone()
                }
                other => return Err(ExpandError { msg: format!("fn missing name {other:?}") }),
            };
            let mut receiver = None;
            // skip type params `<T, U>` (depth scan)
            if matches!(atoms.get(i), Some(Atom::Sym(s)) if s == "<") {
                let mut d = 0i32;
                while i < atoms.len() {
                    if let Atom::Sym(s) = &atoms[i] {
                        if s == "<" { d += 1; }
                        if s == ">" { d -= 1; if d == 0 { i += 1; break; } }
                    }
                    i += 1;
                }
            }
            // receiver form: `Ident . Ident` at head
            if name != "fn" && matches!(&atoms.get(i), Some(Atom::Sym(s)) if s == ".") {
                receiver = Some(name);
                if let Some(Atom::Ident(m)) = atoms.get(i + 1) {
                    name = m.clone();
                    i += 2;
                } else {
                    return Err(ExpandError { msg: "receiver fn missing member name".into() });
                }
            }
            if mods.is_empty() && atoms.first().map(|a| matches!(a, Atom::Ident(x) if x == "export")).unwrap_or(false) {
                mods.push("export".into());
            }
            let params = parse_params(&atoms[i..])?;
            let (_, params_v) = params;
            let after = i + params.0;
            let ret_atoms: Vec<Atom> = atoms[after..]
                .iter()
                .filter(|a| match a {
                    Atom::Ident(x) => x != "opaque",
                    Atom::Str(s) => !s.starts_with("opaque:"),
                    _ => true,
                })
                .cloned()
                .collect();
            let ret = if ret_atoms.is_empty() {
                None
            } else {
                Some(dotted(ret_atoms.as_slice()))
            };
            if atoms.iter().any(|a| matches!(a, Atom::Ident(x) if x == "opaque")) {
                let raw = inline
                    .as_ref()
                    .and_then(|b| b.iter().find_map(|x| match x {
                        Atom::Str(s) if s.starts_with("opaque:") => Some(s[7..].to_string()),
                        _ => None,
                    }))
                    .or_else(|| atoms.iter().rev().find_map(|x| match x {
                        Atom::Str(s) if s.starts_with("opaque:") => Some(s[7..].to_string()),
                        _ => None,
                    }));
                if let Some(raw) = raw {
                    return Ok(Core::Fn(FnDecl {
                        name, receiver, params: params_v, ret, modifiers: mods,
                        base: None, body: Body::Opaque(raw), span,
                    }));
                }
            }
            let body = match (&inline, &children) {
                (Some(b), c) if c.is_empty() => Body::Inline(b.clone()),
                (None, c) if !c.is_empty() => Body::Bare(c.iter().map(expand_form).collect::<Result<Vec<_>, _>>()?),
                (None, _) => Body::Inline(Vec::new()),
                (Some(b), c) => {
                    let mut v = vec![Core::Expr(b.clone(), span)];
                    v.extend(c.iter().map(expand_form).collect::<Result<Vec<_>, _>>()?);
                    Body::Block(v)
                }
            };
            Ok(Core::Fn(FnDecl { name, receiver, params: params_v, ret, modifiers: mods, base: None, body, span }))
        }
        "type" => {
            let mut i = 1usize;
            let name = match atoms.get(i) {
                Some(Atom::Ident(n)) => n.clone(),
                other => return Err(ExpandError { msg: format!("type missing name {other:?}") }),
            };
            i += 1;
            let mut kind = String::from("class");
            let mut modifiers = Vec::new();
            loop {
                match atoms.get(i) {
                    Some(Atom::Ident(m)) if matches!(m.as_str(), "export" | "sealed" | "abstract") => {
                        modifiers.push(m.clone());
                        i += 1;
                    }
                    Some(Atom::Ident(k)) if matches!(k.as_str(), "interface" | "record" | "struct") => {
                        kind = k.clone();
                        i += 1;
                    }
                    _ => break,
                }
            }
            let mut primary = None;
            if matches!(atoms.get(i), Some(Atom::Sym(s)) if s == "(") {
                let (off, pv) = parse_params(&atoms[i..])?;
                primary = Some(pv);
                i += off;
            }
            let bases = match atoms.get(i) {
                Some(Atom::Sym(s)) if s == ":" => {
                    let base_start = i + 1;
                    match atoms[base_start..].iter().position(|a| matches!(a, Atom::Sym(x) if x == "{")) {
                        Some(end) if end > 0 => {
                            let b: Vec<String> = split_commas(&atoms[base_start..base_start + end]).iter().map(|g| dotted(g)).collect();
                            i = base_start + end + 1;
                            let _ = i;
                            b
                        }
                        _ => {
                            let b: Vec<String> = split_commas(&atoms[base_start..]).iter().map(|g| dotted(g)).collect();
                            i = atoms.len();
                            let _ = i;
                            b
                        }
                    }
                }
                Some(Atom::Sym(s)) if s == "{" => {
                    i += 1;
                    let _ = i;
                    Vec::new()
                }
                Some(_) => Vec::new(),
                None => Vec::new(),
            };
            let mut members = Vec::new();
            for c in &children {
                let mc = expand_member(c)?;
                members.push(mc);
            }
            Ok(Core::Type(TypeDecl { name, kind, primary, bases, modifiers, members, span }))
        }
        "var" | "const" | "volatile" | "export" => {
            let mut mods = Vec::new();
            let mut i = if kw == "export" {
                mods.push("export".into());
                match atoms.get(1) {
                    Some(Atom::Ident(second)) => {
                        if matches!(second.as_str(), "const" | "volatile") {
                            mods.push(second.clone());
                            2
                        } else if second == "var" {
                            2
                        } else {
                            if atoms.len() > 1 && matches!(atoms.get(1), Some(Atom::Sym(_))) {
                                return Ok(Core::Expr(atoms, span));
                            }
                            1
                        }
                    }
                    _ => return Ok(Core::Expr(atoms, span)),
                }
            } else {
                if kw != "var" {
                    mods.push(kw.clone());
                }
                1
            };
            if atoms.get(i).map(|a| matches!(a, Atom::Ident(x) if x == "var")).unwrap_or(false) {
                i += 1;
            }
            // grammar: `["const"|"volatile"] "var" ["mut"] ident` — `mut` sits
            // after the var word (canon); drives the emitter's `let` choice
            if atoms.get(i).map(|a| matches!(a, Atom::Ident(x) if x == "mut")).unwrap_or(false) {
                mods.push("mut".into());
                i += 1;
            }
            #[allow(unused_assignments)]
            let mut name = String::new();
            if let Some(Atom::Ident(n)) = atoms.get(i) {
                name = n.clone();
                i += 1;
                let _ = name;
            } else {
                return Err(ExpandError { msg: "var missing name".into() });
            }
            let mut ty = None;
            if !matches!(atoms.get(i), Some(Atom::Sym(s)) if s == "=") {
                if matches!(atoms.get(i), Some(Atom::Sym(s)) if s == ":") {
                    let ts = i + 1;
                    match atoms[ts..].iter().position(|a| matches!(a, Atom::Sym(x) if x == "=")) {
                        Some(end) => {
                            ty = Some(dotted(&atoms[ts..ts + end]));
                            i = ts + end + 1;
                        }
                        None => ty = Some(dotted(&atoms[ts..])),
                    }
                }
            }
            let value = match atoms.get(i) {
                Some(Atom::Sym(s)) if s == "=" => Some(atoms[i + 1..].to_vec()),
                _ => None,
            };
            // `var X = {` + children + `} as const`-class obj-lits: reader
            // puts member lines as children; value = Some([]) empty or None
            let objlit = if !children.is_empty() {
                Some(children.iter().map(expand_form).collect::<Result<Vec<_>, _>>()?)
            } else {
                None
            };
            Ok(Core::Var(VarDecl { name, ty, value, modifiers: mods, objlit, span }))
        }
        _ => {
            let is_flow = matches!(kw.as_str(), "if" | "for" | "while" | "lock" | "match" | "try" | "else" | "catch" | "finally");
            let has_else = atoms.iter().any(|a| matches!(a, Atom::Ident(x) if x == "else"));
            if is_flow {
                return expand_stmt(&atoms, inline, children, span);
            }
            if has_else {
                // brace-glued compound head from canon: `if p{a=1}else …`
                let st = split_glued_if(&atoms, inline.clone(), children.clone(), span)?;
                if let Some(st) = st {
                    return Ok(st);
                }
            }
            // `X switch{arms}` trailing keyword: expr-positional match
            if atoms.len() > 1
                && matches!(&atoms[atoms.len() - 1], Atom::Ident(x) if x == "switch")
            {
                let head_atoms = atoms.clone();
                let arms = split_switch_arms(&children)?;
                return Ok(Core::Stmt { head: head_atoms, body: Body::Inline(Vec::new()), arms, span });
            }
            Ok(Core::Expr(atoms, span))
        }
    }
}

/// Classify a control-flow line (or else/catch/finally fragment) into a
/// structured `Core::Stmt`.
fn expand_stmt(
    atoms: &[Atom],
    inline: Option<Vec<Atom>>,
    children: Vec<Form>,
    span: Span,
) -> Result<Core, ExpandError> {
    let kw = match &atoms[0] {
        Atom::Ident(id) => id.clone(),
        _ => return Ok(Core::Expr(atoms.to_vec(), span)),
    };
    if matches!(kw.as_str(), "try" | "catch" | "finally" | "else") {
        let mut body_atoms: Vec<Atom> = Vec::new();
        let mut pat: Vec<Atom> = Vec::new();
        if kw == "try" {
            body_atoms = atoms[1..].to_vec();
        } else if kw == "catch" {
            // `catch Pat binder? fused-stmt` — binder = idents after the
            // pattern up to a stmt keyword
            let mut i = 1usize;
            while i < atoms.len() {
                if let Atom::Ident(x) = &atoms[i] {
                    if matches!(x.as_str(), "return" | "throw" | "await" | "if" | "for" | "while" | "match") {
                        break;
                    }
                }
                i += 1;
            }
            pat = atoms[1..i].to_vec();
            body_atoms = atoms[i..].to_vec();
        } else if kw == "finally" {
            body_atoms = atoms[1..].to_vec();
        } else {
            // else / else if c{…} / else <expr>:
            // - `else if …` → pattern = cond; an inline brace-GLUED body
            //   (`{stmt}` inside the atom run) becomes the arm body
            // - `else <expr>` → whole run is the body
            if atoms.len() > 1 && matches!(&atoms[1], Atom::Ident(x) if x == "if") {
                // cond runs up to a top-level `{`
                let mut end = atoms.len();
                let mut d = 0i32;
                for (k, a) in atoms.iter().enumerate().skip(2) {
                    if let Atom::Sym(sx) = a {
                        match sx.as_str() {
                            "{" if d == 0 => {
                                end = k;
                                break;
                            }
                            "(" | "[" => d += 1,
                            ")" | "]" => d -= 1,
                            _ => {}
                        }
                    }
                }
                pat = atoms[1..end].to_vec(); // includes the `if`
                // brace-glued inline body after the cond?
                if end < atoms.len() && matches!(&atoms[end], Atom::Sym(sx) if sx == "{") {
                    // `{…}` balanced inline → body
                    let mut d2 = 0i32;
                    let mut close = None;
                    for (k, a) in atoms.iter().enumerate().skip(end) {
                        if let Atom::Sym(sx) = a {
                            if sx == "{" { d2 += 1; }
                            if sx == "}" {
                                d2 -= 1;
                                if d2 == 0 {
                                    close = Some(k);
                                    break;
                                }
                            }
                        }
                    }
                    if let Some(cc) = close {
                        body_atoms = atoms[end + 1..cc].to_vec();
                    }
                }
            } else if atoms.len() > 1 {
                body_atoms = atoms[1..].to_vec();
            }
        }
        let inline_body = if body_atoms.is_empty() { inline } else { Some(body_atoms) };
        let body = match (&inline_body, &children) {
            (Some(b), c) if c.is_empty() => Body::Inline(b.clone()),
            (None, c) if !c.is_empty() => Body::Bare(c.iter().map(expand_form).collect::<Result<Vec<_>, _>>()?),
            (Some(b), c) => {
                let mut v = vec![Core::Expr(b.clone(), span)];
                v.extend(c.iter().map(expand_form).collect::<Result<Vec<_>, _>>()?);
                Body::Block(v)
            }
            (None, _) => Body::Inline(Vec::new()),
        };
        let head_atoms = vec![Atom::Ident(kw)];
        // normalize: an empty-pattern arm IS the clause body (try/finally)
        if pat.is_empty() {
            return Ok(Core::Stmt { head: head_atoms, body, arms: Vec::new(), span });
        }
        let arms = vec![Arm { pattern: pat, body, span }];
        return Ok(Core::Stmt { head: head_atoms, body: Body::Inline(Vec::new()), arms, span });
    }
    let body = match (&inline, &children) {
        (Some(b), c) if c.is_empty() => Body::Inline(b.clone()),
        (None, c) if !c.is_empty() => Body::Bare(c.iter().map(expand_form).collect::<Result<Vec<_>, _>>()?),
        (Some(b), c) => {
            let mut v = vec![Core::Expr(b.clone(), span)];
            v.extend(c.iter().map(expand_form).collect::<Result<Vec<_>, _>>()?);
            Body::Block(v)
        }
        (None, _) => Body::Inline(Vec::new()),
    };
    // else-if/else fragments among the CHILDREN = arms of this stmt — but
    // only where an `else` can genuinely belong to THIS construct (`if`/`match`
    // bodies); for other kw the fragments are siblings that belong to the
    // preceding `if` (assoc_list folds them — e.g. `while …{ if a{…}
    // else if b{…} }` must attach to the NESTED if, not the while)
    let mut arms: Vec<Arm> = Vec::new();
    let mut body = body;
    if matches!(kw.as_str(), "if" | "match") {
        match &mut body {
            Body::Bare(v) | Body::Block(v) => {
                let mut kept: Vec<Core> = Vec::new();
                for st in v.drain(..) {
                    if let Core::Stmt { head: h2, body: b2, arms: a2, span: fspan } = st {
                        let h0 = h2.first().map(|a| matches!(a, Atom::Ident(x) if x == "else")).unwrap_or(false);
                        if h0 {
                            let (pat, bod) = match a2.first() {
                                Some(a3) if !a3.pattern.is_empty() => (a3.pattern.clone(), a3.body.clone()),
                                _ => (Vec::new(), b2),
                            };
                            let mut p = vec![Atom::Ident("else".into())];
                            p.extend(pat);
                            arms.push(Arm { pattern: p, body: bod, span: fspan });
                            continue;
                        }
                        kept.push(Core::Stmt { head: h2, body: b2, arms: a2, span: fspan });
                    } else {
                        kept.push(st);
                    }
                }
                *v = kept;
            }
            _ => {}
        }
    }
    Ok(Core::Stmt { head: atoms.to_vec(), body, arms, span })
}

/// Canon shape `if c{then…}else{…}` — reader keeps `{then}` + `else` fused in
/// the atom run with the else body as `inline`.
fn split_glued_if(
    atoms: &[Atom],
    inline: Option<Vec<Atom>>,
    _children: Vec<Form>,
    span: Span,
) -> Result<Option<Core>, ExpandError> {
    let mut paren = 0i32;
    let mut open = None;
    let mut close = None;
    for (k, a) in atoms.iter().enumerate() {
        if let Atom::Sym(s) = a {
            match s.as_str() {
                "(" | "[" => paren += 1,
                ")" | "]" => paren -= 1,
                "{" if paren == 0 && open.is_none() => open = Some(k),
                "}" if paren == 0 => close = Some(k),
                _ => {}
            }
        }
    }
    let (Some(oi), Some(ci)) = (open, close) else {
        return Ok(None);
    };
    let cond = atoms[1..oi].to_vec();
    let then_body = Body::Inline(atoms[oi + 1..ci].to_vec());
    let after = &atoms[ci + 1..];
    let mut arms = vec![Arm { pattern: Vec::new(), body: then_body, span }];
    if after.first().map(|a| matches!(a, Atom::Ident(x) if x == "else")).unwrap_or(false) {
        let pat = after[1..].to_vec();
        arms.push(Arm { pattern: pat, body: Body::Inline(inline.unwrap_or_default()), span });
    }
    Ok(Some(Core::Stmt { head: [vec![Atom::Ident("if".into())], cond].concat(), body: Body::Inline(Vec::new()), arms, span }))
}

/// `X switch{}` arms (`ICollection<T> items new(items.Count)` / `_ x` / `+1 f()`)
fn split_switch_arms(children: &[Form]) -> Result<Vec<Arm>, ExpandError> {
    let mut arms = Vec::new();
    for c in children {
        let (atoms, child_span) = match c {
            Form::Line { atoms, inline_body, span, .. } => {
                match inline_body {
                    Some(b) if !b.is_empty() => {
                        let _ = (atoms, b);
                        continue;
                    }
                    _ => (atoms.clone(), *span),
                }
            }
            _ => continue,
        };
        if atoms.is_empty() {
            continue;
        }
        // `_ …` / `else …` default arm
        if matches!(&atoms[0], Atom::Ident(x) if x == "_" || x == "else") {
            arms.push(Arm { pattern: vec![Atom::Sym("else".into())], body: Body::Inline(atoms[1..].to_vec()), span: child_span });
            continue;
        }
        // pattern = dotted/type/member run (`ICollection<T>`, `PP.BeforeForm`,
        // `+1`); a BARE capitalized call-head with no dot-prefix = body;
        // binder = lowercase ident
        let mut k = 0usize;
        for (idx, a) in atoms.iter().enumerate() {
            match a {
                Atom::Sym(s) if matches!(s.as_str(), "+" | "-") => {
                    k = idx + 1;
                }
                Atom::Number(_) => {
                    k = idx + 1;
                    let _ = k;
                }
                Atom::Sym(s) if matches!(s.as_str(), "<" | ">" | "?" | "[" | "]" | "|" | ".") => {
                    k = idx + 1;
                    let _ = k;
                }
                Atom::Ident(x) => {
                    if idx > 0 && x.chars().next().is_some_and(|ch| ch.is_lowercase()) {
                        k = idx;
                        break;
                    }
                    let dotted_here = idx > 0 && matches!(&atoms[idx - 1], Atom::Sym(s2) if s2 == ".");
                    if !dotted_here && matches!(atoms.get(idx + 1), Some(Atom::Sym(s2)) if s2 == "(") {
                        // bare call-head = body start (not a dotted member)
                        k = idx;
                        break;
                    }
                    if x == "new" {
                        k = idx;
                        break;
                    }
                    k = idx + 1;
                }
                _ => {
                    k = idx;
                    break;
                }
            }
        }
        let (pat, body) = if k < atoms.len() && k > 0 {
            // break atom decides: `(`/`[`/`new` = call-head (body starts AT
            // k), lowercase ident = binder (body starts at k+1)
            let skip = if matches!(&atoms[k], Atom::Sym(s) if s == "(" || s == "[")
                || matches!(&atoms[k], Atom::Ident(x) if x == "new")
            {
                0
            } else {
                1
            };
            (atoms[..k].to_vec(), Body::Inline(atoms[k + skip..].to_vec()))
        } else {
            (atoms[..k.min(atoms.len())].to_vec(), Body::Inline(Vec::new()))
        };
        arms.push(Arm { pattern: pat, body, span: child_span });
    }
    Ok(arms)
}

fn expand_member(f: &Form) -> Result<Core, ExpandError> {
    let (atoms, inline, children, span) = match f {
        Form::Line { atoms, inline_body, children, span } =>
            (atoms.clone(), inline_body.clone(), children.clone(), *span),
        _ => unreachable!(),
    };
    if atoms.is_empty() {
        return Ok(Core::Expr(Vec::new(), span));
    }
    let kw = match &atoms[0] {
        Atom::Ident(id) => id.clone(),
        other => return Err(ExpandError { msg: format!("member start {other}") }),
    };
    match kw.as_str() {
        "fn" | "async" | "type" => expand_line(f),
        "record" | "struct" if !inline.is_some() => {
            let name = match atoms.get(1) {
                Some(Atom::Ident(n)) => n.clone(),
                _ => String::new(),
            };
            let mut members: Vec<Core> = Vec::new();
            for c in &children {
                let (ca, cspan) = match c {
                    crate::ast::Form::Line { atoms, span, .. } =>
                        (atoms.clone(), *span),
                    _ => continue,
                };
                if ca.is_empty() {
                    continue;
                }
                let mut nm = String::new();
                let k = 0usize;
                let _ = k;
                let mut j = 0usize;
                while j < ca.len() {
                    if let Atom::Ident(n) = &ca[j] {
                        if nm.is_empty() {
                            nm = n.clone();
                        }
                    }
                    j += 1;
                }
                members.push(Core::Var(crate::expander::VarDecl {
                    name: nm,
                    ty: None,
                    value: None,
                    modifiers: vec![],
                    objlit: None,
                    span: cspan,
                }));
            }
            Ok(Core::Type(TypeDecl {
                name,
                kind: kw.clone(),
                primary: None,
                bases: vec![],
                modifiers: vec![],
                members,
                span,
            }))
        }
        "ctor" => {
            let (off, params_v) = parse_params(&atoms[1..])?;
            let after = 1 + off;
            let mut base = None;
            let mut head_end = atoms.len();
            if matches!(atoms.get(after), Some(Atom::Sym(s)) if s == ":") {
                let bs = after + 1;
                match atoms[bs..].iter().position(|a| matches!(a, Atom::Sym(x) if x == "{")) {
                    Some(be) => {
                        base = Some(spaced(&atoms[bs..bs + be]));
                        head_end = bs + be;
                    }
                    None => base = Some(spaced(&atoms[bs..])),
                }
            }
            let body = match (&inline, &children) {
                (Some(b), _) => Body::Inline(b.clone()),
                (None, c) => Body::Bare(c.iter().map(expand_form).collect::<Result<Vec<_>, _>>()?),
            };
            let _ = head_end;
            Ok(Core::Fn(FnDecl {
                name: "<ctor>".into(),
                receiver: None,
                params: params_v,
                ret: None,
                modifiers: vec!["ctor".into()],
                base,
                body,
                span,
            }))
        }
        "opaque" if matches!(atoms.get(1), Some(Atom::Ident(k)) if matches!(k.as_str(), "var" | "const")) => {
            let mut i = 2usize;
            if atoms.get(i).map(|a| matches!(a, Atom::Ident(x) if x == "var" || x == "const")).unwrap_or(false) {
                i += 1;
            }
            let name = match atoms.get(i) {
                Some(Atom::Ident(n)) => n.clone(),
                _ => return Err(ExpandError { msg: "opaque var missing name".into() }),
            };
            i += 1;
            let mut ty = None;
            if matches!(atoms.get(i), Some(Atom::Sym(s)) if s == ":") {
                let ts = i + 1;
                match atoms[ts..].iter().position(|a| matches!(a, Atom::Sym(x) if x == "=")) {
                    Some(e) => {
                        ty = Some(spaced(&atoms[ts..ts + e]));
                        i = ts + e + 1;
                    }
                    None => ty = Some(spaced(&atoms[ts..])),
                }
            }
            let value = match atoms.get(i) {
                Some(Atom::Sym(s)) if s == "=" => Some(atoms[i + 1..].to_vec()),
                _ => None,
            };
            Ok(Core::Var(VarDecl { name, ty, value, modifiers: vec!["opaque".into()], objlit: None, span }))
        }
        "opaque" if matches!(atoms.get(1), Some(Atom::Ident(k)) if matches!(k.as_str(), "struct" | "record" | "interface" | "type")) => {
            let name = match atoms.get(2) {
                Some(Atom::Ident(n)) => n.clone(),
                _ => return Err(ExpandError { msg: "nested type missing name".into() }),
            };
            let kind = match atoms.get(1) {
                Some(Atom::Ident(k)) => k.clone(),
                _ => unreachable!(),
            };
            let (off, pv) = parse_params(&atoms[3..])?;
            let mut members = Vec::new();
            for (n, ty) in &pv {
                members.push(Core::Var(VarDecl {
                    name: n.clone(),
                    ty: Some(ty.clone()),
                    value: None,
                    modifiers: vec![],
                    objlit: None,
                    span: SPAN_SYNTH, // synthesized from param list, not source lines
                }));
            }
            let _ = off;
            Ok(Core::Type(TypeDecl { name, kind, primary: None, bases: vec![], modifiers: vec![], members, span }))
        }
        _ => {
            // field `Type name[= x]` / property `Type name => expr`
            let split = atoms.iter().enumerate()
                .find(|(_, a)| matches!(a, Atom::Sym(s) if s == "=" || s == "=>"))
                .map(|(k, _)| k)
                .unwrap_or(atoms.len());
            if split == 0 || !matches!(&atoms[split - 1], Atom::Ident(_)) {
                return Err(ExpandError { msg: "member field missing name".into() });
            }
            let name_at = split - 1;
            let name = match &atoms[name_at] {
                Atom::Ident(n) => n.clone(),
                _ => unreachable!(),
            };
            let ty_atoms = &atoms[..name_at];
            let ty_str = if ty_atoms.is_empty() { None } else { Some(spaced(ty_atoms)) };
            let (value, is_prop) = match atoms.get(split) {
                Some(Atom::Sym(s)) if s == "=" => (Some(atoms[split + 1..].to_vec()), false),
                Some(Atom::Sym(s)) if s == "=>" => (Some(atoms[split + 1..].to_vec()), true),
                _ => (None, false),
            };
            Ok(Core::Var(VarDecl { name, ty: ty_str, value, modifiers: if is_prop { vec!["computed".into()] } else { vec![] }, objlit: None, span }))
        }
    }
}

/// parse `(name Type [ = default], …)` starting at `(`; returns (consumed, params)
fn parse_params(atoms: &[Atom]) -> Result<(usize, Vec<(String, String)>), ExpandError> {
    if !matches!(atoms.first(), Some(Atom::Sym(s)) if s == "(") {
        return Err(ExpandError { msg: "expected ( in param list".into() });
    }
    let mut depth = 0i32;
    let mut close = None;
    for (k, a) in atoms.iter().enumerate() {
        if let Atom::Sym(s) = a {
            match s.as_str() {
                "(" => depth += 1,
                ")" => {
                    depth -= 1;
                    if depth == 0 {
                        close = Some(k);
                        break;
                    }
                }
                _ => {}
            }
        }
    }
    let ci = close.ok_or_else(|| ExpandError { msg: "unclosed params".into() })?;
    let inner = &atoms[1..ci];
    let groups = split_commas(inner);
    let mut params = Vec::new();
    for g in groups {
        if g.is_empty() {
            continue;
        }
        let mut name = String::new();
        let mut ty = String::new();
        let mut default: Option<Vec<Atom>> = None;
        // param default `= x`: split before name/type parsing
        let top_eq = g.iter().enumerate().find(|(k, a)| {
            matches!(a, Atom::Sym(s2) if s2 == "=")
                && g[..*k].iter().filter(|b| matches!(b, Atom::Sym(x2) if x2 == "(" || x2 == ")")).count() % 2 == 0
        });
        let g_main: Vec<Atom> = match top_eq {
            Some((ei, _)) => {
                default = Some(g[ei + 1..].to_vec());
                g[..ei].to_vec()
            }
            None => g.clone(),
        };
        let g = &g_main;
        if matches!(&g[0], Atom::Ident(id) if id == "out" || id == "ref" || id == "in") {
            // modifier prefix: `out Type name` — name = lowercase-tail ident
            let rest: Vec<Atom> = g[1..].to_vec();
            let caps = |s2: &str| s2.chars().next().is_some_and(|c| c.is_uppercase());
            let ids: Vec<(usize, String)> = rest.iter().enumerate()
                .filter(|(_, a)| matches!(a, Atom::Ident(_)))
                .map(|(i, a)| (i, match a { Atom::Ident(n) => n.clone(), _ => unreachable!() }))
                .collect();
            match ids.last() {
                Some((li, ln)) if !caps(ln) && ids.len() > 1 => {
                    name = ln.clone();
                    ty = format!("out {}", spaced(&rest[..*li]));
                }
                Some((_, ln)) => {
                    name = ln.clone();
                    ty = "out".into();
                }
                None => { name = "out".into(); }
            }
        } else {
            let top_colon = g.iter().enumerate().find(|(k, a)| {
                matches!(a, Atom::Sym(s2) if s2 == ":")
                    && g[..*k].iter().filter(|b| matches!(b, Atom::Sym(x2) if x2 == "(" || x2 == ")")).count() % 2 == 0
            });
            if let Some((ci2, _)) = top_colon {
                name = dotted(&g[..ci2]);
                ty = spaced(&g[ci2 + 1..]);
            } else {
                let caps = |s2: &str| s2.chars().next().is_some_and(|c| c.is_uppercase());
                let ids: Vec<(usize, String)> = g.iter().enumerate()
                    .filter(|(_, a)| matches!(a, Atom::Ident(_)))
                    .map(|(i, a)| (i, match a { Atom::Ident(n) => n.clone(), _ => unreachable!() }))
                    .collect();
                match ids.len() {
                    0 => { ty = dotted(g); }
                    1 => { name = ids[0].1.clone(); }
                    _ => {
                        let last = ids.last().unwrap();
                        if !caps(&last.1) {
                            name = last.1.clone();
                            ty = spaced(&g[..last.0]);
                        } else {
                            name = ids[0].1.clone();
                            ty = spaced(&g[ids[0].0 + 1..]);
                        }
                    }
                }
            }
        }
        // default re-attached for canon stability; emitter ignores types
        if let Some(d) = &default {
            ty = format!("{ty} = {}", crate::printer::render_atoms(d).trim_end());
        }
        params.push((name, ty));
    }
    Ok((ci + 1, params))
}

/// split an atom run on top-level commas ((), [] and <> all nest)
fn split_commas(atoms: &[Atom]) -> Vec<Vec<Atom>> {
    let mut out: Vec<Vec<Atom>> = vec![Vec::new()];
    let mut depth = 0i32;
    for a in atoms {
        if let Atom::Sym(s) = a {
            match s.as_str() {
                "(" | "[" | "<" => depth += 1,
                ")" | "]" | ">" => depth -= 1,
                "," if depth == 0 => {
                    out.push(Vec::new());
                    continue;
                }
                _ => {}
            }
        }
        out.last_mut().unwrap().push(a.clone());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn golden_01_expands() {
        let core = expand("@1 @ns Acme.Users\n\nfn GetUser(Int id) User?{await Users.Find id}\n").unwrap();
        assert_eq!(core.len(), 2);
        assert!(is_closed(&core));
        assert_eq!(kind(&core[0]), "header");
        assert_eq!(kind(&core[1]), "fn");
        match &core[1] {
            Core::Fn(f) => {
                assert_eq!(f.name, "GetUser");
                assert_eq!(f.params.len(), 1);
                assert_eq!(f.params[0].0, "id");
                assert_eq!(f.params[0].1, "Int");
                assert_eq!(f.ret.as_deref(), Some("User?"));
            }
            _ => panic!(),
        }
    }

    #[test]
    fn golden_02_expands() {
        let src = "\
@1 @ns Acme.Orders

type Order export{
  List<LineItem> LineItems
  Customer Customer
  fn Total() Decimal{LineItems.map(li=>li.Amount).sum()}
}
";
        let core = expand(src).unwrap();
        assert!(is_closed(&core));
        match &core[1] {
            Core::Type(t) => {
                assert_eq!(t.name, "Order");
                assert!(t.modifiers.contains(&"export".to_string()));
                assert_eq!(t.members.len(), 3);
            }
            _ => panic!(),
        }
    }

    #[test]
    fn unknown_keyword_rejected() {
        assert!(expand("type\nfn f() Int{1}\n").is_err());
        let core = expand("frob x y\n").unwrap();
        assert_eq!(kind(&core[0]), "expr");
    }

    #[test]
    fn directive_with_uses() {
        let core = expand("@1 @ns Cheerio @use domhandler\n").unwrap();
        match &core[0] {
            Core::Header { ns, uses, .. } => {
                assert_eq!(ns.as_deref(), Some("Cheerio"));
                assert_eq!(uses, &vec!["domhandler".to_string()]);
            }
            _ => panic!(),
        }
    }

    #[test]
    fn stmt_if_else_expands() {
        let core = expand("@1\n\nfn f() Int{\n  if p{\n    a=1\n  }else{\n    b=2\n  }\n}\n").unwrap();
        let Core::Fn(f) = &core[1] else { panic!() };
        let Body::Bare(stmts) = &f.body else { panic!() };
        assert!(matches!(stmts[0], Core::Stmt { .. }));
        match &stmts[0] {
            Core::Stmt { body, arms, .. } => {
                assert!(matches!(body, Body::Bare(_)));
                assert_eq!(arms.len(), 1); // else-arm
            }
            _ => panic!(),
        }
    }

    #[test]
    fn match_arms_associate() {
        let core = expand(
            "fn Handle(Request r) Response{\n  match r.Kind{\n    \"GET\" Read(r.Path)\n    else Response.Err 405 5\n  }\n}\n",
        ).unwrap();
        let Core::Fn(f) = &core[0] else { panic!() };
        let Body::Bare(stmts) = &f.body else { panic!() };
        match &stmts[0] {
            Core::Stmt { arms, .. } => {
                assert_eq!(arms.len(), 2);
            }
            _ => panic!(),
        }
    }

    #[test]
    fn try_catch_folds() {
        let core = expand(
            "async fn Load(Int id) User?{\n  try await Repo.Get id\n  catch NotFound return null\n  catch Error e throw e\n  finally Log(\"done\")\n}\n",
        ).unwrap();
        let Core::Fn(f) = &core[0] else { panic!() };
        let Body::Bare(stmts) = &f.body else { panic!() };
        assert_eq!(stmts.len(), 1, "catch/finally siblings fold into the try");
        match &stmts[0] {
            Core::Stmt { arms, .. } => assert_eq!(arms.len(), 3),
            _ => panic!(),
        }
    }

    #[test]
    fn else_chain_attaches_to_nested_if_not_enclosing_loop() {
        // loop-test finding (qwen draft): `if a{…}` inline-closed + deep-indented
        // `else if` lines must fold into the nested if, NOT the enclosing while
        let core = expand(
            "fn F(String u) Int{\n  var mut x=0\n  while u.length>0{\n    if u == 'h' { x=1 }\n    else if u == 'm' { x=2 }\n    else if u == 's' { x=3 }\n  }\n  return x\n}\n",
        ).unwrap();
        let Core::Fn(f) = &core[0] else { panic!() };
        let Body::Bare(stmts) = &f.body else { panic!() };
        let Core::Stmt { arms, body, .. } = &stmts[1] else { panic!() };
        assert!(arms.is_empty(), "while must not swallow else frags: {} arms", arms.len());
        let Body::Bare(inner) = body else { panic!() };
        match &inner[0] {
            Core::Stmt { arms: if_arms, .. } => assert_eq!(if_arms.len(), 2, "else-ifs belong to the nested if"),
            _ => panic!(),
        }
    }
}
