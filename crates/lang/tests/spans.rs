//! PLAN 2.6: span tracking — spans reach core forms; the JS emitter exposes
//! an IR↔JS line map (js-target.md §7); emitted text is marker-free and
//! byte-identical with or without the map; the corpus maps every non-blank
//! JS line back to a source construct.

mod common;

use ir_lang::ast::{Atom, Span};
use ir_lang::expander::{self, Body, Core};
use ir_lang::js;
use ir_lang::reader;

const SNIPPET: &str = "\
@1 @ns Acme.Orders

fn Handle(Request r) Response{
  if r.Kind == \"A\"{
    Log(r.Path)
  }else{
    throw Err
  }
  match r.Kind{
    \"GET\" Read(r.Path)
    else Response.Err 405 5
  }
  var z=Compute
  return z
}
";

#[test]
fn reader_records_construct_spans() {
    let forms = reader::read(SNIPPET).unwrap();
    assert_eq!(forms.len(), 2);
    let ir_lang::ast::Form::Directive(_, sp) = &forms[0] else { panic!() };
    assert_eq!((sp.lo, sp.hi), (1, 1));
    let ir_lang::ast::Form::Line { span, children, .. } = &forms[1] else { panic!() };
    assert_eq!((span.lo, span.hi), (3, 3)); // fn header line
    // fn children: if(4) match(9) var(13) return-expr(14)
    assert_eq!(children.len(), 4);
    let lines: Vec<(usize, usize)> = children
        .iter()
        .map(|c| match c {
            ir_lang::ast::Form::Line { span, .. } => (span.lo, span.hi),
            ir_lang::ast::Form::Directive(_, sp) => (sp.lo, sp.hi),
        })
        .collect();
    assert_eq!(lines, vec![(4, 4), (9, 9), (13, 13), (14, 14)]);
    // if construct: Log child (5), else fragment on the close line (6)
    let ir_lang::ast::Form::Line { children: if_kids, .. } = &children[0] else { panic!() };
    let if_lines: Vec<(usize, usize)> = if_kids
        .iter()
        .map(|c| match c {
            ir_lang::ast::Form::Line { span, .. } => (span.lo, span.hi),
            ir_lang::ast::Form::Directive(_, sp) => (sp.lo, sp.hi),
        })
        .collect();
    assert_eq!(if_lines, vec![(5, 5), (6, 6)]);
    // else fragment's own block child: throw (7)
    let ir_lang::ast::Form::Line { children: else_kids, .. } = &if_kids[1] else { panic!() };
    assert_eq!(
        else_kids
            .iter()
            .map(|c| match c {
                ir_lang::ast::Form::Line { span, .. } => span.lo,
                ir_lang::ast::Form::Directive(_, sp) => sp.lo,
            })
            .collect::<Vec<_>>(),
        vec![7]
    );
}

#[test]
fn expander_threads_spans_into_core() {
    let core = expander::expand(SNIPPET).unwrap();
    let Core::Fn(f) = &core[1] else { panic!() };
    assert_eq!((f.span.lo, f.span.hi), (3, 3));
    let Body::Bare(stmts) = &f.body else { panic!() };
    assert_eq!(stmts.len(), 4);
    // if stmt: span = its header line (4); else arm = the `}else{` line (6)
    let Core::Stmt { head, body, arms, span } = &stmts[0] else { panic!() };
    assert_eq!((span.lo, span.hi), (4, 4));
    assert!(matches!(head.first(), Some(Atom::Ident(x)) if x == "if"));
    let Body::Bare(inner) = body else { panic!() };
    match &inner[0] {
        Core::Expr(_, sp) => assert_eq!(sp.lo, 5),
        _ => panic!(),
    }
    assert_eq!(arms.len(), 1);
    assert_eq!((arms[0].span.lo, arms[0].span.hi), (6, 6));
    let Body::Bare(else_body) = &arms[0].body else { panic!() };
    match &else_body[0] {
        Core::Expr(atoms, sp) => {
            assert!(matches!(atoms.first(), Some(Atom::Ident(x)) if x == "throw"));
            assert_eq!(sp.lo, 7);
        }
        _ => panic!(),
    }
    // match stmt: arms carry their own IR lines (arm line / else line)
    let Core::Stmt { arms, span, .. } = &stmts[1] else { panic!() };
    assert_eq!((span.lo, span.hi), (9, 9));
    assert_eq!(arms.len(), 2);
    let get_arm = arms.iter().find(|a| matches!(a.pattern.first(), Some(Atom::Str(_)))).unwrap();
    assert_eq!(get_arm.span.lo, 10);
    let else_arm = arms.iter().find(|a| matches!(a.pattern.first(), Some(Atom::Ident(x)) if x == "else")).unwrap();
    assert_eq!(else_arm.span.lo, 11);
    // var + return expr spans
    let Core::Var(v) = &stmts[2] else { panic!() };
    assert_eq!(v.span.lo, 13);
    let Core::Expr(_, sp) = &stmts[3] else { panic!() };
    assert_eq!(sp.lo, 14);
}

#[test]
fn emit_map_lookups() {
    let core = expander::expand(SNIPPET).unwrap();
    let (js, map) = js::emit_with_map(&core).unwrap();
    // mapped output is byte-identical to the plain emit
    assert_eq!(js, js::emit(&core).unwrap());
    // js line 4 = `    Log(r.Path);` → IR line 5
    assert_eq!(map.js_to_ir(4), Some(Span { lo: 5, hi: 5 }));
    // innermost entry wins for shared lines: `throw Err;` in the else body
    assert_eq!(map.js_to_ir(6), Some(Span { lo: 7, hi: 7 }));
    // reverse: IR arm line 10 ("GET" arm) → its case/break js lines
    assert_eq!(map.ir_to_js(10), Some((12, 14)));
    // `}` of the if-else chain falls inside the else-arm region: it maps to
    // the `}else{` line (innermost entry, not the fallback)
    assert_eq!(map.js_to_ir(7), Some(Span { lo: 6, hi: 6 }));
}

#[test]
fn corpus_emit_map_covers_nonblank_lines() {
    let files = common::corpus_files();
    assert!(files.len() >= 26, "corpus missing on disk: {}", files.len());
    let mut unmapped: Vec<String> = Vec::new();
    for (path, src) in &files {
        let core = expander::expand(src)
            .unwrap_or_else(|e| panic!("{}: expand failed: {e}", path.display()));
        let (js, map) = js::emit_with_map(&core)
            .unwrap_or_else(|e| panic!("{}: emit failed: {e}", path.display()));
        assert!(
            !js.contains('\u{1}') && !js.contains('\u{2}'),
            "{}: span markers leaked into output",
            path.display()
        );
        for e in &map.entries {
            assert!(e.js_lo >= 1 && e.js_lo <= e.js_hi, "{}: bad range", path.display());
        }
        // every non-blank JS line resolves to a construct (synthetic forms
        // may legitimately map to 0 — still mapped)
        for (i, line) in js.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            if map.js_to_ir(i + 1).is_none() {
                unmapped.push(format!("{}:{} {}", path.display(), i + 1, line.trim()));
            }
        }
    }
    assert!(
        unmapped.is_empty(),
        "unmapped JS lines:\n{}",
        unmapped.join("\n")
    );
}

#[test]
fn lower_errors_carry_ir_line() {
    // an orphan `catch` fragment (no try to fold into) fails lowering; the
    // error must name the IR line it came from (emit_core attaches span.lo)
    let src = "fn F(Bool p){\n  catch NotFound return null\n}\n";
    let err = js::emit(&expander::expand(src).unwrap());
    match err {
        Ok(_) => panic!("expected lower error for orphan catch"),
        Err(e) => {
            assert_eq!(e.ir_line, Some(2));
            assert!(e.to_string().contains("IR line 2"), "{e}");
        }
    }
}