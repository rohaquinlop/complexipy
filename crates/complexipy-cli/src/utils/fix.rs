use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use complexipy_core::classes::{FileComplexity, FunctionComplexity, RefactorPlan};
use complexipy_core::fix::{LineMap, apply_fixes, parses};
use complexipy_core::fs_guard::{MAX_SOURCE_BYTES, is_within, read_bounded, write_atomic};
use complexipy_core::sanitize::escape_controls;
use complexipy_core::utils::split_lines;
use complexipy_core::{AnalysisOptions, cognitive_complexity::code_complexity_shared};

use crate::output::fix::{format_fix_diff, format_fix_summary, no_fixes_output, pass_label};

const MAX_FIX_PASSES: usize = 8;

struct FixTarget {
    display: String,
    path: PathBuf,
    text: String,
    plans: Vec<RefactorPlan>,
    changed: bool,
    line_map: LineMap,
    reported_skips: HashSet<(String, u64, u64)>,
}

pub struct FixPassResult {
    pub console: String,
    pub written: HashSet<String>,
}

fn plans_over_threshold(
    functions: &[FunctionComplexity],
    max_complexity_allowed: u64,
) -> Vec<RefactorPlan> {
    functions
        .iter()
        .filter(|function| function.complexity > max_complexity_allowed)
        .flat_map(|function| function.refactor_plans.iter().cloned())
        .collect()
}

fn files_with_plans(
    files_complexities: &[FileComplexity],
    invocation_path: &str,
    max_complexity_allowed: u64,
) -> Vec<(String, PathBuf, Vec<RefactorPlan>)> {
    files_complexities
        .iter()
        .filter_map(|file| {
            let plans = plans_over_threshold(&file.functions, max_complexity_allowed);
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
    max_complexity_allowed: u64,
) -> FixPassResult {
    let mut targets: Vec<FixTarget> = Vec::new();
    for (display, path, plans) in
        files_with_plans(files_complexities, invocation_path, max_complexity_allowed)
    {
        let text = match read_fix_target(&path, Path::new(invocation_path)) {
            Ok(text) => text,
            Err(error) => {
                eprintln!("Failed to read {}: {}", escape_controls(&display), error);
                continue;
            }
        };
        targets.push(FixTarget {
            display,
            path,
            plans,
            changed: false,
            line_map: LineMap::new(split_lines(&text).count() as u64),
            reported_skips: HashSet::new(),
            text,
        });
    }

    let fix_options = AnalysisOptions {
        plans_above: Some(max_complexity_allowed),
        ..options.clone()
    };
    let mut parts: Vec<String> = Vec::new();
    let mut written = HashSet::new();
    let mut applied_any = false;
    for pass in 0..MAX_FIX_PASSES {
        let mut changed = false;
        for target in targets.iter_mut() {
            if target.plans.is_empty() {
                continue;
            }
            let mut report = apply_fixes(&target.text, &target.plans);
            report.skipped.retain(|skipped| {
                let (line_start, line_end) = target
                    .line_map
                    .original_span(skipped.line_start, skipped.line_end);
                target
                    .reported_skips
                    .insert((skipped.rule_id.clone(), line_start, line_end))
            });

            if report.applied.is_empty() {
                if !dry_run {
                    let summary =
                        format_fix_summary(&target.display, &report, colored, &target.line_map);
                    if !summary.is_empty() {
                        parts.push(summary);
                    }
                }
                target.plans.clear();
                continue;
            }

            if !dry_run {
                if let Err(error) = write_patched(
                    &target.path,
                    Path::new(invocation_path),
                    &target.text,
                    &report.patched,
                ) {
                    eprintln!(
                        "Failed to write {}: {}",
                        escape_controls(&target.display),
                        error
                    );
                    target.plans.clear();
                    continue;
                }
                written.insert(target.display.clone());
            }
            applied_any = true;
            changed = true;
            target.changed = true;
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
                let summary =
                    format_fix_summary(&target.display, &report, colored, &target.line_map);
                if !summary.is_empty() {
                    parts.push(summary);
                }
            }
            for fix in report.applied.iter().rev() {
                target.line_map.register_fix(
                    fix.line_start,
                    fix.line_end,
                    fix.replacement.lines().count() as u64,
                );
            }
            target.text = report.patched;
        }

        if !changed {
            break;
        }
        for target in targets.iter_mut() {
            if target.changed {
                target.plans = analyze_plans(&target.text, &fix_options, max_complexity_allowed);
                target.changed = false;
            }
        }
    }

    if !applied_any {
        parts.push(no_fixes_output());
    }

    FixPassResult {
        console: parts.join("\n"),
        written,
    }
}

fn analyze_plans(
    code: &str,
    options: &AnalysisOptions,
    max_complexity_allowed: u64,
) -> Vec<RefactorPlan> {
    code_complexity_shared(code, options)
        .map(|complexity| plans_over_threshold(&complexity.functions, max_complexity_allowed))
        .unwrap_or_default()
}

fn resolve_target(invocation_path: &str, file_path: &str) -> PathBuf {
    Path::new(invocation_path).join(file_path)
}

/// Reads a file `--fix` may rewrite: a regular file, not a symlink, inside
/// the invocation directory.
fn read_fix_target(target: &Path, root: &Path) -> io::Result<String> {
    if !fs::symlink_metadata(target)?.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "is a symlink or not a regular file, and --fix rewrites only regular files",
        ));
    }
    if !is_within(root, target) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "is outside the directory complexipy runs from",
        ));
    }
    String::from_utf8(read_bounded(target, Some(root), MAX_SOURCE_BYTES)?)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
}

/// Replaces `target` with `patched` only when the file on disk still holds
/// `original`, so a concurrent edit is never overwritten.
fn write_patched(target: &Path, root: &Path, original: &str, patched: &str) -> io::Result<()> {
    if !parses(patched) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "patched source does not parse",
        ));
    }
    if read_fix_target(target, root)? != original {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "the file changed on disk since complexipy read it",
        ));
    }
    write_atomic(target, patched.as_bytes())
}

#[cfg(test)]
mod tests;
