use std::fs;

use serde_json::Value;
use tempfile::tempdir;

use crate::types::DiffBaseline;
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
        None,
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
        "https://complexipy.com/understanding-scores/"
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
        None,
    );

    assert!(result.is_err());
}

#[test]
fn sarif_uri_is_percent_encoded() {
    let dir = tempdir().expect("tempdir should work");
    let output = dir.path().join("results.sarif");
    let files = vec![file_complexity(
        "src/my file#1?.py",
        "my file#1?.py",
        vec![function_complexity("f", 20)],
    )];

    let sarif = write_and_load(&output, &files, 15, false);

    assert_eq!(
        sarif["runs"][0]["results"][0]["locations"][0]["physicalLocation"]["artifactLocation"]["uri"],
        "src/my%20file%231%3F.py"
    );
}

#[test]
fn sarif_points_at_a_published_schema_and_fingerprints_each_result() {
    let dir = tempdir().expect("tempdir should work");
    let output = dir.path().join("results.sarif");
    let moved = FunctionComplexity {
        line_start: 30,
        line_end: 40,
        ..function_complexity("f", 20)
    };

    let before = write_and_load(
        &output,
        &[file_complexity(
            "src/a.py",
            "a.py",
            vec![function_complexity("f", 20)],
        )],
        15,
        false,
    );
    let after = write_and_load(
        &output,
        &[file_complexity("src/a.py", "a.py", vec![moved])],
        15,
        false,
    );

    assert!(
        before["$schema"]
            .as_str()
            .unwrap()
            .starts_with("https://docs.oasis-open.org/sarif/")
    );
    let fingerprint =
        &before["runs"][0]["results"][0]["partialFingerprints"]["complexipyFindingHash/v1"];
    assert_eq!(fingerprint.as_str().map(str::len), Some(64));
    assert_eq!(
        fingerprint,
        &after["runs"][0]["results"][0]["partialFingerprints"]["complexipyFindingHash/v1"]
    );
}

fn baseline(enforced: bool) -> DiffBaseline {
    let key = |name: &str| ("src/a.py".to_string(), name.to_string());
    DiffBaseline {
        complexities: [
            (key("fresh"), None),
            (key("kept"), Some(30)),
            (key("worse"), Some(18)),
            (key("better"), Some(40)),
        ]
        .into_iter()
        .collect(),
        failing: enforced.then(|| [key("fresh"), key("worse")].into_iter().collect()),
    }
}

fn gated_files() -> Vec<FileComplexity> {
    vec![file_complexity(
        "src/a.py",
        "a.py",
        vec![
            function_complexity("fresh", 20),
            function_complexity("kept", 30),
            function_complexity("worse", 25),
            function_complexity("better", 35),
            function_complexity("untouched", 16),
        ],
    )]
}

fn store_with(output: &std::path::Path, baseline: &DiffBaseline) -> Value {
    store_sarif(
        output.to_str().unwrap(),
        &gated_files(),
        15,
        false,
        Some(baseline),
    )
    .expect("should write");
    serde_json::from_str(&fs::read_to_string(output).expect("should read")).expect("should parse")
}

fn result_fields(doc: &Value, field: &str) -> Vec<(String, Option<String>)> {
    doc["runs"][0]["results"]
        .as_array()
        .expect("results")
        .iter()
        .map(|result| {
            (
                result["locations"][0]["logicalLocations"][0]["name"]
                    .as_str()
                    .unwrap()
                    .to_string(),
                result[field].as_str().map(str::to_string),
            )
        })
        .collect()
}

fn named(pairs: &[(&str, Option<&str>)]) -> Vec<(String, Option<String>)> {
    pairs
        .iter()
        .map(|(name, value)| (name.to_string(), value.map(str::to_string)))
        .collect()
}

#[test]
fn a_gated_diff_marks_each_result_with_its_baseline_state_and_verdict() {
    let dir = tempdir().expect("tempdir should work");
    let output = dir.path().join("results.sarif");

    let doc = store_with(&output, &baseline(true));

    assert_eq!(
        result_fields(&doc, "baselineState"),
        named(&[
            ("fresh", Some("new")),
            ("kept", Some("unchanged")),
            ("worse", Some("updated")),
            ("better", Some("updated")),
            ("untouched", Some("unchanged")),
        ])
    );
    assert_eq!(
        result_fields(&doc, "level"),
        named(&[
            ("fresh", Some("error")),
            ("kept", Some("note")),
            ("worse", Some("error")),
            ("better", Some("note")),
            ("untouched", Some("note")),
        ])
    );
}

#[test]
fn a_diff_that_does_not_gate_keeps_the_level_and_the_fingerprints() {
    let dir = tempdir().expect("tempdir should work");
    let gated = store_with(&dir.path().join("gated.sarif"), &baseline(true));
    let plain = store_with(&dir.path().join("plain.sarif"), &baseline(false));
    let without = write_and_load(&dir.path().join("none.sarif"), &gated_files(), 15, false);

    assert!(
        result_fields(&plain, "level")
            .iter()
            .all(|(_, level)| level.as_deref() == Some("warning"))
    );
    assert_eq!(
        result_fields(&plain, "baselineState"),
        result_fields(&gated, "baselineState")
    );
    assert!(
        result_fields(&without, "baselineState")
            .iter()
            .all(|(_, state)| state.is_none())
    );
    let fingerprints = |doc: &Value| -> Vec<Value> {
        doc["runs"][0]["results"]
            .as_array()
            .unwrap()
            .iter()
            .map(|result| result["partialFingerprints"].clone())
            .collect()
    };
    assert_eq!(fingerprints(&gated), fingerprints(&without));
}

#[test]
fn every_result_carries_a_baseline_state_when_any_does() {
    let dir = tempdir().expect("tempdir should work");
    let output = dir.path().join("results.sarif");
    let mut planned = function_complexity("kept", 30);
    planned.refactor_plans = vec![refactor_plan()];
    let mut below = function_complexity("small", 3);
    below.refactor_plans = vec![refactor_plan()];
    let files = vec![file_complexity("src/a.py", "a.py", vec![planned, below])];

    store_sarif(
        output.to_str().unwrap(),
        &files,
        15,
        true,
        Some(&baseline(true)),
    )
    .expect("should write");
    let doc: Value =
        serde_json::from_str(&fs::read_to_string(&output).expect("should read")).expect("parse");

    let states: Vec<(&str, &str)> = doc["runs"][0]["results"]
        .as_array()
        .expect("results")
        .iter()
        .map(|result| {
            (
                result["ruleId"].as_str().unwrap(),
                result["baselineState"].as_str().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        states,
        vec![
            ("CC001", "unchanged"),
            ("C001", "unchanged"),
            ("C001", "unchanged")
        ]
    );
}
