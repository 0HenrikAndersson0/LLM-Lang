fn main() {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    // `--expand` before the subcommand = debug view on parse (task 1.5);
    // `ircc expand <files>` = expanded-core debug view directly (task 2.5 alias)
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
                match llm_lang::cs_read::read_cs(&src) {
                    Ok(core) => print!("{}", llm_lang::printer::print(&core)),
                    Err(e) => println!("// {path}: CS-READ-ERR {e}"),
                }
            }
        }
        Some("canon") => canon_files(&args[1..]),
        Some("verify") => verify_files(&args[1..]),
        Some("doc") => doc_files(&args[1..]),
        Some("install-skill") => install_skill(&args[1..]),
        Some("js-kinds") => {
            // PLAN 4.2 groundwork: node-kind histogram over a JS/TS source dir
            for dir in &args[1..] {
                match llm_lang::js_read::corpus_kinds(std::path::Path::new(dir)) {
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
                "usage: ircc [--expand] lex|parse|expand|to-js|from-js <files...>\n       ircc to-js --map <files...>  write <file>.js + <file>.map.json (IR↔JS line map, 2.6)\n       ircc from-js <js/ts files>   JS → canonical IR text (4.4 round-trip leg)\n       ircc verify <files...>       canon-identity lint (exit 1 on drift)\n       ircc doc <name>              print embedded docs: spec|tutorial|fidelity|versioning|skill|agents\n       ircc install-skill [--agents | --claude | --opencode | --agents-md] [--dir <path>] [--force]\n       ircc js-kinds <dir>        node-kind histogram over a JS/TS dir (4.2 groundwork)\n       --expand  show macro-expanded core forms instead of the canonical print"
            );
            std::process::exit(2);
        }
    }
}

fn lex_files(paths: &[String]) {
    for path in paths {
        let src = std::fs::read_to_string(path).expect("read");
        match llm_lang::lexer::lex(&src) {
            Ok(l) => println!("{path}: {} tokens", l.tokens.len()),
            Err(e) => println!("{path}: ERROR {e}"),
        }
    }
}

fn parse_files(paths: &[String], debug: bool) {
    for path in paths {
        let src = std::fs::read_to_string(path).expect("read");
        if debug {
            match llm_lang::expander::expand(&src) {
                Ok(core) => print!("{}", llm_lang::printer::print_debug(&core)),
                Err(e) => println!("{path}: ERROR {e}"),
            }
            continue;
        }
        match llm_lang::reader::read(&src) {
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
        match llm_lang::expander::expand(&src) {
            Ok(core) => print!("{}", llm_lang::printer::print_debug(&core)),
            Err(e) => println!("{path}: ERROR {e}"),
        }
    }
}

fn to_js_files(paths: &[String], map: bool) {
    for path in paths {
        let src = std::fs::read_to_string(path).expect("read");
        match llm_lang::expander::expand(&src) {
            Ok(core) => match llm_lang::js::emit_with_map(&core) {
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

fn verify_files(paths: &[String]) {
    // task 1.7's print→print identity as a user lint: canon text is stable
    // across re-parse; any drift = the file would churn every tool round
    let mut bad = 0;
    for path in paths {
        let Ok(src) = std::fs::read_to_string(path) else {
            bad += 1;
            println!("{path}: unreadable (missing or not UTF-8)");
            continue;
        };
        match llm_lang::expander::expand(&src) {
            Ok(core) => {
                let canon = llm_lang::printer::print(&core);
                // machine-canon files must be their own fixed point: canon1
                // drift (file != print(expand(file))) is invisible to the
                // two-round compare below if round-3 re-normalizes — the
                // dict-initializer row was round-stable under the old lint
                // yet lost data every round. Hand-canon files are fixture
                // INPUT (comment rows, blank lines, pre-normalization shapes)
                // and keep the two-round check only.
                let hand_canon = src
                    .lines()
                    .any(|l| l.trim_start().starts_with("//") || l.trim().is_empty());
                if !hand_canon {
                    let file_canon: String = src
                        .lines()
                        .collect::<Vec<_>>()
                        .join("\n");
                    if canon.trim_end() != file_canon.trim_end() {
                        bad += 1;
                        println!("{path}: canon DRIFT (file != print(expand(file)))");
                        continue;
                    }
                }
                match llm_lang::expander::expand(&canon) {
                    Ok(core2) => {
                        let canon2 = llm_lang::printer::print(&core2);
                        if canon == canon2 {
                            println!("{path}: OK ({} lines, {} bytes canon)", canon.lines().count(), canon.len());
                        } else {
                            bad += 1;
                            println!("{path}: UNSTABLE canon (print→print drift)");
                        }
                    }
                    Err(e) => {
                        bad += 1;
                        println!("{path}: canon does not re-parse: {e}");
                    }
                }
            }
            Err(e) => {
                bad += 1;
                println!("{path}: expand failed: {e}");
            }
        }
    }
    if bad > 0 {
        std::process::exit(1);
    }
}

fn canon_files(paths: &[String]) {
    for path in paths {
        let src = std::fs::read_to_string(path).expect("read");
        match llm_lang::expander::expand(&src) {
            Ok(core) => print!("{}", llm_lang::printer::print(&core)),
            Err(e) => println!("// {path}: ERROR {e}"),
        }
    }
}

fn to_cs_files(paths: &[String]) {
    for path in paths {
        let src = std::fs::read_to_string(path).expect("read");
        match llm_lang::expander::expand(&src) {
            Ok(core) => match llm_lang::cs::emit(&core) {
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
        match llm_lang::js_read::read_js(&src) {
            Ok(core) => print!("{}", llm_lang::printer::print(&core)),
            Err(e) => println!("// {path}: JS-READ-ERR {e}"),
        }
    }
}

// ---------------------------------------------------------------------------
// embedded docs + skill install (downstream distribution)

const ASSET_SKILL: &str = include_str!("../assets/SKILL.md");
const ASSET_AGENTS: &str = include_str!("../assets/AGENTS-MD.example");

fn doc_files(paths: &[String]) {
    let docs: &[(&str, &str)] = &[
        ("skill", ASSET_SKILL),
        ("agents", ASSET_AGENTS),
        ("spec", include_str!("../assets/docs/spec.md")),
        ("tutorial", include_str!("../assets/docs/tutorial.md")),
        ("fidelity", include_str!("../assets/docs/fidelity.md")),
        ("versioning", include_str!("../assets/docs/versioning.md")),
    ];
    if paths.is_empty() || paths[0] == "list" {
        println!("embedded docs:");
        for (name, _) in docs {
            println!("  ircc doc {name}");
        }
        return;
    }
    let name = paths[0].as_str();
    match docs.iter().find(|(n, _)| *n == name) {
        Some((_, text)) => print!("{text}"),
        None => {
            println!("unknown doc '{name}' — try: ircc doc list");
            std::process::exit(2);
        }
    }
}

fn install_skill(args: &[String]) {
    let mut force = false;
    let mut targets: Vec<&str> = vec![];
    let mut dir: Option<String> = None;
    let mut i = 0usize;
    while i < args.len() {
        match args[i].as_str() {
            "--force" => force = true,
            "--agents" => targets.push("agents"),
            "--claude" => targets.push("claude"),
            "--opencode" => targets.push("opencode"),
            "--agents-md" => targets.push("agents-md"),
            "--dir" => {
                i += 1;
                match args.get(i) {
                    Some(d) => dir = Some(d.clone()),
                    None => {
                        println!("--dir needs a path");
                        std::process::exit(2);
                    }
                }
            }
            other => {
                println!("unknown flag '{other}' — usage: ircc install-skill [--agents|claude|opencode|agents-md] [--dir <path>] [--force]");
                std::process::exit(2);
            }
        }
        i += 1;
    }
    if targets.is_empty() {
        targets.push("agents");
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let mut bad = 0;
    let _ = &mut dir;
    for t in targets {
        let (path, text, note): (std::path::PathBuf, &str, &str) = match t {
            "agents" => (
                std::path::PathBuf::from(&home)
                    .join(".agents/skills/llm-lang/SKILL.md"),
                ASSET_SKILL,
                "(the neutral skills convention — opencode auto-loads it; Claude-compatible loaders too)",
            ),
            "claude" => (
                std::path::PathBuf::from(&home)
                    .join(".claude/skills/llm-lang/SKILL.md"),
                ASSET_SKILL,
                "(Claude Code global skills)",
            ),
            "opencode" => (
                std::path::PathBuf::from(".opencode/skills/llm-lang/SKILL.md"),
                ASSET_SKILL,
                "(this project's opencode skills)",
            ),
            "agents-md" => (
                std::path::PathBuf::from(
                    dir.clone().unwrap_or_else(|| ".".to_string()),
                )
                .join("AGENTS.md"),
                ASSET_AGENTS,
                "(Codex / Gemini CLI / Cursor read the repo-root AGENTS.md)",
            ),
            other => {
                println!("unknown target '{other}'");
                std::process::exit(2);
            }
        };
        if path.exists() && !force {
            bad += 1;
            println!(
                "{}: exists — not overwritten (pass --force, or merge manually via `ircc doc {}`)",
                path.display(),
                if t == "agents-md" { "agents" } else { "skill" }
            );
            continue;
        }
        if let Some(parent) = path.parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                println!("{}: create_dir_all failed: {e}", parent.display());
                bad += 1;
                continue;
            }
        }
        match std::fs::write(&path, text) {
            Ok(_) => println!("{}: written {note}", path.display()),
            Err(e) => {
                println!("{}: write failed: {e}", path.display());
                bad += 1;
            }
        }
    }
    if bad > 0 {
        std::process::exit(1);
    }
}

fn count(forms: &[llm_lang::ast::Form]) -> (usize, usize) {
    let mut lines = 0;
    let mut dirs = 0;
    for f in forms {
        match f {
            llm_lang::ast::Form::Directive(..) => dirs += 1,
            llm_lang::ast::Form::Line { children, .. } => {
                lines += 1;
                let (l, d) = count(children);
                lines += l;
                dirs += d;
            }
        }
    }
    (lines, dirs)
}