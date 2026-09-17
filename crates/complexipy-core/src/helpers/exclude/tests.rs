use std::fs;

use tempfile::TempDir;

use super::*;

const FILES: [&str; 3] = ["legacy/old.py", "src/new.py", "mix.py"];
const PATTERNS: [&str; 4] = ["legacy/**", "**/legacy/**", "legacy", "src/*.py"];
const VALIDITY_CORPUS: [&str; 16] = [
    "legacy/**",
    "**/legacy/**",
    "legacy",
    "src/*.py",
    "**",
    "**/*",
    "./legacy/**",
    "a/**b",
    "/abs/legacy/**",
    "mix{**/.py,z}",
    "src\\*.py",
    "a\\[b",
    "",
    "[unclosed",
    "**.[",
    "src/**/[",
];

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
fn both_matchers_agree_on_a_pattern_list() {
    let dir = tree();
    let root = canonical_root(&dir);
    let patterns = vec!["src/*.py".to_string(), "**/legacy/**".to_string()];

    agreement(&root, &patterns);
}

#[test]
fn both_matchers_agree_on_a_brace_group() {
    let dir = tree();
    let root = canonical_root(&dir);
    let patterns = vec!["mix{**/.py,z}".to_string()];

    agreement(&root, &patterns);
}

#[test]
fn both_matchers_agree_on_a_list_holding_a_brace_group() {
    let dir = tree();
    let root = canonical_root(&dir);
    let patterns = vec!["mix{**/.py,z}".to_string(), "src/*.py".to_string()];

    agreement(&root, &patterns);
}

#[test]
fn both_matchers_agree_on_a_pattern_with_separators() {
    let dir = tree();
    let root = canonical_root(&dir);

    for pattern in ["src\\*.py", "legacy\\**", "legacy\\old.py"] {
        agreement(&root, &[pattern.to_string()]);
    }
}

#[test]
fn a_pattern_is_reported_when_the_walker_refuses_it() {
    let dir = tree();
    let root = canonical_root(&dir);

    for pattern in VALIDITY_CORPUS {
        let patterns = vec![pattern.to_string()];
        let reported = invalid_exclude_patterns(&patterns);
        let refused = get_paths_to_process(&root, patterns).is_err();

        assert_eq!(
            reported.is_empty(),
            !refused,
            "the report and the walker disagreed on {:?}",
            pattern
        );
    }
}

#[test]
fn an_invalid_pattern_does_not_disable_the_valid_ones() {
    let dir = tree();
    let root = canonical_root(&dir);
    let patterns = vec!["[unclosed".to_string(), "legacy/**".to_string()];

    assert_eq!(
        invalid_exclude_patterns(&patterns),
        vec!["[unclosed".to_string()]
    );
    assert!(is_path_excluded(
        &format!("{}/legacy/old.py", root),
        &root,
        &patterns
    ));
    assert!(!is_path_excluded(
        &format!("{}/mix.py", root),
        &root,
        &patterns
    ));
    assert!(!is_path_excluded(
        &format!("{}/src/new.py", root),
        &root,
        &patterns
    ));
}

#[test]
fn a_normal_list_does_not_overflow() {
    assert!(!exclude_list_overflows(&[]));
    assert!(!exclude_list_overflows(&["legacy/**".to_string()]));
    assert!(!exclude_list_overflows(&[
        "legacy/**".to_string(),
        "src/*.py".to_string(),
    ]));
}

#[test]
fn a_malformed_list_is_not_reported_as_overflowing() {
    let patterns = vec!["[unclosed".to_string(), "legacy/**".to_string()];

    assert!(!exclude_list_overflows(&patterns));
}

#[test]
fn a_list_too_large_for_one_program_is_reported_as_overflowing() {
    let patterns: Vec<String> = (0..5_000)
        .map(|index| format!("**/dir{index}/**/*.py"))
        .collect();

    assert!(invalid_exclude_patterns(&patterns).is_empty());
    assert!(exclude_list_overflows(&patterns));
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
