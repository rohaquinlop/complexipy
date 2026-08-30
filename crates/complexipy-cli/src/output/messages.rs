use owo_colors::OwoColorize;

use crate::utils::snapshot::SnapshotEvaluation;
use complexipy_core::classes::RemovableIgnore;

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
            .map(|message| format!("{}: {}", "Snapshot watermark".bold().red(), message))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

pub fn ignored_summary_output(location_count: usize, no_ignore: bool) -> String {
    let mut output = if location_count == 0 {
        "No ignore comments found.".to_string()
    } else {
        format!("Found {} suppressed location(s).", location_count)
    };
    if location_count > 0 && no_ignore {
        output.push_str("\n(all markers ignored due to --no-ignore)");
    }
    output
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

pub fn diff_flags_warning() -> String {
    "--diff and --diff-only both set. Using --diff-only (visual only, no enforcement).".to_string()
}

#[cfg(test)]
mod tests;
