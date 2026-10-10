//! Golden-corpus snapshot tests (task 1.7): every fixture file in
//! bench/variants/d and bench/corpus/real/ir must parse, expand into closed
//! core forms, and print back to a stable canonical tree (print→print is
//! identity). File-not-in-tree runs are tolerated as skipped.

mod common;

use llm_lang::expander;
use llm_lang::printer;

fn corpus_files() -> Vec<(std::path::PathBuf, String)> {
    common::corpus_files()
}

#[test]
fn every_fixture_parses_and_prints() {
    let files = corpus_files();
    assert!(
        files.len() >= 26,
        "expected the full corpus on disk; found {}",
        files.len()
    );
    // canonical layout is stable across re-print
    for (path, src) in &files {
        let core = expander::expand(src)
            .unwrap_or_else(|e| panic!("{}: expand failed: {e}", path.display()));
        assert!(
            expander::is_closed(&core),
            "{}: core forms not closed",
            path.display()
        );
        let text = printer::print(&core);
        // re-read the printed canon: must still expand
        let core2 = expander::expand(&text)
            .unwrap_or_else(|e| panic!("{}: printed canon does not re-parse: {e}\n{text}", path.display()));
        let text2 = printer::print(&core2);
        assert_eq!(
            text, text2,
            "{}: print is not stable (print→print must be identity)",
            path.display()
        );
    }
}

#[test]
fn golden01_canonical_shape() {
    let core = expander::expand("@1 @ns Acme.Users\n\nfn GetUser(Int id) User?{await Users.Find id}\n").unwrap();
    let text = printer::print(&core);
    assert_eq!(
        text,
        "@1 @ns Acme.Users\n\nfn GetUser(id Int) User?{await Users.Find id}\n"
    );
}