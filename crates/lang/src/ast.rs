//! AST: the reader produces a line/block structure; the expander lowers it
//! to core forms. Atoms keep enough structure for both printers.

#[derive(Debug, Clone, PartialEq)]
pub enum Form {
    /// file header directive: `@1`, `@ns X.Y`, `@use P` — payload is the raw atoms
    Directive(Vec<Atom>),
    /// a logical line construct: header atoms + optional inline `{...}` body
    /// + child statements (from deeper-indent lines)
    Line {
        atoms: Vec<Atom>,
        /// inline body between `{...}` on the header line (atoms, not parsed forms)
        inline_body: Option<Vec<Atom>>,
        /// continuation lines forming a braceless body
        children: Vec<Form>,
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