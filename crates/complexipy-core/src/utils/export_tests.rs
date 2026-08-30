use std::fs;

use tempfile::tempdir;

use crate::classes::{
    Applicability, CodeSuggestion, FileComplexity, FunctionComplexity, RefactorPlan, RuleCategory,
};
use crate::utils::{ExportError, output_csv_shared, output_json_shared};

fn function_complexity(name: &str, complexity: u64) -> FunctionComplexity {
    FunctionComplexity {
        name: name.to_string(),
        complexity,
        line_start: 0,
        line_end: 0,
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
        kind: "test".to_string(),
        title: "Extract method".to_string(),
        line_start: 1,
        line_end: 5,
        column_start: 0,
        current_complexity: 5,
        estimated_reduction: 2,
        estimated_complexity_after: 3,
        reduction_is_measured: true,
        rule_id: "C001".to_string(),
        category: RuleCategory::Complexity,
        applicability: Applicability::MachineApplicable,
        description: "test plan".to_string(),
        explanation: "".to_string(),
        references: vec![],
        suggestion: None::<CodeSuggestion>,
        help: None,
        doc_url: "".to_string(),
    }
}

#[test]
fn csv_writes_header_and_rows() {
    let dir = tempdir().expect("tempdir should work");
    let output = dir.path().join("results.csv");
    let files = vec![file_complexity(
        "src/a.py",
        "a.py",
        vec![function_complexity("f", 5)],
    )];

    output_csv_shared(output.to_str().unwrap(), files, "asc", true, 0).expect("should write");

    let content = fs::read_to_string(&output).expect("should read");
    assert_eq!(
        content,
        "Path,File Name,Function Name,Cognitive Complexity\nsrc/a.py,a.py,f,5\n"
    );
}

#[test]
fn csv_asc_sorts_by_complexity_keeping_ties() {
    let dir = tempdir().expect("tempdir should work");
    let output = dir.path().join("results.csv");
    let files = vec![
        file_complexity("src/b.py", "b.py", vec![function_complexity("high", 9)]),
        file_complexity(
            "src/a.py",
            "a.py",
            vec![
                function_complexity("low", 1),
                function_complexity("tie1", 5),
                function_complexity("tie2", 5),
            ],
        ),
    ];

    output_csv_shared(output.to_str().unwrap(), files, "asc", true, 0).expect("should write");

    let content = fs::read_to_string(&output).expect("should read");
    let rows: Vec<&str> = content.lines().skip(1).collect();
    assert_eq!(rows[0], "src/a.py,a.py,low,1");
    assert_eq!(rows[1], "src/a.py,a.py,tie1,5");
    assert_eq!(rows[2], "src/a.py,a.py,tie2,5");
    assert_eq!(rows[3], "src/b.py,b.py,high,9");
}

#[test]
fn csv_desc_reverses_complexity_order() {
    let dir = tempdir().expect("tempdir should work");
    let output = dir.path().join("results.csv");
    let files = vec![
        file_complexity("src/a.py", "a.py", vec![function_complexity("low", 1)]),
        file_complexity("src/b.py", "b.py", vec![function_complexity("high", 9)]),
    ];

    output_csv_shared(output.to_str().unwrap(), files, "desc", true, 0).expect("should write");

    let content = fs::read_to_string(&output).expect("should read");
    let rows: Vec<&str> = content.lines().skip(1).collect();
    assert_eq!(rows[0], "src/b.py,b.py,high,9");
    assert_eq!(rows[1], "src/a.py,a.py,low,1");
}

#[test]
fn csv_file_name_sorts_by_path_keeping_function_order() {
    let dir = tempdir().expect("tempdir should work");
    let output = dir.path().join("results.csv");
    let files = vec![
        file_complexity(
            "src/z.py",
            "z.py",
            vec![
                function_complexity("first", 9),
                function_complexity("second", 1),
            ],
        ),
        file_complexity("src/a.py", "a.py", vec![function_complexity("only", 5)]),
    ];

    output_csv_shared(output.to_str().unwrap(), files, "file_name", true, 0).expect("should write");

    let content = fs::read_to_string(&output).expect("should read");
    let rows: Vec<&str> = content.lines().skip(1).collect();
    assert_eq!(rows[0], "src/a.py,a.py,only,5");
    assert_eq!(rows[1], "src/z.py,z.py,first,9");
    assert_eq!(rows[2], "src/z.py,z.py,second,1");
}

#[test]
fn csv_name_spelling_sorts_by_path() {
    let dir = tempdir().expect("tempdir should work");
    let output = dir.path().join("results.csv");
    let files = vec![
        file_complexity("src/b.py", "b.py", vec![function_complexity("f", 5)]),
        file_complexity("src/a.py", "a.py", vec![function_complexity("g", 5)]),
    ];

    output_csv_shared(output.to_str().unwrap(), files, "name", true, 0).expect("should write");

    let content = fs::read_to_string(&output).expect("should read");
    let rows: Vec<&str> = content.lines().skip(1).collect();
    assert_eq!(rows[0], "src/a.py,a.py,g,5");
    assert_eq!(rows[1], "src/b.py,b.py,f,5");
}

#[test]
fn csv_filters_below_threshold_when_not_detailed() {
    let dir = tempdir().expect("tempdir should work");
    let output = dir.path().join("results.csv");
    let files = vec![file_complexity(
        "src/a.py",
        "a.py",
        vec![
            function_complexity("below", 3),
            function_complexity("at", 5),
            function_complexity("above", 7),
        ],
    )];

    output_csv_shared(output.to_str().unwrap(), files, "asc", false, 5).expect("should write");

    let content = fs::read_to_string(&output).expect("should read");
    let rows: Vec<&str> = content.lines().skip(1).collect();
    assert_eq!(rows, vec!["src/a.py,a.py,above,7"]);
}

#[test]
fn csv_invalid_sort_rejected() {
    let dir = tempdir().expect("tempdir should work");
    let output = dir.path().join("results.csv");
    let files = vec![file_complexity(
        "src/a.py",
        "a.py",
        vec![function_complexity("f", 5)],
    )];

    let result = output_csv_shared(output.to_str().unwrap(), files, "bogus", true, 0);

    assert_eq!(result, Err(ExportError::InvalidSort("bogus".to_string())));
    assert!(!output.exists());
}

#[test]
fn csv_write_failure_is_io_error() {
    let dir = tempdir().expect("tempdir should work");

    let result = output_csv_shared(dir.path().to_str().unwrap(), vec![], "asc", true, 0);

    assert!(matches!(result, Err(ExportError::Io(_))));
}

#[test]
fn json_writes_entries_with_trailing_newline() {
    let dir = tempdir().expect("tempdir should work");
    let output = dir.path().join("results.json");
    let files = vec![file_complexity(
        "src/a.py",
        "a.py",
        vec![function_complexity("f", 5)],
    )];

    output_json_shared(output.to_str().unwrap(), files, true, 0, false).expect("should write");

    let content = fs::read_to_string(&output).expect("should read");
    assert!(content.ends_with('\n'));
    let parsed: serde_json::Value = serde_json::from_str(&content).expect("should parse");
    assert_eq!(parsed.as_array().expect("array").len(), 1);
    let entry = &parsed[0];
    assert_eq!(entry["path"], "src/a.py");
    assert_eq!(entry["file_name"], "a.py");
    assert_eq!(entry["function_name"], "f");
    assert_eq!(entry["complexity"], 5);
    assert_eq!(entry["refactor_plans"], serde_json::json!([]));
}

#[test]
fn json_includes_refactor_plans_when_requested() {
    let dir = tempdir().expect("tempdir should work");
    let output = dir.path().join("results.json");
    let files = vec![file_complexity(
        "src/a.py",
        "a.py",
        vec![FunctionComplexity {
            refactor_plans: vec![refactor_plan()],
            ..function_complexity("f", 5)
        }],
    )];

    output_json_shared(output.to_str().unwrap(), files, true, 0, true).expect("should write");

    let content = fs::read_to_string(&output).expect("should read");
    let parsed: serde_json::Value = serde_json::from_str(&content).expect("should parse");
    assert_eq!(
        parsed[0]["refactor_plans"].as_array().expect("array").len(),
        1
    );
    assert_eq!(parsed[0]["refactor_plans"][0]["title"], "Extract method");
}

#[test]
fn json_filters_below_threshold_when_not_detailed() {
    let dir = tempdir().expect("tempdir should work");
    let output = dir.path().join("results.json");
    let files = vec![file_complexity(
        "src/a.py",
        "a.py",
        vec![
            function_complexity("below", 3),
            function_complexity("above", 7),
        ],
    )];

    output_json_shared(output.to_str().unwrap(), files, false, 5, false).expect("should write");

    let content = fs::read_to_string(&output).expect("should read");
    let parsed: serde_json::Value = serde_json::from_str(&content).expect("should parse");
    assert_eq!(parsed.as_array().expect("array").len(), 1);
    assert_eq!(parsed[0]["function_name"], "above");
}

#[test]
fn json_write_failure_is_io_error() {
    let dir = tempdir().expect("tempdir should work");

    let result = output_json_shared(dir.path().to_str().unwrap(), vec![], true, 0, false);

    assert!(matches!(result, Err(ExportError::Io(_))));
}
