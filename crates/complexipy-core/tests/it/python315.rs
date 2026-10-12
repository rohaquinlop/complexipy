//! Tests for Python 3.15 syntax: lazy imports and unpacking in comprehensions.

use complexipy_core::AnalysisOptions;
use complexipy_core::cognitive_complexity::code_complexity_shared;

fn score(source: &str) -> u64 {
    code_complexity_shared(source, &AnalysisOptions::default())
        .expect("source should parse")
        .complexity
}

fn function_returning(expression: &str) -> String {
    format!("def f(xss):\n    if xss:\n        return {expression}\n    return None\n")
}

#[test]
fn lazy_imports_parse_and_add_nothing() {
    let body = "\ndef f(x):\n    if x:\n        return json, path\n";
    let lazy = format!("lazy import json\nlazy from os import path\n{body}");
    let eager = format!("import json\nfrom os import path\n{body}");

    assert_eq!(score(&lazy), score(&eager));
    assert_eq!(score(&lazy), 1);
}

#[test]
fn unpacking_comprehensions_score_like_plain_ones() {
    let pairs = [
        ("[*xs for xs in xss", "[xs for xs in xss", "]"),
        ("{*xs for xs in xss", "{xs for xs in xss", "}"),
        ("(*xs for xs in xss", "(xs for xs in xss", ")"),
        ("{**xs for xs in xss", "{k: v for k, v in xss", "}"),
    ];

    for (unpacked, plain, close) in pairs {
        for condition in ["", " if xs and xss"] {
            let unpacked = function_returning(&format!("{unpacked}{condition}{close}"));
            let plain = function_returning(&format!("{plain}{condition}{close}"));
            assert_eq!(score(&unpacked), score(&plain), "{unpacked}");
        }
    }
}

#[test]
fn lazy_is_still_a_name() {
    let source = "def f(xs):\n    lazy = 1\n    if lazy:\n        return xs\n";

    assert_eq!(score(source), 1);
}
