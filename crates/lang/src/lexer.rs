use std::fmt;

/// A lexical token from an IR source file.
#[derive(Debug, Clone, PartialEq)]
pub enum Tok {
    /// `@1`, `@ns`, `@use` directive marker (the `@` itself; payload parsed by reader)
    At,
    /// identifier or keyword: [letter_][letter_0-9?!$*]*
    Ident(String),
    /// integer or decimal literal, optionally with suffix: `42`, `100m`, `1_000`, `1.5f`
    Number(String),
    /// string literal payload after unescaping: "…{expr}…"
    Str(String),
    /// regex literal: body + closing / + flags
    Regex(String),
    /// punctuation/operator
    Sym(String),
    /// end of file
    Eof,
}

impl fmt::Display for Tok {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Tok::At => write!(f, "@"),
            Tok::Ident(s) => write!(f, "{s}"),
            Tok::Number(s) => write!(f, "{s}"),
            Tok::Str(s) => write!(f, "\"{s}\""),
            Tok::Regex(s) => write!(f, "/{s}"),
            Tok::Sym(s) => write!(f, "{s}"),
            Tok::Eof => write!(f, "<eof>"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct LexError {
    pub line: usize,
    pub col: usize,
    pub msg: String,
}

impl fmt::Display for LexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}: {}", self.line, self.col, self.msg)
    }
}

#[derive(Debug)]
pub struct Lexed {
    pub tokens: Vec<(Tok, usize, usize)>, // token, line, col
}

/// Multi-char operator set, longest-match first.
const SYMS3: &[&str] = &["...", "===", "??="];
const SYMS2: &[&str] = &[
    "&&", "||", "==", "!=", "<=", ">=", "+=", "-=", "*=", "/=", "%=", "=>", "->", "??", "..",
    "!!",
];
const SYMS1: &[&str] = &["(", ")", "{", "}", "[", "]", "<", ">", ",", ":", ";", ".", "|", "&",
    "+", "-", "*", "/", "%", "=", "!", "?", "@", "_", "^", "~", "#"];

fn is_ident_start(c: char) -> bool {
    c.is_alphabetic() || c == '_' || c == '$'
}

fn is_ident_continue(c: char) -> bool {
    c.is_alphanumeric() || matches!(c, '_' | '$' | '?' | '!' | '·')
}

/// Distinguish regex `/.../` from the division operator `/` and comments:
/// a regex follows `(`/`=`/`,`/start-of-line and doesn't start with `/` or space.
fn looks_like_regex(chars: &[char], i: usize) -> bool {
    // comment already handled before this call; here i points at `/` itself
    if i == 0 {
        // start of file: `/(a)/g` is a regex; `//` was caught as comment
        return true;
    }
    // find the last significant (non-space, non-newline) char before i
    let mut j = i;
    let mut prev;
    loop {
        if j == 0 {
            return true; // only whitespace before: standalone regex
        }
        prev = chars[j - 1];
        if prev != ' ' && prev != '\n' && prev != '\t' {
            break;
        }
        j -= 1;
    }
    match prev {
        '(' | ',' | '=' | '[' | '{' | ':' | '!' | '&' | '|' | '?' | '+' | '-' | '*' | '%' |
        '<' | '>' | ';' => true,
        // after an identifier/number: could be either. In JS practice regex
        // can't follow a value (division); choose division. Regexes that start
        // a line follow `(`/`=` above or begin after EOL — treat newline-
        // skipped as significant (same rule).
        _ => false,
    }
}

pub fn lex(src: &str) -> Result<Lexed, LexError> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = src.chars().collect();
    let mut i = 0usize;
    let mut line = 1usize;
    let mut col = 1usize;

    macro_rules! push {
        ($t:expr, $l:expr, $c:expr) => {
            tokens.push(($t, $l, $c))
        };
    }

    while i < chars.len() {
        let c = chars[i];
        let (tl, tc) = (line, col);

        // whitespace / line tracking
        if c == '\n' {
            line += 1;
            col = 1;
            i += 1;
            continue;
        }
        if c.is_whitespace() {
            i += 1;
            col += 1;
            continue;
        }

        // comments: // to end of line
        if c == '/' && i + 1 < chars.len() && chars[i + 1] == '/' {
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
                col += 1;
            }
            continue;
        }

        // strings with interpolation: "..."
        if c == '"' {
            let mut payload = String::new();
            i += 1;
            col += 1;
            let mut closed = false;
            while i < chars.len() {
                let ch = chars[i];
                if ch == '"' {
                    i += 1;
                    col += 1;
                    closed = true;
                    break;
                }
                if ch == '\\' && i + 1 < chars.len() {
                    // unescape now (payload = post-unescape content)
                    payload.push(unescape_char(chars[i + 1]));
                    i += 2;
                    col += 2;
                    continue;
                }
                if ch == '\n' {
                    return Err(LexError {
                        line: tl,
                        col: tc,
                        msg: "unterminated string (newline in string literal)".into(),
                    });
                }
                payload.push(ch);
                i += 1;
                col += 1;
            }
            if !closed {
                return Err(LexError { line: tl, col: tc, msg: "unterminated string".into() });
            }
            push!(Tok::Str(payload), tl, tc);
            continue;
        }

        // single-quoted strings/chars: no interpolation (design doc §1)
        if c == '\'' {
            let mut payload = String::new();
            i += 1;
            col += 1;
            let mut closed = false;
            while i < chars.len() {
                let ch = chars[i];
                if ch == '\'' {
                    i += 1;
                    col += 1;
                    closed = true;
                    break;
                }
                if ch == '\\' && i + 1 < chars.len() {
                    payload.push(ch);
                    payload.push(chars[i + 1]);
                    i += 2;
                    col += 2;
                    continue;
                }
                if ch == '\n' {
                    return Err(LexError {
                        line: tl,
                        col: tc,
                        msg: "unterminated single-quoted string".into(),
                    });
                }
                payload.push(ch);
                i += 1;
                col += 1;
            }
            if !closed {
                return Err(LexError { line: tl, col: tc, msg: "unterminated single-quoted string".into() });
            }
            push!(Tok::Str(payload), tl, tc);
            continue;
        }

        // numbers: digits with _, suffix letters m/f/d etc, decimals
        if c.is_ascii_digit() {
            let mut s = String::new();
            let mut has_dot = false;
            while i < chars.len() {
                let ch = chars[i];
                if ch.is_ascii_digit() || ch == '_' {
                    s.push(ch);
                    i += 1;
                    col += 1;
                } else if ch == '.' && !has_dot
                    && i + 1 < chars.len()
                    && (chars[i + 1].is_ascii_digit())
                {
                    has_dot = true;
                    s.push(ch);
                    i += 1;
                    col += 1;
                } else if ch.is_ascii_alphabetic() {
                    // suffix (m, f, d, L, ul, x…): consume letters
                    while i < chars.len() && chars[i].is_ascii_alphanumeric() {
                        s.push(chars[i]);
                        i += 1;
                        col += 1;
                    }
                    break;
                } else {
                    break;
                }
            }
            push!(Tok::Number(s), tl, tc);
            continue;
        }

        // backtick template: alias of interpolated string (design doc §1)
        if c == '`' {
            let mut payload = String::new();
            i += 1;
            col += 1;
            let mut closed = false;
            while i < chars.len() {
                let ch = chars[i];
                if ch == '`' {
                    i += 1;
                    col += 1;
                    closed = true;
                    break;
                }
                if ch == '\\' && i + 1 < chars.len() {
                    payload.push(ch);
                    payload.push(chars[i + 1]);
                    i += 2;
                    col += 2;
                    continue;
                }
                if ch == '\n' {
                    return Err(LexError {
                        line: tl,
                        col: tc,
                        msg: "unterminated template literal".into(),
                    });
                }
                payload.push(ch);
                i += 1;
                col += 1;
            }
            if !closed {
                return Err(LexError { line: tl, col: tc, msg: "unterminated template literal".into() });
            }
            push!(Tok::Str(payload), tl, tc);
            continue;
        }

        // regex literal: /.../flags — payload kept verbatim
        if c == '/' && looks_like_regex(&chars, i) {
            let mut payload = String::new();
            i += 1; // opening /
            col += 1;
            let mut in_class = false; // char-class [...] can contain raw /
            let mut closed = false;
            while i < chars.len() {
                let ch = chars[i];
                if ch == '\\' && i + 1 < chars.len() {
                    payload.push(ch);
                    payload.push(chars[i + 1]);
                    i += 2;
                    col += 2;
                    continue;
                }
                if ch == '[' {
                    in_class = true;
                } else if ch == ']' {
                    in_class = false;
                } else if ch == '/' && !in_class {
                    // include the closing delimiter, then flags
                    payload.push(ch);
                    i += 1;
                    col += 1;
                    while i < chars.len() && chars[i].is_ascii_alphabetic() {
                        payload.push(chars[i]);
                        i += 1;
                        col += 1;
                    }
                    closed = true;
                    break;
                }
                if ch == '\n' {
                    return Err(LexError { line: tl, col: tc, msg: "unterminated regex literal".into() });
                }
                payload.push(ch);
                i += 1;
                col += 1;
            }
            if !closed {
                return Err(LexError { line: tl, col: tc, msg: "unterminated regex literal".into() });
            }
            push!(Tok::Regex(payload), tl, tc);
            continue;
        }

        // identifiers / keywords
        if is_ident_start(c) {
            let mut s = String::new();
            while i < chars.len() && is_ident_continue(chars[i]) {
                s.push(chars[i]);
                i += 1;
                col += 1;
            }
            push!(Tok::Ident(s), tl, tc);
            continue;
        }

        // operators, longest-match
        let rest: String = chars[i..].iter().collect();
        let matched = SYMS3
            .iter()
            .chain(SYMS2.iter())
            .find(|s| rest.starts_with(**s))
            .copied();
        if let Some(m) = matched {
            let n = m.chars().count();
            push!(Tok::Sym(m.to_string()), tl, tc);
            i += n;
            col += n;
            continue;
        }
        if SYMS1.contains(&c.to_string().as_str()) {
            push!(Tok::Sym(c.to_string()), tl, tc);
            i += 1;
            col += 1;
            continue;
        }

        return Err(LexError {
            line: tl,
            col: tc,
            msg: format!("unexpected character {c:?}"),
        });
    }

    push!(Tok::Eof, line, col);
    Ok(Lexed { tokens })
}

#[cfg(test)]
mod tests {
    use super::*;
    use Tok::*;

    fn toks(src: &str) -> Vec<Tok> {
        lex(src).unwrap().tokens.into_iter().map(|(t, _, _)| t).collect()
    }

    #[test]
    fn directives() {
        assert_eq!(
            toks("@1 @ns Acme.Users"),
            vec![Sym("@".into()), Number("1".into()), Sym("@".into()), Ident("ns".into()), Ident("Acme".into()), Sym(".".into()), Ident("Users".into()), Eof]
        );
    }

    #[test]
    fn fn_header() {
        assert_eq!(
            toks("fn GetUser(Int id) User?{"),
            vec![
                Ident("fn".into()), Ident("GetUser".into()),
                Sym("(".into()), Ident("Int".into()), Ident("id".into()),
                Sym(")".into()), Ident("User?".into()), Sym("{".into()),
                Eof
            ]
        );
    }

    #[test]
    fn ops_longest_match() {
        assert_eq!(
            toks("o=>o.Paid&&o.Total>100m"),
            vec![
                Ident("o".into()), Sym("=>".into()), Ident("o".into()), Sym(".".into()),
                Ident("Paid".into()), Sym("&&".into()), Ident("o".into()), Sym(".".into()),
                Ident("Total".into()), Sym(">".into()), Number("100m".into()), Eof
            ]
        );
    }

    #[test]
    fn string_interp() {
        assert_eq!(
            toks("\"Hello, {u.Name}!\""),
            vec![Str("Hello, {u.Name}!".into()), Eof]
        );
        // unescape happens at lex time: payload holds post-unescape content
        assert_eq!(toks("\"a\\\"b\""), vec![Str("a\"b".into()), Eof]);
        // single-quoted strings lex, no interpolation
        assert_eq!(toks("'k'"), vec![Str("k".into()), Eof]);
        assert_eq!(toks("'[cheerio object]'"), vec![Str("[cheerio object]".into()), Eof]);
        assert!(lex("'never").is_err());
    }

    #[test]
    fn comments_dropped() {
        // `//` inside a string is not a comment
        assert_eq!(toks("// note\nfn f()"), vec![Ident("fn".into()), Ident("f".into()), Sym("(".into()), Sym(")".into()), Eof]);
        assert_eq!(toks("\"http://x\""), vec![Str("http://x".into()), Eof]);
    }

    #[test]
    fn numbers() {
        assert_eq!(toks("1_000_000d 0.5f 42"), vec![Number("1_000_000d".into()), Number("0.5f".into()), Number("42".into()), Eof]);
    }

    #[test]
    fn spread_and_ranges() {
        assert_eq!(
            toks("...baseOptions .. 0..len"),
            vec![Sym("...".into()), Ident("baseOptions".into()), Sym("..".into()), Number("0".into()), Sym("..".into()), Ident("len".into()), Eof]
        );
    }

    #[test]
    fn regex_literals() {
        // regex token payload = body + closing `/` + flags (opening `/`
        // implicit in the token type)
        assert_eq!(toks("= /[._-](\\w|$)/g"), vec![Sym("=".into()), Regex("[._-](\\w|$)/g".into()), Eof]);
        assert_eq!(toks("const r20 = /%20/g"), vec![
            Ident("const".into()), Ident("r20".into()), Sym("=".into()), Regex("%20/g".into()), Eof]);
        assert_eq!(toks("/\\r?\\n/g"), vec![Regex("\\r?\\n/g".into()), Eof]);
        // division stays division
        assert_eq!(toks("a / b"), vec![Ident("a".into()), Sym("/".into()), Ident("b".into()), Eof]);
        assert_eq!(toks("absolute/1000"), vec![Ident("absolute".into()), Sym("/".into()), Number("1000".into()), Eof]);
    }

    #[test]
    fn backtick_templates() {
        assert_eq!(toks("`Hello ${x}`"), vec![Str("Hello ${x}".into()), Eof]);
        assert!(lex("`oops").is_err());
    }

    #[test]
    fn unterminated_string_errors() {
        assert!(lex("\"oops").is_err());
        assert!(lex("\"multi\nline\"").is_err());
    }

    #[test]
    fn unexpected_char_errors() {
        // `~` is lexed as a symbol (used in opaque passthrough only); use a
        // genuinely invalid char for the error path
        assert!(lex("§").is_err());
        let e = lex("\n\n  §bad").unwrap_err();
        assert_eq!(e.line, 3);
        // and confirm position tracking works on valid multi-line input
        let ok = lex("\n\n  @1").unwrap();
        assert_eq!(ok.tokens[0], (Sym("@".into()), 3, 3));
    }
}
fn unescape_char(c: char) -> char {
    match c {
        'n' => '\n',
        't' => '\t',
        'r' => '\r',
        '0' => '\0',
        other => other,
    }
}
