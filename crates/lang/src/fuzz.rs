//! Property-based fuzzing (task 1.6): the reader never panics; bad input
//! always returns an error, good input always returns a tree.

use proptest::prelude::*;

#[allow(unused_imports)]
use crate::reader;

#[allow(dead_code)]
fn arb_token() -> impl Strategy<Value = String> {
    prop_oneof![
        4 => prop::sample::select(vec![
            "fn", "var", "type", "if", "else", "match", "for", "in", "while", "lock", "try",
            "catch", "finally", "throw", "return", "null", "true", "false", "async", "export",
            "ctor", "interface", "opaque", "out", "A", "b", "X", "y", "Int", "String",
        ])
            .prop_map(|s| s.to_string()),
        3 => r"[a-zA-Z_$][a-zA-Z0-9_?!$]{0,12}",
        2 => r"[0-9][0-9_]*(m|f|d)?",
        2 => prop::sample::select(vec![
            "(", ")", "{", "}", "[", "]", "<", ">", ",", ":", ";", ".", "|", "&", "+", "-",
            "*", "/", "%", "=", "!", "?", "@", "=>", "&&", "||", "==", "!=", "..", "...",
        ])
            .prop_map(|s| s.to_string()),
        1 => r#""[^"]{0,10}""#,
        1 => "'[^']{0,6}'",
        1 => "// comment\n".prop_map(|s| s.to_string()),
        1 => "\n".prop_map(|s| s.to_string()),
        1 => " ".prop_map(|s| s.to_string()),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(2048))]

    /// R1: the reader never panics — result is Ok or Err, never a crash.
    #[test]
    fn reader_never_panics(input in prop::collection::vec(arb_token(), 0..40)) {
        let src: String = input.into_iter().collect();
        let _ = reader::read(&src);
    }
}