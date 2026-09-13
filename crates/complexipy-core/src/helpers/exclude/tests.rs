use std::fs;

use tempfile::TempDir;

use super::*;

const FILES: [&str; 3] = ["legacy/old.py", "src/new.py", "mix.py"];
const PATTERNS: [&str; 4] = ["legacy/**", "**/legacy/**", "legacy", "src/*.py"];

fn tree() -> TempDir {
    let dir = TempDir::new().unwrap();

    fs::create_dir_all(dir.path().join("legacy")).unwrap();
    fs::create_dir_all(dir.path().join("src")).unwrap();
    fs::write(
        dir.path().join("legacy/old.py"),
        "def old():\n    return 1\n",
    )
    .unwrap();
    fs::write(dir.path().join("src/new.py"), "def new():\n    return 1\n").unwrap();
    fs::write(dir.path().join("mix.py"), "def mix():\n    return 1\n").unwrap();
    dir
}

fn canonical_root(dir: &TempDir) -> String {
    dir.path()
        .canonicalize()
        .unwrap()
        .to_string_lossy()
        .replace('\\', "/")
}

fn agreement(root: &str, patterns: &[String]) {
    let mut kept = get_paths_to_process(root, patterns.to_vec()).unwrap();
    kept.sort();

    for file in FILES {
        let path = format!("{}/{}", root, file);
        let expected = !is_path_excluded(&path, root, patterns);

        assert_eq!(
            kept.contains(&path),
            expected,
            "the walker and the matcher disagreed on {} for {:?}",
            file,
            patterns
        );
    }
}

#[test]
fn both_matchers_agree_on_relative_patterns() {
    let dir = tree();
    let root = canonical_root(&dir);

    for pattern in PATTERNS {
        agreement(&root, &[pattern.to_string()]);
    }
}

#[test]
fn both_matchers_agree_on_an_absolute_pattern() {
    let dir = tree();
    let root = canonical_root(&dir);
    let patterns = vec![format!("{}/legacy/**", root)];

    agreement(&root, &patterns);
}

#[test]
fn both_matchers_reject_an_invalid_pattern() {
    let dir = tree();
    let root = canonical_root(&dir);
    let patterns = vec!["[unclosed".to_string()];

    assert!(get_paths_to_process(&root, patterns.clone()).is_err());
    assert!(!is_path_excluded(
        &format!("{}/legacy/old.py", root),
        &root,
        &patterns
    ));
}

#[test]
fn a_sibling_directory_is_not_inside_the_root() {
    let dir = tree();
    let root = canonical_root(&dir);
    let sibling = format!("{}x/legacy/old.py", root);

    assert!(!is_path_excluded(
        &sibling,
        &root,
        &["legacy/**".to_string()]
    ));
    assert!(is_path_excluded(
        &format!("{}/legacy/old.py", root),
        &root,
        &["legacy/**".to_string()]
    ));
}
