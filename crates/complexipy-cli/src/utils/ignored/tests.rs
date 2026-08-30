use std::fs;

use serde_json::Value;
use tempfile::tempdir;

use crate::types::OutputFormat;
use crate::utils::ignored::{handle_removable_ignores, handle_report_ignored};

const SIMPLE_MARKED: &str = "def simple(x):  # complexipy: ignore\n    return x + 1\n";

const COMPLEX_MARKED: &str = "def complex_fn(data):  # complexipy: ignore\n    if data:\n        for item in data:\n            if item:\n                return item\n    return None\n";

fn marked_file(dir: &std::path::Path, name: &str, content: &str) -> String {
    fs::write(dir.join(name), content).expect("should write");
    dir.join(name).to_string_lossy().into_owned()
}

#[test]
fn report_disabled_returns_empty() {
    let dir = tempdir().expect("tempdir should work");
    let file = marked_file(dir.path(), "a.py", SIMPLE_MARKED);

    let (locations, json_path) = handle_report_ignored(
        false,
        &[file],
        &[],
        &[OutputFormat::Json],
        None,
        false,
        dir.path().to_str().unwrap(),
    )
    .expect("should succeed");

    assert!(locations.is_empty());
    assert_eq!(json_path, None);
}

#[test]
fn report_writes_ignored_json_next_to_output() {
    let dir = tempdir().expect("tempdir should work");
    let file = marked_file(dir.path(), "a.py", SIMPLE_MARKED);

    let (locations, json_path) = handle_report_ignored(
        true,
        &[file],
        &[],
        &[OutputFormat::Json],
        None,
        false,
        dir.path().to_str().unwrap(),
    )
    .expect("should succeed");

    assert_eq!(locations.len(), 1);
    assert_eq!(locations[0].line, 1);
    assert_eq!(locations[0].comment, "# complexipy: ignore");

    let expected_path = dir.path().join("complexipy-ignored.json");
    assert_eq!(
        json_path,
        Some(expected_path.to_string_lossy().into_owned())
    );
    let content = fs::read_to_string(&expected_path).expect("should read");
    assert!(content.ends_with('\n'));
    let parsed: Vec<Value> = serde_json::from_str(&content).expect("should parse");
    assert_eq!(parsed.len(), 1);
    assert_eq!(parsed[0]["line"], 1);
    assert_eq!(parsed[0]["comment"], "# complexipy: ignore");
    assert!(parsed[0]["path"].as_str().expect("path").ends_with("a.py"));
}

#[test]
fn report_without_json_format_writes_no_file() {
    let dir = tempdir().expect("tempdir should work");
    let file = marked_file(dir.path(), "a.py", SIMPLE_MARKED);

    let (locations, json_path) = handle_report_ignored(
        true,
        &[file],
        &[],
        &[OutputFormat::Csv],
        None,
        false,
        dir.path().to_str().unwrap(),
    )
    .expect("should succeed");

    assert_eq!(locations.len(), 1);
    assert_eq!(json_path, None);
    assert!(!dir.path().join("complexipy-ignored.json").exists());
}

#[test]
fn report_without_comments_writes_no_file() {
    let dir = tempdir().expect("tempdir should work");
    let file = marked_file(dir.path(), "a.py", "def plain(x):\n    return x\n");

    let (locations, json_path) = handle_report_ignored(
        true,
        &[file],
        &[],
        &[OutputFormat::Json],
        None,
        false,
        dir.path().to_str().unwrap(),
    )
    .expect("should succeed");

    assert!(locations.is_empty());
    assert_eq!(json_path, None);
}

#[test]
fn removable_ignores_below_threshold() {
    let dir = tempdir().expect("tempdir should work");
    let file = marked_file(dir.path(), "a.py", SIMPLE_MARKED);

    let removable = handle_removable_ignores(&[file], &[], 15, dir.path().to_str().unwrap());

    assert_eq!(removable.len(), 1);
    assert_eq!(removable[0].function, "simple");
    assert_eq!(removable[0].complexity, 0);
    assert_eq!(removable[0].line, 1);
}

#[test]
fn removable_ignores_above_threshold_excluded() {
    let dir = tempdir().expect("tempdir should work");
    let file = marked_file(dir.path(), "a.py", COMPLEX_MARKED);

    let removable = handle_removable_ignores(&[file], &[], 2, dir.path().to_str().unwrap());

    assert!(removable.is_empty());
}
