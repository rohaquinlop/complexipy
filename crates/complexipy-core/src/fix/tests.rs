use super::{SkipReason, apply_fixes, fixable, parses};
use crate::classes::{Applicability, CodeSuggestion, RefactorPlan, RuleCategory};

const TWO_FUNCTIONS: &str =
    "def a():\n    if x:\n        return 1\n\n\ndef b():\n    if y:\n        return 2";

const TWO_FUNCTIONS_FIXED: &str = "def a():\n    pass\n\n\ndef b():\n    pass";

fn plan(rule_id: &str, line_start: u64, line_end: u64, replacement: &str) -> RefactorPlan {
    RefactorPlan {
        kind: String::new(),
        title: "Test plan".to_string(),
        line_start,
        line_end,
        column_start: 0,
        current_complexity: 0,
        estimated_reduction: 0,
        estimated_complexity_after: 0,
        reduction_is_measured: false,
        rule_id: rule_id.to_string(),
        category: RuleCategory::Complexity,
        applicability: Applicability::MachineApplicable,
        description: String::new(),
        explanation: String::new(),
        references: vec![],
        suggestion: Some(CodeSuggestion {
            replacement: replacement.to_string(),
            applicability: Applicability::MachineApplicable,
            description: String::new(),
            spliceable: true,
        }),
        help: None,
        doc_url: String::new(),
    }
}

#[test]
fn two_fixes_apply_bottom_to_top() {
    let plans = vec![
        plan("C007", 2, 3, "    pass"),
        plan("C002", 7, 8, "    pass"),
    ];

    let report = apply_fixes(TWO_FUNCTIONS, &plans);

    assert_eq!(report.patched, TWO_FUNCTIONS_FIXED);
    assert_eq!(report.applied.len(), 2);
    assert_eq!(report.applied[0].line_start, 2);
    assert_eq!(report.applied[0].title, "Test plan");
    assert_eq!(report.applied[1].line_start, 7);
    assert!(report.skipped.is_empty());
    assert!(parses(&report.patched));
}

#[test]
fn informational_and_non_spliceable_plans_never_apply() {
    let mut informational = plan("C001", 2, 3, "    pass");
    informational.applicability = Applicability::Informational;
    informational
        .suggestion
        .as_mut()
        .expect("suggestion exists")
        .applicability = Applicability::Informational;
    let mut placeholder = plan("C005", 7, 8, "    pass");
    placeholder
        .suggestion
        .as_mut()
        .expect("suggestion exists")
        .spliceable = false;

    let report = apply_fixes(TWO_FUNCTIONS, &[informational, placeholder]);

    assert_eq!(report.patched, TWO_FUNCTIONS);
    assert!(report.applied.is_empty());
    assert_eq!(report.skipped.len(), 2);
    assert!(
        report
            .skipped
            .iter()
            .all(|skipped| skipped.reason == SkipReason::NotFixable)
    );
}

#[test]
fn overlapping_span_is_skipped_after_the_lower_fix_applies() {
    let plans = vec![
        plan("C007", 2, 4, "    pass"),
        plan("C002", 4, 5, "    pass"),
    ];

    let report = apply_fixes(TWO_FUNCTIONS, &plans);

    assert_eq!(report.applied.len(), 1);
    assert_eq!(report.applied[0].line_start, 4);
    assert_eq!(report.skipped.len(), 1);
    assert_eq!(report.skipped[0].line_start, 2);
    assert_eq!(report.skipped[0].reason, SkipReason::Overlap);
    assert!(parses(&report.patched));
}

#[test]
fn unparseable_splice_is_reverted_and_others_apply() {
    let plans = vec![plan("C007", 2, 3, "def (("), plan("C002", 7, 8, "    pass")];

    let report = apply_fixes(TWO_FUNCTIONS, &plans);

    assert_eq!(report.applied.len(), 1);
    assert_eq!(report.applied[0].line_start, 7);
    assert_eq!(report.skipped.len(), 1);
    assert_eq!(report.skipped[0].line_start, 2);
    assert_eq!(report.skipped[0].reason, SkipReason::ParseFailure);
    assert!(parses(&report.patched));
    assert!(report.patched.contains("if x:"));
    assert!(report.patched.contains("    pass"));
}

#[test]
fn a_fix_at_the_end_of_file_keeps_the_trailing_newline() {
    let source = "def a():\n    if x:\n        if y:\n            return 1\n";
    let plans = vec![plan("C007", 2, 4, "    if x and y:\n        return 1")];

    let report = apply_fixes(source, &plans);

    assert_eq!(
        report.patched,
        "def a():\n    if x and y:\n        return 1\n"
    );
}

#[test]
fn a_fix_keeps_the_line_endings_of_a_crlf_file() {
    let source =
        "def a():\r\n    if x:\r\n        if y:\r\n            return 1\r\n    return 0\r\n";
    let plans = vec![plan("C007", 2, 4, "    if x and y:\n        return 1")];

    let report = apply_fixes(source, &plans);

    assert_eq!(
        report.patched,
        "def a():\r\n    if x and y:\r\n        return 1\r\n    return 0\r\n"
    );
}

#[test]
fn fixable_gate_reads_metadata_only() {
    let mut plan = plan("C999", 1, 1, "pass");
    assert!(fixable(&plan));

    plan.applicability = Applicability::MaybeIncorrect;
    assert!(!fixable(&plan));

    plan.applicability = Applicability::MachineApplicable;
    plan.suggestion = None;
    assert!(!fixable(&plan));
}
