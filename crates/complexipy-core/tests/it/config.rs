#![cfg(feature = "config")]
//! Tests for config file discovery and the [tool.complexipy.lsp] section.

use complexipy_core::config::{
    ConfigFileKind, ConfigSource, DiagnosticRange, HoverScope, InlayHints, LspConfig,
    config_candidates, is_config_file_name, read_complexipy_config, read_config_file,
};

use std::fs as fs_sync;
use tempfile::TempDir;

fn write(dir: &TempDir, name: &str, content: &str) {
    fs_sync::write(dir.path().join(name), content).unwrap();
}

fn source(dir: &TempDir) -> Option<ConfigSource<LspConfig>> {
    read_complexipy_config(&dir.path().to_string_lossy()).0
}

#[test]
fn lsp_config_defaults_without_section() {
    let config: LspConfig = toml::from_str("max-complexity-allowed = 20").unwrap();

    assert_eq!(config.max_complexity_allowed, 20);
    assert_eq!(config.lsp.inlay_hints, InlayHints::Threshold);
    assert!(!config.lsp.per_line_hints);
    assert!(config.lsp.diagnostics);
    assert_eq!(config.lsp.diagnostic_range, DiagnosticRange::Name);
    assert_eq!(config.lsp.hover, HoverScope::Signature);
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
        diagnostic-range = "function"
        hover = "never"
        "#,
    )
    .unwrap();

    assert_eq!(config.max_complexity_allowed, 12);
    assert_eq!(config.exclude.into_vec(), vec!["legacy/*".to_string()]);
    assert!(config.no_ignore);
    assert_eq!(config.lsp.inlay_hints, InlayHints::Always);
    assert!(config.lsp.per_line_hints);
    assert!(!config.lsp.diagnostics);
    assert_eq!(config.lsp.diagnostic_range, DiagnosticRange::Function);
    assert_eq!(config.lsp.hover, HoverScope::Never);
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
fn lsp_config_rejects_unknown_enum_values() {
    for key in ["inlay-hints", "diagnostic-range", "hover"] {
        let source = format!("[lsp]\n{} = \"sometimes\"", key);
        let result: Result<LspConfig, _> = toml::from_str(&source);

        assert!(result.is_err(), "{} accepted an unknown value", key);
    }
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

    let config = source(&dir).unwrap().value;

    assert_eq!(config.max_complexity_allowed, 3);
}

#[test]
fn reader_falls_back_to_dot_variant() {
    let dir = TempDir::new().unwrap();
    write(&dir, ".complexipy.toml", "max-complexity-allowed = 9");

    let config = source(&dir).unwrap().value;

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

    let config = source(&dir).unwrap().value;

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

    let (source, errors) = read_complexipy_config::<LspConfig>(&dir.path().to_string_lossy());
    let config = source.unwrap().value;

    assert_eq!(config.max_complexity_allowed, 11);
    assert_eq!(errors.len(), 1);
    assert!(errors[0].starts_with("Failed to parse "));
    assert!(errors[0].contains("complexipy.toml"));
}

#[test]
fn reader_reports_a_malformed_file_without_a_fallback() {
    let dir = TempDir::new().unwrap();
    write(&dir, "complexipy.toml", "max-complexity-allowed = ");

    let (source, errors) = read_complexipy_config::<LspConfig>(&dir.path().to_string_lossy());

    assert!(source.is_none());
    assert_eq!(errors.len(), 1);
}

#[test]
fn readers_agree_on_all_three_sources() {
    let dir = TempDir::new().unwrap();

    write(&dir, "complexipy.toml", "max-complexity-allowed = 4");
    let direct = source(&dir).unwrap().value;
    assert_eq!(direct.max_complexity_allowed, 4);
    fs_sync::remove_file(dir.path().join("complexipy.toml")).unwrap();

    write(&dir, ".complexipy.toml", "max-complexity-allowed = 4");
    let dot = source(&dir).unwrap().value;
    assert_eq!(dot, direct);
    fs_sync::remove_file(dir.path().join(".complexipy.toml")).unwrap();

    write(
        &dir,
        "pyproject.toml",
        "[tool.complexipy]\nmax-complexity-allowed = 4",
    );
    let pyproject = source(&dir).unwrap().value;
    assert_eq!(pyproject, direct);
}

#[test]
fn a_type_error_falls_through_to_the_next_file() {
    let dir = TempDir::new().unwrap();
    write(&dir, "complexipy.toml", "max-complexity-allowed = \"x\"");
    write(
        &dir,
        "pyproject.toml",
        "[tool.complexipy]\nmax-complexity-allowed = 7",
    );

    let (source, warnings) = read_complexipy_config::<LspConfig>(&dir.path().to_string_lossy());

    assert_eq!(source.unwrap().value.max_complexity_allowed, 7);
    assert_eq!(warnings.len(), 1);
    assert!(warnings[0].contains("complexipy.toml"));
}

#[test]
fn a_shadowed_pyproject_section_is_a_warning() {
    let dir = TempDir::new().unwrap();
    write(&dir, "complexipy.toml", "[lsp]\nper-line-hints = true");
    write(
        &dir,
        "pyproject.toml",
        "[tool.complexipy]\nno-ignore = true",
    );

    let (source, warnings) = read_complexipy_config::<LspConfig>(&dir.path().to_string_lossy());

    assert!(!source.unwrap().value.no_ignore);
    assert_eq!(warnings.len(), 1);
    assert!(warnings[0].contains("pyproject.toml is not applied"));
}

#[test]
fn a_pyproject_without_a_section_is_not_shadowed() {
    let dir = TempDir::new().unwrap();
    write(&dir, "complexipy.toml", "max-complexity-allowed = 3");
    write(&dir, "pyproject.toml", "[tool.ruff]\nline-length = 80");

    let (_, warnings) = read_complexipy_config::<LspConfig>(&dir.path().to_string_lossy());

    assert!(warnings.is_empty());
}

#[test]
fn a_named_config_file_is_read_whole_or_by_section() {
    let dir = TempDir::new().unwrap();
    write(&dir, "ci.toml", "max-complexity-allowed = 5");
    write(
        &dir,
        "pyproject.toml",
        "[tool.complexipy]\nmax-complexity-allowed = 6",
    );

    let whole: ConfigSource<LspConfig> = read_config_file(&dir.path().join("ci.toml")).unwrap();
    let section: ConfigSource<LspConfig> =
        read_config_file(&dir.path().join("pyproject.toml")).unwrap();

    assert_eq!(whole.value.max_complexity_allowed, 5);
    assert_eq!(section.value.max_complexity_allowed, 6);
}

#[test]
fn a_syntax_error_names_the_position_but_never_the_contents() {
    let dir = TempDir::new().unwrap();
    write(
        &dir,
        "complexipy.toml",
        "max-complexity-allowed = 2\naws_access_key_id = AKIAFAKE1234\n",
    );

    let (_, warnings) = read_complexipy_config::<LspConfig>(&dir.path().to_string_lossy());

    assert_eq!(warnings.len(), 1);
    assert!(warnings[0].ends_with("TOML syntax error at line 2, column 21"));
    assert!(!warnings[0].contains("AKIAFAKE1234"));
}

#[test]
fn a_type_error_names_the_key_but_never_the_value() {
    let dir = TempDir::new().unwrap();
    write(
        &dir,
        "complexipy.toml",
        "max-complexity-allowed = \"[Open](command:workbench.action.terminal.new)\"\n",
    );

    let (_, warnings) = read_complexipy_config::<LspConfig>(&dir.path().to_string_lossy());

    assert_eq!(warnings.len(), 1);
    assert!(warnings[0].ends_with("invalid value for \"max-complexity-allowed\""));
    assert!(!warnings[0].contains("command:"));
}

#[cfg(unix)]
#[test]
fn a_config_symlink_out_of_the_directory_is_skipped_unread() {
    let dir = TempDir::new().unwrap();
    let outside = TempDir::new().unwrap();
    fs_sync::write(
        outside.path().join("credentials"),
        "[default]\naws_access_key_id = AKIAFAKE1234\n",
    )
    .unwrap();
    std::os::unix::fs::symlink(
        outside.path().join("credentials"),
        dir.path().join("pyproject.toml"),
    )
    .unwrap();
    std::os::unix::fs::symlink("/dev/zero", dir.path().join("complexipy.toml")).unwrap();

    let (source, warnings) = read_complexipy_config::<LspConfig>(&dir.path().to_string_lossy());

    assert!(source.is_none());
    assert_eq!(warnings.len(), 2);
    assert!(
        warnings
            .iter()
            .all(|warning| warning.starts_with("Skipped "))
    );
    assert!(
        warnings
            .iter()
            .all(|warning| !warning.contains("AKIAFAKE1234"))
    );
}

#[test]
fn a_missing_named_config_file_is_an_error() {
    let dir = TempDir::new().unwrap();

    let result = read_config_file::<LspConfig>(&dir.path().join("missing.toml"));

    assert!(result.is_err());
}
