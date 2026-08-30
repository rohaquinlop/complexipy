use crate::output::refactor::{
    get_applicability_icon, get_applicability_name, get_category_icon, get_category_name,
    output_refactor_plans,
};
use crate::types::FunctionRow;
use complexipy_core::classes::{Applicability, CodeSuggestion, RefactorPlan, RuleCategory};

fn strip_ansi(input: &str) -> String {
    let mut result = String::new();
    let mut chars = input.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            for next in chars.by_ref() {
                if next == 'm' {
                    break;
                }
            }
        } else {
            result.push(c);
        }
    }
    result
}

fn plan() -> RefactorPlan {
    RefactorPlan {
        kind: "extract-method".to_string(),
        title: "Extract method".to_string(),
        line_start: 2,
        line_end: 4,
        column_start: 4,
        current_complexity: 5,
        estimated_reduction: 2,
        estimated_complexity_after: 3,
        reduction_is_measured: true,
        rule_id: "C001".to_string(),
        category: RuleCategory::Complexity,
        applicability: Applicability::MachineApplicable,
        description: "Extract the body".to_string(),
        explanation: "Reduces nesting".to_string(),
        references: vec!["https://example.com/ref".to_string()],
        suggestion: Some(CodeSuggestion {
            replacement: "return x".to_string(),
            applicability: Applicability::MachineApplicable,
            description: "Replace body".to_string(),
            spliceable: true,
        }),
        help: None,
        doc_url: "https://example.com/c001".to_string(),
    }
}

fn row_with_plans() -> FunctionRow {
    FunctionRow {
        name: "f".to_string(),
        complexity: 5,
        passed: false,
        path: "src/a.py".to_string(),
        file_name: "a.py".to_string(),
        refactor_plans: vec![plan()],
        additional_refactor_plans: 1,
    }
}

#[test]
fn category_icons_and_names() {
    assert_eq!(get_category_icon(&RuleCategory::Complexity), "\u{25b2}");
    assert_eq!(get_category_name(&RuleCategory::Complexity), "Complexity");
    assert_eq!(get_category_icon(&RuleCategory::Readability), "\u{25c6}");
    assert_eq!(get_category_name(&RuleCategory::Readability), "Readability");
}

#[test]
fn applicability_icons_and_names() {
    assert_eq!(
        get_applicability_icon(&Applicability::MachineApplicable),
        "*"
    );
    assert_eq!(
        get_applicability_name(&Applicability::MachineApplicable),
        "Safe to apply"
    );
    assert_eq!(get_applicability_icon(&Applicability::MaybeIncorrect), "!");
    assert_eq!(
        get_applicability_name(&Applicability::MaybeIncorrect),
        "Needs review"
    );
    assert_eq!(get_applicability_icon(&Applicability::Informational), "i");
    assert_eq!(
        get_applicability_name(&Applicability::Informational),
        "Informational"
    );
}

#[test]
fn single_plan_output_structure() {
    let output = strip_ansi(&output_refactor_plans(&row_with_plans(), ""));

    assert!(output.contains("[1] C001 Extract method"));
    assert!(output.contains("--> src/a.py:2:4"));
    assert!(output.contains("Category: \u{25b2} Complexity | Applicability: * Safe to apply"));
    assert!(output.contains("Lines 2-4 -> Reduction: -2 complexity (5 -> 3)"));
    assert!(output.contains("Extract the body"));
    assert!(output.contains("Reduces nesting"));
    assert!(output.contains("Suggestion: * Safe to apply"));
    assert!(output.contains("References:"));
    assert!(output.contains("https://example.com/c001"));
    assert!(output.contains("... and 1 more suggestion"));
}

#[test]
fn unmeasured_reduction_uses_qualifier() {
    let mut p = plan();
    p.reduction_is_measured = false;
    let mut r = row_with_plans();
    r.refactor_plans = vec![p];

    let output = strip_ansi(&output_refactor_plans(&r, ""));

    assert!(output.contains("Estimated reduction"));
    assert!(output.contains("-~2"));
}

#[test]
fn no_plans_returns_empty() {
    let mut r = row_with_plans();
    r.refactor_plans = vec![];

    assert_eq!(output_refactor_plans(&r, ""), "");
}

#[test]
fn help_rendered_when_no_suggestion() {
    let mut p = plan();
    p.suggestion = None;
    p.help = Some("See the docs".to_string());
    let mut r = row_with_plans();
    r.refactor_plans = vec![p];

    let output = strip_ansi(&output_refactor_plans(&r, ""));

    assert!(output.contains("Help:"));
    assert!(output.contains("See the docs"));
}

#[test]
fn caret_span_rendered_from_source() {
    let p = plan();
    let output = crate::output::refactor::output_single_plan_for_test(
        &p,
        1,
        "src/a.py",
        Some(&[
            "line one".to_string(),
            "    if x:".to_string(),
            "        return 1".to_string(),
            "    return 0".to_string(),
        ]),
    );

    assert!(output.contains("2 | "));
    assert!(output.contains("^"));
}
