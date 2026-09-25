use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use complexipy_core::classes::{FileComplexity, RefactorPlan};
use complexipy_core::fix::{apply_fixes, fixable, parses};
use complexipy_core::{AnalysisOptions, cognitive_complexity::code_complexity_shared};

use crate::output::fix::{format_fix_diff, format_fix_summary, no_fixes_output, pass_label};

const MAX_FIX_PASSES: usize = 8;

struct FixTarget {
    display: String,
    path: PathBuf,
    text: String,
    plans: Vec<RefactorPlan>,
}

pub struct FixPassResult {
    pub console: String,
    pub wrote: bool,
}

fn fixable_files(
    files_complexities: &[FileComplexity],
    invocation_path: &str,
) -> Vec<(String, PathBuf, Vec<RefactorPlan>)> {
    files_complexities
        .iter()
        .filter_map(|file| {
            let plans: Vec<_> = file
                .functions
                .iter()
                .flat_map(|function| function.refactor_plans.clone())
                .filter(fixable)
                .collect();
            if plans.is_empty() {
                return None;
            }
            Some((
                file.path.clone(),
                resolve_target(invocation_path, &file.path),
                plans,
            ))
        })
        .collect()
}

pub fn run_fix_pass(
    files_complexities: &[FileComplexity],
    dry_run: bool,
    invocation_path: &str,
    colored: bool,
    options: &AnalysisOptions,
) -> FixPassResult {
    let mut targets: Vec<FixTarget> = Vec::new();
    for (display, path, plans) in fixable_files(files_complexities, invocation_path) {
        let text = match fs::read_to_string(&path) {
            Ok(text) => text,
            Err(error) => {
                eprintln!("Failed to read {}: {}", display, error);
                continue;
            }
        };
        targets.push(FixTarget {
            display,
            path,
            text,
            plans,
        });
    }

    let mut parts: Vec<String> = Vec::new();
    let mut wrote = false;
    for pass in 0..MAX_FIX_PASSES {
        let mut changed = false;
        for target in targets.iter_mut() {
            if target.plans.is_empty() {
                continue;
            }
            let report = apply_fixes(&target.text, &target.plans);

            if report.applied.is_empty() {
                if !dry_run {
                    let summary = format_fix_summary(&target.display, &report, colored);
                    if !summary.is_empty() {
                        parts.push(summary);
                    }
                }
                target.plans.clear();
                continue;
            }

            if !dry_run {
                if let Err(error) = write_patched(&target.path, &report.patched) {
                    eprintln!("Failed to write {}: {}", target.display, error);
                    target.plans.clear();
                    continue;
                }
                wrote = true;
            }
            changed = true;
            if pass > 0 {
                parts.push(pass_label(pass + 1, colored));
            }
            parts.push(format_fix_diff(
                &target.display,
                &target.text,
                &report,
                colored,
            ));
            if !dry_run {
                let summary = format_fix_summary(&target.display, &report, colored);
                if !summary.is_empty() {
                    parts.push(summary);
                }
            }
            target.text = report.patched;
        }

        if !changed {
            break;
        }
        for target in targets.iter_mut() {
            target.plans = fixable_plans(&target.text, options);
        }
    }

    let console = if parts.is_empty() {
        no_fixes_output()
    } else {
        parts.join("\n")
    };

    FixPassResult { console, wrote }
}

fn fixable_plans(code: &str, options: &AnalysisOptions) -> Vec<RefactorPlan> {
    code_complexity_shared(code, options)
        .map(|complexity| {
            complexity
                .functions
                .into_iter()
                .flat_map(|function| function.refactor_plans)
                .filter(fixable)
                .collect()
        })
        .unwrap_or_default()
}

fn resolve_target(invocation_path: &str, file_path: &str) -> PathBuf {
    Path::new(invocation_path).join(file_path)
}

fn write_patched(target: &Path, patched: &str) -> io::Result<()> {
    if !parses(patched) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "patched source does not parse",
        ));
    }
    let permissions = fs::metadata(target)?.permissions();
    let file_name = target
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    let temp = target.with_file_name(format!(".{file_name}.complexipy-tmp"));
    fs::write(&temp, patched)?;
    let _ = fs::set_permissions(&temp, permissions);
    if let Err(error) = fs::rename(&temp, target) {
        let _ = fs::remove_file(&temp);
        return Err(error);
    }
    Ok(())
}

#[cfg(test)]
mod tests;
