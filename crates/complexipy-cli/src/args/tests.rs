use clap::Parser;

use crate::args::CliArgs;

fn parse(args: &[&str]) -> CliArgs {
    CliArgs::try_parse_from(args).expect("cli args should parse")
}

#[test]
fn bare_long_flag_before_path_is_not_consumed() {
    let cli = parse(&[
        "complexipy",
        "--suggest-refactors",
        "tests/fixtures/refactor_plans/collapsible_if_skips_blank_line_before_comment_sibling.py",
    ]);

    assert_eq!(cli.suggest_refactors, Some(true));
    assert_eq!(
        cli.paths,
        vec![
            "tests/fixtures/refactor_plans/collapsible_if_skips_blank_line_before_comment_sibling.py"
        ]
    );
}

#[test]
fn bare_flag_after_path_parses() {
    let cli = parse(&["complexipy", "src", "--failed"]);

    assert_eq!(cli.failed, Some(true));
    assert_eq!(cli.paths, vec!["src"]);
}

#[test]
fn equals_form_parses_explicit_values() {
    let cli = parse(&[
        "complexipy",
        "src",
        "--quiet=false",
        "--suggest-refactors=true",
    ]);

    assert_eq!(cli.quiet, Some(false));
    assert_eq!(cli.suggest_refactors, Some(true));
    assert_eq!(cli.paths, vec!["src"]);
}

#[test]
fn short_flag_equals_form_parses() {
    let cli = parse(&["complexipy", "src", "-q=false"]);

    assert_eq!(cli.quiet, Some(false));
    assert_eq!(cli.paths, vec!["src"]);
}

#[test]
fn short_flag_cluster_parses_as_bare_flags() {
    let cli = parse(&["complexipy", "src", "-qf"]);

    assert_eq!(cli.quiet, Some(true));
    assert_eq!(cli.failed, Some(true));
    assert_eq!(cli.paths, vec!["src"]);
}

#[test]
fn space_separated_value_is_a_path() {
    let cli = parse(&["complexipy", "--quiet", "false"]);

    assert_eq!(cli.quiet, Some(true));
    assert_eq!(cli.paths, vec!["false"]);
}

#[test]
fn all_bool_flags_accept_equals_values() {
    let cli = parse(&[
        "complexipy",
        "src",
        "--snapshot-create=false",
        "--snapshot-ignore=false",
        "--quiet=false",
        "--ignore-complexity=false",
        "--failed=false",
        "--staged=false",
        "--suggest-refactors=false",
        "--check-script=false",
        "--no-ignore=false",
        "--report-ignored=false",
    ]);

    assert_eq!(cli.snapshot_create, Some(false));
    assert_eq!(cli.snapshot_ignore, Some(false));
    assert_eq!(cli.quiet, Some(false));
    assert_eq!(cli.ignore_complexity, Some(false));
    assert_eq!(cli.failed, Some(false));
    assert_eq!(cli.staged, Some(false));
    assert_eq!(cli.suggest_refactors, Some(false));
    assert_eq!(cli.check_script, Some(false));
    assert_eq!(cli.no_ignore, Some(false));
    assert_eq!(cli.report_ignored, Some(false));
    assert_eq!(cli.paths, vec!["src"]);
}

#[test]
fn plain_equals_form_parses() {
    let cli = parse(&["complexipy", "src", "--plain=false"]);

    assert_eq!(cli.plain, Some(false));
    assert_eq!(cli.paths, vec!["src"]);
}
