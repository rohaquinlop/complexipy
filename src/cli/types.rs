pub use crate::classes::RefactorPlan;
use clap::ValueEnum;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Config {
    paths: Vec<String>,
    #[serde(default)]
    exclude: Vec<String>,
    #[serde(default = "default_max_complexity")]
    max_complexity_allowed: u64,
    #[serde(default)]
    snapshot_create: bool,
    #[serde(default)]
    snapshot_ignore: bool,
    #[serde(default)]
    failed: bool,
    #[serde(default)]
    suggest_refactors: bool,
    #[serde(default)]
    color: Color,
    #[serde(default)]
    sort: Sort,
    #[serde(default)]
    quiet: bool,
    #[serde(default)]
    ignore_complexity: bool,
    #[serde(default)]
    version: bool,
    top: Option<u64>,
    #[serde(default)]
    plain: bool,
    #[serde(default)]
    output_format: Vec<OutputFormat>,
    output: Option<String>,
    diff: Option<String>,
    diff_only: Option<String>,
    #[serde(default)]
    staged: bool,
    #[serde(default)]
    check_script: bool,
    #[serde(default)]
    no_ignore: bool,
    #[serde(default)]
    report_ignored: bool,
}

fn default_max_complexity() -> u64 {
    15
}

#[derive(Deserialize, Clone, ValueEnum)]
#[serde(rename_all = "snake_case")]
pub enum Color {
    Auto,
    Yes,
    No,
}

impl Default for Color {
    fn default() -> Self {
        Self::Auto
    }
}

#[derive(Deserialize, Clone, ValueEnum)]
#[serde(rename_all = "snake_case")]
pub enum Sort {
    Asc,
    Desc,
    File_name,
}

impl Default for Sort {
    fn default() -> Self {
        Self::Asc
    }
}

#[derive(Deserialize, Clone, ValueEnum)]
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

pub enum DiffStatus {
    REGRESSED,
    IMPROVED,
    UNCHANGED,
    NEW,
    REMOVED,
}

pub struct DiffEntry {
    file_path: String,
    func_name: String,
    old_complexity: Option<u64>,
    new_complexity: Option<u64>,
}

impl DiffEntry {
    pub fn status(&self) -> DiffStatus {
        match (self.old_complexity, self.new_complexity) {
            (None, ..) => DiffStatus::NEW,
            (.., None) => DiffStatus::REMOVED,
            (Some(old), Some(new)) => {
                if new > old {
                    DiffStatus::REGRESSED
                } else if new < old {
                    DiffStatus::IMPROVED
                } else {
                    DiffStatus::UNCHANGED
                }
            }
        }
    }

    pub fn delta(&self) -> Option<u64> {
        match (self.old_complexity, self.new_complexity) {
            (None, ..) => None,
            (.., None) => None,
            (Some(old), Some(new)) => Some(new - old),
        }
    }
}

pub struct FunctionRow {
    name: String,
    complexity: u64,
    passed: bool,
    path: String,
    file_name: String,
    refactor_plans: Vec<RefactorPlan>,
    additional_refactor_plans: u64,
}

pub struct FileEntry {
    path: String,
    functions: Vec<FunctionRow>,
}
