//! Integration test for the stable public API of complexipy-core.

use std::fs;

use tempfile::tempdir;

use complexipy_core::{
    Applicability, CodeComplexity, CodeSuggestion, DiffEntry, DiffStatus, FileComplexity,
    FunctionComplexity, IgnoredLocation, LineComplexity, RefactorPlan, RemovableIgnore,
    RuleCategory, code_complexity, collect_all_ignored_locations,
    collect_removable_ignored_locations, compute_diff, file_complexity, has_regressions,
};

#[test]
fn stable_api_types_exist() {
    let _ = std::mem::size_of::<Applicability>();
    let _ = std::mem::size_of::<CodeComplexity>();
    let _ = std::mem::size_of::<CodeSuggestion>();
    let _ = std::mem::size_of::<DiffEntry>();
    let _ = std::mem::size_of::<DiffStatus>();
    let _ = std::mem::size_of::<FileComplexity>();
    let _ = std::mem::size_of::<FunctionComplexity>();
    let _ = std::mem::size_of::<IgnoredLocation>();
    let _ = std::mem::size_of::<LineComplexity>();
    let _ = std::mem::size_of::<RefactorPlan>();
    let _ = std::mem::size_of::<RemovableIgnore>();
    let _ = std::mem::size_of::<RuleCategory>();
}

#[test]
fn stable_api_code_complexity_works() {
    let result =
        code_complexity("def f(x):\n    return x\n", false, false).expect("should analyze");
    assert_eq!(result.functions.len(), 1);
    assert_eq!(result.functions[0].name, "f");
}

#[test]
fn stable_api_file_complexity_works() {
    let dir = tempdir().expect("tempdir should work");
    let file = dir.path().join("m.py");
    fs::write(&file, "def f(x):\n    return x\n").expect("should write");

    let result = file_complexity(file.to_str().unwrap(), false, false).expect("should analyze");
    assert_eq!(result.functions[0].name, "f");
}

#[test]
fn stable_api_diff_and_ratchet_work() {
    let entry = DiffEntry {
        file_path: "a.py".to_string(),
        func_name: "f".to_string(),
        old_complexity: Some(2),
        new_complexity: Some(6),
    };
    assert_eq!(entry.status(), DiffStatus::Regressed);
    assert!(has_regressions(&[entry], 5));

    assert!(compute_diff(&[], "HEAD", ".").is_empty());
}

#[test]
fn stable_api_collectors_work() {
    let (locations, failed) = collect_all_ignored_locations(&[], &[], ".").expect("should run");
    assert!(locations.is_empty());
    assert!(failed.is_empty());

    let (removable, failed) =
        collect_removable_ignored_locations(&[], &[], 15, ".").expect("should run");
    assert!(removable.is_empty());
    assert!(failed.is_empty());
}
