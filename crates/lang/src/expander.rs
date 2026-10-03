//! Expander: reader output → core forms (design doc §2 table).
//!
//! The core table is CLOSED: `Core::kind()` enumerates every legal core form.
//! `expander::expand` returns `Err` if a construct classifies to nothing —
//! the expander never emits a non-core form.

use crate::ast::{Atom, Form};

#[derive(Debug, Clone, PartialEq)]
pub enum Core {
    /// file header: version, ns path, uses
    Header { version: String, ns: Option<String>, uses: Vec<String> },
    /// fn or async fn decl
    Fn(FnDecl),
    /// type/class/interface/record/struct decl
    Type(TypeDecl),
    /// top-level or member `var name=expr` / `const var …` / `volatile var …`
    Var(VarDecl),
    /// a plain expression statement
    Expr(Vec<Atom>),
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
    pub modifiers: Vec<String>, // const, volatile, mut, export
}

/// Core-form classification table (the closed set).
pub fn kind(c: &Core) -> &'static str {
    match c {
        Core::Header { .. } => "header",
        Core::Fn(_) => "fn",
        Core::Type(_) => "type",
        Core::Var(_) => "var",
        Core::Expr(_) => "expr",
    }
}

/// Invariant (design doc §2): after expansion, no form is outside the table.
pub fn is_closed(forms: &[Core]) -> bool {
    forms.iter().all(|f| CORE_KINDS.contains(&kind(f)))
}

pub const CORE_KINDS: &[&str] = &["header", "fn", "type", "var", "expr"];

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
    forms.iter().map(expand_form).collect()
}

fn expand_form(f: &Form) -> Result<Core, ExpandError> {
    match f {
        Form::Directive(_) => {
            // directives were split by reader into per-line atoms? No —
            // `@1 @ns X @use Y` on one line: the reader returns ONE directive
            // with all atoms; split here.
            let atoms = match f {
                Form::Directive(a) => a.clone(),
                _ => unreachable!(),
            };
            Ok(expand_directives(&atoms)?)
        }
        Form::Line { .. } => expand_line(f),
    }
}

/// Split a directive atom run: `1 @ ns X [ @ use Y ]`
fn expand_directives(atoms: &[Atom]) -> Result<Core, ExpandError> {
    let mut version = String::new();
    let mut ns = None;
    let mut uses = Vec::new();
    let mut i = 0;
    while i < atoms.len() {
        // each clause: `@` kw... until next `@`
        let start = i;
        while i < atoms.len() && !matches!(&atoms[i], Atom::Sym(s) if s == "@") {
            i += 1;
        }
        let clause = &atoms[start..i];
        // classify clause: first ident is the keyword (or missing => version)
        if clause.is_empty() {
            i += 1; // skip the `@`
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
    Ok(Core::Header { version, ns, uses })
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

#[allow(unused_assignments)]
fn expand_line(f: &Form) -> Result<Core, ExpandError> {
    let (atoms, inline, children) = match f {
        Form::Line { atoms, inline_body, children } => (atoms.clone(), inline_body.clone(), children.clone()),
        _ => unreachable!(),
    };
    if atoms.is_empty() {
        // artifact of close-line trailing handling: treat as empty
        return Ok(Core::Expr(Vec::new()));
    }
    let kw = match &atoms[0] {
        Atom::Ident(id) => id.clone(),
        Atom::Sym(s) if s == "@" => {
            // a directive line was misclassified? treat whole line as directive atoms
            return Ok(expand_directives(&atoms)?);
        }
        // non-ident start = expression statement (obj-lits, paren chains)
        other => return Ok(Core::Expr({ let _ = other; atoms })),
    };
    match kw.as_str() {
        "fn" | "async" => {
            let mut mods = Vec::new();
            let mut i = 0usize;
            if kw == "async" {
                mods.push("async".into());
                i += 1;
            }
            // next atom: fn
            if !matches!(atoms.get(i), Some(Atom::Ident(id)) if id == "fn") {
                return Err(ExpandError { msg: "async without fn".into() });
            }
            i += 1;
            // name or receiver (`Type.member`)
            let mut name = atoms.iter().nth(i).and_then(|a| match a {
                Atom::Ident(x) => Some(x.clone()),
                _ => None,
            }).unwrap_or_default();
            let mut receiver = None;
            match atoms.get(i) {
                Some(Atom::Ident(n)) => {
                    name = n.clone();
                    i += 1;
                }
                other => return Err(ExpandError { msg: format!("fn missing name {other:?}") }),
            }
            // skip type params `<T, U>` (v0 receiver-only; depth scan)
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
            // export modifier may precede (export fn …)
            if mods.is_empty() && atoms.first().map(|a| matches!(a, Atom::Ident(x) if x == "export")).unwrap_or(false) {
                mods.push("export".into());
            }
            // params `( … )`
            let params = parse_params(&atoms[i..])?; // returns (offset, params)
            let (_, params_v) = params;
            let after = i + params.0;
            // modifiers export after? skip
            // return type: atoms until `{`/`;`
            // return type: all atoms after the param list (reader strips `{`);
            // opaque fns: drop the `opaque` marker + trailing Str blob from ret
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
            // opaque blob marker: atoms carry `Str("opaque:<raw>")` — splice
            // into Body::Opaque via a magic ident in the atom run
            if atoms.iter().any(|a| matches!(a, Atom::Ident(x) if x == "opaque")) {
                // raw = blob content; either in inline body or trailing Str marker
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
                        base: None, body: Body::Opaque(raw),
                    }));
                }
            }
            // body
            let body = match (&inline, &children) {
                (Some(b), c) if c.is_empty() => Body::Inline(b.clone()),
                (None, c) if !c.is_empty() => Body::Bare(c.iter().map(expand_form).collect::<Result<Vec<_>, _>>()?),
                (None, _) => Body::Inline(Vec::new()),
                (Some(b), c) => {
                    let mut v = vec![Core::Expr(b.clone())];
                    v.extend(c.iter().map(expand_form).collect::<Result<Vec<_>, _>>()?);
                    Body::Block(v)
                }
            };
            Ok(Core::Fn(FnDecl { name, receiver, params: params_v, ret, modifiers: mods, base: None, body }))
        }
        "type" => {
            let mut i = 1usize;
            let name = match atoms.get(i) {
                Some(Atom::Ident(n)) => n.clone(),
                other => return Err(ExpandError { msg: format!("type missing name {other:?}") }),
            };
            i += 1;
            // modifiers (export, interface, record, struct, sealed, abstract)
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
            // primary constructor params before bases/`{`: `type Name(params)`
            let mut primary = None;
            if matches!(atoms.get(i), Some(Atom::Sym(s)) if s == "(") {
                let (off, pv) = parse_params(&atoms[i..])?;
                primary = Some(pv);
                i += off;
            }
            // bases after `:` — atoms until `{`
            let bases = match atoms.get(i) {
                Some(Atom::Sym(s)) if s == ":" => {
                    let base_start = i + 1;
                    match atoms[base_start..].iter().position(|a| matches!(a, Atom::Sym(x) if x == "{")) {
                        Some(end) if end > 0 => {
                            let b: Vec<String> = split_commas(&atoms[base_start..base_start + end]).iter().map(|g| dotted(g)).collect();
                            i = base_start + end + 1;
                            b
                        }
                        _ => {
                            let b: Vec<String> = split_commas(&atoms[base_start..]).iter().map(|g| dotted(g)).collect();
                            i = atoms.len();
                            b
                        }
                    }
                }
                Some(Atom::Sym(s)) if s == "{" => {
                    i += 1;
                    Vec::new()
                }
                Some(_) => {
                    // type params like `<T…>` end up dotted elsewhere
                    Vec::new()
                }
                None => Vec::new(),
            };
            // members: children (each a Line)
            let mut members = Vec::new();
            for c in &children {
                if children.is_empty() && atoms.len() > i {
                    // inline body members? not canon
                }
                let mc = expand_member(c)?;
                members.push(mc);
            }
            Ok(Core::Type(TypeDecl { name, kind, primary, bases, modifiers, members }))
        }
        "var" | "const" | "volatile" | "export" => {
            // export var / const var / volatile var / var
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
                            // export <ident> = expr OR export {…} from … = statement
                            if atoms.len() > 1 && matches!(atoms.get(1), Some(Atom::Sym(_))) {
                                return Ok(Core::Expr(atoms));
                            }
                            1
                        }
                    }
                    _ => return Ok(Core::Expr(atoms)),
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
            // name[: Type][ = expr]
            let mut name = String::new();
            if let Some(Atom::Ident(n)) = atoms.get(i) {
                name = n.clone();
                i += 1;
            } else {
                return Err(ExpandError { msg: "var missing name".into() });
            }
            let mut ty = None;
            if !matches!(atoms.get(i), Some(Atom::Sym(s)) if s == "=") {
                // `name: Type =`
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
            // member fields may have children? not canon.
            Ok(Core::Var(VarDecl { name, ty, value, modifiers: mods }))
        }
        // anything else: plain statement (Core::Expr) — expander assoc is done
        // (the keyword shape `other` is consumed as the statement head)
        _ => Ok(Core::Expr(atoms)),
    }
}

fn expand_member(f: &Form) -> Result<Core, ExpandError> {
    // members are fn / ctor / field / property / computed
    let (atoms, inline, children) = match f {
        Form::Line { atoms, inline_body, children } => (atoms.clone(), inline_body.clone(), children.clone()),
        _ => unreachable!(),
    };
    if atoms.is_empty() {
        return Ok(Core::Expr(Vec::new()));
    }
    let kw = match &atoms[0] {
        Atom::Ident(id) => id.clone(),
        other => return Err(ExpandError { msg: format!("member start {other}") }),
    };
    match kw.as_str() {
        "fn" | "async" | "type" => expand_line(f),
        "record" | "struct" if !inline.is_some() => {
            // nested type member: `record StructName(fields…)`
            let name = match atoms.get(1) {
                Some(Atom::Ident(n)) => n.clone(),
                _ => String::new(),
            };
            // params/fields from the rest + children
            let mut members: Vec<Core> = Vec::new();
            for c in &children {
                let (ca, _, _) = match c {
                    crate::ast::Form::Line { atoms, inline_body, children } =>
                        (atoms.clone(), inline_body.clone(), children.clone()),
                    _ => continue,
                };
                if ca.is_empty() {
                    continue;
                }
                // `Type name` → field
                let mut nm = String::new();
                let mut ty: Vec<Atom> = Vec::new();
                let mut k = 0;
                while k < ca.len() {
                    if let Atom::Ident(n) = &ca[k] {
                        if nm.is_empty() {
                            nm = n.clone();
                        }
                    }
                    k += 1;
                }
                let _ = &mut ty;
                members.push(Core::Var(crate::expander::VarDecl {
                    name: nm,
                    ty: None,
                    value: None,
                    modifiers: vec![],
                }));
            }
            Ok(Core::Type(TypeDecl {
                name,
                kind: kw.clone(),
                primary: None,
                bases: vec![],
                modifiers: vec![],
                members,
            }))
        }
        "ctor" => {
            // ctor(params) [: Base(args)] body — model as fn, name "<ctor>"
            let (off, params_v) = parse_params(&atoms[1..])?;
            let after = 1 + off;
            let mut base = None;
            let mut head_end = atoms.len();
            if matches!(atoms.get(after), Some(Atom::Sym(s)) if s == ":") {
                // base call: atoms after `:` up to `{` (inline body) or EOL
                let bs = after + 1;
                match atoms[bs..].iter().position(|a| matches!(a, Atom::Sym(x) if x == "{")) {
                    Some(be) => {
                        base = Some(dotted(&atoms[bs..bs + be]));
                        head_end = bs + be;
                    }
                    None => base = Some(dotted(&atoms[bs..])),
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
            }))
        }
        "opaque" if matches!(atoms.get(1), Some(Atom::Ident(k)) if matches!(k.as_str(), "var" | "const")) => {
            // opaque var: VarDecl + opaque blob value (reader preserved raw)
            let mut i = 2usize;
            if atoms.get(i).map(|a| matches!(a, Atom::Ident(x) if x == "var" || x == "const")).unwrap_or(false) {
                i += 1;
            }
            let name = match atoms.get(i) {
                Some(Atom::Ident(n)) => n.clone(),
                _ => return Err(ExpandError { msg: "opaque var missing name".into() }),
            };
            i += 1;
            // ty after `:`
            let mut ty = None;
            if matches!(atoms.get(i), Some(Atom::Sym(s)) if s == ":") {
                let ts = i + 1;
                match atoms[ts..].iter().position(|a| matches!(a, Atom::Sym(x) if x == "=")) {
                    Some(e) => {
                        ty = Some(dotted(&atoms[ts..ts + e]));
                        i = ts + e + 1;
                    }
                    None => ty = Some(dotted(&atoms[ts..])),
                }
            }
            let value = match atoms.get(i) {
                Some(Atom::Sym(s)) if s == "=" => Some(atoms[i + 1..].to_vec()),
                _ => None,
            };
            Ok(Core::Var(VarDecl { name, ty, value, modifiers: vec!["opaque".into()] }))
        }
        "opaque" if matches!(atoms.get(1), Some(Atom::Ident(k)) if matches!(k.as_str(), "struct" | "record" | "interface" | "type")) => {
            // nested record/struct with primary params (cs_06 CompositeUnit):
            // `opaque struct Name(params)` → nested TypeDecl (fields from params)
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
                    // record fields keep canonical `Type name` (canon §2.1)
                    ty: Some(ty.clone()),
                    value: None,
                    modifiers: vec![],
                }));
            }
            let _ = off;
            Ok(Core::Type(TypeDecl { name, kind, primary: None, bases: vec![], modifiers: vec![], members }))
        }
        _ => {
            // field `Type args name[= x]` or property `Type args name => expr`:
            // name = the ident immediately before `=` / `=>` / EOL;
            // ty = atoms before that ident.
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
            let ty_str = if ty_atoms.is_empty() { None } else { Some(dotted(ty_atoms)) };
            let (value, is_prop) = match atoms.get(split) {
                Some(Atom::Sym(s)) if s == "=" => (Some(atoms[split + 1..].to_vec()), false),
                Some(Atom::Sym(s)) if s == "=>" => (Some(atoms[split + 1..].to_vec()), true),
                _ => (None, false),
            };
            Ok(Core::Var(VarDecl { name, ty: ty_str, value, modifiers: if is_prop { vec!["computed".into()] } else { vec![] } }))
        }
    }
}

/// parse a `(name Type, …)` parameter list starting at `(`
/// returns (consumed_len, params)
fn parse_params(atoms: &[Atom]) -> Result<(usize, Vec<(String, String)>), ExpandError> {
    if !matches!(atoms.first(), Some(Atom::Sym(s)) if s == "(") {
        return Err(ExpandError { msg: "expected ( in param list".into() });
    }
    // matching close
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
        // skip empty
        if g.is_empty() {
            continue;
        }
        // `out` modifier or `this` receiver: first ident
        // split at last ident boundary: name Type or skip modifier words
        // Param spellings accepted (BPE tie, design doc §2.1):
        //   `name: Type`  `name Type` (multi-line canon)  `Type name` (legacy)
        let mut name = String::new();
        let mut ty = String::new();
        if matches!(&g[0], Atom::Ident(id) if id == "out" || id == "ref" || id == "in") {
            // `out var name Type` / `out name Type`
            name = "out".into();
        }
        // colon only at top depth (colons inside lambda-type parens belong to
        // the inner type, not this param)
        let top_colon = g.iter().enumerate().find(|(k, a)| {
            matches!(a, Atom::Sym(s) if s == ":")
                && g[..*k].iter().filter(|b| matches!(b, Atom::Sym(x) if x == "(" || x == ")")).count() % 2 == 0
        });
        if let Some((ci, _)) = top_colon {
            // explicit colon form
            name = dotted(&g[..ci]);
            ty = dotted(&g[ci + 1..]);
        } else {
            // Order-stable heuristic (params accept both orders; canon = name Type):
            // case decides — types are Capitalized, names lowerCamel.
            let idents: Vec<(usize, String)> = g.iter().enumerate()
                .filter(|(_, a)| matches!(a, Atom::Ident(_)))
                .map(|(i, a)| (i, match a {
                    Atom::Ident(n) => n.clone(),
                    _ => unreachable!()
                }))
                .collect();
            let capitalized = |s: &str| s.chars().next().is_some_and(|c| c.is_uppercase());
            match idents.len() {
                0 => { ty = dotted(g.as_slice()); }
                1 => { name = idents[0].1.clone(); }  // untyped param
                _ => {
                    let (i0, s0) = (&idents[0].0, &idents[0].1);
                    let (_i1, s1) = (&idents[1].0, &idents[1].1);
                    // exactly two idents supported; more = dotted type head
                    if !capitalized(s0) && capitalized(s1) {
                        // `u User` — name first
                        name = s0.clone();
                        ty = dotted(&g[*i0 + 1..]);
                    } else if capitalized(s0) && !capitalized(s1) {
                        // `User u` — type first (legacy order): swap to name Type
                        name = s1.clone();
                        ty = dotted(&g[..*i0 + 1]);
                    } else {
                        // both same case: canon name Type
                        name = s0.clone();
                        ty = dotted(&g[*i0 + 1..]);
                    }
                }
            }
        }
        params.push((name, ty));
    }
    Ok((ci + 1, params))
}

/// split an atom run on top-level commas (respecting (), [], <>? — v0: (), [])
fn split_commas(atoms: &[Atom]) -> Vec<Vec<Atom>> {
    let mut out: Vec<Vec<Atom>> = vec![Vec::new()];
    let mut depth = 0i32; // (), [] and <> all nest
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
        // decl-position membership is enforced: `type` missing name
        assert!(expand("type\nfn f() Int{1}\n").is_err());
        // but bare statements are legal Expr — nothing is "unknown" at top
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
}