use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

pub const DEFAULT_MAX_COMPLEXITY_ALLOWED: u64 = 15;

#[derive(Deserialize, Debug, Clone, PartialEq)]
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
#[serde(rename_all = "snake_case")]
pub enum InlayHints {
    Always,
    #[default]
    Threshold,
    Never,
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub struct LspSection {
    #[serde(default)]
    pub inlay_hints: InlayHints,
    #[serde(default)]
    pub per_line_hints: bool,
    #[serde(default = "default_true")]
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
    pub lsp: LspSection,
}

impl Default for LspConfig {
    fn default() -> Self {
        Self {
            max_complexity_allowed: DEFAULT_MAX_COMPLEXITY_ALLOWED,
            exclude: StringOrList::default(),
            no_ignore: false,
            lsp: LspSection::default(),
        }
    }
}

pub fn config_candidates(invocation_path: &str) -> Vec<ConfigCandidate> {
    let invocation_path = Path::new(invocation_path);

    [
        ("complexipy.toml", ConfigFileKind::ComplexipyToml),
        (".complexipy.toml", ConfigFileKind::DotComplexipyToml),
        ("pyproject.toml", ConfigFileKind::PyprojectToml),
    ]
    .into_iter()
    .filter_map(|(file_name, kind)| {
        let path = invocation_path.join(file_name);
        path.exists().then_some(ConfigCandidate { path, kind })
    })
    .collect()
}

pub fn load_candidate_value(candidate: &ConfigCandidate) -> Option<toml::Value> {
    let content = fs::read_to_string(&candidate.path).ok()?;

    match candidate.kind {
        ConfigFileKind::ComplexipyToml | ConfigFileKind::DotComplexipyToml => {
            parse_toml(&content, &candidate.path)
        }
        ConfigFileKind::PyprojectToml => section_of(parse_toml(&content, &candidate.path)?),
    }
}

pub fn read_complexipy_config(invocation_path: &str) -> Option<ConfigSource> {
    config_candidates(invocation_path)
        .into_iter()
        .find_map(|candidate| {
            load_candidate_value(&candidate).map(|value| ConfigSource {
                path: candidate.path,
                value,
            })
        })
}

fn parse_toml(content: &str, path: &Path) -> Option<toml::Value> {
    match toml::from_str(content) {
        Ok(value) => Some(value),
        Err(e) => {
            eprintln!("Failed to parse {}: {}", path.display(), e);
            None
        }
    }
}

fn section_of(value: toml::Value) -> Option<toml::Value> {
    Some(value.get("tool")?.get("complexipy")?.clone())
}
