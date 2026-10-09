//! JS emitter (Phase 2): core forms → modern ES modules.
//! Design: docs/js-target.md §4 lowering table. Expressions stay verbatim
//! atom runs (Route 1 infix ≈ JS infix); structure lives in Core/Stmt.
//!
//! Span tracking (2.6): every emitted core form is wrapped in invisible
//! `M~irLo..irHiM … M` markers (2.6 span threads); `extract_map` walks the
//! marked text once, strips the markers, and produces an IR↔JS line map
//! (js-target.md §7). `emit` output is byte-identical with or without map.

use crate::ast::{Atom, Span};
use crate::expander::{Arm, Body, Core, FnDecl, TypeDecl, VarDecl};

#[derive(Debug)]
pub struct JsLowerError {
    pub msg: String,
    /// IR line the failed form came from (None = synthetic/dispatch-level)
    pub ir_line: Option<usize>,
}

impl JsLowerError {
    fn if_no_ir(mut self, line: usize) -> JsLowerError {
        if self.ir_line.is_none() && line > 0 {
            self.ir_line = Some(line);
        }
        self
    }
}

impl std::fmt::Display for JsLowerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.ir_line {
            Some(l) => write!(f, "js: {} (IR line {l})", self.msg),
            None => write!(f, "js: {}", self.msg),
        }
    }
}

// ---- span tracking (PLAN 2.6) ----

/// open marker: `\u{1}~irLo..irHi\u{1}`; close marker: `\u{2}` (distinct
/// control chars — no ambiguity between the span-header close and a region
/// close; IR atom text is never control-char-bearing)
const MARK: char = '\u{1}';
const MARK2: char = '\u{2}';

fn mark(seg: &str, span: Span) -> String {
    let mut s = format!("{M}~{lo}..{hi}{M}", M = MARK, lo = span.lo, hi = span.hi);
    if let Some(core) = seg.strip_suffix('\n') {
        // close the region on its last CONTENT line, not the next line start
        s.push_str(core);
        s.push(MARK2);
        s.push('\n');
    } else {
        s.push_str(seg);
        s.push(MARK2);
    }
    s
}

/// every emitted Core form carries its IR span — single dispatch point
fn core_span(c: &Core) -> Span {
    match c {
        Core::Header { span, .. }
        | Core::Stmt { span, .. }
        | Core::Expr(_, span)
        | Core::Clause(_, span) => *span,
        Core::Fn(f) => f.span,
        Core::Type(t) => t.span,
        Core::Var(v) => v.span,
    }
}

/// one mapping: emitted JS lines [js_lo..js_hi] ↔ IR lines [ir_lo..ir_hi]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapEntry {
    pub js_lo: usize,
    pub js_hi: usize,
    pub ir_lo: usize,
    pub ir_hi: usize,
}

#[derive(Debug, Clone, Default)]
pub struct SourceMap {
    pub entries: Vec<MapEntry>,
}

impl SourceMap {
    /// JS line → IR span. Innermost (latest-starting) entry containing the
    /// line wins; unmapped lines (blank, `}`-only) fall back to the nearest
    /// preceding entry — stack traces anchor to the IR statement above them.
    pub fn js_to_ir(&self, js_line: usize) -> Option<Span> {
        let mut best: Option<&MapEntry> = None;
        for e in &self.entries {
            if e.js_lo <= js_line && js_line <= e.js_hi {
                if best.map(|b| e.js_lo > b.js_lo).unwrap_or(true) {
                    best = Some(e);
                }
            }
        }
        if let Some(e) = best {
            return Some(Span { lo: e.ir_lo, hi: e.ir_hi });
        }
        self.entries
            .iter()
            .rev()
            .find(|e| e.js_lo <= js_line)
            .map(|e| Span { lo: e.ir_lo, hi: e.ir_hi })
    }

    /// IR line → emitted JS line range (first innermost entry covering it)
    pub fn ir_to_js(&self, ir_line: usize) -> Option<(usize, usize)> {
        let mut best: Option<&MapEntry> = None;
        for e in &self.entries {
            if e.ir_lo <= ir_line && ir_line <= e.ir_hi {
                if best.map(|b| e.ir_lo > b.ir_lo).unwrap_or(true) {
                    best = Some(e);
                }
            }
        }
        best.map(|e| (e.js_lo, e.js_hi))
    }

    /// sidecar JSON (js-target.md §7): `{"source":…,"mappings":[[jsLo,jsHi,irLo,irHi],…]}`
    pub fn to_json(&self, source: &str) -> String {
        let mut s = String::from("{\"source\":");
        s.push_str(&serde_json_string(source));
        s.push_str(",\"mappings\":[");
        let mut sorted = self.entries.clone();
        sorted.sort_by_key(|e| (e.js_lo, e.js_hi));
        for (i, e) in sorted.iter().enumerate() {
            if i > 0 {
                s.push(',');
            }
            s.push_str(&format!(
                "[{},{},{},{}]",
                e.js_lo, e.js_hi, e.ir_lo, e.ir_hi
            ));
        }
        s.push_str("]}");
        s
    }
}

fn serde_json_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' | '\\' => {
                out.push('\\');
                out.push(c);
            }
            _ => out.push(c),
        }
    }
    out.push('"');
    out
}

/// walk marked text once: strip markers, emit entries with JS line numbers
fn extract_map(src: &str) -> (String, SourceMap) {
    let chars: Vec<char> = src.chars().collect();
    let mut out = String::with_capacity(src.len());
    let mut entries: Vec<MapEntry> = Vec::new();
    let mut stack: Vec<(Span, usize)> = Vec::new();
    let mut line = 1usize;
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == MARK {
            if chars.get(i + 1) == Some(&'~') {
                if let Some((sp, ni)) = parse_marker_span(&chars, i + 2) {
                    stack.push((sp, line));
                    i = ni + 2; // past the trailing MARK
                    continue;
                }
            }
            // stray/malformed marker: passthrough (defensive)
            out.push(chars[i]);
            i += 1;
        } else if chars[i] == MARK2 {
            if let Some((sp, open_line)) = stack.pop() {
                entries.push(MapEntry {
                    js_lo: open_line,
                    js_hi: line,
                    ir_lo: sp.lo,
                    ir_hi: sp.hi,
                });
            }
            i += 1;
            continue;
        } else {
            if chars[i] == '\n' {
                line += 1;
            }
            out.push(chars[i]);
            i += 1;
        }
    }
    // unclosed regions (emitter bug guard): flush at the last line
    while let Some((sp, open_line)) = stack.pop() {
        entries.push(MapEntry {
            js_lo: open_line,
            js_hi: line.saturating_sub(1).max(open_line),
            ir_lo: sp.lo,
            ir_hi: sp.hi,
        });
    }
    (out, SourceMap { entries })
}

/// parse `<digits>..<digits>` starting at j; returns (Span, index of last digit)
fn parse_marker_span(chars: &[char], j: usize) -> Option<(Span, usize)> {
    let (lo, a) = parse_usize(chars, j)?;
    if !chars.get(a).is_some_and(|c| *c == '.') || !chars.get(a + 1).is_some_and(|c| *c == '.') {
        return None;
    }
    let (hi, b) = parse_usize(chars, a + 2)?;
    Some((Span { lo, hi }, b - 1))
}

/// parse digits at j → (value, index AFTER the digits)
fn parse_usize(chars: &[char], j: usize) -> Option<(usize, usize)> {
    let mut k = j;
    let mut v = 0usize;
    while k < chars.len() && chars[k].is_ascii_digit() {
        v = v * 10 + (chars[k] as usize - '0' as usize);
        k += 1;
    }
    if k == j {
        return None;
    }
    Some((v, k))
}

pub fn emit(core: &[Core]) -> Result<String, JsLowerError> {
    emit_with_map(core).map(|(s, _)| s)
}

/// emit + span map (PLAN 2.6): output text is identical to `emit`
pub fn emit_with_map(core: &[Core]) -> Result<(String, SourceMap), JsLowerError> {
    let marked = emit_marked(core)?;
    Ok(extract_map(&marked))
}

fn emit_marked(core: &[Core]) -> Result<String, JsLowerError> {
    let mut out = String::new();
    let mut first_item = true;
    for c in core {
        match c {
            Core::Header { ns, uses, span, .. } => {
                // @ns → dropped; @use → side-effect import (names unknown in v0)
                let _ = ns;
                if !uses.is_empty() && !first_item {
                    out.push('\n');
                }
                let mut block = String::new();
                for u in uses {
                    block.push_str(&format!("import \"./{}.js\";\n", u_to_path(u)));
                }
                if !uses.is_empty() {
                    out.push_str(&mark(&block, *span));
                    out.push('\n');
                }
            }
            _ => {
                if !first_item {
                    out.push('\n');
                }
                out.push_str(&emit_core(c, 0)?);
                out.push('\n');
            }
        }
        first_item = false;
    }
    Ok(out)
}

fn u_to_path(u: &str) -> String {
    u.replace('.', "/")
}

fn emit_core(c: &Core, level: usize) -> Result<String, JsLowerError> {
    let span = core_span(c);
    match emit_core_raw(c, level) {
        Ok(s) => Ok(mark(&s, span)),
        Err(e) => Err(e.if_no_ir(span.lo)),
    }
}

fn emit_core_raw(c: &Core, level: usize) -> Result<String, JsLowerError> {
    match c {
        Core::Header { .. } => Ok(String::new()), // handled in emit_marked()
        Core::Fn(f) => emit_fn(f, level, false),
        Core::Type(t) => emit_type(t, level),
        Core::Var(v) => emit_var(v, level, false),
        Core::Expr(atoms, _) => {
            // `opaque type …` = TS-only surface (fidelity §6.6): drop in JS
            if atoms.first().map(|a| matches!(a, Atom::Ident(x) if x == "opaque")).unwrap_or(false)
                && atoms.get(1).map(|a| matches!(a, Atom::Ident(x) if x == "type")).unwrap_or(false)
            {
                return Ok(String::new());
            }
            let txt = expr(atoms);
            if txt.contains("rest-omitted") {
                // hand-rendition placeholder (bench fidelity note), not code
                return Ok(String::new());
            }
            Ok(format!("{}{};", indent(level), txt))
        }
        Core::Stmt { head, body, arms, .. } => emit_stmt(head, body, arms, level),
        Core::Clause(..) => Ok(String::new()), // associated into owner
    }
}

fn indent(level: usize) -> String {
    "  ".repeat(level)
}

fn emit_fn(f: &FnDecl, level: usize, in_class: bool) -> Result<String, JsLowerError> {
    // IR leaves async implicit when a body awaits (fidelity §3: asyncness is
    // fn-level only); JS requires the keyword — detect and upgrade
    let body_awaits = fn_awaits(&f.body);
    let is_ctor = f.modifiers.iter().any(|m| m == "ctor" || f.name == "<ctor>");
    let exported = f.modifiers.iter().any(|m| m == "export");
    let is_async = f.modifiers.iter().any(|m| m == "async") || body_awaits;
    if let Some(_recv) = &f.receiver {
        // JS has no extension methods → free function, receiver as param 0
        // (lowering note, js-target.md §4)
    }
    let mut s = String::new();
    if exported {
        s.push_str("export ");
    }
    if is_async {
        s.push_str("async ");
    }
    let _ = is_ctor;
    s.push_str("function ");
    s.push_str(&f.name);
    s.push_str(&params_js(&f.params, is_ctor));
    s.push(' ');
    s.push_str(&emit_body(&f.body, level, f.ret.is_some())?);
    if in_class {
        // methods print without `function` keyword — handled by emit_type
    }
    let _ = in_class;
    Ok(s)
}

fn params_js(params: &[(String, String)], _is_ctor: bool) -> String {
    let mut s = String::from("(");
    for (i, (n, _t)) in params.iter().enumerate() {
        // type half is ERASED (js-target §1); bare name — TS optional `?`
        // and any annotation residue are not valid JS surface
        let name = n.trim_end_matches('?');
        if i > 0 {
            s.push_str(", ");
        }
        s.push_str(name);
    }
    s.push(')');
    s
}

/// does this fn body contain an `await` anywhere? (async upgrade check)
fn fn_awaits(b: &Body) -> bool {
    match b {
        Body::Inline(a) => a.iter().any(|x| matches!(x, Atom::Ident(i) if i == "await")),
        Body::Bare(v) | Body::Block(v) => v.iter().any(|c| match c {
            Core::Expr(a, _) => a.iter().any(|x| matches!(x, Atom::Ident(i) if i == "await")),
            Core::Stmt { head, body, .. } => {
                head.iter().any(|x| matches!(x, Atom::Ident(i) if i == "await")) || fn_awaits(body)
            }
            Core::Fn(f2) => fn_awaits(&f2.body),
            _ => false,
        }),
        Body::Opaque(_) => false,
    }
}

fn emit_body(b: &Body, level: usize, ret_tail: bool) -> Result<String, JsLowerError> {
    match b {
        Body::Inline(atoms) if atoms.is_empty() => Ok("{}".to_string()),
        Body::Inline(atoms) => {
            // single expr → return-fused (JS fns usually return)
            Ok(format!("{{ return {}; }}", expr(atoms)))
        }
        Body::Bare(stmts) | Body::Block(stmts) => {
            let mut s = String::from("{\n");
            let last_i = stmts.len().saturating_sub(1);
            for (si, st) in stmts.iter().enumerate() {
                // IR value semantics: a ret-typed fn whose body ends in a bare
                // expression returns that value (corpus: `for…; acc`).
                // Statement-valued tails (if/match/try) are the author's job.
                if ret_tail && si == last_i {
                    if let Core::Expr(atoms, _) = st {
                        let is_opaque_blob = atoms.len() == 1
                            && matches!(atoms[0], Atom::Str(ref x) if x.starts_with("opaque:"));
                        if !atoms.is_empty()
                            && !is_opaque_blob
                            && !atoms.iter().any(|a| matches!(a, Atom::Ident(x) if x == "return" || x == "throw"))
                        {
                            let txt = expr(atoms);
                            if !txt.contains("rest-omitted") {
                                s.push_str(&indent(level + 1));
                                s.push_str(&format!("return {};\n", txt));
                                continue;
                            }
                        }
                    }
                }
                s.push_str(&emit_core(st, level + 1)?);
                s.push('\n');
            }
            s.push_str(&indent(level));
            s.push('}');
            Ok(s)
        }
        Body::Opaque(raw) => Ok(raw.clone()), // verbatim (source was JS or C#; C# blobs won't run as JS — flagged in fidelity)
    }
}

fn emit_type(t: &TypeDecl, level: usize) -> Result<String, JsLowerError> {
    let ind = indent(level);
    let exported = t.modifiers.iter().any(|m| m == "export");
    if t.kind == "interface" {
        // no type surface in JS: emit JSDoc marker + skip (js-target §4)
        let mut s = format!("{ind}/** @interface {0} */\n{ind}class {0} {{}}", t.name);
        if exported {
            s = format!("{ind}/** @interface {0} */\n{ind}export class {0} {{}}", t.name);
        }
        return Ok(s);
    }
    let mut s = String::new();
    if exported {
        s.push_str("export ");
    }
    s.push_str(&format!("class {0}", t.name));
    if let Some(base) = t.bases.first() {
        s.push_str(&format!(" extends {}", js_type(base)));
    }
    s.push_str(" {\n");
    let mut ctor_seen = false;
    for m in &t.members {
        // overload sets canonicalize (fidelity §6.4): ONE constructor in JS
        if matches!(m, Core::Fn(f2) if f2.modifiers.iter().any(|x| x == "ctor") || f2.name == "<ctor>") {
            if ctor_seen {
                continue;
            }
            ctor_seen = true;
        }
        s.push_str(&emit_member(m, level + 1)?);
        s.push('\n');
    }
    s.push_str(&ind);
    s.push('}');
    Ok(s)
}

fn emit_member(c: &Core, level: usize) -> Result<String, JsLowerError> {
    let span = core_span(c);
    match emit_member_raw(c, level) {
        Ok(s) => Ok(mark(&s, span)),
        Err(e) => Err(e.if_no_ir(span.lo)),
    }
}

fn emit_member_raw(c: &Core, level: usize) -> Result<String, JsLowerError> {
    match c {
        Core::Var(v) if v.modifiers.iter().any(|m| m == "computed") => {
            // `Type name => expr` — getter
            let ind = indent(level);
            let e = v.value.as_ref().map(|a| expr(a)).unwrap_or_default();
            Ok(format!("{0}get {1}() {{\n{0}  return {2};\n{0}}}", ind, v.name, e))
        }
        Core::Var(v) => {
            // class field: `name = e` or bare `name;`
            let ind = indent(level);
            match &v.value {
                Some(a) if !a.is_empty() => Ok(format!("{0}{1} = {2};", ind, v.name, expr(a))),
                _ => Ok(format!("{0}{1};", ind, v.name)),
            }
        }
        other => emit_member_fnlike(other, level),
    }
}

fn emit_member_fnlike(c: &Core, level: usize) -> Result<String, JsLowerError> {
    match c {
        Core::Fn(f) => {
            let ind = indent(level);
            let is_ctor = f.modifiers.iter().any(|m| m == "ctor" || f.name == "<ctor>");
            let is_async = f.modifiers.iter().any(|m| m == "async") || fn_awaits(&f.body);
            let receiver_prefix = match &f.receiver {
                Some(r) => format!("static {} ", r), // `fn T.m` member = static-ish? no: extension → static method on T's type — v1: extension on instance type; emit plain method
                None => String::new(),
            };
            let _ = receiver_prefix;
            let name = if is_ctor { "constructor".to_string() } else { f.name.clone() };
            let mut s = ind.clone();
            if is_async {
                s.push_str("async ");
            }
            s.push_str(&name);
            s.push_str(&params_js(&f.params, is_ctor));
            s.push(' ');
            s.push_str(&emit_body_ctor(&f.body, level, is_ctor, &f.base, f.ret.is_some())?);
            Ok(s)
        }
        Core::Type(t) => emit_type(t, level),
        Core::Var(v) => emit_member_var(v, level),
        _ => emit_core(c, level),
    }
}

fn emit_member_var(v: &VarDecl, level: usize) -> Result<String, JsLowerError> {
    emit_var(v, level, true)
}

fn emit_var(v: &VarDecl, level: usize, _member: bool) -> Result<String, JsLowerError> {
    let ind = indent(level);
    if v.modifiers.iter().any(|m| m == "opaque") {
        // opaque var: verbatim blob only if captured
        return Ok(String::new());
    }
    let kw = if v.modifiers.iter().any(|m| m == "mut") { "let" } else { "const" };
    // `: ty` never materializes (js-target §1: locals untyped, no TS surface)
    match (&v.value, &v.objlit) {
        (Some(a), _) if !a.is_empty() => Ok(format!("{}{} {} = {};", ind, kw, v.name, expr(a))),
        (None, _) => Ok(format!("{}{} {};", ind, kw, v.name)),
        (Some(_), None) => Ok(format!("{}{} {};", ind, kw, v.name)),
        (_, Some(items)) => {
            let mut s = format!("{}{} {} = {{", ind, kw, v.name);
            let mut first = true;
            for it in items {
                if let Core::Expr(atoms, _) = it {
                    if atoms.is_empty() {
                        continue;
                    }
                    if !first {
                        s.push(',');
                    }
                    s.push('\n');
                    s.push_str(&indent(level + 1));
                    s.push_str(&expr(atoms));
                    first = false;
                }
            }
            if !first {
                s.push('\n');
                s.push_str(&ind);
            }
            s.push_str("};");
            Ok(s)
        }
    }
}

fn emit_body_ctor(b: &Body, level: usize, is_ctor: bool, base: &Option<String>, ret_tail: bool) -> Result<String, JsLowerError> {
    if !is_ctor {
        return emit_body(b, level, ret_tail);
    }
    let mut s = String::from("{\n");
    if let Some(base) = base {
        s.push_str(&format!("{}super({});\n", indent(level + 1), base));
    }
    match b {
        Body::Inline(atoms) if !atoms.is_empty() => {
            // fused single stmt — ctor body statements, not returns
            s.push_str(&format!("\n{}{};", indent(level + 1), expr(atoms)));
        }
        Body::Bare(stmts) | Body::Block(stmts) => {
            for st in stmts {
                s.push_str(&emit_core(st, level + 1)?);
                s.push('\n');
            }
        }
        _ => {}
    }
    s.push_str(&indent(level));
    s.push('}');
    Ok(s)
}

/// Core::Stmt → JS control flow.
fn emit_stmt(head: &[Atom], body: &Body, arms: &[Arm], level: usize) -> Result<String, JsLowerError> {
    let ind = indent(level);
    let kw = match head.first() {
        Some(Atom::Ident(k)) => k.clone(),
        _ => return Ok(format!("{ind}{};", expr(head))),
    };
    match kw.as_str() {
        "for" => emit_for(head, body, arms, level),
        "while" => {
            let cond = expr(&head[1..]);
            Ok(format!("{}while ({}) {}", ind, cond, emit_stmt_body_js(body, level)?))
        }
        "lock" => {
            // no locks in JS: body inlines (lowering note)
            emit_stmt_body_open(body, level)
        }
        "if" => emit_if(head, body, arms, level),
        "match" => emit_match(head, body, arms, level),
        "try" => emit_try(head, body, arms, level),
        _ if head.last().map(|a| matches!(a, Atom::Ident(k) if k == "switch")).unwrap_or(false) => {
            // switch-EXPRESSION statement: type-test arms → instanceof chain
            let disc = expr(&head[..head.len() - 1]);
            let mut s = String::new();
            for (ai, a) in arms.iter().enumerate() {
                let pat_txt = render_local(&a.pattern);
                let cond_txt = if pat_txt == "else" {
                    String::new()
                } else if pat_txt.contains('.') {
                    // dotted enum member → equality
                    format!("({disc} == {pat_txt})")
                } else if pat_txt.starts_with('+') || pat_txt.starts_with('-') {
                    // literal arm (`+1`, `-1`): equality
                    format!("({disc} === {})", pat_txt.replace(' ', ""))
                } else {
                    // bare class/type pattern → instanceof with generics erased
                    let t = js_type(pat_txt.trim());
                    let mut o2 = String::new();
                    let mut gd2 = 0i32;
                    for ch in t.chars() {
                        match ch {
                            '<' => gd2 += 1,
                            '>' => gd2 -= 1,
                            _ if gd2 == 0 => o2.push(ch),
                            _ => {}
                        }
                    }
                    format!("({disc} instanceof {o2})")
                };
                let body_txt = match &a.body {
                    Body::Inline(bs) if !bs.is_empty() => expr(bs),
                    _ => String::new(),
                };
                if ai == 0 {
                    s.push_str(&format!("{ind}if ({cond_txt}) {{"));
                } else if cond_txt.is_empty() {
                    s.push_str(&format!("{ind}else {{"));
                } else {
                    s.push_str(&format!("{ind}else if ({cond_txt}) {{"));
                }
                if !body_txt.is_empty() {
                    s.push_str(&format!("{ind}  {body_txt};"));
                }
                s.push_str(&format!("{ind}}}"));
            }
            Ok(s)
        }
        "catch" | "finally" | "else" => {
            // orphan fragment — the emitter flattens try/catch chains; a
            // fragment arriving here means its parent didn't claim it
            Err(JsLowerError { msg: format!("orphan {kw} fragment"), ir_line: None })
        }
        _ => Ok(format!("{ind}{} {};", expr(head), "")),
    }
}

fn emit_stmt_body_js(b: &Body, level: usize) -> Result<String, JsLowerError> {
    match b {
        Body::Inline(a) if a.is_empty() => Ok("{}".to_string()),
        Body::Inline(a) => Ok(format!("{{ {}; }}", expr(a))),
        Body::Bare(stmts) | Body::Block(stmts) => {
            let mut s = String::from("{\n");
            for st in stmts {
                s.push_str(&emit_core(st, level + 1)?);
                s.push('\n');
            }
            s.push_str(&indent(level));
            s.push('}');
            Ok(s)
        }
        Body::Opaque(raw) => Ok(raw.clone()),
    }
}

fn emit_stmt_body_open(b: &Body, level: usize) -> Result<String, JsLowerError> {
    match b {
        Body::Inline(a) if a.is_empty() => Ok(String::new()),
        Body::Inline(a) => Ok(format!("{};\n", expr(a))),
        Body::Bare(stmts) | Body::Block(stmts) => {
            let mut s = String::new();
            for st in stmts {
                s.push_str(&emit_core(st, level)?);
                s.push('\n');
            }
            Ok(s)
        }
        Body::Opaque(raw) => Ok(raw.clone()),
    }
}

fn emit_for(head: &[Atom], body: &Body, _arms: &[Arm], level: usize) -> Result<String, JsLowerError> {
    let ind = indent(level);
    // shapes: `for x in xs` | `for i in 0..len` | C-style `for init; cond; step`
    if head.len() >= 4 && head[2] == Atom::Ident("in".into()) {
        let var = match &head[1] {
            Atom::Ident(v) => v.clone(),
            other => format!("{other}"),
        };
        let it = &head[3..];
        // range? `..` at top (unparenthesized atoms)
        if let Some(di) = it.iter().position(|a| matches!(a, Atom::Sym(s) if s == "..")) {
            let lo = expr(&it[..di]);
            let hi = expr(&it[di + 1..]);
            return Ok(format!(
                "{ind}for (let {var} = {lo}; {var} < {hi}; {var}++) {}",
                emit_stmt_body_js(body, level)?
            ));
        }
        return Ok(format!(
            "{ind}for (const {var} of {}) {}",
            expr(it),
            emit_stmt_body_js(body, level)?
        ));
    }
    // C-style: init; cond; step segments at top level (parens nest in head) —
    // verbatim segments (js-target §4 lowering row; additive case)
    let rest = &head[1..];
    if let Some(s1) = rest.iter().position(|a| matches!(a, Atom::Sym(s) if s == ";")) {
        let (init_a, after1) = (&rest[..s1], &rest[s1 + 1..]);
        if let Some(s2) = after1.iter().position(|a| matches!(a, Atom::Sym(s) if s == ";")) {
            let (cond_a, step_a) = (&after1[..s2], &after1[s2 + 1..]);
            // `x = e` init needs a binder in ESM strict mode
            let init = match init_a {
                [Atom::Ident(v), Atom::Sym(s), ..] if s == "=" => {
                    format!("let {v} = {}", expr(&init_a[2..]))
                }
                other => expr(other),
            };
            return Ok(format!(
                "{ind}for ({}; {}; {}) {}",
                init,
                expr(cond_a),
                expr(step_a),
                emit_stmt_body_js(body, level)?
            ));
        }
    }
    Err(JsLowerError { msg: "for: unsupported shape".into(), ir_line: None })
}

fn emit_if(head: &[Atom], body: &Body, arms: &[Arm], level: usize) -> Result<String, JsLowerError> {
    let ind = indent(level);
    let cond = expr(&head[1..]);
    let mut s = format!("{}if ({}) {}", ind, cond, emit_stmt_body_js(body, level)?);
    for a in arms {
        let p_txt = expr(&a.pattern);
        let arm_txt = if p_txt.is_empty() || p_txt == "else" {
            Some(format!(" else {}", emit_stmt_body_js(&a.body, level)?))
        } else if let Some(rest) = p_txt.strip_prefix("else") {
            let cond2 = rest.trim_start().strip_prefix("if").map(|x| x.trim()).unwrap_or(rest.trim());
            Some(format!(" else if ({}) {}", cond2, emit_stmt_body_js(&a.body, level)?))
        } else {
            None // non-else resid (pre-2.6 behavior): nothing emitted
        };
        if let Some(arm_txt) = arm_txt {
            s.push_str(&mark(&arm_txt, a.span));
        }
    }
    Ok(s)
}

fn emit_match(head: &[Atom], _body: &Body, arms: &[Arm], level: usize) -> Result<String, JsLowerError> {
    let ind = indent(level);
    let disc = expr(&head[1..]);
    let mut s = format!("{}switch ({}) {{\n", ind, disc);
    for a in arms {
        let p_txt = expr(&a.pattern);
        if p_txt.is_empty() {
            continue;
        }
        let arm_seg = if p_txt == "else" {
            match &a.body {
                Body::Inline(bs) if !bs.is_empty() => {
                    Some(format!("{0}  default:\n{0}  {1};\n{0}  break;\n", ind, expr(bs)))
                }
                Body::Inline(_) => Some(format!("{ind}  default:\n")),
                b => {
                    let mut seg = format!("{ind}  default: {{\n");
                    seg.push_str(&emit_stmt_body_block(b, level + 1)?);
                    seg.push_str(&format!("{ind}  break;\n{ind}  }}\n"));
                    Some(seg)
                }
            }
        } else {
            match &a.body {
                Body::Inline(bs) if !bs.is_empty() => {
                    Some(format!("{0}  case {1}:\n{0}  {2};\n{0}  break;\n", ind, p_txt, expr(bs)))
                }
                Body::Inline(_) => Some(format!("{ind}  case {p_txt}:\n{ind}  break;\n")),
                b => {
                    let mut seg = format!("{ind}  case {p_txt}: {{\n");
                    seg.push_str(&emit_stmt_body_block(b, level + 1)?);
                    seg.push_str(&format!("{ind}  break;\n{ind}  }}\n"));
                    Some(seg)
                }
            }
        };
        if let Some(seg) = arm_seg {
            s.push_str(&mark(&seg, a.span));
        }
    }
    s.push_str(&format!("{ind}}}"));
    Ok(s)
}

fn emit_stmt_body_block(b: &Body, level: usize) -> Result<String, JsLowerError> {
    match b {
        Body::Bare(stmts) | Body::Block(stmts) => {
            let mut s = String::new();
            for st in stmts {
                s.push_str(&emit_core(st, level)?);
                s.push('\n');
            }
            Ok(s)
        }
        Body::Inline(a) => Ok(format!("{}{};\n", indent(level), expr(a))),
        Body::Opaque(raw) => Ok(raw.clone()),
    }
}

fn emit_try(_head: &[Atom], body: &Body, arms: &[Arm], level: usize) -> Result<String, JsLowerError> {
    let ind = indent(level);
    // JS has ONE catch clause: fold all catch-arms into an instanceof guard
    // chain; finally stays a real clause (js-target §4)
    let mut s = format!("{}try {}", ind, emit_stmt_body_js(body, level)?);
    let mut catch_body = String::with_capacity(64);
    let mut have_catch = false;
    let mut finally_body: Option<Body> = None;
    for a in arms {
        let words: &[Atom] = &a.pattern;
        let kw = match words.first() {
            Some(Atom::Ident(k)) if k == "catch" || k == "finally" => k.clone(),
            _ => "catch".to_string(),
        };
        if kw == "finally" {
            finally_body = Some(a.body.clone());
            continue;
        }
        // rest = [Class] or [Class, binder]; binder defaults to `e`
        let rest: &[Atom] = if kw == "catch" && words.len() > 1 {
            &words[1..]
        } else {
            &words[words.len().min(1)..]
        };
        let (c, b) = match rest {
            [c] => (render_local(&[c.clone()]), "e".to_string()),
            [c, b2, ..] => (render_local(&[c.clone()]), render_local(&[b2.clone()])),
            _ => ("Error".to_string(), "e".to_string()),
        };
        // generics/param-syntax erase: `ICollection<T>` → `ICollection`
        let class_ = {
            let mut out = String::new();
            let mut gd = 0i32;
            for ch in c.chars() {
                match ch {
                    '<' => gd += 1,
                    '>' => gd -= 1,
                    '?' => {}
                    _ if gd == 0 => out.push(ch),
                    _ => {}
                }
            }
            out
        };
        let binder = b;
        let guard_body = match &a.body {
            Body::Inline(bs) if !bs.is_empty() => format!("%N%{};%~", expr(bs)),
            Body::Inline(_) => String::new(),
            b => emit_stmt_body_block(b, level + 3)?,
        };
        catch_body.push_str(&format!(
            "%N%{ind}  if ({binder} instanceof {class_}) {{%~{guard_body}%N%{ind}  }} else%~"
        ));
        have_catch = true;
    }
    if have_catch {
        // terminating else: rethrow
        catch_body.push_str(&format!("%N%{ind}    throw e;%~"));
        s.push_str(&format!(
            " catch (e) {{{}{}}}{}",
            catch_body.replace("%N%", "\n").replace("%~", ""),
            "\n",
            ""
        ));
    }
    if let Some(fb) = &finally_body {
        match fb {
            Body::Inline(bs) if !bs.is_empty() => {
                s.push_str(&format!(" finally {{\n{ind}  {};\n{ind}}}", expr(bs)));
            }
            Body::Inline(_) => s.push_str(" finally {}"),
            b => {
                s.push_str(" finally {\n");
                s.push_str(&emit_stmt_body_block(b, level + 1)?);
                s.push_str(&ind);
                s.push('}');
            }
        }
    }
    Ok(s)
}

/// local render for pattern atoms (class names, binders — no rewrites needed
/// but route through expr for Strs etc.)
fn render_local(atoms: &[Atom]) -> String {
    crate::printer::render_atoms(atoms).trim().to_string()
}

pub fn tpl(s: &str) -> String {
    let mut out = String::new();
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0usize;
    let mut in_str = None;
    while i < chars.len() {
        let c = chars[i];
        match in_str {
            Some(q) => {
                out.push(c);
                if c == '\\' && i + 1 < chars.len() {
                    out.push(chars[i + 1]);
                    i += 2;
                    continue;
                }
                if c == q {
                    in_str = None;
                }
                i += 1;
            }
            None => {
                if c == '\'' || c == '"' {
                    in_str = Some(c);
                    out.push(c);
                    i += 1;
                } else if c == '{' {
                    out.push_str("${");
                    i += 1;
                } else {
                    out.push(c);
                    i += 1;
                }
            }
        }
    }
    out
}

/// Atom run → JS text (js-target §3). Single-pass recursive descent:
/// brackets recurse; Ident+( = call; Ident+operand = bare prefix-call;
/// `as Type` cast erased; postfix `!` dropped; strings templated.
pub fn expr(atoms: &[Atom]) -> String {
    // `(Type)x` cast preprocessing: `(Ident)` followed by an operand ⇒ drop
    let mut clean: Vec<Atom> = Vec::with_capacity(atoms.len());
    let mut i = 0usize;
    while i < atoms.len() {
        if matches!(&atoms[i], Atom::Sym(s2) if s2 == "(")
            && matches!(atoms.get(i + 1), Some(Atom::Ident(k)) if k.chars().next().is_some_and(|c| c.is_uppercase()))
            && matches!(atoms.get(i + 2), Some(Atom::Sym(s2)) if s2 == ")")
            && matches!(atoms.get(i + 3), Some(Atom::Ident(_)))
            && atoms[i + 1..i + 2].iter().all(|a| matches!(a, Atom::Ident(_)))
        {
            i += 3; // drop the cast group
            continue;
        }
        clean.push(atoms[i].clone());
        i += 1;
    }
    let r = dedup_spaces(expr_at(&strip_comments(&clean)).trim_end());
    r
}

fn strip_comments(atoms: &[Atom]) -> Vec<Atom> {
    // comment lines (`//` sym + payload) may ride in expr runs; drop them
    atoms.iter().cloned().collect()
}

fn expr_at(atoms: &[Atom]) -> String {
    let mut out = String::new();
    let mut i = 0usize;
    let n = atoms.len();
    while i < n {
        match &atoms[i] {
            Atom::Ident(id) => {
                let id = id.strip_suffix('!').unwrap_or(id);
                if id == "$" {
                    // `$"interpolated"` — verbatim string payload (already
                    // templated via js_string)
                    match atoms.get(i + 1) {
                        Some(Atom::Str(k)) => {
                            out.push_str(&js_string(k));
                            i += 1;
                        }
                        _ => out.push('$'),
                    }
                    i += 1;
                    continue;
                }
                if id == "as" {
                    i += 1 + skip_type(&atoms[i + 1..]);
                    continue;
                }
                if id == "out" {
                    // C# out-ref arg: drop only the modifier words (`out
                    // var`); the ident stays as a placeholder arg (the
                    // callee must return results in JS — lowering note)
                    i += 1;
                    if matches!(atoms.get(i), Some(Atom::Ident(x2)) if x2 == "var" || x2 == "const") {
                        i += 1;
                    }
                    continue;
                }
                if id == "is" && i > 0 {
                    // rewrite whole run as an is-pattern
                    if let Some(p) = lower_is_pattern(atoms) {
                        out.clear();
                        out.push_str(&p);
                        i = n;
                        continue;
                    }
                }
                out.push_str(id);
                if is_js_keyword(id) {
                    out.push(' ');
                    i += 1;
                    continue;
                }
                match atoms.get(i + 1) {
                    Some(Atom::Sym(s2)) if s2 == "(" => {
                        let head_word = if out.ends_with("new") {
                            out.truncate(out.len() - 3 + 0);
                            out.push_str("new Array");
                            true
                        } else {
                            false
                        };
                        i += 2; // past ident + opener
                        let inner = bracket_slice(atoms, &mut i, ')');
                        let rendered = expr(&inner);
                        let _ = head_word;
                        // `out x` skip leaves a trailing `,` — trim it
                        let trimmed = rendered.trim_end().strip_suffix(',').map(|x| x.to_string()).unwrap_or(rendered);
                        out.push_str(&format!("({})", trimmed));
                    }
                    Some(a) if bare_arg_start(a) => {
                        // bare prefix-call: collect operand-atom spans
                        i += 1;
                        let mut spans: Vec<(usize, usize)> = Vec::new();
                        let depth: Vec<char> = Vec::new();
                        let mut arg_lo = i;
                        while i < n {
                            match &atoms[i] {
                                Atom::Sym(s2)
                                    if depth.is_empty()
                                        && matches!(
                                            s2.as_str(),
                                            ")" | "]" | "," | "}" | "=" | "+" | "-" | "*"
                                                | "/" | "%" | "&&" | "||" | "<" | ">" | "<="
                                                | ">=" | "==" | "!=" | "?" | ":" | "." | "!"
                                        ) =>
                                {
                                    break;
                                }
                                Atom::Sym(s2)
                                    if matches!(s2.as_str(), "(" | "[") && depth.is_empty() =>
                                {
                                    // paren/bracket group arg (lambda, array):
                                    // consume whole group as ONE arg span
                                    let c = match s2.as_str() {
                                        "(" => ')',
                                        _ => ']',
                                    };
                                    let lo = i + 1;
                                    let inner = bracket_slice(atoms, &mut i, c);
                                    spans.push((lo, i.saturating_sub(1)));
                                    let _ = inner;
                                    i += 1; // past the group closer
                                    arg_lo = i;
                                    continue;
                                }
                                Atom::Sym(s2) if s2 == "," && depth.is_empty() => {
                                    spans.push((arg_lo, i));
                                    i += 1;
                                    arg_lo = i;
                                    continue;
                                }
                                // bare-call args are ATOMS (grammar §1):
                                // each operand atom = one argument
                                Atom::Sym(s2) if depth.is_empty() && spans.is_empty() && arg_lo == i => {
                                    // leading sym arg: treat as its own span
                                    spans.push((i, i + 1));
                                    i += 1;
                                    arg_lo = i;
                                    continue;
                                }
                                _ => {}
                            }
                            i += 1;
                        }
                        if arg_lo < i {
                            // exclude the terminating `)`/`]` itself
                            let end = match &atoms[i - 1] {
                                Atom::Sym(s2) if matches!(s2.as_str(), ")" | "]") => i - 1,
                                _ => i,
                            };
                            if arg_lo < end {
                                spans.push((arg_lo, end));
                            }
                        }
                        // bare-call grammar: args are single ATOMS — split
                        // multi-atom spans (except balanced groups already
                        // collected)
                        let mut fine: Vec<(usize, usize)> = Vec::new();
                        for (lo, hi) in &spans {
                            if hi - lo == 1 {
                                fine.push((*lo, *hi));
                            } else if atoms[*lo..*hi].iter().any(|a| matches!(a, Atom::Sym(s2) if s2 == "(")) {
                                fine.push((*lo, *hi));
                            } else {
                                let mut k2 = *lo;
                                while k2 < *hi {
                                    fine.push((k2, k2 + 1));
                                    k2 += 1;
                                }
                            }
                        }
                        spans = fine;
                        let parts: Vec<String> = spans
                            .iter()
                            .map(|(lo, hi)| expr(&atoms[*lo..(*hi).min(n)]))
                            .collect();
                        out.push_str(&format!("({})", parts.join(", ")));
                        continue;
                    }
                    _ => {}
                }
            }
            Atom::Number(k) => out.push_str(&js_number(k)),
            Atom::Str(k) => out.push_str(&js_string(k)),
            Atom::Regex(k) => {
                out.push('/');
                out.push_str(k);
            }
            Atom::Sym(s2) => match s2.as_str() {
                "!" => {
                    if !prev_is_value(&out) {
                        out.push('!');
                    }
                }
                "(" | "[" | "{" => {
                    let c = match s2.as_str() {
                        "(" => ')',
                        "[" => ']',
                        _ => '}',
                    };
                    i += 1; // past the opener; slice starts here
                    let inner = bracket_slice(atoms, &mut i, c);
                    // `(UInt)x` prefix cast: all-caps single ident content +
                    // a next operand → drop the group
                    if c == ')' {
                        let cast_like = inner.len() >= 1
                            && inner.first().map(|a| {
                            // C# cast-target type name: Capitalized word
                            // (`UInt`, `Int`); not all-caps (`UInt` has lc)
                            matches!(a, Atom::Ident(k) if k.chars().next().is_some_and(|ch| ch.is_uppercase()))
                        }).unwrap_or(false)
                           
                            && inner.iter().all(|a| matches!(a, Atom::Ident(_)))
                            && matches!(atoms.get(i + 1), Some(Atom::Ident(_)));
                        if cast_like {
                            continue; // drop the cast group
                        }
                        // `nameof(x)` → `"x"`
                        if out.ends_with("nameof") {
                            out.truncate(out.len() - 6);
                            let name = render_local(&inner);
                            out.push_str(&format!("{name:?}"));
                            continue;
                        }
                    }
                    // `[^1]` hat-index → `.at(-1)`: `xs[^1]` lexes [`[`, ^, 1, ]
                    let is_hat = inner.first().map(|a| matches!(a, Atom::Sym(s3) if s3 == "^")).unwrap_or(false)
                        && c == ']';
                    if is_hat {
                        let idx_txt = expr(&inner[1..]);
                        out.push_str(&format!(".at(-{})", idx_txt));
                        i += 1; // consume the `]` closer
                        continue;
                    }
                    // array spread: `[...x]` / corpus hand-form `[.. x]`
                    if c == ']' {
                        if let Some(s3) = inner.first().map(|a| match a {
                            Atom::Sym(s3) => s3.as_str(),
                            _ => "",
                        }) {
                            if s3 == "..." || s3 == ".." {
                                out.push_str(&format!("[...{}]", expr(&inner[1..])));
                                i += 1; // consume the `]` closer
                                continue;
                            }
                        }
                    }
                    let _ = is_hat;
                    out.push_str(&format!(
                        "{o}{j}{c}",
                        o = s2.chars().next().unwrap(),
                        j = expr(&inner),
                        c = c
                    ));
                }
                "?" => out.push('?'),
                ":" => {
                    if !out.ends_with(' ') {
                        out.push_str(": ");
                    }
                    // `: throw e` (C# throw-expr ternary) → JS needs an
                    // expression: wrap `throw` in an IIFE
                    if matches!(atoms.get(i + 1), Some(Atom::Ident(x2)) if x2 == "throw") {
                        // rewrite: find the throw run to the end of this
                        // ternary (next `,`/`)`/`}` at depth 0) and wrap
                        let lo = i + 1;
                        let mut j2 = lo + 1;
                        while j2 < n {
                            match &atoms[j2] {
                                Atom::Sym(s3) if matches!(s3.as_str(), "," | ")" | "]" | "}") => break,
                                _ => j2 += 1,
                            }
                        }
                        let inner = expr(&atoms[lo..j2]);
                        out.truncate(out.len() - 2); // drop ": " we just added
                        out.push_str(&format!(": (() => {{ {inner}; }})()"));
                        i = j2; // atoms[j2] = the terminator; bottom +=1 → past it
                    }
                }
                "," => {
                    while out.ends_with(' ') {
                        out.pop();
                    }
                    out.push_str(", ");
                }
                _ => out.push_str(s2),
            },
        }
        i += 1;
    }
    out
}

/// `is`-pattern → boolean expr: `x is > 1 and < 5` → `(x > 1 && x < 5)`,
/// `x is {} b` → `x != null` (binder unused), `x is not {} b` → `x == null`.
fn lower_is_pattern(atoms: &[Atom]) -> Option<String> {
    let is_pos = atoms.iter().position(|a| matches!(a, Atom::Ident(x) if x == "is"))?;
    let subject = expr(&atoms[..is_pos]);
    let rest = &atoms[is_pos + 1..];
    if rest.first().map(|a| matches!(a, Atom::Ident(x) if x == "not")).unwrap_or(false) {
        // `is not { } b` → binder-null check
        return Some(format!("({subject} == null)"));
    }
    // `is { } ident` → non-null check
    if rest.first().map(|a| matches!(a, Atom::Sym(s) if s == "{")).unwrap_or(false) {
        return Some(format!("({subject} != null)"));
    }
    // `is null` / `is not null`
    if rest.len() >= 1 && matches!(&rest[rest.len() - 1], Atom::Ident(x) if x == "null") {
        let is_neg = matches!(&rest[0], Atom::Ident(x) if x == "not");
        return Some(format!("({subject} {} null)", if is_neg { "!=" } else { "==" }));
    }
    // infix chain: `> 1 and < 5` → join `and` → &&
    let mut out = String::new();
    let mut ops: Vec<String> = Vec::new();
    let mut parts: Vec<Vec<Atom>> = Vec::new();
    let mut cur: Vec<Atom> = Vec::new();
    for a in rest {
        match a {
            Atom::Ident(x) if x == "and" => {
                ops.push("&&".into());
                parts.push(std::mem::take(&mut cur));
            }
            Atom::Ident(x) if x == "or" => {
                ops.push("||".into());
                parts.push(std::mem::take(&mut cur));
            }
            other => cur.push(other.clone()),
        }
    }
    parts.push(cur);
    let rendered: Vec<String> = parts.iter().map(|p| format!("({subject} {})", expr(p))).collect();
    out.push_str(&rendered.join(&ops.iter().map(|o| format!("{o} ")).collect::<String>()));
    let _ = &ops;
    Some(format!("({out})"))
}

/// slice between brackets; i ends ON the closer (caller advances)
fn bracket_slice<'a>(atoms: &'a [Atom], i: &mut usize, close: char) -> &'a [Atom] {
    // kind-stack: only the MATCHING kind closes this slice (mixed nesting
    // `( items[0] … )` can't be cut short by an inner `]`)
    let mut stack: Vec<char> = vec![close];
    let start = *i;
    while *i < atoms.len() {
        if let Atom::Sym(s2) = &atoms[*i] {
            match s2.as_str() {
                "(" | "[" | "{" => stack.push(s2.chars().next().unwrap()),
                ")" | "]" | "}" => {
                    let opened = stack.pop().unwrap_or(close);
                    let closes_it = match opened {
                        '(' => s2 == ")",
                        '[' => s2 == "]",
                        '{' => s2 == "}",
                        _ => s2.chars().next() == Some(opened),
                    };
                    if closes_it {
                        if stack.is_empty() {
                            break;
                        }
                    } else {
                        // not OUR closer — put it back and stop (its owner
                        // closes outside this slice)
                        stack.push(opened);
                        break;
                    }
                }
                _ => {}
            }
        }
        *i += 1;
    }
    // i ON the closer after return — callers advance explicitly
    &atoms[start..(*i).min(atoms.len())]
}

fn bare_arg_start(a: &Atom) -> bool {
    matches!(a, Atom::Ident(x) if !is_js_keyword(x))
        || matches!(a, Atom::Number(_) | Atom::Str(_) | Atom::Regex(_))
}

/// skip a type run after `as` (idents, `|` unions, generics, `?`, `[]`)
fn skip_type(atoms: &[Atom]) -> usize {
    let mut k = 0usize;
    let mut depth = 0i32;
    while k < atoms.len() {
        match &atoms[k] {
            Atom::Ident(_) => k += 1,
            Atom::Sym(s2) => match s2.as_str() {
                "<" => {
                    depth += 1;
                    k += 1;
                }
                ">" => {
                    if depth == 0 {
                        break;
                    }
                    depth -= 1;
                    k += 1;
                }
                "?" | "." | "[" | "]" | "|" | "!" => k += 1,
                _ => {
                    if depth > 0 {
                        k += 1;
                    } else {
                        break;
                    }
                }
            },
            _ => break,
        }
    }
    k
}

fn prev_is_value(out: &str) -> bool {
    let c = out.chars().last();
    matches!(c, Some(')') | Some(']') | Some('}'))
        || out.ends_with(|c: char| c.is_alphanumeric() || c == '_' || c == '$')
}

/// JS keyword set (atom pass-through + IR control words)
fn is_js_keyword(id: &str) -> bool {
    matches!(
        id,
        "await"
            | "return"
            | "throw"
            | "typeof"
            | "new"
            | "delete"
            | "if"
            | "else"
            | "for"
            | "in"
            | "while"
            | "match"
            | "try"
            | "catch"
            | "finally"
            | "var"
            | "const"
            | "let"
            | "fn"
            | "lock"
            | "is"
            | "not"
            | "and"
            | "true"
            | "false"
            | "null"
            | "undefined"
            | "this"
            | "case"
            | "default"
            | "break"
            | "continue"
            | "instanceof"
            | "of"
            | "async"
            | "export"
            | "import"
            | "from"
            | "as"
            | "void"
    )
}

fn js_number(k: &str) -> String {
    k.strip_suffix('m')
        .or_else(|| k.strip_suffix('d'))
        .unwrap_or(k)
        .to_string()
}

/// IR string payload → JS literal: `{x}` → backticks, else double quotes
fn js_string(k: &str) -> String {
    if k.contains('{') {
        format!("`{}`", tpl(k))
    } else {
        format!("{:?}", k)
    }
}

fn dedup_spaces(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut last_space = false;
    for c in s.chars() {
        if c == ' ' {
            if !last_space {
                out.push(c);
            }
            last_space = true;
        } else {
            out.push(c);
            last_space = false;
        }
    }
    out
}

/// type surface → erased/JS-shaped (generics dropped; `?` dropped)
fn js_type(t: &str) -> String {
    t.trim_end_matches('?').to_string()
}
