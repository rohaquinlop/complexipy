//! Unit tests for `crate::rules::complexity`.
//!
//! so this stays a child module of the code it tests and can reach that module's
//! private helpers through `super::` without widening their visibility.

use super::{
    collect_free_names, collect_loop_if_chain, combine_conditions_chain, contains_multiline_string,
    enclosing_header_indices, extract_condition_from_line, fallback_boolean_count,
    generate_collapsible_if_suggestion_chain, generate_loop_guard_suggestion,
    generate_predicate_suggestion, has_else_branch, needs_parens_for_and, render_statement_context,
    strip_top_level_not,
};
use crate::refactor_plans::{ComplexityRegion, RegionKind};
use crate::utils::LineIndex;

fn combine(parts: &[&str]) -> String {
    let owned: Vec<String> = parts.iter().map(|s| s.to_string()).collect();
    combine_conditions_chain(&owned)
}

#[test]
fn collect_free_names_includes_attribute_bases_and_skips_builtins() {
    assert_eq!(
        collect_free_names("user.is_active and len(items) > 0"),
        Some(vec!["user".to_string(), "items".to_string()])
    );
}

#[test]
fn collect_free_names_deduplicates_in_first_use_order() {
    assert_eq!(
        collect_free_names("a > 1 and a < 2 or b"),
        Some(vec!["a".to_string(), "b".to_string()])
    );
}

#[test]
fn collect_free_names_includes_self() {
    assert_eq!(
        collect_free_names("self.x > 0 and self.y < 1"),
        Some(vec!["self".to_string()])
    );
}

#[test]
fn collect_free_names_returns_none_for_walrus() {
    assert_eq!(collect_free_names("(n := len(items)) > 3 and n < 10"), None);
}

#[test]
fn collect_free_names_returns_none_for_lambda() {
    assert_eq!(collect_free_names("(lambda x: x > 0)(v) and v < 10"), None);
}

#[test]
fn collect_free_names_returns_none_for_comprehension() {
    assert_eq!(
        collect_free_names("all(x > 0 for x in items) and items"),
        None
    );
}

#[test]
fn collect_free_names_returns_empty_for_literal_only_conditions() {
    assert_eq!(collect_free_names("1 < 2 and 3 < 4"), Some(vec![]));
}

#[test]
fn enclosing_header_indices_finds_the_function_chain() {
    let source = "def sample(a, b):\n    if a and b or a:\n        return 1\n";
    let index = LineIndex::new(source);
    assert_eq!(enclosing_header_indices(source, &index, 2), Some(vec![1]));
}

#[test]
fn enclosing_header_indices_collects_nested_blocks() {
    let source = "def f(items):\n    for item in items:\n        if item.active and item.ready:\n            pass\n";
    let index = LineIndex::new(source);
    assert_eq!(
        enclosing_header_indices(source, &index, 3),
        Some(vec![1, 2])
    );
}

#[test]
fn enclosing_header_indices_skips_decorators() {
    let source = "@deco\ndef f(x):\n    if x and x > 0:\n        pass\n";
    let index = LineIndex::new(source);
    assert_eq!(enclosing_header_indices(source, &index, 3), Some(vec![2]));
}

#[test]
fn enclosing_header_indices_returns_none_for_module_level_conditions() {
    let source = "if a and b:\n    pass\n";
    let index = LineIndex::new(source);
    assert_eq!(enclosing_header_indices(source, &index, 1), None);
}

#[test]
fn enclosing_header_indices_returns_none_for_a_broken_chain() {
    let source = "x = 1\n    def f():\n        if a and b:\n            pass\n";
    let index = LineIndex::new(source);
    assert_eq!(enclosing_header_indices(source, &index, 3), None);
}

#[test]
fn enclosing_header_indices_collects_else_chains() {
    let source = "if a:\n    pass\nelse:\n    if b and c:\n        pass\n";
    let index = LineIndex::new(source);
    assert_eq!(
        enclosing_header_indices(source, &index, 4),
        Some(vec![1, 3])
    );
}

#[test]
fn enclosing_header_indices_collects_elif_chains() {
    let source = "def f(x, y):\n    if x > 0:\n        pass\n    elif y > 0:\n        if y > 1 and y < 9:\n            pass\n";
    let index = LineIndex::new(source);
    assert_eq!(
        enclosing_header_indices(source, &index, 5),
        Some(vec![1, 2, 4])
    );
}

#[test]
fn predicate_suggestion_shows_the_enclosing_function_context() {
    let source = "def sample(a, b):\n    if a and b or a:\n        return 1\n    return 0\n";
    let region = ComplexityRegion {
        kind: RegionKind::BooleanCondition,
        line_start: 2,
        line_end: 2,
        ..Default::default()
    };

    let suggestion = generate_predicate_suggestion(
        &region,
        source,
        &LineIndex::new(source),
        &Default::default(),
    )
    .unwrap();
    assert_eq!(
        suggestion.replacement,
        "def _check_condition_L2(a, b) -> bool:\n    return a and b or a\n\ndef sample(a, b):\n    if _check_condition_L2(a, b):\n        ...\n    ...\n"
    );
}

#[test]
fn predicate_suggestion_falls_back_to_a_bare_call_at_module_level() {
    let source = "if a and b or a:\n    pass\n";
    let region = ComplexityRegion {
        kind: RegionKind::BooleanCondition,
        line_start: 1,
        line_end: 1,
        ..Default::default()
    };

    let suggestion = generate_predicate_suggestion(
        &region,
        source,
        &LineIndex::new(source),
        &Default::default(),
    )
    .unwrap();
    assert_eq!(
        suggestion.replacement,
        "def _check_condition_L1(a, b) -> bool:\n    return a and b or a\n\nif _check_condition_L1(a, b):\n    ..."
    );
}

#[test]
fn render_statement_context_places_a_placeholder_for_skipped_statements() {
    let source = "def wait_until(items, limit):\n    i = 0\n    while i < len(items) and i < limit:\n        i += 1\n";
    let context = render_statement_context(
        &[1],
        source,
        &LineIndex::new(source),
        3,
        4,
        "while _check_condition_L3(i, items, limit):",
    );

    assert_eq!(
        context,
        "def wait_until(items, limit):\n    ...\n    while _check_condition_L3(i, items, limit):\n        ...\n    ...\n"
    );
}

fn detect_step(source: &str, from_line: u64, to_line_excl: u64) -> usize {
    super::detect_indent_step_range(source, &LineIndex::new(source), from_line, to_line_excl)
}

#[test]
fn detect_indent_step_pairs_structural_lines() {
    let source = "if a:\n    stmt\n";
    assert_eq!(detect_step(source, 1, 3), 4);
}

#[test]
fn detect_indent_step_skips_blank_lines_when_pairing() {
    let source = "if a:\n\n    stmt\n";
    assert_eq!(detect_step(source, 1, 4), 4);
}

#[test]
fn detect_indent_step_skips_comment_lines_when_pairing() {
    let source = "if a:\n\n    # note\n    stmt\n";
    assert_eq!(detect_step(source, 1, 5), 4);
}

#[test]
fn detect_indent_step_returns_default_without_an_increase() {
    let source = "stmt\n\nstmt\n";
    assert_eq!(detect_step(source, 1, 4), 4);
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
    let source = "for x in y:\n    if a:\n        if b:\n            pass\n";
    let index = LineIndex::new(source);
    let loop_region = ComplexityRegion {
        kind: RegionKind::Loop,
        children: vec![if_region(1, vec![if_region(2, vec![])])],
        ..Default::default()
    };

    let chain = collect_loop_if_chain(&loop_region, source, &index);

    assert_eq!(chain.len(), 2);
    assert_eq!(chain[0].nesting, 1);
    assert_eq!(chain[1].nesting, 2);
}

#[test]
fn collect_loop_if_chain_stops_at_an_else_branch() {
    let source = "for x in y:\n    if a:\n        if b:\n            pass\n        else:\n            pass\n";
    let index = LineIndex::new(source);
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

    let chain = collect_loop_if_chain(&loop_region, source, &index);

    // The inner if has an else, so only the outer if is hoistable -- the
    // else-bearing if stays behind as the chain's untouched tail.
    assert_eq!(chain.len(), 1);
    assert_eq!(chain[0].nesting, 1);
}

#[test]
fn collect_loop_if_chain_is_empty_without_a_nested_if() {
    let source = "for x in y:\n    pass\n";
    let loop_region = ComplexityRegion {
        kind: RegionKind::Loop,
        ..Default::default()
    };

    assert!(collect_loop_if_chain(&loop_region, source, &LineIndex::new(source)).is_empty());
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

    let suggestion =
        generate_loop_guard_suggestion(&region, source, &LineIndex::new(source)).unwrap();
    assert!(suggestion.spliceable);
    assert_eq!(
        suggestion.replacement,
        "for x in y:\n    if not (a):\n        continue\n    if not (b):\n        continue\n    pass"
    );
}

#[test]
fn loop_guard_suggestion_keeps_leading_statements() {
    let source = "for x in y:\n    total += x\n    if a:\n        pass\n";
    let region = loop_region(vec![if_stmt(3, 4, 1)], 4);

    let suggestion =
        generate_loop_guard_suggestion(&region, source, &LineIndex::new(source)).unwrap();
    assert_eq!(
        suggestion.replacement,
        "for x in y:\n    total += x\n    if not (a):\n        continue\n    pass"
    );
}

#[test]
fn loop_guard_suggestion_parenthesizes_inverted_or_conditions() {
    let source = "for x in y:\n    if a or b:\n        pass\n";
    let region = loop_region(vec![if_stmt(2, 3, 1)], 3);

    let suggestion =
        generate_loop_guard_suggestion(&region, source, &LineIndex::new(source)).unwrap();
    assert_eq!(
        suggestion.replacement,
        "for x in y:\n    if not (a or b):\n        continue\n    pass"
    );
}

#[test]
fn loop_guard_suggestion_strips_a_redundant_not() {
    let source = "for x in y:\n    if not a:\n        pass\n";
    let region = loop_region(vec![if_stmt(2, 3, 1)], 3);

    let suggestion =
        generate_loop_guard_suggestion(&region, source, &LineIndex::new(source)).unwrap();
    assert_eq!(
        suggestion.replacement,
        "for x in y:\n    if a:\n        continue\n    pass"
    );
}

#[test]
fn strip_top_level_not_refuses_mixed_operators() {
    assert_eq!(strip_top_level_not("not a"), Some("a".to_string()));
    assert_eq!(
        strip_top_level_not("not a == b"),
        Some("a == b".to_string())
    );
    assert_eq!(
        strip_top_level_not("not (a and b)"),
        Some("(a and b)".to_string())
    );
    assert_eq!(strip_top_level_not("not a or b"), None);
    assert_eq!(strip_top_level_not("not a and b"), None);
    assert_eq!(strip_top_level_not("a not in b"), None);
}

#[test]
fn has_else_branch_ignores_string_content_lines() {
    let source = "if a:\n    x = \"\"\"foo\nelse:\nbar\"\"\"\n    if b:\n        pass\n";
    let region = if_stmt(1, 6, 0);

    assert!(!has_else_branch(&region, source, &LineIndex::new(source)));
}

#[test]
fn contains_multiline_string_detects_spanned_content() {
    let plain: Vec<&str> = "x = 1\ny = 2\n".split('\n').collect();
    assert!(!contains_multiline_string(&plain));

    let single_line_triple: Vec<&str> = "x = \"\"\"foo\"\"\"\n".split('\n').collect();
    assert!(!contains_multiline_string(&single_line_triple));

    let spanned: Vec<&str> = "x = \"\"\"a\nb\"\"\"\n".split('\n').collect();
    assert!(contains_multiline_string(&spanned));
}

#[test]
fn loop_guard_suggestion_refuses_multiline_string_between_members() {
    let source = "for x in y:\n    if a:\n        doc = \"\"\"x\n    else:\n    y\"\"\"\n        if b:\n            pass\n";
    let region = loop_region(
        vec![ComplexityRegion {
            kind: RegionKind::If,
            line_start: 2,
            line_end: 6,
            nesting: 1,
            children: vec![if_stmt(5, 6, 2)],
            ..Default::default()
        }],
        6,
    );

    assert!(generate_loop_guard_suggestion(&region, source, &LineIndex::new(source)).is_none());
}

#[test]
fn collapsible_if_suggestion_refuses_multiline_string_body() {
    let source = "if a:\n    if b:\n        doc = \"\"\"x\n            y\"\"\"\n";
    let outermost = if_stmt(1, 4, 0);
    let innermost = if_stmt(2, 4, 1);
    let conditions = vec!["a".to_string(), "b".to_string()];

    assert!(
        generate_collapsible_if_suggestion_chain(
            &outermost,
            &innermost,
            &conditions,
            source,
            &LineIndex::new(source)
        )
        .is_none()
    );
}

#[test]
fn loop_guard_suggestion_keeps_trailing_statements_at_loop_indent() {
    let source = "for x in y:\n    if a:\n        pass\n    total += 1\n";
    let region = loop_region(vec![if_stmt(2, 3, 1)], 4);

    let suggestion =
        generate_loop_guard_suggestion(&region, source, &LineIndex::new(source)).unwrap();
    assert_eq!(
        suggestion.replacement,
        "for x in y:\n    if not (a):\n        continue\n    pass\n    total += 1"
    );
}

#[test]
fn loop_guard_suggestion_keeps_statements_between_chain_members() {
    let source = "for x in y:\n    if a:\n        total += x\n        if b:\n            pass\n";
    let region = loop_region(
        vec![ComplexityRegion {
            kind: RegionKind::If,
            line_start: 2,
            line_end: 5,
            nesting: 1,
            children: vec![if_stmt(4, 5, 2)],
            ..Default::default()
        }],
        5,
    );

    let suggestion =
        generate_loop_guard_suggestion(&region, source, &LineIndex::new(source)).unwrap();
    assert_eq!(
        suggestion.replacement,
        "for x in y:\n    if not (a):\n        continue\n    total += x\n    if not (b):\n        continue\n    pass"
    );
}

#[test]
fn loop_guard_suggestion_shifts_between_statements_at_each_level() {
    let source = "for x in y:\n    if a:\n        if b:\n            stmt\n            if c:\n                pass\n";
    let region = loop_region(
        vec![ComplexityRegion {
            kind: RegionKind::If,
            line_start: 2,
            line_end: 6,
            nesting: 1,
            children: vec![ComplexityRegion {
                kind: RegionKind::If,
                line_start: 3,
                line_end: 6,
                nesting: 2,
                children: vec![if_stmt(5, 6, 3)],
                ..Default::default()
            }],
            ..Default::default()
        }],
        6,
    );

    let suggestion =
        generate_loop_guard_suggestion(&region, source, &LineIndex::new(source)).unwrap();
    assert_eq!(
        suggestion.replacement,
        "for x in y:\n    if not (a):\n        continue\n    if not (b):\n        continue\n    stmt\n    if not (c):\n        continue\n    pass"
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

    let suggestion =
        generate_loop_guard_suggestion(&region, source, &LineIndex::new(source)).unwrap();
    assert_eq!(
        suggestion.replacement,
        "for x in y:\n    if not (a):\n        continue\n    if b:\n        pass\n    else:\n        pass"
    );
}

#[test]
fn loop_guard_suggestion_preserves_a_multiline_header() {
    let source = "for x in (\n    items):\n    if a:\n        pass\n";
    let region = loop_region(vec![if_stmt(3, 4, 1)], 4);

    // The header continuation lines pass through unchanged; only the body
    // is transformed.
    let suggestion =
        generate_loop_guard_suggestion(&region, source, &LineIndex::new(source)).unwrap();
    assert!(suggestion.spliceable);
    assert_eq!(
        suggestion.replacement,
        "for x in (\n    items):\n    if not (a):\n        continue\n    pass"
    );
}

#[test]
fn loop_guard_suggestion_refuses_a_first_member_with_its_own_else() {
    let source = "for x in y:\n    if a:\n        pass\n    else:\n        pass\n";
    let region = loop_region(vec![if_stmt(2, 4, 1)], 4);

    // The guard `if not a: continue` would skip the `else` branch entirely.
    assert!(generate_loop_guard_suggestion(&region, source, &LineIndex::new(source)).is_none());
}

#[test]
fn loop_guard_suggestion_refuses_a_loop_level_else() {
    let source =
        "def f():\n    for x in y:\n        if a:\n            pass\n    else:\n        pass\n";
    let region = ComplexityRegion {
        kind: RegionKind::Loop,
        line_start: 2,
        line_end: 5,
        children: vec![if_stmt(3, 4, 1)],
        ..Default::default()
    };

    // The loop's own `else` would dangle after the guards.
    assert!(generate_loop_guard_suggestion(&region, source, &LineIndex::new(source)).is_none());
}

#[test]
fn loop_guard_suggestion_refuses_an_unextractable_condition() {
    let source = "for x in y:\n    if (a and\n            b):\n        pass\n";
    let region = loop_region(vec![if_stmt(2, 4, 1)], 4);

    // The first chain member's condition spans lines; no guard text exists.
    assert!(generate_loop_guard_suggestion(&region, source, &LineIndex::new(source)).is_none());
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
    let suggestion =
        generate_loop_guard_suggestion(&region, source, &LineIndex::new(source)).unwrap();
    assert_eq!(
        suggestion.replacement,
        "for x in y:\n    if not (a):\n        continue\n    if (b and\n            c):\n        pass"
    );
}
