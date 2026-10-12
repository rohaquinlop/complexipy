//! Tests for plans_for_function: on-demand plans for one function.

use complexipy_core::cognitive_complexity::{code_complexity_shared, plans_for_function};
use complexipy_core::{AnalysisOptions, RuleSet};

const SOURCE: &str = "def first(a, b):\n    if a:\n        if b:\n            return 1\n    return 0\n\n\nclass Box:\n    def method(self, a, b):\n        if a:\n            if b:\n                return 1\n        return 0\n\n\ndef quiet(a, b):  # complexipy: ignore[C007]\n    if a:\n        if b:\n            return 1\n    return 0\n";

fn rule_ids(line_start: u64) -> Option<Vec<String>> {
    plans_for_function(SOURCE, line_start, &AnalysisOptions::default())
        .expect("source should parse")
        .map(|(plans, _)| plans.into_iter().map(|plan| plan.rule_id).collect())
}

#[test]
fn builds_the_same_plans_as_a_full_analysis() {
    let full = code_complexity_shared(
        SOURCE,
        &AnalysisOptions {
            with_plans: true,
            rules: RuleSet::default(),
            ..Default::default()
        },
    )
    .expect("source should parse");

    for function in full.functions {
        let (plans, additional) =
            plans_for_function(SOURCE, function.line_start, &AnalysisOptions::default())
                .expect("source should parse")
                .expect("function should be found");
        assert_eq!(plans, function.refactor_plans, "{}", function.name);
        assert_eq!(additional, function.additional_refactor_plans);
    }
}

#[test]
fn finds_a_top_level_function_and_a_method() {
    assert_eq!(rule_ids(1), Some(vec!["C007".to_string()]));
    assert_eq!(rule_ids(9), Some(vec!["C007".to_string()]));
}

#[test]
fn a_bracketed_ignore_removes_the_rule() {
    assert_eq!(rule_ids(16), Some(Vec::new()));
}

#[test]
fn a_line_without_a_function_start_finds_nothing() {
    assert_eq!(rule_ids(2), None);
    assert_eq!(rule_ids(8), None);
}

#[test]
fn source_that_does_not_parse_is_an_error() {
    assert!(plans_for_function("def (:", 1, &AnalysisOptions::default()).is_err());
}
