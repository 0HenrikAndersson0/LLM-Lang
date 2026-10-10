//! Stable check per file: expand→print→expand→print identity.
//! `cargo run -p llm-lang --example dbg -- <files...>`
fn main() {
    for path in std::env::args().skip(1) {
        let src = std::fs::read_to_string(&path).unwrap();
        match llm_lang::expander::expand(&src) {
            Err(e) => println!("{path}: EXP-ERR {e}"),
            Ok(core) => {
                let t1 = llm_lang::printer::print(&core);
                let canon = std::env::var("IR_CANON").ok();
                if let Some(cp) = canon { std::fs::write(cp, &t1).unwrap(); }
                std::fs::write("/tmp/canon_t1.ir", &t1).unwrap();
                match llm_lang::expander::expand(&t1) {
                    Err(e) => println!("{path}: RE-ERR {e}"),
                    Ok(c2) => {
                        let t2 = llm_lang::printer::print(&c2);
                        println!("{path}: {}", if t1 == t2 { "STABLE" } else { "UNSTABLE" });
                        if t1 != t2 {
                            for (i, (a, b)) in t1.lines().zip(t2.lines()).enumerate() {
                                if a != b {
                                    println!("  L{} t1=[{a}]", i + 1);
                                    println!("  L{} t2=[{b}]", i + 1);
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
