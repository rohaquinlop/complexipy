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
    let mut kept = get_paths_to_process(root, patterns.to_vec(), Path::new(root), false)
        .unwrap()
        .files;
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
        let refused = get_paths_to_process(&root, patterns, Path::new(&root), false).is_err();

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

#[cfg(unix)]
#[test]
fn discovery_skips_links_that_leave_the_root_and_entries_that_are_not_files() {
    use std::os::unix::fs::symlink;

    let dir = tree();
    let outside = TempDir::new().unwrap();
    fs::write(outside.path().join("y.py"), "def secret():\n    return 1\n").unwrap();
    symlink(outside.path().join("y.py"), dir.path().join("x.py")).unwrap();
    symlink("/dev/zero", dir.path().join("zero.py")).unwrap();
    symlink(
        dir.path().join("missing.py"),
        dir.path().join("dangling.py"),
    )
    .unwrap();
    fs::create_dir(dir.path().join("pkg.py")).unwrap();
    symlink(dir.path().join("mix.py"), dir.path().join("alias.py")).unwrap();
    let root = canonical_root(&dir);

    let discovery = get_paths_to_process(&root, Vec::new(), Path::new(&root), false).unwrap();
    let mut skipped: Vec<(String, SkipReason)> = discovery
        .skipped
        .into_iter()
        .map(|skipped| {
            let name = skipped.path.rsplit('/').next().unwrap().to_string();
            (name, skipped.reason)
        })
        .collect();
    skipped.sort_by(|a, b| a.0.cmp(&b.0));

    assert_eq!(
        skipped,
        vec![
            ("dangling.py".to_string(), SkipReason::NotRegularFile),
            ("pkg.py".to_string(), SkipReason::NotRegularFile),
            ("x.py".to_string(), SkipReason::OutsideRoot),
            ("zero.py".to_string(), SkipReason::OutsideRoot),
        ]
    );
    assert!(discovery.files.iter().all(|file| file.starts_with(&root)));
    assert!(discovery.files.contains(&format!("{}/mix.py", root)));
}

#[test]
fn files_hidden_by_ignore_rules_are_reported_as_skipped() {
    let dir = tree();
    fs::write(dir.path().join(".ignore"), "legacy/\n").unwrap();
    fs::create_dir(dir.path().join(".impl")).unwrap();
    fs::write(
        dir.path().join(".impl/hidden.py"),
        "def h():\n    return 1\n",
    )
    .unwrap();
    let root = canonical_root(&dir);

    let discovery = get_paths_to_process(&root, Vec::new(), Path::new(&root), true).unwrap();
    let mut hidden: Vec<String> = discovery
        .skipped
        .iter()
        .filter(|skipped| skipped.reason == SkipReason::IgnoreRule)
        .map(|skipped| skipped.path.trim_start_matches(&root).to_string())
        .collect();
    hidden.sort();

    assert_eq!(hidden, vec!["/.impl/hidden.py", "/legacy/old.py"]);
    assert_eq!(discovery.files.len(), 2);
}

#[cfg(target_os = "linux")]
#[test]
fn a_file_name_that_is_not_utf8_is_selected_and_fails_its_read() {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;

    let dir = tree();
    fs::write(
        dir.path().join(OsStr::from_bytes(b"bad\xff.py")),
        "def evil():\n    return 1\n",
    )
    .unwrap();
    let root = canonical_root(&dir);

    let discovery = get_paths_to_process(&root, Vec::new(), Path::new(&root), false).unwrap();

    assert_eq!(discovery.files.len(), 4);
    let lossy = discovery
        .files
        .iter()
        .find(|file| file.contains('\u{fffd}'))
        .expect("the non-UTF-8 file is selected");
    assert!(fs::read(lossy).is_err());
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

#[test]
fn a_walk_below_the_anchor_also_matches_relative_to_the_anchor() {
    let dir = tree();
    let root = canonical_root(&dir);
    fs::create_dir_all(dir.path().join("src/legacy")).unwrap();
    fs::write(
        dir.path().join("src/legacy/inner.py"),
        "def inner():\n    return 1\n",
    )
    .unwrap();
    let files = ["legacy/old.py", "src/new.py", "src/legacy/inner.py"];

    for pattern in ["legacy/**", "src/legacy/**", "src/*.py", "**/legacy/**"] {
        let patterns = vec![pattern.to_string()];
        let mut kept = Vec::new();
        for walked in ["legacy", "src"] {
            kept.extend(
                get_paths_to_process(
                    &format!("{root}/{walked}"),
                    patterns.clone(),
                    Path::new(&root),
                    false,
                )
                .unwrap()
                .files,
            );
        }

        for file in files {
            let path = format!("{root}/{file}");
            let walked = format!("{root}/{}", file.split('/').next().unwrap());
            let matcher = ExcludeMatcher::new(&patterns);
            let expected =
                !(matcher.is_excluded(&path, &walked) || matcher.is_excluded_inside(&path, &root));

            assert_eq!(
                kept.contains(&path),
                expected,
                "the walker and the matcher disagreed on {} for {:?}",
                file,
                pattern
            );
        }
    }
}

#[test]
fn a_configuration_listing_two_paths_excludes_by_its_own_directory() {
    let dir = tree();
    let root = canonical_root(&dir);
    fs::create_dir_all(dir.path().join("src/legacy")).unwrap();
    fs::write(
        dir.path().join("src/legacy/inner.py"),
        "def inner():\n    return 1\n",
    )
    .unwrap();
    let patterns = vec!["legacy/**".to_string()];

    let kept = get_paths_to_process(&format!("{root}/legacy"), patterns, Path::new(&root), false)
        .unwrap()
        .files;

    assert!(kept.is_empty());
}

#[test]
fn a_path_outside_the_root_is_never_excluded_inside_it() {
    let matcher = ExcludeMatcher::new(&["**".to_string()]);

    assert!(!matcher.is_excluded_inside("/elsewhere/a.py", "/repo"));
    assert!(matcher.is_excluded_inside("/repo/a.py", "/repo"));
}

#[test]
fn patterns_that_can_never_match_are_reported() {
    let patterns: Vec<String> = [
        "legacy/",
        "./legacy/**",
        "/legacy/**",
        "../repo/legacy/**",
        "C:/repo/legacy/**",
        ".\\legacy\\**",
        "legacy/**",
        "**/legacy",
        ".hidden/**",
    ]
    .into_iter()
    .map(String::from)
    .collect();

    assert_eq!(
        unmatchable_exclude_patterns(&patterns),
        patterns[..6].to_vec()
    );
}

#[test]
fn the_walker_lists_every_file_once_in_sorted_order() {
    let dir = tree();
    let root = canonical_root(&dir);
    let kept = get_paths_to_process(&root, Vec::new(), Path::new(&root), false)
        .unwrap()
        .files;
    let mut sorted = kept.clone();
    sorted.sort();
    sorted.dedup();

    assert_eq!(kept, sorted);
    assert_eq!(kept.len(), FILES.len());
}

#[test]
fn an_exhaustive_pattern_excludes_the_whole_directory() {
    let dir = tree();
    fs::create_dir_all(dir.path().join("legacy/deep/deeper")).unwrap();
    fs::write(dir.path().join("legacy/deep/deeper/x.py"), "x = 1\n").unwrap();
    let root = canonical_root(&dir);
    let kept = get_paths_to_process(
        &root,
        vec!["legacy/**".to_string()],
        Path::new(&root),
        false,
    )
    .unwrap()
    .files;

    assert!(kept.iter().all(|path| !path.contains("/legacy/")));
    assert_eq!(kept.len(), 2);
}
