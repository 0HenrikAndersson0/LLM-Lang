
fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(|s| s.as_str()) {
        Some("lex") => lex_files(&args[1..]),
        Some("parse") => parse_files(&args[1..]),
        _ => {
            eprintln!("usage: irc lex <files...> | irc parse <files...>");
            std::process::exit(2);
        }
    }
}

fn lex_files(paths: &[String]) {
    for path in paths {
        let src = std::fs::read_to_string(path).expect("read");
        match ir_lang::lexer::lex(&src) {
            Ok(l) => println!("{path}: {} tokens", l.tokens.len()),
            Err(e) => println!("{path}: ERROR {e}"),
        }
    }
}

fn parse_files(paths: &[String]) {
    for path in paths {
        let src = std::fs::read_to_string(path).expect("read");
        match ir_lang::reader::read(&src) {
            Ok(forms) => {
                let (lines, dirs) = count(&forms);
                println!("{path}: {} top forms ({} lines, {} directives)", forms.len(), lines, dirs);
            }
            Err(e) => println!("{path}: ERROR {e}"),
        }
    }
}

fn count(forms: &[ir_lang::ast::Form]) -> (usize, usize) {
    let mut lines = 0;
    let mut dirs = 0;
    for f in forms {
        match f {
            ir_lang::ast::Form::Directive(_) => dirs += 1,
            ir_lang::ast::Form::Line { children, .. } => {
                lines += 1;
                let (l, d) = count(children);
                lines += l;
                dirs += d;
            }
        }
    }
    (lines, dirs)
}
