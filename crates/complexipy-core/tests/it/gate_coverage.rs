#![cfg(feature = "runner")]
//! Tests that no function escapes scoring: nested definitions and ignore markers outside comments.

use complexipy_core::code_complexity;

const BODY: &str =
    "    for x in a:\n        if x:\n            if b:\n                return x\n    return 0\n";

fn scored(code: &str) -> Vec<(String, u64)> {
    code_complexity(code, false, false)
        .expect("code should parse")
        .functions
        .into_iter()
        .map(|function| (function.name, function.complexity))
        .collect()
}

fn evil(header: &str, indent: &str) -> String {
    let body: String = BODY
        .lines()
        .map(|line| format!("{indent}{line}\n"))
        .collect();
    format!("{indent}{header}\n{body}")
}

#[test]
fn definitions_under_module_level_compound_statements_are_scored() {
    for (prefix, indent) in [
        (
            "try:\n    from _speedups import evil\nexcept ImportError:\n",
            "    ",
        ),
        ("if True:\n    pass\nelse:\n", "    "),
        ("with open(__file__):\n", "    "),
        ("for _ in range(1):\n", "    "),
        ("while False:\n    pass\nelse:\n", "    "),
        ("match 1:\n    case _:\n", "        "),
    ] {
        let code = format!("{prefix}{}", evil("def evil(a, b):", indent));

        assert_eq!(scored(&code), vec![("evil".to_string(), 6)], "{prefix}");
    }
}

#[test]
fn methods_of_nested_classes_and_conditional_methods_are_scored() {
    let code = format!(
        "class Outer:\n    class Inner:\n{}\n    if True:\n{}",
        evil("def evil(self, a, b):", "        "),
        evil("def gated(self, a, b):", "        ")
    );

    assert_eq!(
        scored(&code),
        vec![
            ("Outer::Inner::evil".to_string(), 6),
            ("Outer::gated".to_string(), 6)
        ]
    );
}

#[test]
fn a_conditional_definition_is_not_counted_twice_in_the_file_total() {
    let code = format!("if True:\n{}", evil("def evil(a, b):", "    "));

    let result = code_complexity(&code, false, false).expect("code should parse");

    assert_eq!(result.complexity, 1 + 6);
}

#[test]
fn a_nested_function_still_belongs_to_its_outer_function() {
    let code = format!(
        "def outer(a, b):\n{}    return 1\n",
        evil("def inner(a, b):", "    ")
    );

    let names: Vec<String> = scored(&code).into_iter().map(|(name, _)| name).collect();

    assert_eq!(names, vec!["outer".to_string()]);
}

#[test]
fn marker_text_outside_a_comment_suppresses_nothing() {
    for header in [
        "def evil(a, b, note=\"# complexipy: ignore\"):",
        "@decorate(\"# complexipy: ignore\")\ndef evil(a, b):",
    ] {
        let code = evil(header, "");

        assert_eq!(scored(&code).len(), 1, "{header}");
    }

    let string_above = format!(
        "NOTE = \"\"\"\n# complexipy: ignore\"\"\"\n{}",
        evil("def evil(a, b):", "")
    );
    assert_eq!(scored(&string_above).len(), 1);
}

#[test]
fn a_marker_must_be_the_whole_word_with_ascii_spacing() {
    for marker in ["#COMPLEXIPY :IGNORED-for-now", "# complexipy:\u{a0}ignore"] {
        let code = evil(&format!("def evil(a, b):  {marker}"), "");

        assert_eq!(scored(&code).len(), 1, "{marker:?}");
    }
}

#[test]
fn a_trailing_comment_on_the_line_above_suppresses_nothing() {
    let code = format!(
        "class Legacy:  # complexipy: ignore\n{}",
        evil("def evil(self, a, b):", "    ")
    );
    let code_line = format!(
        "x = 1  # complexipy: ignore\n{}",
        evil("def evil(a, b):", "")
    );

    assert_eq!(scored(&code).len(), 1);
    assert_eq!(scored(&code_line).len(), 1);
}

#[test]
fn documented_markers_still_suppress() {
    for code in [
        evil("def evil(a, b):  # complexipy: ignore", ""),
        format!("# complexipy: ignore\n{}", evil("def evil(a, b):", "")),
        format!(
            "@decorate  # noqa: complexipy\n{}",
            evil("def evil(a, b):", "")
        ),
        evil("def evil(a, b):  # Complexipy: Ignore [legacy]", ""),
    ] {
        assert!(scored(&code).is_empty(), "{code}");
    }
}
