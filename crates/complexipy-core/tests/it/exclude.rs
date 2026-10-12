#![cfg(feature = "runner")]

use complexipy_core::{invalid_exclude_patterns, is_path_excluded};

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
fn root_is_matched_as_a_path_not_as_a_text_prefix() {
    let patterns = vec!["y/**".to_string()];

    assert!(!is_path_excluded("/repo/xy/a.py", "/repo/x", &patterns));
    assert!(is_path_excluded("/repo/x/y/a.py", "/repo/x", &patterns));
}

#[test]
fn root_itself_holds_no_relative_path() {
    let patterns = vec!["legacy/**".to_string()];

    assert!(!is_path_excluded("/repo", "/repo", &patterns));
    assert!(is_path_excluded("/repo/legacy/a.py", "/repo", &patterns));
}

#[test]
fn multiple_patterns_are_alternatives() {
    let patterns = vec!["build/**".to_string(), "dist/*.py".to_string()];

    assert!(is_path_excluded("/repo/dist/a.py", "/repo", &patterns));
    assert!(!is_path_excluded("/repo/src/a.py", "/repo", &patterns));
}

#[test]
fn a_valid_pattern_survives_a_malformed_one() {
    let patterns = vec!["[unclosed".to_string(), "build/**".to_string()];

    assert!(is_path_excluded("/repo/build/a.py", "/repo", &patterns));
    assert!(!is_path_excluded("/repo/src/a.py", "/repo", &patterns));
}

#[test]
fn validation_accepts_no_patterns_and_real_globs() {
    assert!(invalid_exclude_patterns(&[]).is_empty());
    assert!(invalid_exclude_patterns(&["legacy/**".to_string()]).is_empty());
}

#[test]
fn validation_reports_only_the_malformed_globs() {
    let patterns = vec!["legacy/**".to_string(), "[unclosed".to_string()];

    assert_eq!(
        invalid_exclude_patterns(&patterns),
        vec!["[unclosed".to_string()]
    );
}
