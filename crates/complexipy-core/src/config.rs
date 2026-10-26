use std::io;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde::de::DeserializeOwned;

use crate::fs_guard::{MAX_CONFIG_BYTES, read_bounded_text};
use crate::rules::RuleSet;

pub const DEFAULT_MAX_COMPLEXITY_ALLOWED: u64 = 15;

#[derive(Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, serde::Serialize))]
#[cfg_attr(feature = "schema", schemars(inline))]
#[serde(untagged)]
pub enum StringOrList<T> {
    One(T),
    Many(Vec<T>),
}

impl<T> Default for StringOrList<T> {
    fn default() -> Self {
        Self::Many(Vec::new())
    }
}

impl<T> StringOrList<T> {
    pub fn into_vec(self) -> Vec<T> {
        match self {
            Self::One(value) => vec![value],
            Self::Many(values) => values,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigFileKind {
    ComplexipyToml,
    DotComplexipyToml,
    PyprojectToml,
}

#[derive(Debug, Clone)]
pub struct ConfigCandidate {
    pub path: PathBuf,
    pub kind: ConfigFileKind,
    pub root: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct ConfigSource<T> {
    pub path: PathBuf,
    pub value: T,
}

fn default_max_complexity_allowed() -> u64 {
    DEFAULT_MAX_COMPLEXITY_ALLOWED
}

fn default_true() -> bool {
    true
}

#[derive(Deserialize, Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, serde::Serialize))]
#[serde(rename_all = "snake_case")]
pub enum InlayHints {
    Always,
    #[default]
    Threshold,
    Never,
}

#[derive(Deserialize, Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, serde::Serialize))]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticRange {
    #[default]
    Name,
    Function,
}

#[derive(Deserialize, Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, serde::Serialize))]
#[serde(rename_all = "snake_case")]
pub enum HoverScope {
    #[default]
    Signature,
    Function,
    Never,
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, serde::Serialize))]
#[cfg_attr(
    feature = "schema",
    schemars(
        description = "Settings for the `complexipy lsp` language server.",
        deny_unknown_fields
    )
)]
#[serde(rename_all = "kebab-case")]
pub struct LspSection {
    #[serde(default)]
    #[cfg_attr(
        feature = "schema",
        schemars(
            description = "When to show the per-function hint: `threshold` only above `max-complexity-allowed`, `always` for every function, `never` for no hint."
        )
    )]
    pub inlay_hints: InlayHints,
    #[serde(default)]
    #[cfg_attr(
        feature = "schema",
        schemars(
            description = "Also show the complexity added by each line, as `+2`. Lines that add nothing are skipped."
        )
    )]
    pub per_line_hints: bool,
    #[serde(default = "default_true")]
    #[cfg_attr(
        feature = "schema",
        schemars(
            description = "Publish a warning for every function above `max-complexity-allowed`."
        )
    )]
    pub diagnostics: bool,
    #[serde(default)]
    #[cfg_attr(
        feature = "schema",
        schemars(
            description = "What the warning underlines: `name` the function name only, `function` the whole function."
        )
    )]
    pub diagnostic_range: DiagnosticRange,
    #[serde(default)]
    #[cfg_attr(
        feature = "schema",
        schemars(
            description = "Where hover answers: `signature` on the `def` lines only, `function` on any line of the function, `never` for no hover."
        )
    )]
    pub hover: HoverScope,
}

impl Default for LspSection {
    fn default() -> Self {
        Self {
            inlay_hints: InlayHints::default(),
            per_line_hints: false,
            diagnostics: true,
            diagnostic_range: DiagnosticRange::default(),
            hover: HoverScope::default(),
        }
    }
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub struct LspConfig {
    #[serde(default = "default_max_complexity_allowed")]
    pub max_complexity_allowed: u64,
    #[serde(default)]
    pub paths: StringOrList<String>,
    #[serde(default)]
    pub exclude: StringOrList<String>,
    #[serde(default)]
    pub no_ignore: bool,
    #[serde(default)]
    pub select: StringOrList<String>,
    #[serde(default)]
    pub ignore: StringOrList<String>,
    #[serde(default)]
    pub lsp: LspSection,
}

impl LspConfig {
    pub fn rule_set(&self) -> RuleSet {
        self.resolve_rules().0
    }

    pub fn resolve_rules(&self) -> (RuleSet, Vec<String>) {
        RuleSet::resolve(
            &self.select.clone().into_vec(),
            &self.ignore.clone().into_vec(),
            &crate::rules::registered_rule_ids(),
        )
    }
}

impl Default for LspConfig {
    fn default() -> Self {
        Self {
            max_complexity_allowed: DEFAULT_MAX_COMPLEXITY_ALLOWED,
            paths: StringOrList::default(),
            exclude: StringOrList::default(),
            no_ignore: false,
            select: StringOrList::default(),
            ignore: StringOrList::default(),
            lsp: LspSection::default(),
        }
    }
}

const CONFIG_FILES: [(&str, ConfigFileKind); 3] = [
    ("complexipy.toml", ConfigFileKind::ComplexipyToml),
    (".complexipy.toml", ConfigFileKind::DotComplexipyToml),
    ("pyproject.toml", ConfigFileKind::PyprojectToml),
];

pub fn config_file_names() -> impl Iterator<Item = &'static str> {
    CONFIG_FILES.iter().map(|(file_name, _)| *file_name)
}

pub fn is_config_file_name(name: &str) -> bool {
    config_file_names().any(|file_name| file_name == name)
}

pub fn config_candidates(invocation_path: &str) -> Vec<ConfigCandidate> {
    let invocation_path = Path::new(invocation_path);

    CONFIG_FILES
        .into_iter()
        .filter_map(|(file_name, kind)| {
            let path = invocation_path.join(file_name);
            path.exists().then(|| ConfigCandidate {
                path,
                kind,
                root: Some(invocation_path.to_path_buf()),
            })
        })
        .collect()
}

/// Reads and parses one candidate. A candidate that is not a regular file,
/// that resolves outside its directory, or that is oversized is an error,
/// and no error repeats the file's contents.
pub fn read_candidate_value(candidate: &ConfigCandidate) -> Result<Option<toml::Value>, String> {
    let content =
        match read_bounded_text(&candidate.path, candidate.root.as_deref(), MAX_CONFIG_BYTES) {
            Ok(content) => content,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(format!("Skipped {}: {}", candidate.path.display(), error)),
        };
    let value = toml::from_str(&content).map_err(|error| {
        format!(
            "Failed to parse {}: {}",
            candidate.path.display(),
            syntax_error_location(&content, &error)
        )
    })?;

    Ok(match candidate.kind {
        ConfigFileKind::ComplexipyToml | ConfigFileKind::DotComplexipyToml => Some(value),
        ConfigFileKind::PyprojectToml => section_of(value),
    })
}

/// Reads the first config file of the invocation directory that parses and
/// deserializes into `T`, as the CLI and the server both do. Every file
/// skipped on the way, and every later file it shadows, is a warning: files
/// are never merged.
pub fn read_complexipy_config<T: DeserializeOwned>(
    invocation_path: &str,
) -> (Option<ConfigSource<T>>, Vec<String>) {
    let mut warnings = Vec::new();
    let mut candidates = config_candidates(invocation_path).into_iter();
    let mut found = None;

    for candidate in candidates.by_ref() {
        match read_candidate_value(&candidate).and_then(|value| deserialize(&candidate, value)) {
            Ok(Some(value)) => {
                found = Some(ConfigSource {
                    path: candidate.path,
                    value,
                });
                break;
            }
            Ok(None) => {}
            Err(error) => warnings.push(error),
        }
    }

    if let Some(source) = &found {
        for shadowed in
            candidates.filter(|candidate| matches!(read_candidate_value(candidate), Ok(Some(_))))
        {
            warnings.push(format!(
                "{} takes precedence over {}; config files are not merged, so {} is not applied",
                source.path.display(),
                shadowed.path.display(),
                shadowed.path.display()
            ));
        }
    }

    (found, warnings)
}

/// Reads one config file named on the command line. A file named
/// `pyproject.toml` contributes its `[tool.complexipy]` section; any other
/// file is read whole.
pub fn read_config_file<T: DeserializeOwned>(path: &Path) -> Result<ConfigSource<T>, String> {
    let kind = match path.file_name().and_then(|name| name.to_str()) {
        Some("pyproject.toml") => ConfigFileKind::PyprojectToml,
        _ => ConfigFileKind::ComplexipyToml,
    };
    let candidate = ConfigCandidate {
        path: path.to_path_buf(),
        kind,
        root: None,
    };
    if !candidate.path.is_file() {
        return Err(format!("Config file {} does not exist", path.display()));
    }
    let value = read_candidate_value(&candidate)
        .and_then(|value| deserialize(&candidate, value))?
        .ok_or_else(|| format!("{} has no [tool.complexipy] section", path.display()))?;

    Ok(ConfigSource {
        path: candidate.path,
        value,
    })
}

fn deserialize<T: DeserializeOwned>(
    candidate: &ConfigCandidate,
    value: Option<toml::Value>,
) -> Result<Option<T>, String> {
    let Some(value) = value else {
        return Ok(None);
    };
    match value.clone().try_into::<T>() {
        Ok(config) => Ok(Some(config)),
        Err(_) => Err(format!(
            "Invalid config in {}: {}",
            candidate.path.display(),
            invalid_keys::<T>(&value)
        )),
    }
}

fn syntax_error_location(content: &str, error: &toml::de::Error) -> String {
    let Some(span) = error.span() else {
        return "TOML syntax error".to_string();
    };
    let before = content.get(..span.start).unwrap_or(content);
    let line = before.matches('\n').count() + 1;
    let column = before
        .rsplit('\n')
        .next()
        .map_or(0, |text| text.chars().count())
        + 1;
    format!("TOML syntax error at line {}, column {}", line, column)
}

fn invalid_keys<T: DeserializeOwned>(value: &toml::Value) -> String {
    let keys: Vec<String> = value
        .as_table()
        .into_iter()
        .flatten()
        .filter(|(key, item)| {
            let mut single = toml::Table::new();
            single.insert(key.to_string(), (*item).clone());
            toml::Value::Table(single).try_into::<T>().is_err()
        })
        .map(|(key, _)| format!("{:?}", key))
        .collect();
    if keys.is_empty() {
        "the settings are not a table of known keys".to_string()
    } else {
        format!("invalid value for {}", keys.join(", "))
    }
}

fn section_of(value: toml::Value) -> Option<toml::Value> {
    Some(value.get("tool")?.get("complexipy")?.clone())
}
