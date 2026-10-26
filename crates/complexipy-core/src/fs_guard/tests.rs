//! Unit tests for `crate::fs_guard`: bounded reads and root containment.

use std::fs;
use std::path::Path;

use tempfile::TempDir;

use super::{is_within, read_bounded, read_bounded_text, write_atomic};

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

#[test]
fn a_regular_file_under_the_limit_is_read() {
    let root = TempDir::new().unwrap();
    let file = root.path().join("a.py");
    fs::write(&file, "x = 1\n").unwrap();

    assert_eq!(
        read_bounded_text(&file, Some(root.path()), 1024).unwrap(),
        "x = 1\n"
    );
}

#[test]
fn a_file_over_the_limit_is_refused() {
    let root = TempDir::new().unwrap();
    let file = root.path().join("big.json");
    fs::write(&file, vec![b'x'; 2048]).unwrap();

    assert!(read_bounded(&file, None, 1024).is_err());
}

#[test]
fn a_directory_is_refused() {
    let root = TempDir::new().unwrap();
    fs::create_dir(root.path().join("complexipy.toml")).unwrap();

    assert!(read_bounded(&root.path().join("complexipy.toml"), None, 1024).is_err());
}

#[cfg(unix)]
#[test]
fn a_device_and_a_fifo_are_refused_without_blocking() {
    let root = TempDir::new().unwrap();
    std::os::unix::fs::symlink("/dev/zero", root.path().join("zero.py")).unwrap();
    let fifo = root.path().join("fifo.py");
    let status = std::process::Command::new("mkfifo")
        .arg(&fifo)
        .status()
        .unwrap();
    assert!(status.success());

    assert!(read_bounded(&root.path().join("zero.py"), None, 1024).is_err());
    assert!(read_bounded(&fifo, None, 1024).is_err());
}

#[cfg(unix)]
#[test]
fn a_symlink_out_of_the_root_is_refused_only_when_a_root_is_given() {
    let root = TempDir::new().unwrap();
    let outside = TempDir::new().unwrap();
    fs::write(outside.path().join("secret"), "token = 1\n").unwrap();
    let link = root.path().join("complexipy.toml");
    std::os::unix::fs::symlink(outside.path().join("secret"), &link).unwrap();

    assert!(read_bounded(&link, Some(root.path()), 1024).is_err());
    assert!(read_bounded(&link, None, 1024).is_ok());
}

#[test]
fn write_atomic_replaces_the_file_and_leaves_no_temporary_file() {
    let root = TempDir::new().unwrap();
    let file = root.path().join("complexipy-results.json");
    fs::write(&file, "old").unwrap();

    write_atomic(&file, b"new").unwrap();

    assert_eq!(fs::read_to_string(&file).unwrap(), "new");
    assert_eq!(fs::read_dir(root.path()).unwrap().count(), 1);
}

#[cfg(unix)]
#[test]
fn write_atomic_replaces_a_symlink_instead_of_following_it() {
    let root = TempDir::new().unwrap();
    let outside = TempDir::new().unwrap();
    let victim = outside.path().join("results.txt");
    fs::write(&victim, "keep me").unwrap();
    let link = root.path().join("complexipy-results.json");
    std::os::unix::fs::symlink(&victim, &link).unwrap();

    write_atomic(&link, b"[]").unwrap();

    assert_eq!(fs::read_to_string(&victim).unwrap(), "keep me");
    assert!(fs::symlink_metadata(&link).unwrap().is_file());
    assert_eq!(fs::read_to_string(&link).unwrap(), "[]");
}

#[cfg(unix)]
#[test]
fn a_failed_write_leaves_the_old_file_whole() {
    use std::os::unix::fs::PermissionsExt;

    let root = TempDir::new().unwrap();
    let dir = root.path().join("locked");
    fs::create_dir(&dir).unwrap();
    let file = dir.join("complexipy-snapshot.json");
    fs::write(&file, "[]").unwrap();
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o555)).unwrap();

    let result = write_atomic(&file, b"[{}]");
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o755)).unwrap();

    assert!(result.is_err());
    assert_eq!(fs::read_to_string(&file).unwrap(), "[]");
}
