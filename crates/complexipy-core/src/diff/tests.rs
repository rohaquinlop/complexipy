use std::fs;
use std::path::Path;
use std::process::Command;

use tempfile::tempdir;

use crate::classes::FileComplexity;
use crate::diff::{
    DiffEntry, DiffStatus, GitObject, UnresolvedFile, UnresolvedReason, read_object,
};
use crate::diff::{
    compute_diff, compute_diff_report, compute_staged_diff, has_regressions, resolve_diff_flags,
};

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
fn a_batch_response_reads_blobs_and_skips_other_objects() {
    let output = b"1a2b blob 6\nx = 1\n\nHEAD:gone.py missing\n3c4d tree 3\nabc\n\
HEAD:a name.py missing\n5e6f blob 2\n\xff\xfe\n";
    let mut reader = std::io::Cursor::new(&output[..]);

    assert_eq!(
        read_object(&mut reader),
        Some(GitObject::Text("x = 1\n".to_string()))
    );
    assert_eq!(read_object(&mut reader), Some(GitObject::Missing));
    assert_eq!(read_object(&mut reader), Some(GitObject::Unreadable));
    assert_eq!(read_object(&mut reader), Some(GitObject::Missing));
    assert_eq!(read_object(&mut reader), Some(GitObject::Missing));
    assert_eq!(read_object(&mut reader), None);
}

#[test]
fn compute_diff_reads_each_file_at_its_repository_path() {
    let dir = tempdir().expect("tempdir should work");
    init_repo(dir.path());
    let src = dir.path().join("src");
    fs::create_dir_all(src.join("pkg")).expect("should create");
    fs::write(src.join("example.py"), SIMPLE).expect("should write");
    fs::write(src.join("pkg/inner.py"), WITH_IF).expect("should write");
    fs::write(dir.path().join("inner.py"), SIMPLE).expect("should write");
    commit_all(dir.path(), "initial");

    let current = vec![
        file_complexity("example.py", "example.py", &[("simple", 2)]),
        file_complexity("pkg/inner.py", "inner.py", &[("with_if", 1)]),
        file_complexity("added.py", "added.py", &[("simple", 0)]),
    ];
    let entries = compute_diff(&current, "HEAD", src.to_str().unwrap());

    assert_eq!(entries.len(), 3);
    assert_eq!(entries[0].old_complexity, Some(0));
    assert_eq!(entries[1].func_name, "with_if");
    assert_eq!(entries[1].status(), DiffStatus::Unchanged);
    assert_eq!(entries[2].status(), DiffStatus::New);
}

#[test]
fn compute_diff_reads_the_repository_of_the_file_from_outside_it() {
    let dir = tempdir().expect("tempdir should work");
    let outside = tempdir().expect("tempdir should work");
    init_repo(dir.path());
    fs::create_dir(dir.path().join("pkg")).expect("should create");
    fs::write(dir.path().join("pkg/a.py"), WITH_IF).expect("should write");
    commit_all(dir.path(), "initial");
    let absolute = dir.path().join("pkg/a.py");

    let current = vec![file_complexity(
        absolute.to_str().unwrap(),
        "a.py",
        &[("with_if", 3), ("added", 0)],
    )];
    let report = compute_diff_report(&current, "HEAD", outside.path().to_str().unwrap());

    assert!(report.unresolved.is_empty());
    let statuses: Vec<(&str, DiffStatus)> = report
        .entries
        .iter()
        .map(|entry| (entry.func_name.as_str(), entry.status()))
        .collect();
    assert_eq!(
        statuses,
        vec![
            ("added", DiffStatus::New),
            ("with_if", DiffStatus::Regressed)
        ]
    );
}

#[test]
fn compute_diff_reports_a_file_outside_any_repository_as_unresolved() {
    let dir = tempdir().expect("tempdir should work");
    fs::write(dir.path().join("loose.py"), SIMPLE).expect("should write");

    let current = vec![file_complexity("loose.py", "loose.py", &[("simple", 0)])];
    let report = compute_diff_report(&current, "HEAD", dir.path().to_str().unwrap());

    assert_eq!(
        report.unresolved,
        vec![UnresolvedFile {
            file_path: "loose.py".to_string(),
            reason: UnresolvedReason::NotInRepository,
        }]
    );
    assert_eq!(report.entries.len(), 1);
    assert_eq!(report.entries[0].status(), DiffStatus::New);
}

#[test]
fn compute_diff_reports_a_ref_missing_from_the_repository_as_unresolved() {
    let dir = tempdir().expect("tempdir should work");
    init_repo(dir.path());
    fs::write(dir.path().join("a.py"), SIMPLE).expect("should write");
    fs::write(dir.path().join("b.py"), SIMPLE).expect("should write");
    commit_all(dir.path(), "initial");

    let current = vec![
        file_complexity("a.py", "a.py", &[("simple", 0)]),
        file_complexity("b.py", "b.py", &[("simple", 0)]),
    ];
    let report = compute_diff_report(&current, "NONEXISTENT_REF", dir.path().to_str().unwrap());

    let repository = dir.path().canonicalize().unwrap().display().to_string();
    let reasons: Vec<&UnresolvedReason> = report
        .unresolved
        .iter()
        .map(|unresolved| &unresolved.reason)
        .collect();
    assert_eq!(
        reasons,
        vec![
            &UnresolvedReason::UnknownRef {
                repository: repository.clone()
            },
            &UnresolvedReason::UnknownRef { repository }
        ]
    );
    assert!(
        report
            .entries
            .iter()
            .all(|entry| entry.status() == DiffStatus::New)
    );
}

const FLAT: &str = "def f(x):\n    return x\n";

const BRANCHED: &str = "def f(x):\n    if x:\n        return 1\n    return 0\n";

fn repository_with(dir: &Path, source: &str) {
    fs::create_dir_all(dir).expect("should create");
    init_repo(dir);
    fs::write(dir.join("a.py"), source).expect("should write");
    commit_all(dir, "commit");
}

/// An outer repository that pins `sub` at a commit where `f` has
/// complexity 0, while `sub` itself has moved on to complexity 1.
fn outer_pinning_an_older_sub() -> tempfile::TempDir {
    let outer = tempdir().expect("tempdir should work");
    let sub = outer.path().join("sub");
    repository_with(&sub, FLAT);
    init_repo(outer.path());
    commit_all(outer.path(), "pin sub");
    fs::write(sub.join("a.py"), BRANCHED).expect("should write");
    commit_all(&sub, "branch");
    outer
}

/// The status of `f`, now of complexity 1, in `repository/a.py` against
/// `HEAD`, and why the file could not be read, if it could not.
fn diff_of_a_py(repository: &Path, invocation: &Path) -> (Vec<DiffStatus>, Vec<UnresolvedReason>) {
    let path = repository.join("a.py");
    let current = vec![file_complexity(path.to_str().unwrap(), "a.py", &[("f", 1)])];
    let report = compute_diff_report(&current, "HEAD", invocation.to_str().unwrap());
    (
        report.entries.iter().map(DiffEntry::status).collect(),
        report
            .unresolved
            .into_iter()
            .map(|file| file.reason)
            .collect(),
    )
}

fn canonical(path: &Path) -> String {
    path.canonicalize().unwrap().display().to_string()
}

#[test]
fn compute_diff_reads_a_nested_repository_at_the_commit_the_outer_one_pins() {
    let outer = outer_pinning_an_older_sub();

    assert_eq!(
        diff_of_a_py(&outer.path().join("sub"), outer.path()),
        (vec![DiffStatus::Regressed], vec![])
    );
}

#[test]
fn compute_diff_reads_a_nested_repository_at_its_own_ref_when_invoked_inside_it() {
    let outer = outer_pinning_an_older_sub();
    let sub = outer.path().join("sub");

    assert_eq!(
        diff_of_a_py(&sub, &sub),
        (vec![DiffStatus::Unchanged], vec![])
    );
}

#[test]
fn compute_diff_follows_pins_through_each_level_of_nesting() {
    let outer = tempdir().expect("tempdir should work");
    let middle = outer.path().join("middle");
    let inner = middle.join("inner");
    repository_with(&inner, FLAT);
    init_repo(&middle);
    commit_all(&middle, "pin inner");
    init_repo(outer.path());
    commit_all(outer.path(), "pin middle");
    fs::write(inner.join("a.py"), BRANCHED).expect("should write");
    commit_all(&inner, "branch");
    commit_all(&middle, "advance inner");

    assert_eq!(
        diff_of_a_py(&inner, outer.path()),
        (vec![DiffStatus::Regressed], vec![])
    );
}

fn outer_without_sub() -> tempfile::TempDir {
    let outer = tempdir().expect("tempdir should work");
    init_repo(outer.path());
    fs::write(outer.path().join("README.md"), "placeholder").expect("should write");
    commit_all(outer.path(), "initial");
    repository_with(&outer.path().join("sub"), BRANCHED);
    outer
}

#[test]
fn compute_diff_reads_an_untracked_nested_repository_at_its_own_ref() {
    let outer = outer_without_sub();

    assert_eq!(
        diff_of_a_py(&outer.path().join("sub"), outer.path()),
        (vec![DiffStatus::Unchanged], vec![])
    );
}

#[test]
fn compute_diff_reports_a_submodule_the_change_adds_as_new() {
    let outer = outer_without_sub();
    let sub = outer.path().join("sub");
    git(outer.path(), &["add", "sub"]);

    assert_eq!(
        diff_of_a_py(&sub, outer.path()),
        (
            vec![DiffStatus::New],
            vec![UnresolvedReason::NotPinned {
                repository: canonical(&sub)
            }]
        )
    );
}

#[test]
fn compute_diff_reports_a_ref_the_outer_repository_lacks_against_the_outer_one() {
    let outer = tempdir().expect("tempdir should work");
    let sub = outer.path().join("sub");
    repository_with(&sub, BRANCHED);
    init_repo(outer.path());
    git(outer.path(), &["add", "sub"]);

    assert_eq!(
        diff_of_a_py(&sub, outer.path()),
        (
            vec![DiffStatus::New],
            vec![UnresolvedReason::UnknownRef {
                repository: canonical(outer.path())
            }]
        )
    );
}

#[test]
fn compute_diff_reads_a_linked_worktree_from_outside_it() {
    let dir = tempdir().expect("tempdir should work");
    let primary = dir.path().join("primary");
    let linked = dir.path().join("linked");
    repository_with(&primary, BRANCHED);
    git(
        &primary,
        &["worktree", "add", "-q", linked.to_str().unwrap()],
    );

    assert_eq!(
        diff_of_a_py(&linked, dir.path()),
        (vec![DiffStatus::Unchanged], vec![])
    );
}

#[test]
fn compute_diff_reports_a_pinned_commit_the_nested_repository_lacks_as_new() {
    let outer = tempdir().expect("tempdir should work");
    let sub = outer.path().join("sub");
    repository_with(&sub, BRANCHED);
    init_repo(outer.path());
    let missing = "1".repeat(40);
    git(
        outer.path(),
        &[
            "update-index",
            "--add",
            "--cacheinfo",
            &format!("160000,{},sub", missing),
        ],
    );
    git(
        outer.path(),
        &["commit", "-q", "-m", "pin a missing commit"],
    );

    assert_eq!(
        diff_of_a_py(&sub, outer.path()),
        (
            vec![DiffStatus::New],
            vec![UnresolvedReason::PinnedCommitMissing {
                repository: canonical(&sub),
                commit: missing,
            }]
        )
    );
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

#[test]
fn an_option_like_ref_never_reaches_git_show_as_an_option() {
    let dir = tempdir().unwrap();
    init_repo(dir.path());
    fs::write(dir.path().join("mod.py"), SIMPLE).unwrap();
    commit_all(dir.path(), "initial");
    let target = dir.path().join("written");
    let git_ref = format!("--output={}", target.display());

    let entries = compute_diff(
        &[file_complexity("mod.py", "mod.py", &[("simple", 0)])],
        &git_ref,
        dir.path().to_str().unwrap(),
    );

    assert!(!dir.path().join("written:mod.py").exists());
    assert!(!target.exists());
    assert_eq!(entries[0].status(), DiffStatus::New);
}

#[test]
fn an_option_like_ref_never_reaches_git_diff_as_an_option() {
    let dir = tempdir().unwrap();
    init_repo(dir.path());
    fs::write(dir.path().join("mod.py"), SIMPLE).unwrap();
    commit_all(dir.path(), "initial");
    let victim = dir.path().join("victim.txt");
    fs::write(&victim, "keep me").unwrap();
    let git_ref = format!("--output={}", victim.display());

    let entries = compute_staged_diff(&git_ref, dir.path().to_str().unwrap());

    assert_eq!(entries, Some(Vec::new()));
    assert_eq!(fs::read_to_string(&victim).unwrap(), "keep me");
}

const PROPERTY_PAIR: &str = "class Svc:\n    @property\n    def value(self):\n        for x in self.a:\n            if x:\n                if self.b:\n                    return x\n        return 0\n\n    @value.setter\n    def value(self, v):\n        self.v = v\n";

#[test]
fn a_property_setter_cannot_hide_its_getter_in_a_staged_diff() {
    let dir = tempdir().unwrap();
    init_repo(dir.path());
    fs::write(dir.path().join("a.py"), SIMPLE).unwrap();
    commit_all(dir.path(), "initial");
    fs::write(dir.path().join("svc.py"), PROPERTY_PAIR).unwrap();
    git(dir.path(), &["add", "svc.py"]);

    let entries = compute_staged_diff("HEAD", dir.path().to_str().unwrap()).unwrap();

    let mut keys: Vec<(&str, Option<u64>)> = entries
        .iter()
        .map(|entry| (entry.func_name.as_str(), entry.new_complexity))
        .collect();
    keys.sort();
    assert_eq!(
        keys,
        vec![("Svc::value", Some(6)), ("Svc::value#2", Some(0))]
    );
    assert!(has_regressions(&entries, 3));
}

#[test]
fn function_keys_number_repeated_names_in_source_order() {
    use crate::classes::FunctionComplexity;
    use crate::diff::function_keys;

    let function = |name: &str, line_start: u64| FunctionComplexity {
        name: name.to_string(),
        complexity: 0,
        line_start,
        line_end: line_start,
        line_complexities: vec![],
        refactor_plans: vec![],
        additional_refactor_plans: 0,
    };
    let functions = [function("f", 9), function("g", 5), function("f", 1)];

    assert_eq!(function_keys(&functions), vec!["f#2", "g", "f"]);
}

#[cfg(unix)]
const NESTED: &str = "def nested(a, b, c):\n    if a:\n        if b:\n            if c:\n                return 1\n    return 0\n";

#[cfg(unix)]
fn link_at_head_now_a_regular_file(dir: &Path) {
    init_repo(dir);
    std::os::unix::fs::symlink(NESTED, dir.join("x.py")).unwrap();
    commit_all(dir, "link");
    fs::remove_file(dir.join("x.py")).unwrap();
    fs::write(dir.join("x.py"), NESTED).unwrap();
}

#[cfg(unix)]
#[test]
fn a_link_at_the_ref_is_never_read_as_python() {
    let dir = tempdir().unwrap();
    link_at_head_now_a_regular_file(dir.path());

    let entries = compute_diff(
        &[file_complexity("x.py", "x.py", &[("nested", 6)])],
        "HEAD",
        dir.path().to_str().unwrap(),
    );

    assert_eq!(entries, vec![entry("x.py", "nested", None, Some(6))]);
    assert!(has_regressions(&entries, 3));
}

#[cfg(unix)]
#[test]
fn a_staged_link_made_a_regular_file_is_compared_as_added() {
    let dir = tempdir().unwrap();
    link_at_head_now_a_regular_file(dir.path());
    git(dir.path(), &["add", "x.py"]);

    let entries = compute_staged_diff("HEAD", dir.path().to_str().unwrap()).unwrap();

    assert_eq!(entries, vec![entry("x.py", "nested", None, Some(6))]);
    assert!(has_regressions(&entries, 3));
}

#[cfg(unix)]
#[test]
fn a_path_ending_in_a_carriage_return_is_not_read_as_another_path() {
    let dir = tempdir().unwrap();
    init_repo(dir.path());
    fs::write(dir.path().join("x.py"), NESTED).unwrap();
    fs::write(dir.path().join("x.py\r"), SIMPLE).unwrap();
    commit_all(dir.path(), "initial");
    fs::write(dir.path().join("x.py\r"), NESTED).unwrap();

    let entries = compute_diff(
        &[file_complexity("x.py\r", "x.py\r", &[("nested", 6)])],
        "HEAD",
        dir.path().to_str().unwrap(),
    );

    assert_eq!(entries, vec![entry("x.py\r", "nested", None, Some(6))]);
    assert!(has_regressions(&entries, 3));
}

#[cfg(unix)]
#[test]
fn compute_staged_diff_reads_a_staged_name_that_starts_like_a_stage_number() {
    let dir = tempdir().unwrap();
    init_repo(dir.path());
    fs::write(dir.path().join("new.py"), SIMPLE).unwrap();
    commit_all(dir.path(), "initial");
    fs::write(dir.path().join("0:new.py"), WITH_IF).unwrap();
    git(dir.path(), &["add", "--", "0:new.py"]);

    let entries = compute_staged_diff("HEAD", dir.path().to_str().unwrap()).unwrap();

    assert_eq!(entries, vec![entry("0:new.py", "with_if", None, Some(1))]);
}
