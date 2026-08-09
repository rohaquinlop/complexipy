//! Unit tests for `crate::rules::complexity`.
//!
//! Wired in from `src/rules/complexity.rs` via `#[cfg(test)] #[path = ...] mod tests;`
//! so this stays a child module of the code it tests and can reach that module's
//! private helpers through `super::` without widening their visibility.

use super::{
    collect_loop_if_chain, combine_conditions_chain, extract_condition_from_line,
    fallback_boolean_count, generate_loop_guard_suggestion, needs_parens_for_and,
};
use crate::refactor_plans::{ComplexityRegion, RegionKind};

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
    assert_eq!(extract_condition_from_line("match x:"), None);
    assert_eq!(extract_condition_from_line("with open(f) as fh:"), None);
}

#[test]
fn returns_none_when_no_colon_present() {
    assert_eq!(extract_condition_from_line("if a"), None);
}

#[test]
fn returns_none_when_condition_text_would_be_empty() {
    assert_eq!(extract_condition_from_line("if :"), None);
}

fn if_region(nesting: u64, children: Vec<ComplexityRegion>) -> ComplexityRegion {
    ComplexityRegion {
        kind: RegionKind::If,
        line_start: nesting + 1,
        line_end: nesting + 1,
        nesting,
        children,
        ..Default::default()
    }
}

#[test]
fn collect_loop_if_chain_walks_a_pure_nested_chain() {
    let lines = [
        "for x in y:",
        "    if a:",
        "        if b:",
        "            pass",
    ];
    let loop_region = ComplexityRegion {
        kind: RegionKind::Loop,
        children: vec![if_region(1, vec![if_region(2, vec![])])],
        ..Default::default()
    };

    let chain = collect_loop_if_chain(&loop_region, &lines);

    assert_eq!(chain.len(), 2);
    assert_eq!(chain[0].nesting, 1);
    assert_eq!(chain[1].nesting, 2);
}

#[test]
fn collect_loop_if_chain_stops_at_an_else_branch() {
    let lines = [
        "for x in y:",
        "    if a:",
        "        if b:",
        "            pass",
        "        else:",
        "            pass",
    ];
    let loop_region = ComplexityRegion {
        kind: RegionKind::Loop,
        children: vec![ComplexityRegion {
            kind: RegionKind::If,
            line_start: 2,
            line_end: 6,
            nesting: 1,
            children: vec![ComplexityRegion {
                kind: RegionKind::If,
                line_start: 3,
                line_end: 6,
                nesting: 2,
                ..Default::default()
            }],
            ..Default::default()
        }],
        ..Default::default()
    };

    let chain = collect_loop_if_chain(&loop_region, &lines);

    // The inner if has an else, so only the outer if is hoistable -- the
    // else-bearing if stays behind as the chain's untouched tail.
    assert_eq!(chain.len(), 1);
    assert_eq!(chain[0].nesting, 1);
}

#[test]
fn collect_loop_if_chain_is_empty_without_a_nested_if() {
    let lines = ["for x in y:", "    pass"];
    let loop_region = ComplexityRegion {
        kind: RegionKind::Loop,
        ..Default::default()
    };

    assert!(collect_loop_if_chain(&loop_region, &lines).is_empty());
}

#[test]
fn fallback_boolean_count_sums_known_booleans_plus_one_join_per_merge() {
    let chain = [
        ComplexityRegion {
            bool_op_count: 1,
            ..Default::default()
        },
        ComplexityRegion {
            bool_op_count: 2,
            ..Default::default()
        },
        ComplexityRegion {
            bool_op_count: 0,
            ..Default::default()
        },
    ];
    let refs: Vec<&ComplexityRegion> = chain.iter().collect();

    // 1 + 2 + 0 known booleans, plus (3 - 1) joins from merging 3 conditions.
    assert_eq!(fallback_boolean_count(&refs), 5);
}

fn loop_region(children: Vec<ComplexityRegion>, line_end: u64) -> ComplexityRegion {
    ComplexityRegion {
        kind: RegionKind::Loop,
        line_start: 1,
        line_end,
        children,
        ..Default::default()
    }
}

fn if_stmt(line_start: u64, line_end: u64, nesting: u64) -> ComplexityRegion {
    ComplexityRegion {
        kind: RegionKind::If,
        line_start,
        line_end,
        nesting,
        ..Default::default()
    }
}

#[test]
fn loop_guard_suggestion_is_faithful_for_a_pure_chain() {
    let source = "for x in y:\n    if a:\n        if b:\n            pass\n";
    let region = loop_region(
        vec![ComplexityRegion {
            kind: RegionKind::If,
            line_start: 2,
            line_end: 4,
            nesting: 1,
            children: vec![if_stmt(3, 4, 2)],
            ..Default::default()
        }],
        4,
    );

    let suggestion = generate_loop_guard_suggestion(&region, source).unwrap();
    assert!(suggestion.spliceable);
    assert_eq!(
        suggestion.replacement,
        "for x in y:\n    if not a:\n        continue\n    if not b:\n        continue\n    pass"
    );
}

#[test]
fn loop_guard_suggestion_keeps_leading_statements() {
    let source = "for x in y:\n    total += x\n    if a:\n        pass\n";
    let region = loop_region(vec![if_stmt(3, 4, 1)], 4);

    let suggestion = generate_loop_guard_suggestion(&region, source).unwrap();
    assert_eq!(
        suggestion.replacement,
        "for x in y:\n    total += x\n    if not a:\n        continue\n    pass"
    );
}

#[test]
fn loop_guard_suggestion_keeps_trailing_statements_at_loop_indent() {
    let source = "for x in y:\n    if a:\n        pass\n    total += 1\n";
    let region = loop_region(vec![if_stmt(2, 3, 1)], 4);

    let suggestion = generate_loop_guard_suggestion(&region, source).unwrap();
    assert_eq!(
        suggestion.replacement,
        "for x in y:\n    if not a:\n        continue\n    pass\n    total += 1"
    );
}

#[test]
fn loop_guard_suggestion_keeps_an_else_on_the_last_chain_member() {
    let source = "for x in y:\n    if a:\n        if b:\n            pass\n        else:\n            pass\n";
    let region = loop_region(
        vec![ComplexityRegion {
            kind: RegionKind::If,
            line_start: 2,
            line_end: 6,
            nesting: 1,
            children: vec![if_stmt(3, 6, 2)],
            ..Default::default()
        }],
        6,
    );

    let suggestion = generate_loop_guard_suggestion(&region, source).unwrap();
    assert_eq!(
        suggestion.replacement,
        "for x in y:\n    if not a:\n        continue\n    if b:\n        pass\n    else:\n        pass"
    );
}

#[test]
fn loop_guard_suggestion_preserves_a_multiline_header() {
    let source = "for x in (\n    items):\n    if a:\n        pass\n";
    let region = loop_region(vec![if_stmt(3, 4, 1)], 4);

    // The header continuation lines pass through unchanged; only the body
    // is transformed.
    let suggestion = generate_loop_guard_suggestion(&region, source).unwrap();
    assert!(suggestion.spliceable);
    assert_eq!(
        suggestion.replacement,
        "for x in (\n    items):\n    if not a:\n        continue\n    pass"
    );
}

#[test]
fn loop_guard_suggestion_refuses_a_first_member_with_its_own_else() {
    let source = "for x in y:\n    if a:\n        pass\n    else:\n        pass\n";
    let region = loop_region(vec![if_stmt(2, 4, 1)], 4);

    // The guard `if not a: continue` would skip the `else` branch entirely.
    assert!(generate_loop_guard_suggestion(&region, source).is_none());
}

#[test]
fn loop_guard_suggestion_refuses_a_loop_level_else() {
    let source = "def f():\n    for x in y:\n        if a:\n            pass\n    else:\n        pass\n";
    let region = ComplexityRegion {
        kind: RegionKind::Loop,
        line_start: 2,
        line_end: 5,
        children: vec![if_stmt(3, 4, 1)],
        ..Default::default()
    };

    // The loop's own `else` would dangle after the guards.
    assert!(generate_loop_guard_suggestion(&region, source).is_none());
}

#[test]
fn loop_guard_suggestion_refuses_an_unextractable_condition() {
    let source = "for x in y:\n    if (a and\n            b):\n        pass\n";
    let region = loop_region(vec![if_stmt(2, 4, 1)], 4);

    // The first chain member's condition spans lines; no guard text exists.
    assert!(generate_loop_guard_suggestion(&region, source).is_none());
}

#[test]
fn loop_guard_suggestion_keeps_unextractable_members_in_the_survivor() {
    let source =
        "for x in y:\n    if a:\n        if (b and\n                c):\n            pass\n";
    let region = loop_region(
        vec![ComplexityRegion {
            kind: RegionKind::If,
            line_start: 2,
            line_end: 5,
            nesting: 1,
            children: vec![if_stmt(3, 5, 2)],
            ..Default::default()
        }],
        5,
    );

    // The first member becomes a guard; the unextractable member stays in
    // the survivor block, dedented with it, unchanged.
    let suggestion = generate_loop_guard_suggestion(&region, source).unwrap();
    assert_eq!(
        suggestion.replacement,
        "for x in y:\n    if not a:\n        continue\n    if (b and\n            c):\n        pass"
    );
}
