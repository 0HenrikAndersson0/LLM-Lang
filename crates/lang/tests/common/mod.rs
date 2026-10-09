//! Shared corpus loader (fixtures + real corpus) for integration tests.

use std::path::PathBuf;

pub fn corpus_files() -> Vec<(PathBuf, String)> {
    // integration tests run from crate dir; corpus is 2 levels up
    let base = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../bench");
    let mut out = Vec::new();
    for root in [base.join("variants/d"), base.join("corpus/real/ir")] {
        let Ok(entries) = std::fs::read_dir(&root) else {
            continue;
        };
        for e in entries.flatten() {
            let p = e.path();
            if p.extension().map(|x| x == "ir").unwrap_or(false) {
                let src = std::fs::read_to_string(&p).unwrap();
                out.push((p, src));
            }
        }
    }
    out
}