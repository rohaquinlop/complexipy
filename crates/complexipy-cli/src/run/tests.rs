use std::fs;
use std::path::Path;
use std::process::Command;

use clap::Parser;
use tempfile::tempdir;

use super::analysis_options;
use crate::args::CliArgs;
use crate::output::render::color_lock;
use crate::utils::config::resolve_config;
use complexipy_core::RuleSet;

const SIMPLE: &str = "def simple(x):\n    return x + 1\n";

const COMPLEX: &str = "def complex_func(data):\n    if data:\n        for item in data:\n            if item:\n                for x in item:\n                    if x:\n                        for y in x:\n                            if y:\n                                return y\n    return None\n";

fn run_at(cli: CliArgs, invocation_path: &str) -> std::process::ExitCode {
    let _guard = color_lock();
    super::run_at(cli, invocation_path)
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
    git(dir, &["add", "-A"]);
    git(dir, &["commit", "-q", "-m", "initial"]);
}

fn parse(args: &[&str]) -> CliArgs {
    let mut argv = vec!["complexipy"];
    argv.extend_from_slice(args);
    CliArgs::try_parse_from(argv).expect("cli args should parse")
}

#[test]
fn missing_paths_exits_failure() {
    let dir = tempdir().expect("tempdir should work");

    let exit = run_at(parse(&[]), dir.path().to_str().unwrap());

    assert_eq!(exit, std::process::ExitCode::FAILURE);
}

#[test]
fn clean_run_exits_success() {
    let dir = tempdir().expect("tempdir should work");
    let file = dir.path().join("simple.py");
    fs::write(&file, SIMPLE).expect("should write");

    let exit = run_at(
        parse(&[file.to_str().unwrap()]),
        dir.path().to_str().unwrap(),
    );

    assert_eq!(exit, std::process::ExitCode::SUCCESS);
}

#[test]
fn failed_run_exits_failure() {
    let dir = tempdir().expect("tempdir should work");
    let file = dir.path().join("complex.py");
    fs::write(&file, COMPLEX).expect("should write");

    let exit = run_at(
        parse(&[file.to_str().unwrap(), "--failed"]),
        dir.path().to_str().unwrap(),
    );

    assert_eq!(exit, std::process::ExitCode::FAILURE);
}

#[test]
fn snapshot_create_writes_snapshot_file() {
    let dir = tempdir().expect("tempdir should work");
    let file = dir.path().join("complex.py");
    fs::write(&file, COMPLEX).expect("should write");

    let exit = run_at(
        parse(&[file.to_str().unwrap(), "--snapshot-create"]),
        dir.path().to_str().unwrap(),
    );

    assert_eq!(exit, std::process::ExitCode::SUCCESS);
    assert!(dir.path().join("complexipy-snapshot.json").exists());
}

#[test]
fn invalid_path_exits_failure() {
    let dir = tempdir().expect("tempdir should work");

    let exit = run_at(
        parse(&[dir.path().join("nope.py").to_str().unwrap()]),
        dir.path().to_str().unwrap(),
    );

    assert_eq!(exit, std::process::ExitCode::FAILURE);
}

#[test]
fn diff_regression_exits_failure() {
    let dir = tempdir().expect("tempdir should work");
    let file = dir.path().join("a.py");
    fs::write(&file, SIMPLE).expect("should write");
    init_repo(dir.path());

    fs::write(&file, COMPLEX).expect("should write");
    let exit = run_at(
        parse(&[file.to_str().unwrap(), "--diff", "HEAD"]),
        dir.path().to_str().unwrap(),
    );

    assert_eq!(exit, std::process::ExitCode::FAILURE);
}

#[test]
fn diff_clean_exits_success() {
    let dir = tempdir().expect("tempdir should work");
    let file = dir.path().join("a.py");
    fs::write(&file, COMPLEX).expect("should write");
    init_repo(dir.path());

    let exit = run_at(
        parse(&[file.to_str().unwrap(), "--diff", "HEAD"]),
        dir.path().to_str().unwrap(),
    );

    assert_eq!(exit, std::process::ExitCode::SUCCESS);
}

#[test]
fn plain_flag_accepted() {
    let dir = tempdir().expect("tempdir should work");
    let file = dir.path().join("simple.py");
    fs::write(&file, SIMPLE).expect("should write");

    let exit = run_at(
        parse(&[file.to_str().unwrap(), "--plain"]),
        dir.path().to_str().unwrap(),
    );

    assert_eq!(exit, std::process::ExitCode::SUCCESS);
}

#[test]
fn version_flag_handled_by_clap() {
    let result = CliArgs::try_parse_from(["complexipy", "--version"]);

    assert!(result.is_err());
}

const FIXABLE: &str =
    "def fixable(a, b):\n    if a:\n        if b:\n            return 1\n    return 0\n";

const FIXABLE_FIXED: &str = "def fixable(a, b):\n    if a and b:\n        return 1\n    return 0\n";

const TWO_FIXABLE: &str = "def one(a, b):\n    if a:\n        if b:\n            return 1\n    return 0\n\n\ndef two(a, b):\n    if a:\n        if b:\n            return 2\n    return 0\n";

const TWO_FIXABLE_FIXED: &str = "def one(a, b):\n    if a and b:\n        return 1\n    return 0\n\n\ndef two(a, b):\n    if a and b:\n        return 2\n    return 0\n";

fn fix_every_function(flag: &str, file: &Path) -> CliArgs {
    parse(&[
        flag,
        "--max-complexity-allowed=0",
        "--ignore-complexity",
        file.to_str().unwrap(),
    ])
}

#[test]
fn fix_leaves_functions_under_the_threshold_alone() {
    let dir = tempdir().expect("tempdir should work");
    let file = dir.path().join("fixable.py");
    fs::write(&file, FIXABLE).expect("should write");

    let exit = run_at(
        parse(&["--fix", file.to_str().unwrap()]),
        dir.path().to_str().unwrap(),
    );

    assert_eq!(exit, std::process::ExitCode::SUCCESS);
    assert_eq!(fs::read_to_string(&file).expect("should read"), FIXABLE);
}

#[test]
fn fix_writes_the_suggestion_to_the_file() {
    let dir = tempdir().expect("tempdir should work");
    let file = dir.path().join("fixable.py");
    fs::write(&file, FIXABLE).expect("should write");

    let exit = run_at(
        fix_every_function("--fix", &file),
        dir.path().to_str().unwrap(),
    );

    assert_eq!(exit, std::process::ExitCode::SUCCESS);
    assert_eq!(
        fs::read_to_string(&file).expect("should read"),
        FIXABLE_FIXED
    );
}

#[test]
fn fix_applies_every_fix_in_one_file_bottom_to_top() {
    let dir = tempdir().expect("tempdir should work");
    let file = dir.path().join("two.py");
    fs::write(&file, TWO_FIXABLE).expect("should write");

    let exit = run_at(
        fix_every_function("--fix", &file),
        dir.path().to_str().unwrap(),
    );

    assert_eq!(exit, std::process::ExitCode::SUCCESS);
    assert_eq!(
        fs::read_to_string(&file).expect("should read"),
        TWO_FIXABLE_FIXED
    );
}

#[test]
fn dry_run_writes_nothing() {
    let dir = tempdir().expect("tempdir should work");
    let file = dir.path().join("fixable.py");
    fs::write(&file, FIXABLE).expect("should write");

    let exit = run_at(
        fix_every_function("--dry-run", &file),
        dir.path().to_str().unwrap(),
    );

    assert_eq!(exit, std::process::ExitCode::SUCCESS);
    assert_eq!(fs::read_to_string(&file).expect("should read"), FIXABLE);
}

#[test]
fn fix_runs_regardless_of_the_git_state() {
    let dir = tempdir().expect("tempdir should work");
    fs::write(dir.path().join("simple.py"), SIMPLE).expect("should write");
    init_repo(dir.path());
    fs::write(
        dir.path().join("simple.py"),
        "def simple(x):\n    return x + 2\n",
    )
    .expect("should write");
    let file = dir.path().join("fixable.py");
    fs::write(&file, FIXABLE).expect("should write");

    let exit = run_at(
        fix_every_function("--fix", &file),
        dir.path().to_str().unwrap(),
    );

    assert_eq!(exit, std::process::ExitCode::SUCCESS);
    assert_eq!(
        fs::read_to_string(&file).expect("should read"),
        FIXABLE_FIXED
    );
}

#[test]
fn fix_outside_a_repository_writes() {
    let dir = tempdir().expect("tempdir should work");
    let file = dir.path().join("fixable.py");
    fs::write(&file, FIXABLE).expect("should write");

    let exit = run_at(
        fix_every_function("--fix", &file),
        dir.path().to_str().unwrap(),
    );

    assert_eq!(exit, std::process::ExitCode::SUCCESS);
    assert_eq!(
        fs::read_to_string(&file).expect("should read"),
        FIXABLE_FIXED
    );
}

#[test]
fn a_staged_run_compares_only_the_files_it_selects() {
    let dir = tempdir().expect("tempdir should work");
    fs::write(dir.path().join("README.md"), "placeholder").expect("should write");
    init_repo(dir.path());
    fs::write(dir.path().join("one.py"), SIMPLE).expect("should write");
    fs::write(dir.path().join("two.py"), COMPLEX).expect("should write");
    git(dir.path(), &["add", "one.py", "two.py"]);
    let one = dir.path().join("one.py");

    let exit = run_at(
        parse(&[
            one.to_str().unwrap(),
            "--staged",
            "--max-complexity-allowed",
            "3",
        ]),
        dir.path().to_str().unwrap(),
    );

    assert_eq!(exit, std::process::ExitCode::SUCCESS);
}

fn moved_over_threshold() -> tempfile::TempDir {
    let dir = tempdir().expect("tempdir should work");
    fs::create_dir(dir.path().join("pkg")).expect("should create");
    fs::write(dir.path().join("pkg/complex.py"), COMPLEX).expect("should write");
    init_repo(dir.path());
    git(dir.path(), &["mv", "pkg/complex.py", "pkg/moved.py"]);
    dir
}

#[test]
fn a_moved_file_passes_the_diff_gate() {
    let dir = moved_over_threshold();
    let pkg = dir.path().join("pkg");

    let exit = run_at(
        parse(&[
            pkg.to_str().unwrap(),
            "--diff",
            "HEAD",
            "--max-complexity-allowed",
            "3",
        ]),
        dir.path().to_str().unwrap(),
    );

    assert_eq!(exit, std::process::ExitCode::SUCCESS);
}

#[test]
fn a_moved_file_passes_the_staged_gate() {
    let dir = moved_over_threshold();
    let pkg = dir.path().join("pkg");

    let exit = run_at(
        parse(&[
            pkg.to_str().unwrap(),
            "--staged",
            "--max-complexity-allowed",
            "3",
        ]),
        dir.path().to_str().unwrap(),
    );

    assert_eq!(exit, std::process::ExitCode::SUCCESS);
}

#[test]
fn a_function_moved_to_another_file_fails_the_gate_once_it_grows() {
    let dir = tempdir().expect("tempdir should work");
    fs::write(dir.path().join("a.py"), COMPLEX).expect("should write");
    fs::write(dir.path().join("b.py"), SIMPLE).expect("should write");
    init_repo(dir.path());
    let gate = |b_source: &str| {
        fs::write(dir.path().join("a.py"), "").expect("should write");
        fs::write(dir.path().join("b.py"), b_source).expect("should write");
        run_at(
            parse(&[
                dir.path().to_str().unwrap(),
                "--diff",
                "HEAD",
                "--max-complexity-allowed",
                "3",
            ]),
            dir.path().to_str().unwrap(),
        )
    };
    let grown = format!("{}    if data:\n        return 0\n", COMPLEX);

    assert_eq!(gate(COMPLEX), std::process::ExitCode::SUCCESS);
    assert_eq!(gate(&grown), std::process::ExitCode::FAILURE);
}

/// A repository with `scripts/tool.py` over the threshold and `src/kept.py`
/// committed, gated on `src` with `extra` flags.
fn gate_on_src(prepare: impl Fn(&Path), extra: &[&str]) -> std::process::ExitCode {
    let dir = tempdir().expect("tempdir should work");
    fs::create_dir(dir.path().join("src")).expect("should create");
    fs::create_dir(dir.path().join("scripts")).expect("should create");
    fs::write(dir.path().join("scripts/tool.py"), COMPLEX).expect("should write");
    fs::write(dir.path().join("src/kept.py"), SIMPLE).expect("should write");
    init_repo(dir.path());
    prepare(dir.path());
    let src = dir.path().join("src");
    let mut args = vec![src.to_str().unwrap(), "--max-complexity-allowed", "3"];
    args.extend_from_slice(extra);
    run_at(parse(&args), dir.path().to_str().unwrap())
}

#[test]
fn a_file_moved_into_the_run_from_outside_it_is_new_to_the_gate() {
    let move_in = |dir: &Path| git(dir, &["mv", "scripts/tool.py", "src/tool.py"]);

    assert_eq!(
        gate_on_src(move_in, &["--diff", "HEAD"]),
        std::process::ExitCode::FAILURE
    );
    assert_eq!(
        gate_on_src(move_in, &["--staged"]),
        std::process::ExitCode::FAILURE
    );
}

#[test]
fn a_file_moved_out_of_the_run_pays_for_no_new_function() {
    let move_out = |dir: &Path| {
        let filler: String = (0..40)
            .map(|index| format!("def filler_{index}():\n    return {index}\n"))
            .collect();
        fs::write(dir.join("src/legacy.py"), format!("{COMPLEX}{filler}")).expect("should write");
        git(dir, &["add", "src/legacy.py"]);
        git(dir, &["commit", "-q", "-m", "legacy"]);
        git(dir, &["mv", "src/legacy.py", "scripts/legacy.py"]);
        fs::write(dir.join("src/fresh.py"), COMPLEX).expect("should write");
        git(dir, &["add", "src/fresh.py"]);
    };

    assert_eq!(
        gate_on_src(move_out, &["--diff", "HEAD"]),
        std::process::ExitCode::FAILURE
    );
    assert_eq!(
        gate_on_src(move_out, &["--staged"]),
        std::process::ExitCode::FAILURE
    );
}

#[test]
fn a_file_that_did_not_parse_at_the_ref_is_new_to_the_gate() {
    let fix_syntax = |dir: &Path| {
        fs::write(dir.join("src/kept.py"), "def broken(:\n").expect("should write");
        git(dir, &["commit", "-q", "-am", "broken"]);
        fs::write(dir.join("src/kept.py"), COMPLEX).expect("should write");
    };

    assert_eq!(
        gate_on_src(fix_syntax, &["--diff", "HEAD"]),
        std::process::ExitCode::FAILURE
    );
}

#[test]
fn a_function_an_ignore_marker_hides_pays_for_no_new_function() {
    let dir = tempdir().expect("tempdir should work");
    let file = dir.path().join("a.py");
    fs::write(&file, COMPLEX).expect("should write");
    init_repo(dir.path());
    let marked = COMPLEX.replace(
        "def complex_func(data):",
        "def complex_func(data):  # complexipy: ignore",
    );
    fs::write(
        &file,
        format!("{marked}\n\n{}", COMPLEX.replace("complex_func", "fresh")),
    )
    .expect("should write");

    let exit = run_at(
        parse(&[
            file.to_str().unwrap(),
            "--diff",
            "HEAD",
            "--max-complexity-allowed",
            "3",
        ]),
        dir.path().to_str().unwrap(),
    );

    assert_eq!(exit, std::process::ExitCode::FAILURE);
}

/// The exit codes of a `--diff HEAD` run and of a `--staged` run over `src`,
/// excluding `vendor`, at a threshold of 3, in a repository that committed
/// `files` and then staged `change`.
fn gate_change(files: &[(&str, &str)], change: impl Fn(&Path)) -> Vec<std::process::ExitCode> {
    let dir = tempdir().expect("tempdir should work");
    for (path, content) in files {
        write_file(dir.path(), path, content);
    }
    init_repo(dir.path());
    change(dir.path());
    git(dir.path(), &["add", "-A"]);
    let src = dir.path().join("src");
    [vec!["--diff", "HEAD"], vec!["--staged"]]
        .into_iter()
        .map(|flags| {
            let mut args = vec![
                src.to_str().unwrap(),
                "--max-complexity-allowed",
                "3",
                "--exclude",
                "vendor/**",
            ];
            args.extend(flags);
            run_at(parse(&args), dir.path().to_str().unwrap())
        })
        .collect()
}

fn write_file(dir: &Path, path: &str, content: &str) {
    let path = dir.join(path);
    fs::create_dir_all(path.parent().expect("should have a parent")).expect("should create");
    fs::write(path, content).expect("should write");
}

fn indented(source: &str) -> String {
    source.lines().map(|line| format!("    {line}\n")).collect()
}

fn in_class(class: &str, source: &str) -> String {
    format!("class {class}:\n{}", indented(source))
}

#[test]
fn a_function_an_ignore_marker_hides_counts_after_a_rename_or_a_move() {
    let marked = |name: &str| {
        COMPLEX.replace(
            "def complex_func(data):",
            &format!("def {name}(data):  # complexipy: ignore"),
        )
    };
    let copied = |dir: &Path| write_file(dir, "src/fresh.py", COMPLEX);
    let renamed = |dir: &Path| {
        write_file(dir, "src/legacy.py", &marked("hidden"));
        copied(dir);
    };
    let class_renamed = |dir: &Path| {
        write_file(
            dir,
            "src/legacy.py",
            &in_class("New", &marked("complex_func")),
        );
        copied(dir);
    };
    let moved = |dir: &Path| {
        write_file(dir, "src/legacy.py", SIMPLE);
        write_file(dir, "src/other.py", &marked("complex_func"));
        copied(dir);
    };
    let renumbered = |dir: &Path| {
        write_file(dir, "src/legacy.py", &marked("f"));
        write_file(dir, "src/fresh.py", &COMPLEX.replace("complex_func", "f"));
    };
    let failing = vec![std::process::ExitCode::FAILURE; 2];

    assert_eq!(gate_change(&[("src/legacy.py", COMPLEX)], renamed), failing);
    assert_eq!(
        gate_change(
            &[("src/legacy.py", &in_class("Old", COMPLEX))],
            class_renamed
        ),
        failing
    );
    assert_eq!(
        gate_change(&[("src/legacy.py", &format!("{COMPLEX}{SIMPLE}"))], moved),
        failing
    );
    let two_named_f = format!(
        "def f(x):\n    return x\n{}",
        COMPLEX.replace("complex_func", "f")
    );
    assert_eq!(
        gate_change(&[("src/legacy.py", &two_named_f)], renumbered),
        failing
    );
}

#[test]
fn a_function_whose_name_survives_pays_for_no_copy() {
    let rewritten = COMPLEX.replace("return y", "return x");
    let copy = COMPLEX.replace("complex_func", "complex_func_legacy");
    let rewrite = |dir: &Path, content: &str| write_file(dir, "src/legacy.py", content);
    let failing = vec![std::process::ExitCode::FAILURE; 2];

    assert_eq!(
        gate_change(&[("src/legacy.py", COMPLEX)], |dir| {
            rewrite(dir, &format!("{rewritten}{copy}"))
        }),
        failing
    );
    assert_eq!(
        gate_change(&[("src/legacy.py", COMPLEX)], |dir| {
            rewrite(dir, &format!("{rewritten}{COMPLEX}"))
        }),
        failing
    );
    assert_eq!(
        gate_change(&[("src/legacy.py", &in_class("Parser", COMPLEX))], |dir| {
            rewrite(dir, &in_class("Parser", &format!("{rewritten}{copy}")))
        }),
        failing
    );
}

#[test]
fn a_rename_that_makes_a_function_recursive_fails_the_gate() {
    let walker = "def _walk(node):\n    for child in node:\n        if child:\n            if child.ok:\n                visit(child)\n    return None\n";
    let renamed = |dir: &Path| {
        write_file(
            dir,
            "src/legacy.py",
            &walker.replace("def _walk", "def visit"),
        )
    };

    assert_eq!(
        gate_change(&[("src/legacy.py", walker)], renamed),
        vec![std::process::ExitCode::FAILURE; 2]
    );
}

/// Commits `src/<name>` as a symbolic link whose target text is `COMPLEX`,
/// then removes the link from the working tree.
#[cfg(unix)]
fn commit_complex_link(dir: &Path, name: &str) {
    let link = dir.join("src").join(name);
    std::os::unix::fs::symlink(COMPLEX, &link).expect("should link");
    git(dir, &["add", "-A"]);
    git(dir, &["commit", "-q", "-m", "link"]);
    fs::remove_file(link).expect("should remove");
}

#[cfg(unix)]
#[test]
fn a_symbolic_link_at_the_ref_is_never_read_as_python() {
    let committed: &[(&str, &str)] = &[("src/kept.py", SIMPLE)];
    let link_made_a_file = |dir: &Path| {
        commit_complex_link(dir, "mod.py");
        write_file(dir, "src/mod.py", COMPLEX);
    };
    let link_deleted = |dir: &Path| {
        commit_complex_link(dir, "link.py");
        write_file(dir, "src/fresh.py", COMPLEX);
    };
    let failing = vec![std::process::ExitCode::FAILURE; 2];

    assert_eq!(gate_change(committed, link_made_a_file), failing);
    assert_eq!(gate_change(committed, link_deleted), failing);
}

#[test]
fn a_function_deleted_in_a_nested_repository_pays_for_no_copy_outside_it() {
    let nested_deletion = |dir: &Path| {
        write_file(dir, "src/sub/kept.py", SIMPLE);
        write_file(dir, "src/sub/legacy.py", COMPLEX);
        let sub = dir.join("src/sub");
        init_repo(&sub);
        fs::remove_file(sub.join("legacy.py")).expect("should remove");
        write_file(dir, "src/fresh.py", COMPLEX);
    };

    assert_eq!(
        gate_change(&[("src/kept.py", SIMPLE)], nested_deletion),
        vec![std::process::ExitCode::FAILURE; 2]
    );
}

/// The exit code of a `--staged` run over `src`, at a threshold of 3, after
/// a complex `src/a.py` is staged and `replace` puts something else at that
/// path in the working tree.
fn staged_gate_after(replace: impl Fn(&Path)) -> std::process::ExitCode {
    let dir = tempdir().expect("tempdir should work");
    write_file(dir.path(), "src/kept.py", SIMPLE);
    init_repo(dir.path());
    write_file(dir.path(), "src/a.py", COMPLEX);
    git(dir.path(), &["add", "src/a.py"]);
    fs::remove_file(dir.path().join("src/a.py")).expect("should remove");
    replace(dir.path());
    let src = dir.path().join("src");
    run_at(
        parse(&[
            src.to_str().unwrap(),
            "--staged",
            "--max-complexity-allowed",
            "3",
        ]),
        dir.path().to_str().unwrap(),
    )
}

#[test]
fn a_staged_file_is_compared_whatever_the_working_tree_holds() {
    let directory = |dir: &Path| fs::create_dir(dir.join("src/a.py")).expect("should create");

    assert_eq!(
        staged_gate_after(directory),
        std::process::ExitCode::FAILURE
    );
    #[cfg(unix)]
    {
        let link = |dir: &Path| {
            std::os::unix::fs::symlink("kept.py", dir.join("src/a.py")).expect("should link")
        };
        assert_eq!(staged_gate_after(link), std::process::ExitCode::FAILURE);
    }
}

#[test]
fn a_function_moved_out_of_the_run_pays_for_no_other_function() {
    let edited = COMPLEX.replace("return y", "return x");
    let function_moved_out = |dir: &Path| {
        write_file(dir, "src/legacy.py", SIMPLE);
        write_file(dir, "src/vendor/legacy.py", COMPLEX);
        write_file(dir, "src/fresh.py", &edited);
    };
    let file_moved_out_and_rewritten = |dir: &Path| {
        let filler: String = (0..40)
            .map(|index| format!("def filler_{index}():\n    return {index}\n"))
            .collect();
        fs::remove_file(dir.join("src/legacy.py")).expect("should remove");
        write_file(dir, "src/vendor/legacy.py", &format!("{filler}{COMPLEX}"));
        write_file(dir, "src/kept.py", &format!("{SIMPLE}{edited}"));
    };
    let failing = vec![std::process::ExitCode::FAILURE; 2];

    assert_eq!(
        gate_change(
            &[("src/legacy.py", &format!("{COMPLEX}{SIMPLE}"))],
            function_moved_out
        ),
        failing
    );
    assert_eq!(
        gate_change(
            &[("src/legacy.py", COMPLEX), ("src/kept.py", SIMPLE)],
            file_moved_out_and_rewritten
        ),
        failing
    );
}

#[test]
fn a_deleted_file_that_ignore_rules_hide_pays_for_no_new_function() {
    let dir = tempdir().expect("tempdir should work");
    fs::create_dir_all(dir.path().join("src/generated")).expect("should create");
    fs::write(dir.path().join(".gitignore"), "generated/\n").expect("should write");
    fs::write(dir.path().join("src/kept.py"), SIMPLE).expect("should write");
    init_repo(dir.path());
    fs::write(dir.path().join("src/generated/old.py"), COMPLEX).expect("should write");
    git(dir.path(), &["add", "-f", "src/generated/old.py"]);
    git(dir.path(), &["commit", "-q", "-m", "generated"]);
    fs::remove_file(dir.path().join("src/generated/old.py")).expect("should remove");
    fs::write(dir.path().join("src/fresh.py"), COMPLEX).expect("should write");
    let src = dir.path().join("src");

    let exit = run_at(
        parse(&[
            src.to_str().unwrap(),
            "--diff",
            "HEAD",
            "--max-complexity-allowed",
            "3",
        ]),
        dir.path().to_str().unwrap(),
    );

    assert_eq!(exit, std::process::ExitCode::FAILURE);
}

fn committed_over_threshold(config: &str) -> tempfile::TempDir {
    let dir = tempdir().expect("tempdir should work");
    fs::write(dir.path().join("complex.py"), COMPLEX).expect("should write");
    fs::write(dir.path().join("complexipy.toml"), config).expect("should write");
    init_repo(dir.path());
    dir
}

fn gate(dir: &Path, extra: &[&str]) -> std::process::ExitCode {
    let file = dir.join("complex.py");
    let mut args = vec![file.to_str().unwrap(), "--max-complexity-allowed", "3"];
    args.extend_from_slice(extra);
    run_at(parse(&args), dir.to_str().unwrap())
}

#[test]
fn a_config_diff_section_switches_the_gate_unless_isolated() {
    let dir = committed_over_threshold("[diff]\nbranch = \"HEAD\"\n");

    assert_eq!(gate(dir.path(), &[]), std::process::ExitCode::SUCCESS);
    assert_eq!(
        gate(dir.path(), &["--isolated"]),
        std::process::ExitCode::FAILURE
    );
}

#[test]
fn an_empty_config_branch_keeps_the_absolute_gate() {
    let dir = committed_over_threshold("[diff]\nbranch = \"\"\nstaged = true\n");

    assert_eq!(gate(dir.path(), &[]), std::process::ExitCode::FAILURE);
}

#[test]
fn a_config_file_on_the_command_line_replaces_the_project_config() {
    let dir = committed_over_threshold("[diff]\nbranch = \"HEAD\"\n");
    let pinned = dir.path().join("ci.toml");
    fs::write(&pinned, "max-complexity-allowed = 3\n").expect("should write");
    let file = dir.path().join("complex.py");

    let exit = run_at(
        parse(&[file.to_str().unwrap(), "--config", pinned.to_str().unwrap()]),
        dir.path().to_str().unwrap(),
    );

    assert_eq!(exit, std::process::ExitCode::FAILURE);
}

#[test]
fn a_missing_config_file_on_the_command_line_fails() {
    let dir = tempdir().expect("tempdir should work");
    let file = dir.path().join("simple.py");
    fs::write(&file, SIMPLE).expect("should write");
    let missing = dir.path().join("missing.toml");

    let exit = run_at(
        parse(&[
            file.to_str().unwrap(),
            "--config",
            missing.to_str().unwrap(),
        ]),
        dir.path().to_str().unwrap(),
    );

    assert_eq!(exit, std::process::ExitCode::FAILURE);
}

#[test]
fn an_option_like_config_branch_writes_nothing_through_git() {
    let dir = committed_over_threshold("");
    let victim = dir.path().join("victim.txt");
    fs::write(&victim, "keep me").expect("should write");
    let branch = format!("--output={}", victim.display());
    fs::write(
        dir.path().join("complexipy.toml"),
        format!("[diff]\nbranch = {:?}\nstaged = true\n", branch),
    )
    .expect("should write");

    let exit = gate(dir.path(), &["--quiet"]);

    assert_eq!(exit, std::process::ExitCode::FAILURE);
    assert_eq!(fs::read_to_string(&victim).expect("should read"), "keep me");
}

#[test]
fn a_config_output_outside_the_project_writes_nothing() {
    let dir = tempdir().expect("tempdir should work");
    let outside = tempdir().expect("tempdir should work");
    let target = outside.path().join("existing.json");
    fs::write(&target, "keep me").expect("should write");
    fs::write(
        dir.path().join("complexipy.toml"),
        format!(
            "output-format = [\"json\"]\noutput = {:?}\n",
            target.display().to_string()
        ),
    )
    .expect("should write");
    let file = dir.path().join("simple.py");
    fs::write(&file, SIMPLE).expect("should write");

    let exit = run_at(
        parse(&[file.to_str().unwrap()]),
        dir.path().to_str().unwrap(),
    );

    assert_eq!(exit, std::process::ExitCode::FAILURE);
    assert_eq!(fs::read_to_string(&target).expect("should read"), "keep me");
}

#[cfg(unix)]
#[test]
fn a_directory_run_never_reads_through_links_that_leave_it() {
    let dir = tempdir().expect("tempdir should work");
    let outside = tempdir().expect("tempdir should work");
    fs::write(outside.path().join("y.py"), COMPLEX).expect("should write");
    fs::write(dir.path().join("simple.py"), SIMPLE).expect("should write");
    std::os::unix::fs::symlink(outside.path().join("y.py"), dir.path().join("x.py"))
        .expect("should link");
    std::os::unix::fs::symlink("/dev/zero", dir.path().join("zero.py")).expect("should link");
    std::os::unix::fs::symlink(dir.path().join("gone.py"), dir.path().join("dangling.py"))
        .expect("should link");
    fs::create_dir(dir.path().join("pkg.py")).expect("should create");

    let exit = run_at(
        parse(&[
            dir.path().to_str().unwrap(),
            "--max-complexity-allowed",
            "3",
        ]),
        dir.path().to_str().unwrap(),
    );

    assert_eq!(exit, std::process::ExitCode::SUCCESS);
}

#[test]
fn dry_run_writes_no_cache_and_no_snapshot() {
    let dir = tempdir().expect("tempdir should work");
    let file = dir.path().join("fixable.py");
    fs::write(&file, FIXABLE).expect("should write");
    let snapshot = dir.path().join("complexipy-snapshot.json");
    fs::write(&snapshot, "[]").expect("should write");

    run_at(
        fix_every_function("--dry-run", &file),
        dir.path().to_str().unwrap(),
    );

    assert!(!dir.path().join(".complexipy_cache").exists());
    assert_eq!(fs::read_to_string(&snapshot).expect("should read"), "[]");
}

#[cfg(unix)]
#[test]
fn a_report_path_symlinked_out_of_the_project_is_replaced_not_followed() {
    let dir = tempdir().expect("tempdir should work");
    let outside = tempdir().expect("tempdir should work");
    let victim = outside.path().join("results.txt");
    fs::write(&victim, "keep me").expect("should write");
    let report = dir.path().join("complexipy-results.json");
    std::os::unix::fs::symlink(&victim, &report).expect("should link");
    let file = dir.path().join("simple.py");
    fs::write(&file, SIMPLE).expect("should write");

    let exit = run_at(
        parse(&[file.to_str().unwrap(), "--output-format", "json"]),
        dir.path().to_str().unwrap(),
    );

    assert_eq!(exit, std::process::ExitCode::SUCCESS);
    assert_eq!(fs::read_to_string(&victim).expect("should read"), "keep me");
    assert!(
        fs::symlink_metadata(&report)
            .expect("report exists")
            .is_file()
    );
}

fn builds_plans(args: &[&str]) -> bool {
    let config = resolve_config(None, parse(args), ".").expect("config should resolve");

    analysis_options(&config, RuleSet::default()).with_plans
}

#[test]
fn plain_run_builds_no_plans() {
    assert!(!builds_plans(&["."]));
}

#[test]
fn refactor_flags_build_plans() {
    assert!(builds_plans(&[".", "--suggest-refactors"]));
    assert!(builds_plans(&[".", "--fix"]));
    assert!(builds_plans(&[".", "--dry-run"]));
}

fn plans_above(args: &[&str]) -> Option<u64> {
    let config = resolve_config(None, parse(args), ".").expect("config should resolve");

    analysis_options(&config, RuleSet::default()).plans_above
}

#[test]
fn runs_that_show_only_failing_plans_skip_passing_functions() {
    let limit = Some(10);

    assert_eq!(
        plans_above(&[".", "--max-complexity-allowed", "10", "--dry-run"]),
        limit
    );
    assert_eq!(
        plans_above(&[
            ".",
            "--max-complexity-allowed",
            "10",
            "--suggest-refactors",
            "--failed"
        ]),
        limit
    );
    assert_eq!(
        plans_above(&[".", "--max-complexity-allowed", "10", "--fix", "--quiet"]),
        limit
    );
    assert_eq!(
        plans_above(&[
            ".",
            "--max-complexity-allowed",
            "10",
            "--suggest-refactors",
            "--failed",
            "--output-format",
            "json",
        ]),
        limit
    );
}

#[test]
fn runs_that_show_passing_plans_build_every_plan() {
    assert_eq!(plans_above(&[".", "--suggest-refactors"]), None);
    assert_eq!(plans_above(&[".", "--fix"]), None);
    assert_eq!(
        plans_above(&[
            ".",
            "--suggest-refactors",
            "--quiet",
            "--output-format",
            "json"
        ]),
        None
    );
    for format in ["sarif", "gitlab"] {
        assert_eq!(
            plans_above(&[
                ".",
                "--suggest-refactors",
                "--failed",
                "--output-format",
                format
            ]),
            None
        );
    }
}
