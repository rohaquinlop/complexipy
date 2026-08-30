use std::fs;
use std::path::Path;
use std::process::Command;

use tempfile::tempdir;

use crate::classes::FileComplexity;
use crate::diff::{DiffEntry, DiffStatus};
use crate::diff::{compute_diff, compute_staged_diff, has_regressions, resolve_diff_flags};

const SIMPLE: &str = "def simple(x):\n    return x + 1\n";

const WITH_IF: &str = "def with_if(x):\n    if x:\n        return 1\n    return 0\n";

fn file_complexity(path: &str, file_name: &str, functions: &[(&str, u64)]) -> FileComplexity {
    use crate::classes::FunctionComplexity;
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

fn git(dir: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(dir)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .expect("git should run");
    assert!(status.success(), "git {:?} failed", args);
}

fn init_repo(dir: &Path) {
    git(dir, &["init", "-q"]);
    git(dir, &["config", "user.email", "test@example.com"]);
    git(dir, &["config", "user.name", "Test"]);
}

fn commit_all(dir: &Path, message: &str) {
    git(dir, &["add", "-A"]);
    git(dir, &["commit", "-q", "-m", message]);
}

fn entry(file_path: &str, func_name: &str, old: Option<u64>, new: Option<u64>) -> DiffEntry {
    DiffEntry {
        file_path: file_path.to_string(),
        func_name: func_name.to_string(),
        old_complexity: old,
        new_complexity: new,
    }
}

#[test]
fn status_new() {
    assert_eq!(entry("a.py", "f", None, Some(5)).status(), DiffStatus::New);
}

#[test]
fn status_removed() {
    assert_eq!(
        entry("a.py", "f", Some(5), None).status(),
        DiffStatus::Removed
    );
}

#[test]
fn status_regressed() {
    assert_eq!(
        entry("a.py", "f", Some(3), Some(5)).status(),
        DiffStatus::Regressed
    );
}

#[test]
fn status_improved() {
    assert_eq!(
        entry("a.py", "f", Some(5), Some(3)).status(),
        DiffStatus::Improved
    );
}

#[test]
fn status_unchanged() {
    assert_eq!(
        entry("a.py", "f", Some(5), Some(5)).status(),
        DiffStatus::Unchanged
    );
}

#[test]
fn delta_change_is_signed() {
    assert_eq!(entry("a.py", "f", Some(3), Some(5)).delta(), Some(2));
    assert_eq!(entry("a.py", "f", Some(5), Some(3)).delta(), Some(-2));
    assert_eq!(entry("a.py", "f", Some(5), Some(5)).delta(), Some(0));
    assert_eq!(entry("a.py", "f", None, Some(5)).delta(), None);
    assert_eq!(entry("a.py", "f", Some(5), None).delta(), None);
}

#[test]
fn compute_diff_marks_new_file_functions() {
    let dir = tempdir().expect("tempdir should work");
    init_repo(dir.path());
    fs::write(dir.path().join("README.md"), "placeholder").expect("should write");
    commit_all(dir.path(), "initial");

    let current = vec![file_complexity(
        "src/example.py",
        "example.py",
        &[("simple", 2)],
    )];
    let entries = compute_diff(&current, "HEAD", dir.path().to_str().unwrap());

    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].status(), DiffStatus::New);
    assert_eq!(entries[0].old_complexity, None);
    assert_eq!(entries[0].new_complexity, Some(2));
}

#[test]
fn compute_diff_reports_regressed_and_improved() {
    let dir = tempdir().expect("tempdir should work");
    init_repo(dir.path());
    let src = dir.path().join("src");
    fs::create_dir(&src).expect("should create");
    fs::write(src.join("a.py"), WITH_IF).expect("should write");
    fs::write(src.join("b.py"), WITH_IF).expect("should write");
    commit_all(dir.path(), "initial");

    let current = vec![
        file_complexity("src/a.py", "a.py", &[("with_if", 5)]),
        file_complexity("src/b.py", "b.py", &[("with_if", 0)]),
    ];
    let entries = compute_diff(&current, "HEAD", dir.path().to_str().unwrap());

    let regressed = entries
        .iter()
        .find(|e| e.file_path == "src/a.py")
        .expect("a entry");
    assert_eq!(regressed.status(), DiffStatus::Regressed);
    assert_eq!(regressed.old_complexity, Some(1));
    assert_eq!(regressed.new_complexity, Some(5));

    let improved = entries
        .iter()
        .find(|e| e.file_path == "src/b.py")
        .expect("b entry");
    assert_eq!(improved.status(), DiffStatus::Improved);
    assert_eq!(improved.old_complexity, Some(1));
    assert_eq!(improved.new_complexity, Some(0));
}

#[test]
fn compute_diff_reports_removed_function() {
    let dir = tempdir().expect("tempdir should work");
    init_repo(dir.path());
    let src = dir.path().join("src");
    fs::create_dir(&src).expect("should create");
    fs::write(
        src.join("a.py"),
        "def keep(x):\n    return x\ndef gone(x):\n    return x\n",
    )
    .expect("should write");
    commit_all(dir.path(), "initial");

    let current = vec![file_complexity("src/a.py", "a.py", &[("keep", 0)])];
    let entries = compute_diff(&current, "HEAD", dir.path().to_str().unwrap());

    let removed = entries
        .iter()
        .find(|e| e.func_name == "gone")
        .expect("gone entry");
    assert_eq!(removed.status(), DiffStatus::Removed);
    assert_eq!(removed.old_complexity, Some(0));
    assert_eq!(removed.new_complexity, None);

    let kept = entries
        .iter()
        .find(|e| e.func_name == "keep")
        .expect("keep entry");
    assert_eq!(kept.status(), DiffStatus::Unchanged);
}

#[test]
fn compute_diff_git_error_skips_file() {
    let dir = tempdir().expect("tempdir should work");
    init_repo(dir.path());
    fs::write(dir.path().join("a.py"), SIMPLE).expect("should write");
    commit_all(dir.path(), "initial");

    let current = vec![file_complexity("a.py", "a.py", &[("simple", 2)])];
    let entries = compute_diff(&current, "NONEXISTENT_REF", dir.path().to_str().unwrap());

    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].status(), DiffStatus::New);
}

#[test]
fn compute_diff_unparseable_old_content_skips_file() {
    let dir = tempdir().expect("tempdir should work");
    init_repo(dir.path());
    fs::write(dir.path().join("broken.py"), "def broken(:\n").expect("should write");
    commit_all(dir.path(), "initial");

    let current = vec![file_complexity("broken.py", "broken.py", &[("broken", 1)])];
    let entries = compute_diff(&current, "HEAD", dir.path().to_str().unwrap());

    assert!(entries.is_empty());
}

#[test]
fn resolve_git_path_strips_leading_components() {
    let dir = tempdir().expect("tempdir should work");
    init_repo(dir.path());
    let src = dir.path().join("src");
    fs::create_dir(&src).expect("should create");
    fs::write(src.join("example.py"), SIMPLE).expect("should write");
    commit_all(dir.path(), "initial");

    let current = vec![file_complexity(
        "proj/src/example.py",
        "example.py",
        &[("simple", 2)],
    )];
    let entries = compute_diff(&current, "HEAD", dir.path().to_str().unwrap());

    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].old_complexity, Some(0));
}

#[test]
fn compute_staged_diff_returns_none_outside_repo() {
    let dir = tempdir().expect("tempdir should work");

    let result = compute_staged_diff("HEAD", dir.path().to_str().unwrap());

    assert_eq!(result, None);
}

#[test]
fn compute_staged_diff_reports_new_staged_file() {
    let dir = tempdir().expect("tempdir should work");
    init_repo(dir.path());
    fs::write(dir.path().join("existing.py"), SIMPLE).expect("should write");
    commit_all(dir.path(), "initial");

    fs::write(dir.path().join("new.py"), WITH_IF).expect("should write");
    git(dir.path(), &["add", "new.py"]);

    let entries = compute_staged_diff("HEAD", dir.path().to_str().unwrap()).expect("should diff");
    let new_entry = entries
        .iter()
        .find(|e| e.file_path == "new.py")
        .expect("new entry");
    assert_eq!(new_entry.status(), DiffStatus::New);
    assert_eq!(new_entry.new_complexity, Some(1));
}

#[test]
fn compute_staged_diff_reports_modified_file() {
    let dir = tempdir().expect("tempdir should work");
    init_repo(dir.path());
    fs::write(dir.path().join("a.py"), SIMPLE).expect("should write");
    commit_all(dir.path(), "initial");

    fs::write(
        dir.path().join("a.py"),
        "def simple(x):\n    if x:\n        return 1\n    return 0\n",
    )
    .expect("should write");
    git(dir.path(), &["add", "a.py"]);

    let entries = compute_staged_diff("HEAD", dir.path().to_str().unwrap()).expect("should diff");
    let entry = entries
        .iter()
        .find(|e| e.file_path == "a.py")
        .expect("entry");
    assert_eq!(entry.old_complexity, Some(0));
    assert_eq!(entry.new_complexity, Some(1));
    assert_eq!(entry.status(), DiffStatus::Regressed);
}

#[test]
fn compute_staged_diff_reports_deleted_file() {
    let dir = tempdir().expect("tempdir should work");
    init_repo(dir.path());
    fs::write(dir.path().join("gone.py"), WITH_IF).expect("should write");
    commit_all(dir.path(), "initial");

    fs::remove_file(dir.path().join("gone.py")).expect("should remove");
    git(dir.path(), &["add", "-A"]);

    let entries = compute_staged_diff("HEAD", dir.path().to_str().unwrap()).expect("should diff");
    let entry = entries
        .iter()
        .find(|e| e.file_path == "gone.py")
        .expect("entry");
    assert_eq!(entry.status(), DiffStatus::Removed);
    assert_eq!(entry.old_complexity, Some(1));
    assert_eq!(entry.new_complexity, None);
}

#[test]
fn compute_staged_diff_no_staged_files_returns_empty() {
    let dir = tempdir().expect("tempdir should work");
    init_repo(dir.path());
    fs::write(dir.path().join("a.py"), SIMPLE).expect("should write");
    commit_all(dir.path(), "initial");

    let entries = compute_staged_diff("HEAD", dir.path().to_str().unwrap()).expect("should diff");

    assert!(entries.is_empty());
}

#[test]
fn has_regressions_ratchet() {
    assert!(has_regressions(&[entry("a.py", "f", Some(2), Some(6))], 5));
    assert!(!has_regressions(&[entry("a.py", "f", Some(2), Some(5))], 5));
    assert!(!has_regressions(&[entry("a.py", "f", Some(2), Some(4))], 5));
    assert!(has_regressions(&[entry("a.py", "f", Some(8), Some(10))], 5));
    assert!(has_regressions(&[entry("a.py", "f", None, Some(6))], 5));
    assert!(!has_regressions(&[entry("a.py", "f", None, Some(5))], 5));
    assert!(!has_regressions(&[entry("a.py", "f", Some(5), None)], 5));
}

#[test]
fn resolve_diff_flags_staged_defaults_to_head() {
    assert_eq!(
        resolve_diff_flags(None, None, true),
        (Some("HEAD".to_string()), None)
    );
    assert_eq!(
        resolve_diff_flags(Some("dev".to_string()), None, true),
        (Some("dev".to_string()), None)
    );
    assert_eq!(
        resolve_diff_flags(None, Some("dev".to_string()), true),
        (None, Some("dev".to_string()))
    );
}

#[test]
fn resolve_diff_flags_diff_only_wins() {
    assert_eq!(
        resolve_diff_flags(Some("dev".to_string()), Some("main".to_string()), false),
        (None, Some("main".to_string()))
    );
}

#[test]
fn resolve_diff_flags_plain_passthrough() {
    assert_eq!(
        resolve_diff_flags(Some("dev".to_string()), None, false),
        (Some("dev".to_string()), None)
    );
    assert_eq!(
        resolve_diff_flags(None, Some("dev".to_string()), false),
        (None, Some("dev".to_string()))
    );
}
