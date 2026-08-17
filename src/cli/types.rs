use serde::Deserialize;

#[derive(Deserialize)]
pub struct RunConfig {
    paths: Vec<String>,
    max_complexity_allowed: u64,
    snapshot_create: bool,
    snapshot_ignore: bool,
    quiet: bool,
    ignore_complexity: bool,
    failed: bool,
    color: Color,
    sort: Sort,
    output_format: Vec<OutputFormat>,
    exclude: Vec<String>,
    check_script: bool,
    no_ignore: bool,
    report_ignored: bool,
    plain: bool,
    suggest_refactors: bool,
    top: Option<u64>,
    diff: Option<String>,
    diff_only: Option<String>,
    staged: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Color {
    auto,
    yes,
    no,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Sort {
    asc,
    desc,
    file_name,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutputFormat {
    csv,
    json,
    gitlab,
    sarif,
}

impl OutputFormat {
    pub fn default_output_filename(&self) -> String {
        let file_name = "complexipy-results";
        let extension = match self {
            Self::csv => "csv",
            Self::json => "json",
            Self::gitlab => "gitlab.json",
            Self::sarif => "sarif",
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
