pub use crate::classes::RefactorPlan;
use clap::ValueEnum;
use serde::Deserialize;

#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub struct Config {
    #[serde(default)]
    pub paths: StringOrList<String>,
    #[serde(default)]
    pub exclude: StringOrList<String>,
    #[serde(default = "default_max_complexity")]
    pub max_complexity_allowed: u64,
    #[serde(default)]
    pub snapshot_create: bool,
    #[serde(default)]
    pub snapshot_ignore: bool,
    #[serde(default)]
    pub quiet: bool,
    #[serde(default)]
    pub ignore_complexity: bool,
    #[serde(default)]
    pub failed: bool,
    #[serde(default)]
    pub color: Color,
    #[serde(default)]
    pub sort: Sort,
    #[serde(default)]
    pub output_format: StringOrList<OutputFormat>,
    pub output: Option<String>,
    pub diff: Option<DiffSection>,
    #[serde(default)]
    pub cache_dir: Option<toml::Value>,
    #[serde(default)]
    pub check_script: bool,
    #[serde(default)]
    pub no_ignore: bool,
    #[serde(default)]
    pub report_ignored: bool,
}

fn default_max_complexity() -> u64 {
    15
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct DiffSection {
    pub branch: Option<String>,
    pub staged: Option<bool>,
}

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

#[derive(Debug, Clone, PartialEq)]
pub struct RunConfig {
    pub paths: Vec<String>,
    pub max_complexity_allowed: u64,
    pub snapshot_create: bool,
    pub snapshot_ignore: bool,
    pub quiet: bool,
    pub ignore_complexity: bool,
    pub failed: bool,
    pub color: Color,
    pub sort: Sort,
    pub output_format: Vec<OutputFormat>,
    pub output: Option<String>,
    pub exclude: Vec<String>,
    pub cache_dir: Option<String>,
    pub check_script: bool,
    pub no_ignore: bool,
    pub report_ignored: bool,
    pub plain: bool,
    pub suggest_refactors: bool,
    pub top: Option<u64>,
    pub diff: Option<String>,
    pub diff_only: Option<String>,
    pub staged: bool,
}

#[derive(Deserialize, Clone, Debug, PartialEq, Eq, ValueEnum, Default)]
#[serde(rename_all = "snake_case")]
pub enum Color {
    #[default]
    Auto,
    Yes,
    No,
}

#[derive(Deserialize, Clone, Debug, PartialEq, Eq, ValueEnum, Default)]
#[serde(rename_all = "snake_case")]
pub enum Sort {
    #[default]
    Asc,
    Desc,
    #[value(name = "file_name")]
    FileName,
}

#[derive(Deserialize, Clone, Debug, PartialEq, Eq, ValueEnum)]
#[serde(rename_all = "snake_case")]
pub enum OutputFormat {
    Csv,
    Json,
    Gitlab,
    Sarif,
}

impl OutputFormat {
    pub fn default_output_filename(&self) -> String {
        let file_name = "complexipy-results";
        let extension = match self {
            Self::Csv => "csv",
            Self::Json => "json",
            Self::Gitlab => "gitlab.json",
            Self::Sarif => "sarif",
        };
        format!("{}.{}", file_name, extension)
    }
}

pub struct ExitReport {
    display_ok: bool,
    snapshot_ok: bool,
    paths_ok: bool,
    diff_ok: bool,
    enforce_diff: bool,
}

impl ExitReport {
    pub fn success(&self) -> bool {
        if self.enforce_diff {
            self.diff_ok && self.paths_ok && self.snapshot_ok
        } else {
            self.display_ok && self.snapshot_ok && self.paths_ok
        }
    }
}

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum DiffStatus {
    Regressed,
    Improved,
    Unchanged,
    New,
    Removed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffEntry {
    pub file_path: String,
    pub func_name: String,
    pub old_complexity: Option<u64>,
    pub new_complexity: Option<u64>,
}

impl DiffEntry {
    pub fn status(&self) -> DiffStatus {
        match (self.old_complexity, self.new_complexity) {
            (None, ..) => DiffStatus::New,
            (.., None) => DiffStatus::Removed,
            (Some(old), Some(new)) => {
                if new > old {
                    DiffStatus::Regressed
                } else if new < old {
                    DiffStatus::Improved
                } else {
                    DiffStatus::Unchanged
                }
            }
        }
    }

    pub fn delta(&self) -> Option<i64> {
        match (self.old_complexity, self.new_complexity) {
            (None, ..) => None,
            (.., None) => None,
            (Some(old), Some(new)) => Some(new as i64 - old as i64),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionRow {
    pub name: String,
    pub complexity: u64,
    pub passed: bool,
    pub path: String,
    pub file_name: String,
    pub refactor_plans: Vec<RefactorPlan>,
    pub additional_refactor_plans: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileEntry {
    pub path: String,
    pub functions: Vec<FunctionRow>,
}
