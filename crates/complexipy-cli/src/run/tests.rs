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
