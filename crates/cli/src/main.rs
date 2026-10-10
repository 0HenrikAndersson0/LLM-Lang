fn main() {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    // `--expand` before the subcommand = debug view on parse (task 1.5);
    // `irc expand <files>` = expanded-core debug view directly (task 2.5 alias)
    let debug = args.iter().position(|a| a == "--expand").map(|i| args.remove(i)).is_some();
    // `--map` on to-js: write <file>.js + <file>.map.json sidecar (task 2.6)
    let map = args.iter().position(|a| a == "--map").map(|i| args.remove(i)).is_some();
    match args.first().map(|s| s.as_str()) {
        Some("lex") => lex_files(&args[1..]),
        Some("parse") => parse_files(&args[1..], debug),
        Some("expand") => expand_files(&args[1..]),
        Some("to-js") => to_js_files(&args[1..], map),
        Some("to-cs") => to_cs_files(&args[1..]),
        Some("from-js") => from_js_files(&args[1..]),
        Some("from-cs") => {
            for path in &args[1..] {
                let src = std::fs::read_to_string(path).expect("read");
                match ir_lang::cs_read::read_cs(&src) {
                    Ok(core) => print!("{}", ir_lang::printer::print(&core)),
                    Err(e) => println!("// {path}: CS-READ-ERR {e}"),
                }
            }
        }
        Some("canon") => canon_files(&args[1..]),
        Some("js-kinds") => {
            // PLAN 4.2 groundwork: node-kind histogram over a JS/TS source dir
            for dir in &args[1..] {
                match ir_lang::js_read::corpus_kinds(std::path::Path::new(dir)) {
                    Ok((hist, failures)) => {
                        for (k, v) in &hist {
                            println!("{v:>5}  {k}");
                        }
                        for f in &failures {
                            println!("FAILURE  {f}");
                        }
                    }
                    Err(e) => println!("{dir}: ERROR {e}"),
                }
            }
        }
        _ => {
            eprintln!(
                "usage: irc [--expand] lex|parse|expand|to-js|from-js <files...>\n       irc to-js --map <files...>  write <file>.js + <file>.map.json (IR↔JS line map, 2.6)\n       irc from-js <js/ts files>   JS → canonical IR text (4.4 round-trip leg)\n       irc js-kinds <dir>        node-kind histogram over a JS/TS dir (4.2 groundwork)\n       --expand  show macro-expanded core forms instead of the canonical print"
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

fn to_js_files(paths: &[String], map: bool) {
    for path in paths {
        let src = std::fs::read_to_string(path).expect("read");
        match ir_lang::expander::expand(&src) {
            Ok(core) => match ir_lang::js::emit_with_map(&core) {
                Ok((js, sm)) => {
                    if map {
                        let stem = std::path::Path::new(path).with_extension("");
                        let js_path = format!("{}.js", stem.display());
                        let map_path = format!("{}.map.json", stem.display());
                        std::fs::write(&js_path, &js).unwrap_or_else(|e| panic!("write {js_path}: {e}"));
                        std::fs::write(&map_path, sm.to_json(path))
                            .unwrap_or_else(|e| panic!("write {map_path}: {e}"));
                        println!(
                            "{path}: wrote {js_path} ({} lines, {} map entries)",
                            js.lines().count(),
                            sm.entries.len()
                        );
                    } else {
                        print!("{js}");
                    }
                }
                Err(e) => println!("// {path}: LOWER-ERR {e}"),
            },
            Err(e) => println!("// {path}: ERROR {e}"),
        }
    }
}

fn canon_files(paths: &[String]) {
    for path in paths {
        let src = std::fs::read_to_string(path).expect("read");
        match ir_lang::expander::expand(&src) {
            Ok(core) => print!("{}", ir_lang::printer::print(&core)),
            Err(e) => println!("// {path}: ERROR {e}"),
        }
    }
}

fn to_cs_files(paths: &[String]) {
    for path in paths {
        let src = std::fs::read_to_string(path).expect("read");
        match ir_lang::expander::expand(&src) {
            Ok(core) => match ir_lang::cs::emit(&core) {
                Ok(text) => print!("{text}"),
                Err(e) => println!("// {path}: CS-ERR {e}"),
            },
            Err(e) => println!("// {path}: ERROR {e}"),
        }
    }
}

fn from_js_files(paths: &[String]) {
    for path in paths {
        let src = std::fs::read_to_string(path).expect("read");
        match ir_lang::js_read::read_js(&src) {
            Ok(core) => print!("{}", ir_lang::printer::print(&core)),
            Err(e) => println!("// {path}: JS-READ-ERR {e}"),
        }
    }
}

fn count(forms: &[ir_lang::ast::Form]) -> (usize, usize) {
    let mut lines = 0;
    let mut dirs = 0;
    for f in forms {
        match f {
            ir_lang::ast::Form::Directive(..) => dirs += 1,
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