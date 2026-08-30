use std::fs;

use serde_json::{Value, json};

use sha2::{Digest, Sha256};

use crate::utils::paths::normalize_path;
use complexipy_core::classes::{Applicability, FileComplexity, RefactorPlan};
use complexipy_core::utils::ExportError;

const CHECK_NAME: &str = "complexipy/cognitive-complexity";
const DEFAULT_SEVERITY: &str = "minor";

pub fn store_gitlab(
    output_path: &str,
    files: &[FileComplexity],
    max_complexity: u64,
    suggest_refactors: bool,
) -> Result<(), ExportError> {
    let mut report = Vec::new();

    for file in files {
        let normalized_path = normalize_path(&file.path, &file.file_name);
        let relative_path = normalized_path
            .strip_prefix("./")
            .unwrap_or(&normalized_path);

        for function in &file.functions {
            if function.complexity > max_complexity {
                report.push(json!({
                    "description": build_description(
                        &function.name,
                        function.complexity,
                        max_complexity,
                    ),
                    "check_name": CHECK_NAME,
                    "fingerprint": build_fingerprint(
                        "CC001",
                        relative_path,
                        &function.name,
                        function.line_start,
                    ),
                    "severity": DEFAULT_SEVERITY,
                    "location": {
                        "path": relative_path,
                        "lines": {"begin": function.line_start},
                    },
                }));
            }

            if suggest_refactors {
                for plan in &function.refactor_plans {
                    report.push(refactor_plan_issue(relative_path, &function.name, plan));
                }
            }
        }
    }

    let serialized = serde_json::to_string_pretty(&report)
        .map_err(|e| ExportError::Serialize(format!("Failed to serialize GitLab report: {}", e)))?;
    let mut file = fs::File::create(output_path).map_err(|e| {
        ExportError::Io(format!(
            "Failed to create GitLab report at {}: {}",
            output_path, e
        ))
    })?;
    use std::io::Write;
    file.write_all(serialized.as_bytes()).map_err(|e| {
        ExportError::Io(format!(
            "Failed to write GitLab report to {}: {}",
            output_path, e
        ))
    })?;
    file.write_all(b"\n").map_err(|e| {
        ExportError::Io(format!(
            "Failed to write GitLab report to {}: {}",
            output_path, e
        ))
    })?;

    Ok(())
}

fn build_description(function_name: &str, complexity: u64, max_complexity: u64) -> String {
    format!(
        "Function '{}' has cognitive complexity {} (max allowed: {}).",
        function_name, complexity, max_complexity
    )
}

fn build_fingerprint(check_id: &str, path: &str, function_name: &str, line_start: u64) -> String {
    let payload = format!("{}:{}:{}:{}", check_id, path, function_name, line_start);
    let digest = Sha256::digest(payload.as_bytes());
    digest.iter().map(|byte| format!("{:02x}", byte)).collect()
}

fn refactor_plan_severity(applicability: &Applicability) -> &'static str {
    match applicability {
        Applicability::MachineApplicable => "minor",
        Applicability::MaybeIncorrect => "major",
        Applicability::Informational => "info",
    }
}

fn refactor_plan_issue(relative_path: &str, function_name: &str, plan: &RefactorPlan) -> Value {
    json!({
        "description": format!("[{}] {}: {}", plan.rule_id, plan.title, plan.explanation),
        "check_name": format!("complexipy/{}", plan.rule_id.to_lowercase()),
        "fingerprint": build_fingerprint(
            &plan.rule_id,
            relative_path,
            function_name,
            plan.line_start,
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
