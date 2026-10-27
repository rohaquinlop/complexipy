use clap::ValueEnum;
pub use complexipy_core::classes::RefactorPlan;
use complexipy_core::config::DEFAULT_MAX_COMPLEXITY_ALLOWED;
pub use complexipy_core::config::StringOrList;
use serde::Deserialize;

#[derive(Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(test, derive(schemars::JsonSchema, serde::Serialize))]
#[cfg_attr(
    test,
    schemars(
        title = "complexipy",
        description = "Configuration for complexipy, read from complexipy.toml, .complexipy.toml, or [tool.complexipy] in pyproject.toml.",
        deny_unknown_fields
    )
)]
#[serde(rename_all = "kebab-case")]
pub struct Config {
    #[serde(default)]
    #[cfg_attr(test, schemars(description = "Files or directories to analyze."))]
    pub paths: StringOrList<String>,
    #[serde(default)]
    #[cfg_attr(
        test,
        schemars(description = "Glob patterns of files to skip, such as `tests/**`.")
    )]
    pub exclude: StringOrList<String>,
    #[serde(default)]
    #[cfg_attr(
        test,
        schemars(description = "Report only these refactoring rules, such as `C001`.")
    )]
    pub select: StringOrList<String>,
    #[serde(default)]
    #[cfg_attr(
        test,
        schemars(
            description = "Never report these refactoring rules, such as `C007`. Wins over `select`."
        )
    )]
    pub ignore: StringOrList<String>,
    #[serde(default = "default_max_complexity")]
    #[cfg_attr(
        test,
        schemars(
            description = "Functions strictly above this complexity are reported. A function at the limit passes."
        )
    )]
    pub max_complexity_allowed: u64,
    #[serde(default)]
    #[cfg_attr(
        test,
        schemars(
            description = "Save the current violations above the threshold into `complexipy-snapshot.json`."
        )
    )]
    pub snapshot_create: bool,
    #[serde(default)]
    #[cfg_attr(
        test,
        schemars(description = "Skip comparing against the snapshot even if it exists.")
    )]
    pub snapshot_ignore: bool,
    #[serde(default)]
    #[cfg_attr(test, schemars(description = "Suppress output."))]
    pub quiet: bool,
    #[serde(default)]
    #[cfg_attr(
        test,
        schemars(description = "Do not exit with an error when the threshold is breached.")
    )]
    pub ignore_complexity: bool,
    #[serde(default)]
    #[cfg_attr(
        test,
        schemars(description = "Show only functions above the complexity threshold.")
    )]
    pub failed: bool,
    #[serde(default)]
    #[cfg_attr(test, schemars(description = "Use color in the output."))]
    pub color: Color,
    #[serde(default)]
    #[cfg_attr(test, schemars(description = "Sort order of the results."))]
    pub sort: Sort,
    #[serde(default)]
    #[cfg_attr(
        test,
        schemars(description = "Machine-readable output formats to write.")
    )]
    pub output_format: StringOrList<OutputFormat>,
    #[cfg_attr(
        test,
        schemars(
            description = "File or directory for machine-readable output. Use a directory when writing more than one format."
        )
    )]
    pub output: Option<String>,
    #[cfg_attr(test, schemars(description = "Default complexity diff settings."))]
    pub diff: Option<DiffSection>,
    #[serde(default)]
    #[cfg_attr(
        test,
        schemars(
            with = "Option<String>",
            description = "Directory for the analysis cache. Defaults to `.complexipy_cache/` in the invocation directory."
        )
    )]
    pub cache_dir: Option<toml::Value>,
    #[serde(default)]
    #[cfg_attr(
        test,
        schemars(
            description = "Report module-level (script) complexity as a synthetic `<module>` entry."
        )
    )]
    pub check_script: bool,
    #[serde(default)]
    #[cfg_attr(
        test,
        schemars(description = "Analyze every function, disregarding inline ignore comments.")
    )]
    pub no_ignore: bool,
    #[serde(default)]
    #[cfg_attr(
        test,
        schemars(
            description = "List every file and line where an ignore comment suppresses a function."
        )
    )]
    pub report_ignored: bool,
}

fn default_max_complexity() -> u64 {
    DEFAULT_MAX_COMPLEXITY_ALLOWED
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(test, derive(schemars::JsonSchema, serde::Serialize))]
#[cfg_attr(test, schemars(deny_unknown_fields))]
pub struct DiffSection {
    #[cfg_attr(
        test,
        schemars(
            description = "Default git reference for `--diff` and `--diff-only`. An empty string disables the diff."
        )
    )]
    pub branch: Option<String>,
    #[cfg_attr(
        test,
        schemars(description = "Compare staged changes by default, like `--staged`.")
    )]
    pub staged: Option<bool>,
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
    pub select: Vec<String>,
    pub ignore: Vec<String>,
    pub cache_dir: Option<String>,
    pub check_script: bool,
    pub no_ignore: bool,
    pub report_ignored: bool,
    pub plain: bool,
    pub suggest_refactors: bool,
    pub fix: bool,
    pub dry_run: bool,
    pub top: Option<u64>,
    pub diff: Option<String>,
    pub diff_only: Option<String>,
    pub staged: bool,
    pub gate_changes_from_config: Vec<ConfigGateChange>,
}

/// A config file key that changes how the exit code is decided.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigGateChange {
    DiffRef(String),
    StagedDiff,
    SnapshotCreate,
}

#[derive(Deserialize, Clone, Debug, PartialEq, Eq, ValueEnum, Default)]
#[cfg_attr(test, derive(schemars::JsonSchema, serde::Serialize))]
#[serde(rename_all = "snake_case")]
pub enum Color {
    #[default]
    Auto,
    Yes,
    No,
}

#[derive(Deserialize, Clone, Debug, PartialEq, Eq, ValueEnum, Default)]
#[cfg_attr(test, derive(schemars::JsonSchema, serde::Serialize))]
#[serde(rename_all = "snake_case")]
pub enum Sort {
    #[default]
    Asc,
    Desc,
    #[value(name = "file_name")]
    FileName,
}

#[derive(Deserialize, Clone, Debug, PartialEq, Eq, ValueEnum)]
#[cfg_attr(test, derive(schemars::JsonSchema, serde::Serialize))]
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExitReport {
    pub display_ok: bool,
    pub snapshot_ok: bool,
    pub paths_ok: bool,
    pub diff_ok: bool,
    pub enforce_diff: bool,
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

/// What a run's diff knows about each function, by file path and function
/// key.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DiffBaseline {
    /// The complexity each function compares with at the ref, `None` for a
    /// new function.
    pub complexities: std::collections::HashMap<(String, String), Option<u64>>,
    /// When the diff decides the exit code, the functions that fail the run,
    /// as the report's rows judge them.
    pub failing: Option<std::collections::HashSet<(String, String)>>,
}

impl DiffBaseline {
    /// The SARIF `baselineState` of a function: `new`, `updated`, or
    /// `unchanged`, which a function in a file the diff found unchanged also
    /// gets.
    pub fn state(&self, path: &str, key: &str, complexity: u64) -> &'static str {
        match self.complexities.get(&(path.to_string(), key.to_string())) {
            Some(None) => "new",
            Some(Some(previous)) if *previous != complexity => "updated",
            _ => "unchanged",
        }
    }

    /// Whether the function fails the run, or `None` when the diff does not
    /// decide the exit code.
    pub fn fails(&self, path: &str, key: &str) -> Option<bool> {
        let failing = self.failing.as_ref()?;
        Some(failing.contains(&(path.to_string(), key.to_string())))
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

#[cfg(test)]
mod tests;
