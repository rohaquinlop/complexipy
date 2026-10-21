#![cfg(feature = "runner")]
//! Tests for discovery across path arguments: duplicates, exclusion, and report paths.

use std::fs;

use complexipy_core::{
    AnalysisOptions, collect_all_ignored_locations, collect_removable_ignored_locations,
    run_analysis_shared,
};
use tempfile::TempDir;

const MARKED: &str =
    "def marked(a):  # complexipy: ignore\n    if a:\n        return 1\n    return 0\n";

fn project() -> (TempDir, String) {
    let dir = TempDir::new().unwrap();
    fs::create_dir_all(dir.path().join("src/legacy")).unwrap();
    fs::create_dir_all(dir.path().join("legacy")).unwrap();
    fs::write(dir.path().join("src/new.py"), MARKED).unwrap();
    fs::write(dir.path().join("src/legacy/inner.py"), MARKED).unwrap();
    fs::write(dir.path().join("legacy/old.py"), MARKED).unwrap();
    let root = dir
        .path()
        .canonicalize()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    (dir, root)
}

fn arguments(root: &str, relative: &[&str]) -> Vec<String> {
    relative
        .iter()
        .map(|path| format!("{root}/{path}"))
        .collect()
}

fn analyzed_paths(root: &str, paths: &[String], exclude: &[String]) -> Vec<String> {
    let (files, failed) =
        run_analysis_shared(paths, exclude, &AnalysisOptions::default(), root).unwrap();
    assert!(failed.is_empty(), "{failed:?}");
    let mut reported: Vec<String> = files.into_iter().map(|file| file.path).collect();
    reported.sort();
    reported
}

#[test]
fn a_file_reached_through_several_arguments_is_analyzed_once() {
    let (_dir, root) = project();
    let paths = arguments(
        &root,
        &[".", "src", "src/new.py", "./src/new.py", "src/../src"],
    );

    assert_eq!(
        analyzed_paths(&root, &paths, &[]),
        ["legacy/old.py", "src/legacy/inner.py", "src/new.py"]
    );
}

#[test]
fn excludes_also_match_relative_to_the_invocation_directory() {
    let (_dir, root) = project();
    let paths = arguments(&root, &["src", "legacy"]);

    assert_eq!(
        analyzed_paths(&root, &paths, &["legacy/**".to_string()]),
        ["src/new.py"]
    );
}

#[test]
fn ignored_locations_use_the_same_paths_as_the_analysis() {
    let (_dir, root) = project();
    let paths = arguments(&root, &[".", "src", "legacy/old.py"]);

    let (ignored, _) = collect_all_ignored_locations(&paths, &[], &root).unwrap();
    let (removable, _) = collect_removable_ignored_locations(&paths, &[], 15, &root).unwrap();

    let ignored: Vec<&str> = ignored
        .iter()
        .map(|location| location.path.as_str())
        .collect();
    let removable: Vec<&str> = removable
        .iter()
        .map(|location| location.path.as_str())
        .collect();
    assert_eq!(
        ignored,
        ["legacy/old.py", "src/legacy/inner.py", "src/new.py"]
    );
    assert_eq!(removable, ignored);
}
