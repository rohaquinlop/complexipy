//! Unit tests for `crate::utils` ignore directives and for
//! `crate::utils::filter_removable_ignores`.
//!
//! so this stays a child module of the code it tests and can reach that
//! module's private helpers through `super::` without widening visibility.

use crate::classes::FunctionComplexity;
use crate::utils::{collect_ignored_locations, extract_comment_marker, find_noqa_comment};

fn function(name: &str, complexity: u64, line_start: u64, line_end: u64) -> FunctionComplexity {
    FunctionComplexity {
        name: name.to_string(),
        complexity,
        line_start,
        line_end,
        line_complexities: vec![],
        refactor_plans: vec![],
        additional_refactor_plans: 0,
    }
}

#[test]
fn bare_markers_have_no_rule_list() {
    for marker in ["# complexipy: ignore", "# noqa: complexipy"] {
        let directive = extract_comment_marker(marker).expect("marker should parse");
        assert_eq!(directive.rules, None);
    }
}

#[test]
fn bracketed_list_parses_to_uppercase_ids() {
    let directive = extract_comment_marker("def f():  # complexipy: ignore[c007, c001]")
        .expect("marker should parse");

    assert_eq!(
        directive.rules,
        Some(vec!["C007".to_string(), "C001".to_string()])
    );
    assert_eq!(directive.text, "# complexipy: ignore[C007,C001]");
}

#[test]
fn noqa_marker_accepts_the_same_list() {
    let directive =
        extract_comment_marker("def f():  # noqa: complexipy[C007]").expect("marker should parse");

    assert_eq!(directive.rules, Some(vec!["C007".to_string()]));
    assert_eq!(directive.text, "# noqa: complexipy[C007]");
}

#[test]
fn unclosed_list_keeps_whole_function_suppression() {
    let directive =
        extract_comment_marker("# complexipy: ignore[C007").expect("marker should parse");

    assert_eq!(directive.rules, None);
    assert_eq!(directive.text, "# complexipy: ignore");
}

#[test]
fn empty_list_suppresses_no_rules() {
    let directive = extract_comment_marker("# complexipy: ignore[]").expect("marker should parse");

    assert_eq!(directive.rules, Some(Vec::new()));
}

#[test]
fn bracketed_reason_keeps_whole_function_suppression() {
    let directive = extract_comment_marker("# complexipy: ignore [technical debt]")
        .expect("marker should parse");

    assert_eq!(directive.rules, None);
    assert_eq!(directive.text, "# complexipy: ignore");
}

#[test]
fn list_without_rule_id_shape_keeps_whole_function_suppression() {
    for marker in [
        "# complexipy: ignore[wontfix]",
        "# complexipy: ignore[C007, wontfix]",
        "# complexipy: ignore[C]",
    ] {
        let directive = extract_comment_marker(marker).expect("marker should parse");
        assert_eq!(directive.rules, None, "{marker}");
    }
}

#[test]
fn trailing_reason_after_a_list_does_not_change_the_list() {
    let directive =
        extract_comment_marker("# complexipy: ignore[C007] (see issue 208)").expect("parses");

    assert_eq!(directive.rules, Some(vec!["C007".to_string()]));
}

#[test]
fn rule_list_marker_is_found_for_a_decorated_function() {
    let code = "# complexipy: ignore[C007]\n@decorator\ndef f(x):\n    return x\n";
    let offset = code.find("@decorator").expect("decorator should exist");
    let directive = find_noqa_comment(offset, code).expect("directive should be found");

    assert_eq!(directive.rules, Some(vec!["C007".to_string()]));
}

#[test]
fn rule_list_marker_is_found_for_the_function() {
    let code = "def f(x):  # complexipy: ignore[C007]\n    return x\n";
    let directive = find_noqa_comment(0, code).expect("directive should be found");

    assert_eq!(directive.rules, Some(vec!["C007".to_string()]));
}

#[test]
fn collect_ignored_locations_reports_bare_markers_only() {
    let code = "def bare(x):  # complexipy: ignore\n    return x\n\ndef narrowed(x):  # complexipy: ignore[C007]\n    return x\n";
    let locations = collect_ignored_locations(code);

    assert_eq!(locations.len(), 1);
    assert_eq!(locations[0].1, "# complexipy: ignore");
}

#[test]
fn marker_below_threshold_is_removable() {
    let locations = vec![(1, "# complexipy: ignore".to_string())];
    let functions = vec![function("simple", 2, 1, 3)];
    let removable = super::filter_removable_ignores(&locations, &functions, 15);
    assert_eq!(
        removable,
        vec![(
            1,
            "# complexipy: ignore".to_string(),
            "simple".to_string(),
            2
        )]
    );
}

#[test]
fn marker_at_threshold_is_removable() {
    let locations = vec![(5, "# complexipy: ignore".to_string())];
    let functions = vec![function("edge", 15, 5, 9)];
    let removable = super::filter_removable_ignores(&locations, &functions, 15);
    assert_eq!(removable.len(), 1);
}

#[test]
fn marker_above_threshold_is_kept() {
    let locations = vec![(1, "# complexipy: ignore".to_string())];
    let functions = vec![function("complex_fn", 20, 1, 12)];
    let removable = super::filter_removable_ignores(&locations, &functions, 15);
    assert!(removable.is_empty());
}

#[test]
fn marker_matches_containing_function_range() {
    let locations = vec![(3, "# noqa: complexipy".to_string())];
    let functions = vec![function("decorated", 4, 1, 9)];
    let removable = super::filter_removable_ignores(&locations, &functions, 15);
    assert_eq!(removable.len(), 1);
    assert_eq!(removable[0].2, "decorated");
}

#[test]
fn marker_without_containing_function_is_skipped() {
    let locations = vec![(7, "# complexipy: ignore".to_string())];
    let functions = vec![function("first", 2, 1, 5), function("second", 3, 9, 11)];
    let removable = super::filter_removable_ignores(&locations, &functions, 15);
    assert!(removable.is_empty());
}

#[test]
fn marker_never_matches_ignored_analysis() {
    let locations = vec![(2, "# complexipy: ignore".to_string())];
    let functions: Vec<FunctionComplexity> = vec![];
    let removable = super::filter_removable_ignores(&locations, &functions, 15);
    assert!(removable.is_empty());
}

#[test]
fn multiple_markers_keep_source_order() {
    let locations = vec![
        (1, "# complexipy: ignore".to_string()),
        (9, "# noqa: complexipy".to_string()),
    ];
    let functions = vec![
        function("low", 3, 1, 4),
        function("high", 30, 6, 20),
        function("low_again", 5, 22, 30),
    ];
    let removable = super::filter_removable_ignores(&locations, &functions, 15);
    assert_eq!(removable.len(), 1);
    assert_eq!(
        removable[0],
        (1, "# complexipy: ignore".to_string(), "low".to_string(), 3)
    );
}
