use owo_colors::{OwoColorize, Stream, Style};

use crate::utils::snapshot::SnapshotEvaluation;
use complexipy_core::classes::{IgnoredLocation, RemovableIgnore};
pub use complexipy_core::{no_active_rule_warning, unknown_rule_warning};

pub fn handle_snapshot_console(snap: &SnapshotEvaluation, output_snapshot_path: &str) -> String {
    if !snap.should_run {
        return String::new();
    }

    if snap.watermark_messages.is_empty() {
        format!(
            "Snapshot watermark passed. Baseline stored at {}",
            output_snapshot_path
        )
    } else {
        snap.watermark_messages
            .iter()
            .map(|message| {
                format!(
                    "{}: {}",
                    "Snapshot watermark".if_supports_color(Stream::Stdout, |text| text
                        .style(Style::new().bold().red())),
                    message
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
            ignore.path, ignore.line, ignore.function, ignore.complexity, ignore.comment
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

#[cfg(test)]
mod tests;
