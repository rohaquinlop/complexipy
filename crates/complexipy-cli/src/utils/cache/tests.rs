use std::collections::HashMap;
use std::fs;

use serde_json::{Value, json};
use tempfile::tempdir;

use crate::utils::cache::{
    build_cache_key, hash_targets, lexically_clean, remember_previous_functions,
};
use complexipy_core::classes::{FileComplexity, FunctionComplexity};

fn file_complexity(path: &str, file_name: &str, functions: &[(&str, u64)]) -> FileComplexity {
    FileComplexity {
        path: path.to_string(),
        file_name: file_name.to_string(),
        functions: functions
            .iter()
            .map(|(name, complexity)| FunctionComplexity {
                name: name.to_string(),
                complexity: *complexity,
                line_start: 0,
                line_end: 0,
                line_complexities: vec![],
                refactor_plans: vec![],
                additional_refactor_plans: 0,
            })
            .collect(),
        complexity: 0,
    }
}

fn load_functions_file(cache_dir: &std::path::Path) -> Value {
    let path = cache_dir.join("v/cache/functions");
    let content = fs::read_to_string(&path).expect("functions file should exist");
    serde_json::from_str(&content).expect("functions file should parse")
}

#[test]
fn cache_key_matches_python_blake2b() {
    assert_eq!(
        hash_targets("src||tests"),
        "c7f7a0c9374f38debd3947990e36306c"
    );
}

#[test]
fn lexical_clean_normalizes_dot_components() {
    let base = std::path::Path::new("/proj");
    let cleaned = lexically_clean(&base.join("src/../tests/./main.py"));
    assert_eq!(cleaned, base.join("tests/main.py"));
}

#[test]
fn first_run_creates_cache_directory_and_support_files() {
    let dir = tempdir().expect("tempdir should work");
    let inv = dir.path().join("proj");
    fs::create_dir(&inv).expect("should create");

    let result = remember_previous_functions(
        inv.to_str().unwrap(),
        &["src".to_string()],
        &[file_complexity("src/a.py", "a.py", &[("f", 5)])],
        None,
    );

    assert_eq!(result, None);
    let cache_dir = inv.join(".complexipy_cache");
    assert_eq!(
        fs::read_to_string(cache_dir.join(".gitignore")).expect("gitignore"),
        "*\n"
    );
    assert!(cache_dir.join("CACHEDIR.TAG").exists());
    assert!(cache_dir.join("README.md").exists());

    let store = load_functions_file(&cache_dir);
    let entries = store
        .get("entries")
        .expect("entries")
        .as_object()
        .expect("object");
    assert_eq!(entries.len(), 1);
    let entry = entries.values().next().expect("entry");
    assert_eq!(
        entry.get("targets").expect("targets"),
        &json!([inv.join("src").to_string_lossy()])
    );
    assert_eq!(
        entry.get("functions").expect("functions"),
        &json!([{
            "path": "src/a.py",
            "file_name": "a.py",
            "function_name": "f",
            "complexity": 5,
        }])
    );
    assert!(entry.get("updated_at").expect("updated_at").is_number());
}

#[test]
fn second_run_returns_previous_map() {
    let dir = tempdir().expect("tempdir should work");
    let inv = dir.path().join("proj");
    fs::create_dir(&inv).expect("should create");
    let files = [file_complexity("src/a.py", "a.py", &[("f", 5)])];

    let first =
        remember_previous_functions(inv.to_str().unwrap(), &["src".to_string()], &files, None);
    assert_eq!(first, None);

    let second =
        remember_previous_functions(inv.to_str().unwrap(), &["src".to_string()], &files, None);
    assert_eq!(
        second,
        Some(HashMap::from([(
            ("src/a.py".to_string(), "a.py".to_string(), "f".to_string()),
            5
        )]))
    );
}

#[test]
fn gitignore_is_not_recreated_if_exists() {
    let dir = tempdir().expect("tempdir should work");
    let inv = dir.path().join("proj");
    fs::create_dir(&inv).expect("should create");
    let cache_dir = inv.join(".complexipy_cache");
    fs::create_dir_all(&cache_dir).expect("should create");
    fs::write(cache_dir.join(".gitignore"), "custom\n").expect("should write");

    remember_previous_functions(
        inv.to_str().unwrap(),
        &["src".to_string()],
        &[file_complexity("src/a.py", "a.py", &[("f", 5)])],
        None,
    );

    assert_eq!(
        fs::read_to_string(cache_dir.join(".gitignore")).expect("gitignore"),
        "custom\n"
    );
}

#[test]
fn target_sets_share_single_functions_file() {
    let dir = tempdir().expect("tempdir should work");
    let inv = dir.path().join("proj");
    fs::create_dir(&inv).expect("should create");

    remember_previous_functions(
        inv.to_str().unwrap(),
        &["src".to_string()],
        &[file_complexity("src/a.py", "a.py", &[("f", 5)])],
        None,
    );
    remember_previous_functions(
        inv.to_str().unwrap(),
        &["tests".to_string(), "src".to_string()],
        &[file_complexity("tests/t.py", "t.py", &[("g", 3)])],
        None,
    );

    let cache_dir = inv.join(".complexipy_cache");
    let store = load_functions_file(&cache_dir);
    let entries = store
        .get("entries")
        .expect("entries")
        .as_object()
        .expect("object");
    assert_eq!(entries.len(), 2);
}

#[test]
fn custom_cache_dir_honored_when_none_provided() {
    let dir = tempdir().expect("tempdir should work");
    let inv = dir.path().join("proj");
    fs::create_dir(&inv).expect("should create");
    let default_dir = inv.join(".complexipy_cache");

    remember_previous_functions(
        inv.to_str().unwrap(),
        &["src".to_string()],
        &[file_complexity("src/a.py", "a.py", &[("f", 5)])],
        None,
    );

    assert!(default_dir.join("v/cache/functions").exists());
    assert!(!dir.path().join("custom").exists());
}

#[test]
fn nested_custom_cache_dir_created() {
    let dir = tempdir().expect("tempdir should work");
    let inv = dir.path().join("proj");
    fs::create_dir(&inv).expect("should create");
    let nested = dir.path().join("a").join("b").join("c");

    let result = remember_previous_functions(
        inv.to_str().unwrap(),
        &["src".to_string()],
        &[file_complexity("src/a.py", "a.py", &[("f", 5)])],
        Some(nested.to_str().unwrap()),
    );

    assert_eq!(result, None);
    assert!(nested.join("v/cache/functions").exists());
}

#[test]
fn cache_prunes_old_target_set_entries() {
    let dir = tempdir().expect("tempdir should work");
    let inv = dir.path().join("proj");
    fs::create_dir(&inv).expect("should create");
    let cache_dir = inv.join(".complexipy_cache");
    let values_dir = cache_dir.join("v/cache");
    fs::create_dir_all(&values_dir).expect("should create");

    let mut entries = serde_json::Map::new();
    for i in 0..65u64 {
        entries.insert(
            format!("key-{:02}", i),
            json!({ "targets": [], "functions": [], "updated_at": i as f64 }),
        );
    }
    let store = json!({ "entries": entries });
    fs::write(
        values_dir.join("functions"),
        serde_json::to_string_pretty(&store).expect("serialize"),
    )
    .expect("should write");

    remember_previous_functions(
        inv.to_str().unwrap(),
        &["src".to_string()],
        &[file_complexity("src/a.py", "a.py", &[("f", 5)])],
        None,
    );

    let stored = load_functions_file(&cache_dir);
    let entries = stored
        .get("entries")
        .expect("entries")
        .as_object()
        .expect("object");
    assert_eq!(entries.len(), 64);
    assert!(!entries.contains_key("key-00"));
    assert!(!entries.contains_key("key-01"));
    assert!(entries.contains_key("key-63"));
}

#[test]
fn filesystem_failure_returns_none() {
    let dir = tempdir().expect("tempdir should work");
    let inv = dir.path().join("proj");
    fs::create_dir(&inv).expect("should create");
    fs::write(inv.join(".complexipy_cache"), "blocking").expect("should write");

    let result = remember_previous_functions(
        inv.to_str().unwrap(),
        &["src".to_string()],
        &[file_complexity("src/a.py", "a.py", &[("f", 5)])],
        None,
    );

    assert_eq!(result, None);
}

#[test]
fn empty_targets_produce_no_cache_key() {
    let dir = tempdir().expect("tempdir should work");
    let inv = dir.path().join("proj");
    fs::create_dir(&inv).expect("should create");

    let result = remember_previous_functions(
        inv.to_str().unwrap(),
        &[],
        &[file_complexity("src/a.py", "a.py", &[("f", 5)])],
        None,
    );

    assert_eq!(result, None);
    assert!(!inv.join(".complexipy_cache").exists());
}

#[test]
fn malformed_entries_are_skipped() {
    let dir = tempdir().expect("tempdir should work");
    let inv = dir.path().join("proj");
    fs::create_dir(&inv).expect("should create");
    let cache_dir = inv.join(".complexipy_cache");
    let values_dir = cache_dir.join("v/cache");
    fs::create_dir_all(&values_dir).expect("should create");

    let key = build_cache_key(inv.to_str().unwrap(), &["src".to_string()]).expect("key");
    let store = json!({
        "entries": {
            key: {
                "targets": [],
                "functions": [
                    42,
                    { "path": "x.py", "file_name": "x.py", "function_name": "", "complexity": 3 },
                    { "path": "x.py", "file_name": "x.py", "function_name": "b", "complexity": true },
                    { "path": "x.py", "file_name": "x.py", "function_name": "c", "complexity": "abc" },
                    { "path": "x.py", "file_name": "x.py", "function_name": "d", "complexity": "7" },
                    { "path": "x.py", "file_name": "x.py", "function_name": "e", "complexity": 11 },
                ],
                "updated_at": 1.0,
            }
        }
    });
    fs::write(
        values_dir.join("functions"),
        serde_json::to_string_pretty(&store).expect("serialize"),
    )
    .expect("should write");

    let result = remember_previous_functions(
        inv.to_str().unwrap(),
        &["src".to_string()],
        &[file_complexity("src/a.py", "a.py", &[("f", 5)])],
        None,
    );

    assert_eq!(
        result,
        Some(HashMap::from([
            (("x.py".to_string(), "x.py".to_string(), "d".to_string()), 7),
            (
                ("x.py".to_string(), "x.py".to_string(), "e".to_string()),
                11
            ),
        ]))
    );
}
