#![cfg(feature = "config")]
//! Tests for config file discovery and the [tool.complexipy.lsp] section.

use complexipy_core::config::{
    ConfigFileKind, ConfigSource, InlayHints, LspConfig, config_candidates, is_config_file_name,
    read_complexipy_config,
};

use std::fs as fs_sync;
use tempfile::TempDir;

fn write(dir: &TempDir, name: &str, content: &str) {
    fs_sync::write(dir.path().join(name), content).unwrap();
}

fn source(dir: &TempDir) -> Option<ConfigSource> {
    read_complexipy_config(&dir.path().to_string_lossy()).0
}

#[test]
fn lsp_config_defaults_without_section() {
    let config: LspConfig = toml::from_str("max-complexity-allowed = 20").unwrap();

    assert_eq!(config.max_complexity_allowed, 20);
    assert_eq!(config.lsp.inlay_hints, InlayHints::Threshold);
    assert!(!config.lsp.per_line_hints);
    assert!(config.lsp.diagnostics);
    assert!(!config.no_ignore);
}

#[test]
fn lsp_config_rust_default_matches_the_documented_threshold() {
    let config = LspConfig::default();

    assert_eq!(config.max_complexity_allowed, 15);
    assert_eq!(config.lsp.inlay_hints, InlayHints::Threshold);
    assert!(!config.lsp.per_line_hints);
    assert!(config.lsp.diagnostics);
}

#[test]
fn lsp_config_defaults_on_empty_table() {
    let config: LspConfig = toml::from_str("").unwrap();

    assert_eq!(config.max_complexity_allowed, 15);
    assert_eq!(config.exclude.into_vec(), Vec::<String>::new());
}

#[test]
fn lsp_config_reads_kebab_case_keys() {
    let config: LspConfig = toml::from_str(
        r#"
        max-complexity-allowed = 12
        exclude = ["legacy/*"]
        no-ignore = true

        [lsp]
        inlay-hints = "always"
        per-line-hints = true
        diagnostics = false
        "#,
    )
    .unwrap();

    assert_eq!(config.max_complexity_allowed, 12);
    assert_eq!(config.exclude.into_vec(), vec!["legacy/*".to_string()]);
    assert!(config.no_ignore);
    assert_eq!(config.lsp.inlay_hints, InlayHints::Always);
    assert!(config.lsp.per_line_hints);
    assert!(!config.lsp.diagnostics);
}

#[test]
fn lsp_config_ignores_unknown_keys() {
    let config: LspConfig = toml::from_str(
        r#"
        snapshots = true

        [lsp]
        unknown-key = 1
        inlay-hints = "never"
        "#,
    )
    .unwrap();

    assert_eq!(config.lsp.inlay_hints, InlayHints::Never);
}

#[test]
fn lsp_config_rejects_unknown_inlay_hint_value() {
    let result: Result<LspConfig, _> = toml::from_str("[lsp]\ninlay-hints = \"sometimes\"");

    assert!(result.is_err());
}

#[test]
fn candidates_follow_the_documented_order() {
    let dir = TempDir::new().unwrap();
    write(&dir, "complexipy.toml", "");
    write(&dir, ".complexipy.toml", "");
    write(&dir, "pyproject.toml", "");

    let candidates = config_candidates(&dir.path().to_string_lossy());

    let kinds: Vec<ConfigFileKind> = candidates.iter().map(|c| c.kind).collect();
    assert_eq!(
        kinds,
        vec![
            ConfigFileKind::ComplexipyToml,
            ConfigFileKind::DotComplexipyToml,
            ConfigFileKind::PyprojectToml
        ]
    );
}

#[test]
fn config_file_names_match_the_candidates() {
    assert!(is_config_file_name("complexipy.toml"));
    assert!(is_config_file_name(".complexipy.toml"));
    assert!(is_config_file_name("pyproject.toml"));
    assert!(!is_config_file_name("setup.cfg"));
    assert!(!is_config_file_name("sub/complexipy.toml"));
}

#[test]
fn reader_prefers_complexipy_toml() {
    let dir = TempDir::new().unwrap();
    write(&dir, "complexipy.toml", "max-complexity-allowed = 3");
    write(&dir, ".complexipy.toml", "max-complexity-allowed = 9");

    let source = source(&dir).unwrap();
    let config: LspConfig = source.value.try_into().unwrap();

    assert_eq!(config.max_complexity_allowed, 3);
}

#[test]
fn reader_falls_back_to_dot_variant() {
    let dir = TempDir::new().unwrap();
    write(&dir, ".complexipy.toml", "max-complexity-allowed = 9");

    let source = source(&dir).unwrap();
    let config: LspConfig = source.value.try_into().unwrap();

    assert_eq!(config.max_complexity_allowed, 9);
}

#[test]
fn reader_reads_pyproject_section() {
    let dir = TempDir::new().unwrap();
    write(
        &dir,
        "pyproject.toml",
        "[tool.complexipy]\nmax-complexity-allowed = 7\n\n[tool.other]\nx = 1\n",
    );

    let source = source(&dir).unwrap();
    let config: LspConfig = source.value.try_into().unwrap();

    assert_eq!(config.max_complexity_allowed, 7);
}

#[test]
fn reader_returns_none_without_config_files() {
    let dir = TempDir::new().unwrap();

    assert!(source(&dir).is_none());
}

#[test]
fn reader_returns_none_without_pyproject_section() {
    let dir = TempDir::new().unwrap();
    write(&dir, "pyproject.toml", "[tool.ruff]\nline-length = 80\n");

    assert!(source(&dir).is_none());
}

#[test]
fn reader_skips_malformed_file() {
    let dir = TempDir::new().unwrap();
    write(&dir, "complexipy.toml", "max-complexity-allowed = ");
    write(&dir, ".complexipy.toml", "max-complexity-allowed = 11");

    let (source, errors) = read_complexipy_config(&dir.path().to_string_lossy());
    let config: LspConfig = source.unwrap().value.try_into().unwrap();

    assert_eq!(config.max_complexity_allowed, 11);
    assert_eq!(errors.len(), 1);
    assert!(errors[0].starts_with("Failed to parse "));
    assert!(errors[0].contains("complexipy.toml"));
}

#[test]
fn reader_reports_a_malformed_file_without_a_fallback() {
    let dir = TempDir::new().unwrap();
    write(&dir, "complexipy.toml", "max-complexity-allowed = ");

    let (source, errors) = read_complexipy_config(&dir.path().to_string_lossy());

    assert!(source.is_none());
    assert_eq!(errors.len(), 1);
}

#[test]
fn readers_agree_on_all_three_sources() {
    let dir = TempDir::new().unwrap();

    write(&dir, "complexipy.toml", "max-complexity-allowed = 4");
    let direct = source(&dir).unwrap();
    let direct: LspConfig = direct.value.try_into().unwrap();
    assert_eq!(direct.max_complexity_allowed, 4);
    fs_sync::remove_file(dir.path().join("complexipy.toml")).unwrap();

    write(&dir, ".complexipy.toml", "max-complexity-allowed = 4");
    let dot = source(&dir).unwrap();
    let dot: LspConfig = dot.value.try_into().unwrap();
    assert_eq!(dot, direct);
    fs_sync::remove_file(dir.path().join(".complexipy.toml")).unwrap();

    write(
        &dir,
        "pyproject.toml",
        "[tool.complexipy]\nmax-complexity-allowed = 4",
    );
    let pyproject = source(&dir).unwrap();
    let pyproject: LspConfig = pyproject.value.try_into().unwrap();
    assert_eq!(pyproject, direct);
}
