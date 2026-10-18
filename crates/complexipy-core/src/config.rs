use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

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
}

#[derive(Debug, Clone)]
pub struct ConfigSource {
    pub path: PathBuf,
    pub value: toml::Value,
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
}

impl Default for LspSection {
    fn default() -> Self {
        Self {
            inlay_hints: InlayHints::default(),
            per_line_hints: false,
            diagnostics: true,
        }
    }
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub struct LspConfig {
    #[serde(default = "default_max_complexity_allowed")]
    pub max_complexity_allowed: u64,
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
            path.exists().then_some(ConfigCandidate { path, kind })
        })
        .collect()
}

pub fn load_candidate_value(candidate: &ConfigCandidate) -> Option<toml::Value> {
    read_candidate_value(candidate).unwrap_or_else(|error| {
        eprintln!("{}", error);
        None
    })
}

pub fn read_candidate_value(candidate: &ConfigCandidate) -> Result<Option<toml::Value>, String> {
    let Ok(content) = fs::read_to_string(&candidate.path) else {
        return Ok(None);
    };
    let value = toml::from_str(&content)
        .map_err(|e| format!("Failed to parse {}: {}", candidate.path.display(), e))?;

    Ok(match candidate.kind {
        ConfigFileKind::ComplexipyToml | ConfigFileKind::DotComplexipyToml => Some(value),
        ConfigFileKind::PyprojectToml => section_of(value),
    })
}

pub fn read_complexipy_config(invocation_path: &str) -> (Option<ConfigSource>, Vec<String>) {
    let mut errors = Vec::new();

    for candidate in config_candidates(invocation_path) {
        match read_candidate_value(&candidate) {
            Ok(Some(value)) => {
                let source = ConfigSource {
                    path: candidate.path,
                    value,
                };
                return (Some(source), errors);
            }
            Ok(None) => {}
            Err(error) => errors.push(error),
        }
    }

    (None, errors)
}

fn section_of(value: toml::Value) -> Option<toml::Value> {
    Some(value.get("tool")?.get("complexipy")?.clone())
}
