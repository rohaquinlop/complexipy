use std::fs;
use std::path::Path;
use std::process::Command;

use tempfile::tempdir;

use crate::classes::FileComplexity;
use crate::diff::{
    Change, DiffEntry, DiffStatus, GitObject, UnresolvedFile, UnresolvedReason, Verdict, parse_raw,
    read_object,
};
use crate::diff::{
    compute_diff, compute_diff_report, compute_staged_diff, gate_verdicts, has_regressions,
    resolve_diff_flags,
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

/// The staged diff of a run that selects `paths`, relative to `dir`, and
/// covers no other file.
fn staged(dir: &Path, paths: &[&str]) -> Vec<DiffEntry> {
    let current: Vec<FileComplexity> = paths
        .iter()
        .map(|path| file_complexity(path, path, &[]))
        .collect();
    let invocation = dir.canonicalize().expect("should resolve");
    let selected: Vec<std::path::PathBuf> =
        paths.iter().map(|path| invocation.join(path)).collect();
    staged_entries(&current, dir, &|path| {
        selected.iter().any(|file| file == path)
    })
}

/// The entries of the staged diff of `current`, run from `dir`, without a
/// gate.
fn staged_entries(
    current: &[FileComplexity],
    dir: &Path,
    covers_removed: &dyn Fn(&Path) -> bool,
) -> Vec<DiffEntry> {
    compute_staged_diff(current, "HEAD", dir.to_str().unwrap(), covers_removed, None)
        .expect("should diff")
        .entries
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
    fs::write(src.join("a.py"), "def keep(x):\n    return x\n").expect("should write");

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
fn compute_diff_reports_the_functions_of_a_file_unparseable_at_the_ref_as_new() {
    let dir = tempdir().expect("tempdir should work");
    init_repo(dir.path());
    fs::write(dir.path().join("broken.py"), "def broken(:\n").expect("should write");
    commit_all(dir.path(), "initial");

    let current = vec![file_complexity("broken.py", "broken.py", &[("broken", 1)])];
    let entries = compute_diff(&current, "HEAD", dir.path().to_str().unwrap());

    assert_eq!(entries, vec![entry("broken.py", "broken", None, Some(1))]);
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
    let report = compute_diff_report(
        &current,
        "HEAD",
        outside.path().to_str().unwrap(),
        &|_| false,
        None,
    );

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
    let report = compute_diff_report(
        &current,
        "HEAD",
        dir.path().to_str().unwrap(),
        &|_| false,
        None,
    );

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
    let report = compute_diff_report(
        &current,
        "NONEXISTENT_REF",
        dir.path().to_str().unwrap(),
        &|_| false,
        None,
    );

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
    let report = compute_diff_report(
        &current,
        "HEAD",
        invocation.to_str().unwrap(),
        &|_| false,
        None,
    );
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

    let result = compute_staged_diff(&[], "HEAD", dir.path().to_str().unwrap(), &|_| true, None);

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

    let entries = staged(dir.path(), &["existing.py", "new.py"]);
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

    let entries = staged(dir.path(), &["a.py"]);
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

    let entries = staged_entries(&[], dir.path(), &|_| true);
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

    let entries = staged(dir.path(), &["a.py"]);

    assert!(entries.is_empty());
}

#[test]
fn compute_staged_diff_compares_only_the_selected_files() {
    let dir = tempdir().expect("tempdir should work");
    init_repo(dir.path());
    fs::write(dir.path().join("README.md"), "placeholder").expect("should write");
    commit_all(dir.path(), "initial");
    fs::write(dir.path().join("one.py"), SIMPLE).expect("should write");
    fs::write(dir.path().join("two.py"), WITH_IF).expect("should write");
    git(dir.path(), &["add", "one.py", "two.py"]);

    let entries = staged(dir.path(), &["one.py"]);

    assert_eq!(entries, vec![entry("one.py", "simple", None, Some(0))]);
}

#[test]
fn compute_staged_diff_skips_a_staged_deletion_the_run_does_not_cover() {
    let dir = tempdir().expect("tempdir should work");
    init_repo(dir.path());
    fs::write(dir.path().join("kept.py"), SIMPLE).expect("should write");
    fs::write(dir.path().join("gone.py"), WITH_IF).expect("should write");
    commit_all(dir.path(), "initial");
    git(dir.path(), &["rm", "-q", "gone.py"]);
    let gone = dir.path().canonicalize().unwrap().join("gone.py");

    let covered = staged_entries(&[], dir.path(), &|path| path == gone);
    let uncovered = staged(dir.path(), &["kept.py"]);

    assert_eq!(covered, vec![entry("gone.py", "with_if", Some(1), None)]);
    assert!(uncovered.is_empty());
}

#[test]
fn compute_staged_diff_names_files_by_the_path_the_run_reports() {
    let dir = tempdir().expect("tempdir should work");
    init_repo(dir.path());
    fs::create_dir(dir.path().join("pkg")).expect("should create");
    fs::write(dir.path().join("pkg/mod.py"), SIMPLE).expect("should write");
    commit_all(dir.path(), "initial");
    fs::write(dir.path().join("pkg/mod.py"), WITH_IF).expect("should write");
    fs::write(dir.path().join("top.py"), WITH_IF).expect("should write");
    git(dir.path(), &["add", "-A"]);

    let entries = staged(&dir.path().join("pkg"), &["mod.py"]);

    assert_eq!(
        entries,
        vec![
            entry("mod.py", "simple", Some(0), None),
            entry("mod.py", "with_if", None, Some(1)),
        ]
    );
}

#[test]
fn compute_diff_compares_a_renamed_file_with_its_old_path() {
    let dir = tempdir().expect("tempdir should work");
    init_repo(dir.path());
    fs::write(dir.path().join("old.py"), WITH_IF).expect("should write");
    commit_all(dir.path(), "initial");
    git(dir.path(), &["mv", "old.py", "new.py"]);

    let current = vec![file_complexity("new.py", "new.py", &[("with_if", 1)])];
    let report = compute_diff_report(
        &current,
        "HEAD",
        dir.path().to_str().unwrap(),
        &|_| true,
        None,
    );

    assert_eq!(
        report.entries,
        vec![entry("new.py", "with_if", Some(1), Some(1))]
    );
}

#[test]
fn compute_diff_reports_a_deleted_file_only_when_the_run_covers_it() {
    let dir = tempdir().expect("tempdir should work");
    init_repo(dir.path());
    fs::write(dir.path().join("kept.py"), SIMPLE).expect("should write");
    fs::write(dir.path().join("gone.py"), WITH_IF).expect("should write");
    commit_all(dir.path(), "initial");
    fs::remove_file(dir.path().join("gone.py")).expect("should remove");
    let current = vec![file_complexity("kept.py", "kept.py", &[("simple", 0)])];
    let invocation = dir.path().to_str().unwrap();

    let covered = compute_diff_report(&current, "HEAD", invocation, &|_| true, None);
    let uncovered = compute_diff_report(&current, "HEAD", invocation, &|_| false, None);

    assert_eq!(
        covered.entries,
        vec![
            entry("kept.py", "simple", Some(0), Some(0)),
            entry("gone.py", "with_if", Some(1), None),
        ]
    );
    assert_eq!(
        uncovered.entries,
        vec![entry("kept.py", "simple", Some(0), Some(0))]
    );
}

#[test]
fn compute_staged_diff_compares_a_renamed_file_with_its_old_path() {
    let dir = tempdir().expect("tempdir should work");
    init_repo(dir.path());
    fs::write(dir.path().join("old.py"), WITH_IF).expect("should write");
    commit_all(dir.path(), "initial");
    git(dir.path(), &["mv", "old.py", "new.py"]);

    let current = vec![file_complexity("new.py", "new.py", &[])];
    let entries = staged_entries(&current, dir.path(), &|_| true);

    assert_eq!(entries, vec![entry("new.py", "with_if", Some(1), Some(1))]);
}

#[test]
fn compute_staged_diff_reports_nothing_for_a_file_renamed_out_of_the_run() {
    let dir = tempdir().expect("tempdir should work");
    init_repo(dir.path());
    fs::create_dir(dir.path().join("src")).expect("should create");
    fs::create_dir(dir.path().join("scripts")).expect("should create");
    fs::write(dir.path().join("src/tool.py"), WITH_IF).expect("should write");
    commit_all(dir.path(), "initial");
    git(dir.path(), &["mv", "src/tool.py", "scripts/tool.py"]);
    let src = dir.path().canonicalize().unwrap().join("src");

    let entries = staged_entries(&[], dir.path(), &|path| path.starts_with(&src));

    assert!(entries.is_empty());
}

/// A repository with `scripts/tool.py`, `src/kept.py`, and `src/moved.py`
/// committed, and the absolute `src` directory, which the tests' runs cover.
fn split_repository() -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempdir().expect("tempdir should work");
    init_repo(dir.path());
    fs::create_dir(dir.path().join("src")).expect("should create");
    fs::create_dir(dir.path().join("scripts")).expect("should create");
    fs::write(dir.path().join("scripts/tool.py"), WITH_IF).expect("should write");
    fs::write(dir.path().join("src/kept.py"), SIMPLE).expect("should write");
    fs::write(dir.path().join("src/moved.py"), WITH_IF).expect("should write");
    commit_all(dir.path(), "initial");
    let src = dir.path().canonicalize().unwrap().join("src");
    (dir, src)
}

#[test]
fn a_file_renamed_into_the_run_from_outside_it_compares_as_added() {
    let (dir, src) = split_repository();
    git(dir.path(), &["mv", "scripts/tool.py", "src/tool.py"]);
    let invocation = dir.path().to_str().unwrap();
    let covers_src = |path: &Path| path.starts_with(&src);
    let current = vec![file_complexity("src/tool.py", "tool.py", &[("with_if", 1)])];

    let report = compute_diff_report(&current, "HEAD", invocation, &covers_src, None);
    let staged = staged_entries(&current, dir.path(), &covers_src);

    let added = vec![entry("src/tool.py", "with_if", None, Some(1))];
    assert_eq!(report.entries, added);
    assert_eq!(staged, added);
}

#[test]
fn a_file_renamed_out_of_the_run_reports_nothing() {
    let (dir, src) = split_repository();
    git(dir.path(), &["mv", "src/moved.py", "scripts/moved.py"]);
    let invocation = dir.path().to_str().unwrap();
    let covers_src = |path: &Path| path.starts_with(&src);
    let current = vec![file_complexity("src/kept.py", "kept.py", &[("simple", 0)])];

    let report = compute_diff_report(&current, "HEAD", invocation, &covers_src, None);
    let staged = staged_entries(&current, dir.path(), &covers_src);

    assert_eq!(
        report.entries,
        vec![entry("src/kept.py", "simple", Some(0), Some(0))]
    );
    assert!(staged.is_empty());
}

const LEGACY: &str = "def legacy(x):\n    if x:\n        return 1\n    return 0\n";

const MARKED_WITH_FRESH: &str = "def legacy(x):  # complexipy: ignore\n    if x:\n        return 1\n    return 0\n\n\ndef fresh(x):\n    if x:\n        return 1\n    return 0\n";

#[test]
fn a_function_an_ignore_marker_hides_now_is_not_removed() {
    let dir = tempdir().expect("tempdir should work");
    init_repo(dir.path());
    fs::write(dir.path().join("legacy.py"), LEGACY).expect("should write");
    commit_all(dir.path(), "initial");
    fs::write(dir.path().join("legacy.py"), MARKED_WITH_FRESH).expect("should write");
    let invocation = dir.path().to_str().unwrap();
    let current = vec![file_complexity("legacy.py", "legacy.py", &[("fresh", 1)])];

    let report = compute_diff_report(&current, "HEAD", invocation, &|_| true, None);
    git(dir.path(), &["add", "legacy.py"]);
    let staged = staged_entries(&current, dir.path(), &|_| true);

    let added = vec![entry("legacy.py", "fresh", None, Some(1))];
    assert_eq!(report.entries, added);
    assert_eq!(staged, added);
    assert!(has_regressions(&report.entries, 0));
}

#[test]
fn raw_output_leaves_out_only_a_change_whose_path_is_not_utf8() {
    let output = b":100644 100644 a b M\0\xff.py\0:120000 100644 a b T\0x.py\0";

    assert_eq!(
        parse_raw(output),
        vec![Change {
            old: Some("x.py".to_string()),
            new: Some("x.py".to_string()),
            old_regular: false,
            new_regular: true,
        }]
    );
}

#[test]
fn raw_output_pairs_renames_reads_copies_as_added_and_keeps_modes() {
    let output = ":100644 100644 a b R100\0old.py\0new.py\0\
:100755 100644 a b M\0kept.py\0\
:000000 100644 0 b A\0added.py\0\
:120000 000000 a 0 D\0link.py\0\
:120000 100644 a b T\0retyped.py\0\
:100644 100644 a b C075\0source.py\0copy.py\0";

    let change = |old: Option<&str>, new: Option<&str>, old_regular: bool| Change {
        old: old.map(str::to_string),
        new: new.map(str::to_string),
        old_regular,
        new_regular: new.is_some(),
    };
    assert_eq!(
        parse_raw(output.as_bytes()),
        vec![
            change(Some("old.py"), Some("new.py"), true),
            change(Some("kept.py"), Some("kept.py"), true),
            change(None, Some("added.py"), false),
            change(Some("link.py"), None, false),
            change(Some("retyped.py"), Some("retyped.py"), false),
            change(None, Some("copy.py"), true),
        ]
    );
}

const BODY: &str = "for item in data:\n    if item:\n        if item > LIMIT:\n            return item\nreturn None\n";

/// A function with `header` at `indent` spaces whose body is `BODY` with
/// `limit` in it. Each limit makes a different body.
fn defined(header: &str, indent: usize, limit: u64) -> String {
    let pad = " ".repeat(indent);
    let body: String = BODY
        .replace("LIMIT", &limit.to_string())
        .lines()
        .map(|line| format!("{pad}    {line}\n"))
        .collect();
    format!("{pad}{header}\n{body}")
}

/// The file at `path` in `dir`, scored as a run scores it.
fn scored(dir: &Path, path: &str) -> FileComplexity {
    let content = fs::read_to_string(dir.join(path)).expect("should read");
    let code = crate::cognitive_complexity::code_complexity_shared(
        &content,
        &crate::rules::AnalysisOptions::default(),
    )
    .expect("should parse");
    FileComplexity {
        path: path.to_string(),
        file_name: path.to_string(),
        functions: code.functions,
        complexity: code.complexity,
    }
}

/// The gate's verdict on each entry of the diff of `paths` in `dir`
/// against `HEAD`, at a threshold of 3, as `(file, function, verdict)`,
/// with the function a passing one replaces in place of an index.
fn verdicts(dir: &Path, paths: &[&str]) -> Vec<(String, String, String)> {
    let current: Vec<FileComplexity> = paths.iter().map(|path| scored(dir, path)).collect();
    let report = compute_diff_report(&current, "HEAD", dir.to_str().unwrap(), &|_| true, Some(3));
    let verdicts = gate_verdicts(&report.entries, 3, &report.bodies);
    report
        .entries
        .iter()
        .zip(verdicts)
        .map(|(entry, verdict)| {
            let verdict = match verdict {
                Verdict::Fails => "fails".to_string(),
                Verdict::Replaces(index) => format!("replaces {}", report.entries[index].func_name),
                Verdict::Passes => "passes".to_string(),
            };
            (entry.file_path.clone(), entry.func_name.clone(), verdict)
        })
        .collect()
}

fn verdict(file: &str, function: &str, verdict: &str) -> (String, String, String) {
    (file.to_string(), function.to_string(), verdict.to_string())
}

#[test]
fn the_gate_passes_a_body_that_moves_or_is_renamed_as_it_was() {
    let dir = tempdir().expect("tempdir should work");
    init_repo(dir.path());
    let class_old = format!(
        "class Old:\n{}{}",
        defined("def run(self, data):", 4, 2),
        defined("def stop(self, data):", 4, 3)
    );
    fs::write(
        dir.path().join("a.py"),
        format!("{}{class_old}", defined("def process(data):", 0, 1)),
    )
    .expect("should write");
    commit_all(dir.path(), "initial");
    fs::write(
        dir.path().join("a.py"),
        format!(
            "{}class New:\n{}",
            defined("def handle(items):", 0, 1),
            defined("def run(self, data):", 4, 2)
        ),
    )
    .expect("should write");
    fs::write(dir.path().join("b.py"), defined("def stop(data):", 0, 3)).expect("should write");

    assert_eq!(
        verdicts(dir.path(), &["a.py", "b.py"]),
        vec![
            verdict("a.py", "New::run", "replaces Old::run"),
            verdict("a.py", "Old::run", "passes"),
            verdict("a.py", "Old::stop", "passes"),
            verdict("a.py", "handle", "replaces process"),
            verdict("a.py", "process", "passes"),
            verdict("b.py", "stop", "replaces Old::stop"),
        ]
    );
}

#[test]
fn the_gate_fails_a_copy_and_a_move_that_edits_the_body() {
    let dir = tempdir().expect("tempdir should work");
    init_repo(dir.path());
    fs::write(dir.path().join("kept.py"), defined("def kept(data):", 0, 1)).expect("should write");
    fs::write(dir.path().join("a.py"), defined("def process(data):", 0, 2)).expect("should write");
    commit_all(dir.path(), "initial");
    fs::write(dir.path().join("a.py"), "").expect("should write");
    fs::write(
        dir.path().join("b.py"),
        format!(
            "{}{}",
            defined("def copy(data):", 0, 1),
            defined("def process(data):", 0, 2).replace("return None", "return 0")
        ),
    )
    .expect("should write");

    assert_eq!(
        verdicts(dir.path(), &["kept.py", "a.py", "b.py"]),
        vec![
            verdict("kept.py", "kept", "passes"),
            verdict("a.py", "process", "passes"),
            verdict("b.py", "copy", "fails"),
            verdict("b.py", "process", "fails"),
        ]
    );
}

#[test]
fn the_gate_fails_a_copy_of_a_function_whose_key_survives() {
    let original = defined("def parse(data):", 0, 1);
    let rewritten = original.replace("return None", "return 0");
    let gate = |current: String| {
        let dir = tempdir().expect("tempdir should work");
        init_repo(dir.path());
        fs::write(dir.path().join("a.py"), &original).expect("should write");
        commit_all(dir.path(), "initial");
        fs::write(dir.path().join("a.py"), current).expect("should write");
        verdicts(dir.path(), &["a.py"])
    };

    assert_eq!(
        gate(format!(
            "{rewritten}{}",
            defined("def parse_legacy(data):", 0, 1)
        )),
        vec![
            verdict("a.py", "parse", "passes"),
            verdict("a.py", "parse_legacy", "fails"),
        ]
    );
    assert_eq!(
        gate(format!("{rewritten}{original}")),
        vec![
            verdict("a.py", "parse", "passes"),
            verdict("a.py", "parse#2", "fails"),
        ]
    );
}

const WALKER: &str = "def _walk(node):\n    for child in node:\n        if child:\n            if child.ok:\n                visit(child)\n    return None\n";

#[test]
fn the_gate_fails_a_rename_that_makes_a_body_recursive() {
    let visitor = WALKER.replace("def _walk", "def visit");
    let gate = |original: &str, current: &str| {
        let dir = tempdir().expect("tempdir should work");
        init_repo(dir.path());
        fs::write(dir.path().join("a.py"), original).expect("should write");
        commit_all(dir.path(), "initial");
        fs::write(dir.path().join("a.py"), current).expect("should write");
        verdicts(dir.path(), &["a.py"])
    };

    assert_eq!(
        gate(WALKER, &visitor),
        vec![
            verdict("a.py", "_walk", "passes"),
            verdict("a.py", "visit", "fails"),
        ]
    );
    assert_eq!(
        gate(&visitor, WALKER),
        vec![
            verdict("a.py", "_walk", "replaces visit"),
            verdict("a.py", "visit", "passes"),
        ]
    );
}

#[test]
fn the_staged_gate_counts_a_function_an_ignore_marker_hides() {
    let dir = tempdir().expect("tempdir should work");
    init_repo(dir.path());
    fs::write(dir.path().join("a.py"), defined("def process(data):", 0, 1)).expect("should write");
    commit_all(dir.path(), "initial");
    fs::write(
        dir.path().join("a.py"),
        defined("def hidden(data):  # complexipy: ignore", 0, 1),
    )
    .expect("should write");
    fs::write(dir.path().join("b.py"), defined("def process(data):", 0, 1)).expect("should write");
    git(dir.path(), &["add", "-A"]);
    let current = vec![scored(dir.path(), "a.py"), scored(dir.path(), "b.py")];

    let report = compute_staged_diff(
        &current,
        "HEAD",
        dir.path().to_str().unwrap(),
        &|_| true,
        Some(3),
    )
    .expect("should diff");

    assert_eq!(
        gate_verdicts(&report.entries, 3, &report.bodies),
        vec![Verdict::Passes, Verdict::Fails]
    );
}

#[test]
fn a_long_logical_line_leaves_every_new_function_failing() {
    let dir = tempdir().expect("tempdir should work");
    init_repo(dir.path());
    fs::write(dir.path().join("a.py"), defined("def process(data):", 0, 1)).expect("should write");
    commit_all(dir.path(), "initial");
    fs::rename(dir.path().join("a.py"), dir.path().join("b.py")).expect("should move");
    fs::write(
        dir.path().join("chain.py"),
        format!("def chain(a):\n    return {}a\n", "a+".repeat(60_000)),
    )
    .expect("should write");
    let root = dir.path().to_path_buf();

    let failing = std::thread::Builder::new()
        .stack_size(1024 * 1024)
        .spawn(move || verdicts(&root, &["b.py", "chain.py"]))
        .expect("thread should start")
        .join()
        .expect("the gate should not overflow the stack");

    assert_eq!(
        failing,
        vec![
            verdict("b.py", "process", "fails"),
            verdict("chain.py", "chain", "passes"),
            verdict("a.py", "process", "passes"),
        ]
    );
}

#[test]
fn has_regressions_pairs_no_moved_function() {
    let moved = [
        entry("b.py", "process", None, Some(20)),
        entry("a.py", "process", Some(20), None),
    ];

    assert!(has_regressions(&moved, 15));
    assert!(!has_regressions(&moved, 20));
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

    let report = compute_staged_diff(&[], &git_ref, dir.path().to_str().unwrap(), &|_| true, None);

    assert_eq!(report.map(|report| report.entries), Some(Vec::new()));
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

    let entries = staged(dir.path(), &["a.py", "svc.py"]);

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

    let entries = staged(dir.path(), &["x.py"]);

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

    let entries = staged(dir.path(), &["0:new.py"]);

    assert_eq!(entries, vec![entry("0:new.py", "with_if", None, Some(1))]);
}
