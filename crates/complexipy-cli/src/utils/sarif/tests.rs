use std::fs;

use serde_json::Value;
use tempfile::tempdir;

use crate::utils::sarif::store_sarif;
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
    store_sarif(
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
fn sarif_file_created_and_valid() {
    let dir = tempdir().expect("tempdir should work");
    let output = dir.path().join("results.sarif");
    let files = vec![file_complexity(
        "src/a.py",
        "a.py",
        vec![function_complexity("f", 20)],
    )];

    let doc = write_and_load(&output, &files, 15, false);

    assert_eq!(doc["version"], "2.1.0");
    assert!(doc["$schema"].is_string());
    let driver = &doc["runs"][0]["tool"]["driver"];
    assert_eq!(driver["name"], "complexipy");
    assert_eq!(driver["version"], env!("CARGO_PKG_VERSION"));
}

#[test]
fn sarif_contains_violations() {
    let dir = tempdir().expect("tempdir should work");
    let output = dir.path().join("results.sarif");
    let files = vec![file_complexity(
        "src/a.py",
        "a.py",
        vec![function_complexity("f", 20)],
    )];

    let doc = write_and_load(&output, &files, 15, false);

    let results = doc["runs"][0]["results"].as_array().expect("results");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0]["ruleId"], "CC001");
    assert_eq!(results[0]["level"], "warning");
    assert!(
        results[0]["message"]["text"]
            .as_str()
            .expect("message")
            .contains("Function 'f' has a cognitive complexity of 20")
    );
}

#[test]
fn sarif_no_violations_when_threshold_high() {
    let dir = tempdir().expect("tempdir should work");
    let output = dir.path().join("results.sarif");
    let files = vec![file_complexity(
        "src/a.py",
        "a.py",
        vec![function_complexity("f", 5)],
    )];

    let doc = write_and_load(&output, &files, 15, false);

    assert_eq!(doc["runs"][0]["results"], serde_json::json!([]));
}

#[test]
fn sarif_result_has_location() {
    let dir = tempdir().expect("tempdir should work");
    let output = dir.path().join("results.sarif");
    let files = vec![file_complexity(
        "src/a.py",
        "a.py",
        vec![function_complexity("f", 20)],
    )];

    let doc = write_and_load(&output, &files, 15, false);

    let location = &doc["runs"][0]["results"][0]["locations"][0];
    assert_eq!(
        location["physicalLocation"]["artifactLocation"]["uri"],
        "src/a.py"
    );
    assert_eq!(
        location["physicalLocation"]["artifactLocation"]["uriBaseId"],
        "%SRCROOT%"
    );
    assert_eq!(location["physicalLocation"]["region"]["startLine"], 10);
    assert_eq!(location["physicalLocation"]["region"]["endLine"], 20);
    assert_eq!(location["logicalLocations"][0]["name"], "f");
    assert_eq!(location["logicalLocations"][0]["kind"], "function");
}

#[test]
fn sarif_rule_defined() {
    let dir = tempdir().expect("tempdir should work");
    let output = dir.path().join("results.sarif");
    let files = vec![file_complexity(
        "src/a.py",
        "a.py",
        vec![function_complexity("f", 5)],
    )];

    let doc = write_and_load(&output, &files, 15, false);

    let rules = doc["runs"][0]["tool"]["driver"]["rules"]
        .as_array()
        .expect("rules");
    assert_eq!(rules.len(), 1);
    assert_eq!(rules[0]["id"], "CC001");
    assert_eq!(rules[0]["name"], "CognitiveComplexity");
    assert_eq!(
        rules[0]["helpUri"],
        "https://rohaquinlop.github.io/complexipy/understanding-scores/"
    );
}

#[test]
fn sarif_omits_refactor_plan_results_by_default() {
    let dir = tempdir().expect("tempdir should work");
    let output = dir.path().join("results.sarif");
    let files = vec![file_complexity(
        "src/a.py",
        "a.py",
        vec![FunctionComplexity {
            refactor_plans: vec![refactor_plan()],
            ..function_complexity("f", 5)
        }],
    )];

    let doc = write_and_load(&output, &files, 15, false);

    let results = doc["runs"][0]["results"].as_array().expect("results");
    assert!(results.iter().all(|result| result["ruleId"] == "CC001"));
    let rules = doc["runs"][0]["tool"]["driver"]["rules"]
        .as_array()
        .expect("rules");
    assert_eq!(rules.len(), 1);
}

#[test]
fn sarif_includes_refactor_plan_rules_when_requested() {
    let dir = tempdir().expect("tempdir should work");
    let output = dir.path().join("results.sarif");
    let files = vec![file_complexity(
        "src/a.py",
        "a.py",
        vec![FunctionComplexity {
            refactor_plans: vec![refactor_plan()],
            ..function_complexity("f", 20)
        }],
    )];

    let doc = write_and_load(&output, &files, 15, true);

    let results = doc["runs"][0]["results"].as_array().expect("results");
    let plan_result = results
        .iter()
        .find(|result| result["ruleId"] == "C001")
        .expect("plan result");
    assert_eq!(plan_result["level"], "warning");
    assert_eq!(
        plan_result["message"]["text"],
        "Extract method: Reduces nesting"
    );
    assert_eq!(
        plan_result["locations"][0]["physicalLocation"]["region"]["startColumn"],
        4
    );

    let rules = doc["runs"][0]["tool"]["driver"]["rules"]
        .as_array()
        .expect("rules");
    let plan_rule = rules
        .iter()
        .find(|rule| rule["id"] == "C001")
        .expect("plan rule");
    assert_eq!(plan_rule["name"], "extract-method");
    assert_eq!(
        plan_rule["shortDescription"]["text"],
        "Extract the method body"
    );
    assert_eq!(plan_rule["helpUri"], "https://example.com/c001");
    assert_eq!(plan_rule["properties"]["tags"][0], "complexity");
}

#[test]
fn sarif_write_failure_is_io_error() {
    let dir = tempdir().expect("tempdir should work");

    let result = store_sarif(
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
