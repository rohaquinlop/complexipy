use std::path::Path;

use serde_json::{Value, json};

use crate::types::DiffBaseline;
use crate::utils::fingerprint::{finding_fingerprint, function_ranks, plan_rank};
use crate::utils::paths::normalize_path;
use complexipy_core::classes::{Applicability, FileComplexity, FunctionComplexity, RefactorPlan};
use complexipy_core::diff::function_keys;
use complexipy_core::fs_guard::write_atomic;
use complexipy_core::utils::ExportError;

const CHECK_NAME: &str = "complexipy/cognitive-complexity";
const DEFAULT_SEVERITY: &str = "minor";

/// Writes the GitLab Code Quality report. When `baseline` comes from a diff
/// that decides the exit code, a function that fails the run is `major` and
/// one the run lets through is `info`.
pub fn store_gitlab(
    output_path: &str,
    files: &[FileComplexity],
    max_complexity: u64,
    suggest_refactors: bool,
    baseline: Option<&DiffBaseline>,
) -> Result<(), ExportError> {
    let mut report = Vec::new();

    for file in files {
        let normalized_path = normalize_path(&file.path, &file.file_name);
        let relative_path = normalized_path
            .strip_prefix("./")
            .unwrap_or(&normalized_path);
        let ranks = function_ranks(file);

        for (function, key) in file.functions.iter().zip(function_keys(&file.functions)) {
            let function_rank = ranks
                .get(&(function.name.as_str(), function.line_start))
                .copied()
                .unwrap_or(0);
            if function.complexity > max_complexity {
                report.push(json!({
                    "description": build_description(
                        &function.name,
                        function.complexity,
                        max_complexity,
                    ),
                    "check_name": CHECK_NAME,
                    "fingerprint": finding_fingerprint(
                        "CC001",
                        relative_path,
                        &function.name,
                        function_rank,
                        0,
                    ),
                    "severity": complexity_severity(baseline, &file.path, &key),
                    "location": {
                        "path": relative_path,
                        "lines": {"begin": function.line_start},
                    },
                }));
            }

            if suggest_refactors {
                for plan in &function.refactor_plans {
                    report.push(refactor_plan_issue(
                        relative_path,
                        function,
                        function_rank,
                        plan,
                    ));
                }
            }
        }
    }

    let serialized = serde_json::to_string_pretty(&report)
        .map_err(|e| ExportError::Serialize(format!("Failed to serialize GitLab report: {}", e)))?;
    write_atomic(
        Path::new(output_path),
        format!("{}\n", serialized).as_bytes(),
    )
    .map_err(|e| {
        ExportError::Io(format!(
            "Failed to write GitLab report to {}: {}",
            output_path, e
        ))
    })
}

fn complexity_severity(baseline: Option<&DiffBaseline>, path: &str, key: &str) -> &'static str {
    match baseline.and_then(|baseline| baseline.fails(path, key)) {
        Some(true) => "major",
        Some(false) => "info",
        None => DEFAULT_SEVERITY,
    }
}

fn build_description(function_name: &str, complexity: u64, max_complexity: u64) -> String {
    format!(
        "Function '{}' has cognitive complexity {} (max allowed: {}).",
        function_name, complexity, max_complexity
    )
}

fn refactor_plan_severity(applicability: &Applicability) -> &'static str {
    match applicability {
        Applicability::MachineApplicable => "minor",
        Applicability::MaybeIncorrect => "major",
        Applicability::Informational => "info",
    }
}

fn refactor_plan_issue(
    relative_path: &str,
    function: &FunctionComplexity,
    function_rank: usize,
    plan: &RefactorPlan,
) -> Value {
    json!({
        "description": format!("[{}] {}: {}", plan.rule_id, plan.title, plan.explanation),
        "check_name": format!("complexipy/{}", plan.rule_id.to_lowercase()),
        "fingerprint": finding_fingerprint(
            &plan.rule_id,
            relative_path,
            &function.name,
            function_rank,
            plan_rank(function, plan),
        ),
        "severity": refactor_plan_severity(&plan.applicability),
        "location": {
            "path": relative_path,
            "lines": {"begin": plan.line_start, "end": plan.line_end},
        },
    })
}

#[cfg(test)]
mod tests;
