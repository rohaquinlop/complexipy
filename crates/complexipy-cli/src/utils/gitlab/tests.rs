use std::fs;

use serde_json::Value;
use tempfile::tempdir;

use crate::utils::gitlab::store_gitlab;
use complexipy_core::classes::{
    Applicability, CodeSuggestion, FileComplexity, FunctionComplexity, RefactorPlan, RuleCategory,
};

fn function_complexity(name: &str, complexity: u64) -> FunctionComplexity {
    FunctionComplexity {
        name: name.to_string(),
        complexity,
        line_start: 10,
        line_end: 20,
        line_complexities: vec![],
        refactor_plans: vec![],
        additional_refactor_plans: 0,
    }
}

fn file_complexity(
    path: &str,
    file_name: &str,
    functions: Vec<FunctionComplexity>,
) -> FileComplexity {
    FileComplexity {
        path: path.to_string(),
        file_name: file_name.to_string(),
        functions,
        complexity: 0,
    }
}

fn refactor_plan() -> RefactorPlan {
    RefactorPlan {
        kind: "extract-method".to_string(),
        title: "Extract method".to_string(),
        line_start: 11,
        line_end: 15,
        column_start: 4,
        current_complexity: 5,
        estimated_reduction: 2,
        estimated_complexity_after: 3,
        reduction_is_measured: true,
        rule_id: "C001".to_string(),
        category: RuleCategory::Complexity,
        applicability: Applicability::MachineApplicable,
        description: "Extract the method body".to_string(),
        explanation: "Reduces nesting".to_string(),
        references: vec![],
        suggestion: None::<CodeSuggestion>,
        help: None,
        doc_url: "https://example.com/c001".to_string(),
    }
}

fn write_and_load(
    output: &std::path::Path,
    files: &[FileComplexity],
    max_complexity: u64,
    suggest_refactors: bool,
) -> Value {
    store_gitlab(
        output.to_str().unwrap(),
        files,
        max_complexity,
        suggest_refactors,
    )
    .expect("should write");
    let content = fs::read_to_string(output).expect("should read");
    serde_json::from_str(&content).expect("should parse")
}

#[test]
fn gitlab_report_has_single_final_newline() {
    let dir = tempdir().expect("tempdir should work");
    let output = dir.path().join("results.gitlab.json");
    let files = vec![file_complexity(
        "src/a.py",
        "a.py",
        vec![function_complexity("f", 20)],
    )];

    write_and_load(&output, &files, 15, false);

    let content = fs::read_to_string(&output).expect("should read");
    assert!(content.ends_with('\n'));
    assert!(!content.ends_with("\n\n"));
}

#[test]
fn gitlab_report_contains_required_fields() {
    let dir = tempdir().expect("tempdir should work");
    let output = dir.path().join("results.gitlab.json");
    let files = vec![file_complexity(
        "src/a.py",
        "a.py",
        vec![function_complexity("f", 20)],
    )];

    let report = write_and_load(&output, &files, 15, false);
    let issue = &report[0];

    assert!(
        issue["description"]
            .as_str()
            .expect("description")
            .contains("Function 'f' has cognitive complexity 20")
    );
    assert_eq!(issue["check_name"], "complexipy/cognitive-complexity");
    let fingerprint = issue["fingerprint"].as_str().expect("fingerprint");
    assert_eq!(fingerprint.len(), 64);
    assert!(fingerprint.chars().all(|c| c.is_ascii_hexdigit()));
    assert_eq!(issue["severity"], "minor");
    assert_eq!(issue["location"]["path"], "src/a.py");
    assert_eq!(issue["location"]["lines"]["begin"], 10);
}

#[test]
fn gitlab_report_is_empty_when_threshold_is_high() {
    let dir = tempdir().expect("tempdir should work");
    let output = dir.path().join("results.gitlab.json");
    let files = vec![file_complexity(
        "src/a.py",
        "a.py",
        vec![function_complexity("f", 5)],
    )];

    let report = write_and_load(&output, &files, 15, false);

    assert_eq!(report, serde_json::json!([]));
}

#[test]
fn gitlab_report_omits_refactor_plans_by_default() {
    let dir = tempdir().expect("tempdir should work");
    let output = dir.path().join("results.gitlab.json");
    let files = vec![file_complexity(
        "src/a.py",
        "a.py",
        vec![FunctionComplexity {
            refactor_plans: vec![refactor_plan()],
            ..function_complexity("f", 5)
        }],
    )];

    let report = write_and_load(&output, &files, 15, false);

    assert_eq!(report, serde_json::json!([]));
}

#[test]
fn gitlab_report_includes_refactor_plans_when_requested() {
    let dir = tempdir().expect("tempdir should work");
    let output = dir.path().join("results.gitlab.json");
    let files = vec![file_complexity(
        "src/a.py",
        "a.py",
        vec![FunctionComplexity {
            refactor_plans: vec![refactor_plan()],
            ..function_complexity("f", 20)
        }],
    )];

    let report = write_and_load(&output, &files, 15, true);

    let plan_issue = report
        .as_array()
        .expect("report")
        .iter()
        .find(|issue| issue["check_name"] == "complexipy/c001")
        .expect("plan issue");
    assert_eq!(
        plan_issue["description"],
        "[C001] Extract method: Reduces nesting"
    );
    assert_eq!(plan_issue["severity"], "minor");
    assert_eq!(plan_issue["location"]["path"], "src/a.py");
    assert_eq!(plan_issue["location"]["lines"]["begin"], 11);
    assert_eq!(plan_issue["location"]["lines"]["end"], 15);
}

#[test]
fn gitlab_report_normalizes_paths_and_strips_dot_slash() {
    let dir = tempdir().expect("tempdir should work");
    let output = dir.path().join("results.gitlab.json");
    let files = vec![
        file_complexity("src", "a.py", vec![function_complexity("f", 20)]),
        file_complexity("./src/b.py", "b.py", vec![function_complexity("g", 20)]),
    ];

    let report = write_and_load(&output, &files, 15, false);

    let paths: Vec<&str> = report
        .as_array()
        .expect("report")
        .iter()
        .map(|issue| issue["location"]["path"].as_str().expect("path"))
        .collect();
    assert_eq!(paths, vec!["src/a.py", "src/b.py"]);
}

#[test]
fn gitlab_fingerprint_is_stable() {
    let dir = tempdir().expect("tempdir should work");
    let output = dir.path().join("results.gitlab.json");
    let files = vec![file_complexity(
        "src/a.py",
        "a.py",
        vec![function_complexity("f", 20)],
    )];

    let first = write_and_load(&output, &files, 15, false);
    let second = write_and_load(&output, &files, 15, false);

    assert_eq!(first[0]["fingerprint"], second[0]["fingerprint"]);
}

#[test]
fn gitlab_write_failure_is_io_error() {
    let dir = tempdir().expect("tempdir should work");

    let result = store_gitlab(
        dir.path().to_str().unwrap(),
        &[file_complexity(
            "src/a.py",
            "a.py",
            vec![function_complexity("f", 20)],
        )],
        15,
        false,
    );

    assert!(result.is_err());
}
