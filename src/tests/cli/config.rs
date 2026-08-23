//! Wired in from `src/cli/utils/config.rs` via `#[cfg(test)] #[path = ...] mod tests;`

use clap::Parser;

use crate::cli::args::CliArgs;
use crate::cli::types::{Color, Config, OutputFormat, RunConfig, Sort};
use crate::cli::utils::config::{resolve_config, ConfigError};

fn resolve(cli_args: &[&str], toml_source: Option<&str>) -> Result<RunConfig, ConfigError> {
    let mut argv = vec!["complexipy"];
    argv.extend_from_slice(cli_args);
    let cli = CliArgs::try_parse_from(argv).expect("cli args should parse");
    let toml = toml_source.map(|source| toml::from_str(source).expect("toml should parse"));
    resolve_config(toml, cli)
}

#[test]
fn defaults_with_only_paths() {
    let config = resolve(&["src"], None).expect("resolve should succeed");

    assert_eq!(config.paths, vec!["src"]);
    assert_eq!(config.max_complexity_allowed, 15);
    assert_eq!(config.snapshot_create, false);
    assert_eq!(config.snapshot_ignore, false);
    assert_eq!(config.quiet, false);
    assert_eq!(config.ignore_complexity, false);
    assert_eq!(config.failed, false);
    assert_eq!(config.color, Color::Auto);
    assert_eq!(config.sort, Sort::Asc);
    assert_eq!(config.output_format, Vec::<OutputFormat>::new());
    assert_eq!(config.output, None);
    assert_eq!(config.exclude, Vec::<String>::new());
    assert_eq!(config.check_script, false);
    assert_eq!(config.no_ignore, false);
    assert_eq!(config.report_ignored, false);
    assert_eq!(config.plain, false);
    assert_eq!(config.suggest_refactors, false);
    assert_eq!(config.top, None);
    assert_eq!(config.diff, None);
    assert_eq!(config.diff_only, None);
    assert_eq!(config.staged, false);
}

#[test]
fn missing_paths_without_toml() {
    let result = resolve(&[], None);

    assert_eq!(result, Err(ConfigError::MissingPaths));
    assert_eq!(
        result.unwrap_err().to_string(),
        "You need to define paths in the CLI call arguments or in complexipy.toml file"
    );
}

#[test]
fn toml_paths_used_when_cli_absent() {
    let config = resolve(&[], Some("paths = [\"src\"]")).expect("resolve should succeed");

    assert_eq!(config.paths, vec!["src"]);
}

#[test]
fn toml_without_paths_allows_empty() {
    let config = resolve(&[], Some("max-complexity-allowed = 10")).expect("resolve should succeed");

    assert_eq!(config.paths, Vec::<String>::new());
}

#[test]
fn cli_overrides_toml() {
    let config = resolve(
        &["--max-complexity-allowed", "20"],
        Some("max-complexity-allowed = 10"),
    )
    .expect("resolve should succeed");

    assert_eq!(config.max_complexity_allowed, 20);
}

#[test]
fn toml_value_used_when_cli_absent() {
    let config =
        resolve(&["src"], Some("max-complexity-allowed = 10")).expect("resolve should succeed");

    assert_eq!(config.max_complexity_allowed, 10);
}

#[test]
fn toml_boolean_used_when_cli_absent() {
    let config = resolve(
        &["src"],
        Some("quiet = true\nno-ignore = true\nreport-ignored = true"),
    )
    .expect("resolve should succeed");

    assert_eq!(config.quiet, true);
    assert_eq!(config.no_ignore, true);
    assert_eq!(config.report_ignored, true);
}

#[test]
fn cli_boolean_overrides_toml() {
    let config =
        resolve(&["src", "--quiet=false"], Some("quiet = true")).expect("resolve should succeed");

    assert_eq!(config.quiet, false);
}

#[test]
fn output_format_falls_back_to_toml_then_empty() {
    let from_toml = resolve(&["src"], Some("output-format = [\"csv\", \"json\"]"))
        .expect("resolve should succeed");
    assert_eq!(
        from_toml.output_format,
        vec![OutputFormat::Csv, OutputFormat::Json]
    );

    let without_toml = resolve(&["src"], None).expect("resolve should succeed");
    assert_eq!(without_toml.output_format, Vec::<OutputFormat>::new());
}

#[test]
fn cli_output_format_overrides_toml() {
    let config = resolve(
        &["src", "--output-format", "json"],
        Some("output-format = [\"csv\"]"),
    )
    .expect("resolve should succeed");

    assert_eq!(config.output_format, vec![OutputFormat::Json]);
}

#[test]
fn exclude_flattened_from_toml() {
    let config = resolve(&["src"], Some("exclude = \"tests\"")).expect("resolve should succeed");

    assert_eq!(config.exclude, vec!["tests"]);
}

#[test]
fn diff_branch_from_toml() {
    let config =
        resolve(&["src"], Some("[diff]\nbranch = \"main\"")).expect("resolve should succeed");

    assert_eq!(config.diff, Some("main".to_string()));
}

#[test]
fn cli_diff_wins_over_toml_branch() {
    let config = resolve(
        &["src", "--diff", "develop"],
        Some("[diff]\nbranch = \"main\""),
    )
    .expect("resolve should succeed");

    assert_eq!(config.diff, Some("develop".to_string()));
}

#[test]
fn diff_only_blocks_toml_branch() {
    let config = resolve(
        &["src", "--diff-only", "develop"],
        Some("[diff]\nbranch = \"main\""),
    )
    .expect("resolve should succeed");

    assert_eq!(config.diff, None);
    assert_eq!(config.diff_only, Some("develop".to_string()));
}

#[test]
fn staged_from_toml_when_cli_absent() {
    let config = resolve(&["src"], Some("[diff]\nstaged = true")).expect("resolve should succeed");

    assert_eq!(config.staged, true);
}

#[test]
fn cli_staged_wins_over_toml() {
    let config = resolve(&["src", "--staged"], Some("[diff]\nstaged = false"))
        .expect("resolve should succeed");

    assert_eq!(config.staged, true);
}

#[test]
fn kebab_case_keys_deserialize() {
    let source = "max-complexity-allowed = 12
check-script = true
no-ignore = true
report-ignored = true
snapshot-create = true
snapshot-ignore = true
ignore-complexity = true
color = \"no\"
sort = \"desc\"
failed = true
output = \"results.json\"";
    let config: Config = toml::from_str(source).expect("toml should parse");

    assert_eq!(config.max_complexity_allowed, 12);
    assert_eq!(config.check_script, true);
    assert_eq!(config.no_ignore, true);
    assert_eq!(config.report_ignored, true);
    assert_eq!(config.snapshot_create, true);
    assert_eq!(config.snapshot_ignore, true);
    assert_eq!(config.ignore_complexity, true);
    assert_eq!(config.color, Color::No);
    assert_eq!(config.sort, Sort::Desc);
    assert_eq!(config.failed, true);
    assert_eq!(config.output, Some("results.json".to_string()));
}

#[test]
fn single_string_values_accepted() {
    let source = "paths = \"src\"
exclude = \"tests\"
output-format = \"csv\"";
    let config: Config = toml::from_str(source).expect("toml should parse");

    assert_eq!(config.paths.into_vec(), vec!["src".to_string()]);
    assert_eq!(config.exclude.into_vec(), vec!["tests".to_string()]);
    assert_eq!(config.output_format.into_vec(), vec![OutputFormat::Csv]);
}

#[test]
fn diff_section_deserializes() {
    let config: Config =
        toml::from_str("[diff]\nbranch = \"main\"\nstaged = true").expect("toml should parse");

    let section = config.diff.expect("diff section should exist");
    assert_eq!(section.branch, Some("main".to_string()));
    assert_eq!(section.staged, Some(true));
}

#[test]
fn top_and_version_ignored_in_toml() {
    let config =
        resolve(&["src"], Some("top = 5\nversion = true")).expect("resolve should succeed");

    assert_eq!(config.top, None);
}

#[test]
fn plain_and_suggest_refactors_default_false() {
    let config = resolve(&["src"], None).expect("resolve should succeed");

    assert_eq!(config.plain, false);
    assert_eq!(config.suggest_refactors, false);
}

#[test]
fn plain_conflicts_with_quiet() {
    let result = CliArgs::try_parse_from(["complexipy", "src", "--plain", "--quiet"]);

    assert!(result.is_err());
}

#[test]
fn top_must_be_positive() {
    assert!(CliArgs::try_parse_from(["complexipy", "src", "--top", "0"]).is_err());
    assert!(CliArgs::try_parse_from(["complexipy", "src", "--top", "-1"]).is_err());

    let parsed =
        CliArgs::try_parse_from(["complexipy", "src", "--top", "1"]).expect("top 1 should parse");
    assert_eq!(parsed.top, Some(1));
}

#[test]
fn comma_separated_values_parse() {
    let parsed = CliArgs::try_parse_from([
        "complexipy",
        "src",
        "--exclude",
        "tests/**,docs/**",
        "--output-format",
        "csv,json",
    ])
    .expect("comma separated values should parse");

    assert_eq!(parsed.exclude, vec!["tests/**", "docs/**"]);
    assert_eq!(
        parsed.output_format,
        Some(vec![OutputFormat::Csv, OutputFormat::Json])
    );
}
