#![cfg(feature = "runner")]

use complexipy_core::{is_path_excluded, validate_exclude_patterns};

#[test]
fn no_patterns_excludes_nothing() {
    assert!(!is_path_excluded("/repo/src/a.py", "/repo", &[]));
}

#[test]
fn pattern_matches_relative_path() {
    let patterns = vec!["legacy/**".to_string()];

    assert!(is_path_excluded("/repo/legacy/old.py", "/repo", &patterns));
    assert!(!is_path_excluded("/repo/src/a.py", "/repo", &patterns));
}

#[test]
fn pattern_matches_nested_directory() {
    let patterns = vec!["**/generated/*.py".to_string()];

    assert!(is_path_excluded(
        "/repo/pkg/deep/generated/x.py",
        "/repo",
        &patterns
    ));
}

#[test]
fn windows_separators_are_normalized() {
    let patterns = vec!["legacy/**".to_string()];

    assert!(is_path_excluded(
        "C:\\repo\\legacy\\old.py",
        "C:\\repo",
        &patterns
    ));
}

#[test]
fn pattern_outside_root_does_not_match() {
    let patterns = vec!["src/*.py".to_string()];

    assert!(!is_path_excluded("/elsewhere/src/a.py", "/repo", &patterns));
}

#[test]
fn multiple_patterns_are_alternatives() {
    let patterns = vec!["build/**".to_string(), "dist/*.py".to_string()];

    assert!(is_path_excluded("/repo/dist/a.py", "/repo", &patterns));
    assert!(!is_path_excluded("/repo/src/a.py", "/repo", &patterns));
}

#[test]
fn validation_accepts_no_patterns_and_real_globs() {
    assert!(validate_exclude_patterns(&[]).is_ok());
    assert!(validate_exclude_patterns(&["legacy/**".to_string()]).is_ok());
}

#[test]
fn validation_rejects_a_malformed_glob() {
    let error = validate_exclude_patterns(&["[unclosed".to_string()]).unwrap_err();

    assert!(error.contains("invalid exclude pattern"));
}
