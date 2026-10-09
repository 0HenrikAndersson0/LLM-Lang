//! AST: the reader produces a line/block structure; the expander lowers it
//! to core forms. Atoms keep enough structure for both printers.

/// Source span, line-granular: 1-based inclusive line numbers.
/// `lo` is the construct's header line (the anchor for IR↔JS mapping);
/// `hi` is the last line of this form's own header run — child forms carry
/// their own spans, so whole-construct extent is reconstructable by walking.
/// `lo == 0` marks synthetic (built by the expander, not read from source).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub lo: usize,
    pub hi: usize,
}

impl Span {
    pub fn point(line: usize) -> Span {
        Span { lo: line, hi: line }
    }
    /// extent covering both spans (construct-level extent)
    pub fn covering(self, other: Span) -> Span {
        Span { lo: self.lo.min(other.lo), hi: self.hi.max(other.hi) }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Form {
    /// file header directive: `@1`, `@ns X.Y`, `@use P` — payload is the raw atoms
    Directive(Vec<Atom>, Span),
    /// a logical line construct: header atoms + optional inline `{...}` body
    /// + child statements (from deeper-indent lines)
    Line {
        atoms: Vec<Atom>,
        /// inline body between `{...}` on the header line (atoms, not parsed forms)
        inline_body: Option<Vec<Atom>>,
        /// continuation lines forming a braceless body
        children: Vec<Form>,
        span: Span,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Atom {
    Ident(String),
    Number(String),
    /// string/template/regex literal payload (verbatim; interp `{}` inside)
    Str(String),
    Sym(String),
    /// regex literal: body + closing / + flags
    Regex(String),
}

impl std::fmt::Display for Atom {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Atom::Ident(s) => write!(f, "{s}"),
            Atom::Number(s) => write!(f, "{s}"),
            Atom::Str(s) => write!(f, "{s:?}"),
            Atom::Sym(s) => write!(f, "{s}"),
            Atom::Regex(s) => write!(f, "/{s}"),
        }
    }
}