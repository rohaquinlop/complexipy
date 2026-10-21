//! Unit tests for `crate::fs_guard`: root containment of config-derived paths.

use std::path::Path;

use tempfile::TempDir;

use super::is_within;

#[test]
fn relative_paths_inside_the_root_are_within() {
    let root = TempDir::new().unwrap();

    for candidate in [
        ".",
        "src",
        "reports/out.json",
        "./a/../b",
        "missing/deep/file",
    ] {
        assert!(is_within(root.path(), Path::new(candidate)), "{candidate}");
    }
}

#[test]
fn parent_escapes_and_outside_absolute_paths_are_not_within() {
    let root = TempDir::new().unwrap();
    let outside = TempDir::new().unwrap();

    assert!(!is_within(root.path(), Path::new("..")));
    assert!(!is_within(root.path(), Path::new("../sibling/out.json")));
    assert!(!is_within(root.path(), Path::new("a/../../b")));
    assert!(!is_within(root.path(), outside.path()));
}

#[test]
fn an_absolute_path_inside_the_root_is_within() {
    let root = TempDir::new().unwrap();

    assert!(is_within(root.path(), &root.path().join("reports")));
}

#[cfg(unix)]
#[test]
fn a_symlinked_directory_that_leaves_the_root_is_not_within() {
    let root = TempDir::new().unwrap();
    let outside = TempDir::new().unwrap();
    std::os::unix::fs::symlink(outside.path(), root.path().join("reports")).unwrap();

    assert!(!is_within(root.path(), Path::new("reports/out.json")));
    assert!(!is_within(root.path(), Path::new("reports")));
}

#[cfg(unix)]
#[test]
fn a_dangling_symlink_is_not_within() {
    let root = TempDir::new().unwrap();
    std::os::unix::fs::symlink("/nonexistent/complexipy", root.path().join("out.json")).unwrap();

    assert!(!is_within(root.path(), Path::new("out.json")));
}

#[cfg(unix)]
#[test]
fn a_symlink_that_stays_inside_the_root_is_within() {
    let root = TempDir::new().unwrap();
    std::fs::create_dir(root.path().join("real")).unwrap();
    std::os::unix::fs::symlink(root.path().join("real"), root.path().join("alias")).unwrap();

    assert!(is_within(root.path(), Path::new("alias/out.json")));
}
