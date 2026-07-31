//! Unit tests for `crate::rules::complexity`.
//!
//! Wired in from `src/rules/complexity.rs` via `#[cfg(test)] #[path = ...] mod tests;`
//! so this stays a child module of the code it tests and can reach that module's
//! private helpers through `super::` without widening their visibility.

use super::{combine_conditions_chain, extract_condition_from_line, needs_parens_for_and};

fn combine(parts: &[&str]) -> String {
    let owned: Vec<String> = parts.iter().map(|s| s.to_string()).collect();
    combine_conditions_chain(&owned)
}

#[test]
fn parenthesizes_operands_that_bind_looser_than_and() {
    // `or`, ternaries and `:=` all bind looser than `and`; merging them bare
    // still parses but changes the meaning.
    assert_eq!(combine(&["a or b", "c"]), "(a or b) and c");
    assert_eq!(combine(&["a if b else c", "d"]), "(a if b else c) and d");
    assert_eq!(
        combine(&["n := len(items)", "n < 9"]),
        "(n := len(items)) and n < 9"
    );
}

#[test]
fn leaves_operands_that_bind_tighter_than_and_unwrapped() {
    assert_eq!(combine(&["not a", "b"]), "not a and b");
    assert_eq!(combine(&["a == 1", "b"]), "a == 1 and b");
    assert_eq!(combine(&["items[1:2]", "b"]), "items[1:2] and b");
    assert_eq!(combine(&["x in {1: 2}", "b"]), "x in {1: 2} and b");
}

#[test]
fn does_not_parenthesize_on_operators_inside_strings_or_brackets() {
    // `" or "` here is string content, and the `:=` is nested in a call --
    // neither is a top-level operator, so no parentheses are needed.
    assert!(!needs_parens_for_and(r#"s == "a or b""#));
    assert!(!needs_parens_for_and("f(n := 1)"));
    assert!(!needs_parens_for_and("(lambda x: x)(1)"));
    assert!(needs_parens_for_and("a or b"));
}

#[test]
fn extracts_condition_with_walrus_and_merges_it_safely() {
    let cond = extract_condition_from_line("if n := len(items):").unwrap();
    assert_eq!(cond, "n := len(items)");
    assert_eq!(
        combine(&[cond.as_str(), "n < 9"]),
        "(n := len(items)) and n < 9"
    );
}

#[test]
fn extracts_condition_with_trailing_comment_containing_colon() {
    assert_eq!(
        extract_condition_from_line("if a:  # gate: primary"),
        Some("a".to_string())
    );
}

#[test]
fn extracts_condition_with_unparenthesized_walrus() {
    // The `:` in `:=` must not be mistaken for the statement colon, or the
    // assignment is silently dropped and the merged condition still parses.
    assert_eq!(
        extract_condition_from_line("if n := len(items):"),
        Some("n := len(items)".to_string())
    );
}

#[test]
fn extracts_condition_with_walrus_in_while() {
    assert_eq!(
        extract_condition_from_line("while chunk := f.read():"),
        Some("chunk := f.read()".to_string())
    );
}

#[test]
fn extracts_condition_with_parenthesized_walrus() {
    assert_eq!(
        extract_condition_from_line("if (n := len(items)) > 3:"),
        Some("(n := len(items)) > 3".to_string())
    );
}

#[test]
fn extracts_condition_with_dict_literal_colon() {
    assert_eq!(
        extract_condition_from_line("if x in {1: 2}:"),
        Some("x in {1: 2}".to_string())
    );
}

#[test]
fn extracts_condition_with_inline_body() {
    assert_eq!(
        extract_condition_from_line("if a: return b"),
        Some("a".to_string())
    );
}

#[test]
fn extracts_condition_with_colon_inside_string_literal() {
    assert_eq!(
        extract_condition_from_line(r#"if s == "a:b":"#),
        Some(r#"s == "a:b""#.to_string())
    );
}

#[test]
fn extracts_condition_with_fstring_format_spec_colon() {
    assert_eq!(
        extract_condition_from_line(r#"if f"{x:>3}" == y:"#),
        Some(r#"f"{x:>3}" == y"#.to_string())
    );
}

#[test]
fn extracts_condition_from_elif_via_real_elif_path() {
    assert_eq!(
        extract_condition_from_line(r#"elif kind == "a":"#),
        Some(r#"kind == "a""#.to_string())
    );
}

#[test]
fn extracts_condition_from_while_with_trailing_comment_containing_colon() {
    assert_eq!(
        extract_condition_from_line("while ok:  # loop: go"),
        Some("ok".to_string())
    );
}

#[test]
fn extracts_condition_with_colon_inside_bracketed_string() {
    assert_eq!(
        extract_condition_from_line(r#"if d["k:v"] and e:"#),
        Some(r#"d["k:v"] and e"#.to_string())
    );
}

#[test]
fn returns_none_when_condition_spans_multiple_lines_via_unclosed_paren() {
    assert_eq!(extract_condition_from_line("if (a and"), None);
}

#[test]
fn returns_none_for_unrecognized_leading_keyword() {
    assert_eq!(extract_condition_from_line("for x in items:"), None);
}

#[test]
fn returns_none_when_no_colon_present() {
    assert_eq!(extract_condition_from_line("if a"), None);
}

#[test]
fn returns_none_when_condition_text_would_be_empty() {
    assert_eq!(extract_condition_from_line("if :"), None);
}
