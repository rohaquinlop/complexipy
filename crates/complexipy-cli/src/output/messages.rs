use owo_colors::{OwoColorize, Stream, Style};

use crate::types::ConfigGateChange;
use crate::utils::snapshot::SnapshotEvaluation;
use complexipy_core::classes::RemovableIgnore;
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
