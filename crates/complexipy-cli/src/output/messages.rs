use owo_colors::{OwoColorize, Stream, Style};

use crate::types::ConfigGateChange;
use crate::utils::snapshot::SnapshotEvaluation;
use complexipy_core::classes::{IgnoredLocation, RemovableIgnore};
use complexipy_core::runner::{SkipReason, SkippedPath};
use complexipy_core::sanitize::escape_controls;
pub use complexipy_core::{no_active_rule_warning, unknown_rule_warning};

pub fn handle_snapshot_console(snap: &SnapshotEvaluation, output_snapshot_path: &str) -> String {
    if !snap.should_run {
        return String::new();
    }

    if snap.watermark_messages.is_empty() {
        format!(
            "Snapshot watermark passed. Baseline stored at {}",
            escape_controls(output_snapshot_path)
        )
    } else {
        snap.watermark_messages
            .iter()
            .map(|message| {
                format!(
                    "{}: {}",
                    "Snapshot watermark".if_supports_color(Stream::Stdout, |text| text
                        .style(Style::new().bold().red())),
                    escape_controls(message)
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}

pub fn ignored_summary_output(locations: &[IgnoredLocation], no_ignore: bool) -> String {
    if locations.is_empty() {
        return "No ignore comments found.".to_string();
    }
    let mut lines: Vec<String> = locations
        .iter()
        .map(|location| format!("{}:{}  {}", location.path, location.line, location.comment))
        .collect();
    lines.push(format!("Found {} suppressed location(s).", locations.len()));
    if no_ignore {
        lines.push("(all markers ignored due to --no-ignore)".to_string());
    }
    lines.join("\n")
}

pub fn ignored_saved_output(path: &str) -> String {
    format!("Ignored locations saved at {}", path)
}

pub fn removable_ignores_output(removable: &[RemovableIgnore]) -> String {
    if removable.is_empty() {
        return String::new();
    }
    let mut lines = vec![
        "\nThe following ignore comment(s) are no longer necessary (complexity is within the allowed limit) and can be removed:".to_string(),
    ];
    for ignore in removable {
        lines.push(format!(
            "{}:{}  function={} complexity={}  {}",
            escape_controls(&ignore.path),
            ignore.line,
            escape_controls(&ignore.function),
            ignore.complexity,
            escape_controls(&ignore.comment)
        ));
    }
    lines.join("\n")
}

pub fn unmatchable_exclude_warning(pattern: &str) -> String {
    format!(
        "Exclude pattern {:?} can never match: paths are matched relative to the \
         analyzed path without a leading `/` or `./`, and never end with `/`. Use a \
         pattern such as `legacy/**`.",
        pattern
    )
}

pub fn diff_flags_warning() -> String {
    "--diff and --diff-only both set. Using --diff-only (visual only, no enforcement).".to_string()
}

/// The warning for a skipped entry, or `None` for a file an ignore rule
/// hides, which `--report-ignored` lists instead.
pub fn skipped_path_warning(skipped: &SkippedPath) -> Option<String> {
    let reason = match skipped.reason {
        SkipReason::NotRegularFile => "it is not a regular file",
        SkipReason::OutsideRoot => "it is a symlink that resolves outside the analyzed directory",
        SkipReason::Unreadable => "the directory cannot be read",
        SkipReason::IgnoreRule => return None,
    };
    Some(format!("Skipped {}: {}.", skipped.path, reason))
}

pub fn ignore_rule_skips_output(skipped: &[SkippedPath]) -> String {
    let paths: Vec<&str> = skipped
        .iter()
        .filter(|skipped| skipped.reason == SkipReason::IgnoreRule)
        .map(|skipped| skipped.path.as_str())
        .collect();
    if paths.is_empty() {
        return "No Python files were skipped by .gitignore, .ignore, or hidden-path rules."
            .to_string();
    }
    let mut lines = vec![format!(
        "{} Python file(s) were skipped by .gitignore, .ignore, or hidden-path rules:",
        paths.len()
    )];
    lines.extend(
        paths
            .iter()
            .map(|path| format!("  {}", escape_controls(path))),
    );
    lines.join("\n")
}

pub fn nothing_selected_warning(skipped: &[SkippedPath]) -> String {
    let hidden = skipped
        .iter()
        .filter(|skipped| skipped.reason == SkipReason::IgnoreRule)
        .count();
    if hidden == 0 {
        "No Python file was selected, so this run checks nothing.".to_string()
    } else {
        format!(
            "No Python file was selected, so this run checks nothing. {} file(s) were skipped \
             by .gitignore, .ignore, or hidden-path rules; --report-ignored lists them.",
            hidden
        )
    }
}

pub fn config_gate_change_warning(config_path: &str, change: &ConfigGateChange) -> String {
    let effect = match change {
        ConfigGateChange::DiffRef(reference) => format!(
            "sets [diff] branch = {:?}, so the exit code fails only on functions that regressed \
             or appeared since that reference, not on every function over the threshold",
            reference
        ),
        ConfigGateChange::StagedDiff => "sets [diff] staged = true, so the exit code checks only \
             the staged changes, not every function over the threshold"
            .to_string(),
        ConfigGateChange::SnapshotCreate => "sets snapshot-create = true, so this run rewrites \
             complexipy-snapshot.json before checking against it, and every current violation \
             passes"
            .to_string(),
    };
    format!(
        "{} {}. Pass the matching flag, --isolated, or --config <file> to pin the gate in CI.",
        config_path, effect
    )
}

#[cfg(test)]
mod tests;
