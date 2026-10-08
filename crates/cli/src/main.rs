fn main() {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    // `--expand` before the subcommand = debug view on parse (task 1.5);
    // `irc expand <files>` = expanded-core debug view directly (task 2.5 alias)
    let debug = args.iter().position(|a| a == "--expand").map(|i| args.remove(i)).is_some();
    match args.first().map(|s| s.as_str()) {
        Some("lex") => lex_files(&args[1..]),
        Some("parse") => parse_files(&args[1..], debug),
        Some("expand") => expand_files(&args[1..]),
        Some("to-js") => to_js_files(&args[1..]),
        _ => {
            eprintln!(
                "usage: irc [--expand] lex|parse|expand|to-js <files...>\n       --expand  show macro-expanded core forms instead of the canonical print"
            );
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

fn parse_files(paths: &[String], debug: bool) {
    for path in paths {
        let src = std::fs::read_to_string(path).expect("read");
        if debug {
            match ir_lang::expander::expand(&src) {
                Ok(core) => print!("{}", ir_lang::printer::print_debug(&core)),
                Err(e) => println!("{path}: ERROR {e}"),
            }
            continue;
        }
        match ir_lang::reader::read(&src) {
            Ok(forms) => {
                let (lines, dirs) = count(&forms);
                println!("{path}: {} top forms ({} lines, {} directives)", forms.len(), lines, dirs);
            }
            Err(e) => println!("{path}: ERROR {e}"),
        }
    }
}

fn expand_files(paths: &[String]) {
    for path in paths {
        let src = std::fs::read_to_string(path).expect("read");
        match ir_lang::expander::expand(&src) {
            Ok(core) => print!("{}", ir_lang::printer::print_debug(&core)),
            Err(e) => println!("{path}: ERROR {e}"),
        }
    }
}

fn to_js_files(paths: &[String]) {
    for path in paths {
        let src = std::fs::read_to_string(path).expect("read");
        match ir_lang::expander::expand(&src) {
            Ok(core) => match ir_lang::js::emit(&core) {
                Ok(js) => print!("{js}"),
                Err(e) => println!("// {path}: LOWER-ERR {e}"),
            },
            Err(e) => println!("// {path}: ERROR {e}"),
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