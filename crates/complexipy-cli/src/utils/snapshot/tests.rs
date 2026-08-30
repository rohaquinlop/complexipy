use std::fs;

use serde_json::json;
use tempfile::tempdir;

use crate::utils::snapshot::{build_snapshot_map, evaluate_snapshot, merge_snapshot_files};
use complexipy_core::classes::FileComplexity;
use complexipy_core::utils::{create_snapshot_file_shared, load_snapshot_file_shared};

fn file_complexity(path: &str, file_name: &str, functions: &[(&str, u64)]) -> FileComplexity {
    use complexipy_core::classes::FunctionComplexity;
    FileComplexity {
        path: path.to_string(),
        file_name: file_name.to_string(),
        functions: functions
            .iter()
            .map(|(name, complexity)| FunctionComplexity {
                name: name.to_string(),
                complexity: *complexity,
                line_start: 1,
                line_end: 2,
                line_complexities: vec![],
                refactor_plans: vec![],
                additional_refactor_plans: 0,
            })
            .collect(),
        complexity: 0,
    }
}

fn snapshot_path(dir: &tempfile::TempDir) -> std::path::PathBuf {
    dir.path().join("complexipy-snapshot.json")
}

#[test]
fn no_snapshot_file_exists() {
    let dir = tempdir().expect("tempdir should work");
    let path = snapshot_path(&dir);
    let files = vec![file_complexity("a.py", "a.py", &[("f", 5)])];

    let result = evaluate_snapshot(false, false, path.to_str().unwrap(), 15, &files)
        .expect("should evaluate");

    assert!(!result.should_run);
    assert_eq!(result.active_snapshot_map, None);
    assert!(result.watermark_success);
    assert_eq!(result.watermark_messages, Vec::<String>::new());
    assert!(result.snapshot_result);
}

#[test]
fn snapshot_file_exists_not_ignored() {
    let dir = tempdir().expect("tempdir should work");
    let path = snapshot_path(&dir);
    let files = vec![file_complexity("a.py", "a.py", &[("f", 5)])];
    evaluate_snapshot(true, false, path.to_str().unwrap(), 0, &files).expect("should evaluate");

    let result = evaluate_snapshot(false, false, path.to_str().unwrap(), 15, &files)
        .expect("should evaluate");

    assert!(result.should_run);
    let map = result.active_snapshot_map.expect("map");
    assert_eq!(
        map.get(&("a.py".to_string(), "a.py".to_string(), "f".to_string())),
        Some(&5)
    );
}

#[test]
fn snapshot_file_exists_ignored() {
    let dir = tempdir().expect("tempdir should work");
    let path = snapshot_path(&dir);
    let files = vec![file_complexity("a.py", "a.py", &[("f", 5)])];
    evaluate_snapshot(true, false, path.to_str().unwrap(), 0, &files).expect("should evaluate");

    let result = evaluate_snapshot(false, true, path.to_str().unwrap(), 15, &files)
        .expect("should evaluate");

    assert!(!result.should_run);
    assert_eq!(result.active_snapshot_map, None);
}

#[test]
fn snapshot_create_generates_file() {
    let dir = tempdir().expect("tempdir should work");
    let path = snapshot_path(&dir);
    let files = vec![
        file_complexity("a.py", "a.py", &[("high", 20)]),
        file_complexity("b.py", "b.py", &[("low", 2)]),
    ];

    evaluate_snapshot(true, false, path.to_str().unwrap(), 15, &files).expect("should evaluate");

    let stored = load_snapshot_file_shared(path.to_str().unwrap()).expect("should load");
    assert_eq!(stored.len(), 1);
    assert_eq!(stored[0].file_name, "a.py");
    assert_eq!(stored[0].functions[0].name, "high");
}

#[test]
fn watermark_passes_when_no_regressions() {
    let dir = tempdir().expect("tempdir should work");
    let path = snapshot_path(&dir);
    let baseline = vec![file_complexity("a.py", "a.py", &[("f", 20)])];
    evaluate_snapshot(true, false, path.to_str().unwrap(), 15, &baseline).expect("should evaluate");

    let current = vec![file_complexity("a.py", "a.py", &[("f", 20)])];
    let result = evaluate_snapshot(false, false, path.to_str().unwrap(), 15, &current)
        .expect("should evaluate");

    assert!(result.watermark_success);
    assert_eq!(result.watermark_messages, Vec::<String>::new());
}

#[test]
fn watermark_reports_new_violation() {
    let dir = tempdir().expect("tempdir should work");
    let path = snapshot_path(&dir);
    let baseline = vec![file_complexity("a.py", "a.py", &[("f", 5)])];
    evaluate_snapshot(true, false, path.to_str().unwrap(), 15, &baseline).expect("should evaluate");

    let current = vec![file_complexity(
        "a.py",
        "a.py",
        &[("f", 20), ("new_fn", 25)],
    )];
    let result = evaluate_snapshot(false, false, path.to_str().unwrap(), 15, &current)
        .expect("should evaluate");

    assert!(!result.watermark_success);
    assert_eq!(
        result.watermark_messages,
        vec![
            "a.py/a.py:f exceeds 15 but was not part of the snapshot.",
            "a.py/a.py:new_fn exceeds 15 but was not part of the snapshot."
        ]
    );
}

#[test]
fn watermark_reports_increased_complexity() {
    let dir = tempdir().expect("tempdir should work");
    let path = snapshot_path(&dir);
    let baseline = vec![file_complexity("a.py", "a.py", &[("f", 18)])];
    evaluate_snapshot(true, false, path.to_str().unwrap(), 15, &baseline).expect("should evaluate");

    let current = vec![file_complexity("a.py", "a.py", &[("f", 25)])];
    let result = evaluate_snapshot(false, false, path.to_str().unwrap(), 15, &current)
        .expect("should evaluate");

    assert!(!result.watermark_success);
    assert_eq!(
        result.watermark_messages,
        vec!["a.py/a.py:f increased from 18 to 25."]
    );
}

#[test]
fn watermark_without_snapshot_file() {
    let dir = tempdir().expect("tempdir should work");
    let path = snapshot_path(&dir);
    let files = vec![file_complexity("a.py", "a.py", &[("f", 20)])];

    let result = evaluate_snapshot(false, false, path.to_str().unwrap(), 15, &files)
        .expect("should evaluate");

    assert!(!result.should_run);
    assert!(result.watermark_success);
}

#[test]
fn snapshot_result_neutral_when_not_running() {
    let dir = tempdir().expect("tempdir should work");
    let path = snapshot_path(&dir);
    let files = vec![file_complexity("a.py", "a.py", &[("f", 5)])];

    let result = evaluate_snapshot(false, false, path.to_str().unwrap(), 15, &files)
        .expect("should evaluate");

    assert!(result.snapshot_result);
}

#[test]
fn partial_run_preserves_unanalyzed_files() {
    let dir = tempdir().expect("tempdir should work");
    let path = snapshot_path(&dir);
    let both = vec![
        file_complexity("a.py", "a.py", &[("f", 20)]),
        file_complexity("b.py", "b.py", &[("g", 20)]),
    ];
    evaluate_snapshot(true, false, path.to_str().unwrap(), 15, &both).expect("should evaluate");

    let only_a = vec![file_complexity("a.py", "a.py", &[("f", 20)])];
    let result = evaluate_snapshot(false, false, path.to_str().unwrap(), 15, &only_a)
        .expect("should evaluate");

    assert!(result.watermark_success);
    let stored = load_snapshot_file_shared(path.to_str().unwrap()).expect("should load");
    let names: Vec<&str> = stored.iter().map(|f| f.file_name.as_str()).collect();
    assert_eq!(names, vec!["a.py", "b.py"]);
}

#[test]
fn partial_run_keeps_analyzed_file_position() {
    let dir = tempdir().expect("tempdir should work");
    let path = snapshot_path(&dir);
    let all = vec![
        file_complexity("a.py", "a.py", &[("f", 20)]),
        file_complexity("b.py", "b.py", &[("g", 20)]),
        file_complexity("c.py", "c.py", &[("h", 20)]),
    ];
    evaluate_snapshot(true, false, path.to_str().unwrap(), 15, &all).expect("should evaluate");

    let only_b = vec![file_complexity("b.py", "b.py", &[("g", 25)])];
    evaluate_snapshot(true, false, path.to_str().unwrap(), 15, &only_b).expect("should evaluate");

    let stored = load_snapshot_file_shared(path.to_str().unwrap()).expect("should load");
    let names: Vec<&str> = stored.iter().map(|f| f.file_name.as_str()).collect();
    assert_eq!(names, vec!["a.py", "b.py", "c.py"]);
    assert_eq!(stored[1].functions[0].complexity, 25);
}

#[test]
fn repeated_partial_runs_keep_snapshot_byte_identical() {
    let dir = tempdir().expect("tempdir should work");
    let path = snapshot_path(&dir);
    let both = vec![
        file_complexity("a.py", "a.py", &[("f", 20)]),
        file_complexity("b.py", "b.py", &[("g", 20)]),
    ];
    evaluate_snapshot(true, false, path.to_str().unwrap(), 15, &both).expect("should evaluate");

    let only_b = vec![file_complexity("b.py", "b.py", &[("g", 20)])];
    evaluate_snapshot(false, false, path.to_str().unwrap(), 15, &only_b).expect("should evaluate");
    let first_content = fs::read_to_string(&path).expect("should read");

    evaluate_snapshot(false, false, path.to_str().unwrap(), 15, &only_b).expect("should evaluate");

    assert_eq!(
        fs::read_to_string(&path).expect("should read"),
        first_content
    );
}

#[test]
fn new_file_appends_at_end() {
    let dir = tempdir().expect("tempdir should work");
    let path = snapshot_path(&dir);
    let ab = vec![
        file_complexity("a.py", "a.py", &[("f", 20)]),
        file_complexity("b.py", "b.py", &[("g", 20)]),
    ];
    evaluate_snapshot(true, false, path.to_str().unwrap(), 15, &ab).expect("should evaluate");

    let ac = vec![
        file_complexity("a.py", "a.py", &[("f", 20)]),
        file_complexity("c.py", "c.py", &[("h", 20)]),
    ];
    evaluate_snapshot(true, false, path.to_str().unwrap(), 15, &ac).expect("should evaluate");

    let stored = load_snapshot_file_shared(path.to_str().unwrap()).expect("should load");
    let names: Vec<&str> = stored.iter().map(|f| f.file_name.as_str()).collect();
    assert_eq!(names, vec!["a.py", "b.py", "c.py"]);
}

#[test]
fn duplicate_snapshot_entries_collapse_in_place() {
    let dir = tempdir().expect("tempdir should work");
    let path = snapshot_path(&dir);
    let abc = vec![
        file_complexity("a.py", "a.py", &[("f", 20)]),
        file_complexity("b.py", "b.py", &[("g", 20)]),
        file_complexity("c.py", "c.py", &[("h", 20)]),
    ];
    evaluate_snapshot(true, false, path.to_str().unwrap(), 15, &abc).expect("should evaluate");

    let stored = load_snapshot_file_shared(path.to_str().unwrap()).expect("should load");
    let duplicated = vec![
        stored[0].clone(),
        stored[1].clone(),
        stored[1].clone(),
        stored[2].clone(),
    ];
    create_snapshot_file_shared(path.to_str().unwrap(), 0, duplicated).expect("should write");

    let only_b = vec![file_complexity("b.py", "b.py", &[("g", 20)])];
    evaluate_snapshot(false, false, path.to_str().unwrap(), 0, &only_b).expect("should evaluate");

    let stored = load_snapshot_file_shared(path.to_str().unwrap()).expect("should load");
    let names: Vec<&str> = stored.iter().map(|f| f.file_name.as_str()).collect();
    assert_eq!(names, vec!["a.py", "b.py", "c.py"]);
    assert_eq!(stored[1].functions[0].complexity, 20);
}

#[test]
fn partial_run_removes_improved_function() {
    let dir = tempdir().expect("tempdir should work");
    let path = snapshot_path(&dir);
    let both = vec![
        file_complexity("a.py", "a.py", &[("f", 20)]),
        file_complexity("b.py", "b.py", &[("g", 20)]),
    ];
    evaluate_snapshot(true, false, path.to_str().unwrap(), 15, &both).expect("should evaluate");

    let improved_a = vec![file_complexity("a.py", "a.py", &[("f", 2)])];
    let result = evaluate_snapshot(false, false, path.to_str().unwrap(), 15, &improved_a)
        .expect("should evaluate");

    assert!(result.watermark_success);
    let stored = load_snapshot_file_shared(path.to_str().unwrap()).expect("should load");
    let names: Vec<&str> = stored.iter().map(|f| f.file_name.as_str()).collect();
    assert_eq!(names, vec!["b.py"]);
}

#[test]
fn build_snapshot_map_collects_all_functions() {
    let files = vec![file_complexity("a.py", "a.py", &[("f", 3), ("g", 7)])];

    let map = build_snapshot_map(&files);

    assert_eq!(map.len(), 2);
    assert_eq!(
        map.get(&("a.py".to_string(), "a.py".to_string(), "g".to_string())),
        Some(&7)
    );
}

#[test]
fn merge_snapshot_files_replaces_in_place_and_appends() {
    let snapshot = vec![
        file_complexity("a.py", "a.py", &[("f", 20)]),
        file_complexity("b.py", "b.py", &[("g", 20)]),
    ];
    let current = vec![
        file_complexity("a.py", "a.py", &[("f", 25)]),
        file_complexity("c.py", "c.py", &[("h", 20)]),
    ];

    let merged = merge_snapshot_files(snapshot, &current);

    let names: Vec<&str> = merged.iter().map(|f| f.file_name.as_str()).collect();
    assert_eq!(names, vec!["a.py", "b.py", "c.py"]);
    assert_eq!(merged[0].functions[0].complexity, 25);
}

#[test]
fn corrupt_snapshot_propagates_error() {
    let dir = tempdir().expect("tempdir should work");
    let path = snapshot_path(&dir);
    fs::write(&path, json!([{"path": "broken"}]).to_string()).expect("should write");
    let files = vec![file_complexity("a.py", "a.py", &[("f", 5)])];

    let result = evaluate_snapshot(false, false, path.to_str().unwrap(), 15, &files);

    assert!(result.is_err());
}
