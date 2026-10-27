use comfy_table::{Cell, Table};
use owo_colors::{OwoColorize, Stream, Style};

use crate::output::render::rule;
use complexipy_core::diff::{DiffEntry, DiffStatus, Verdict};
use complexipy_core::sanitize::escape_controls;

/// The table of changed entries. With `verdicts`, a Gate column marks the
/// entries that fail the gate and names the pairs it treats as one function
/// moved or renamed, and a closing line counts the failures.
pub fn format_diff(entries: &[DiffEntry], git_ref: &str, verdicts: Option<&[Verdict]>) -> String {
    let changed: Vec<usize> = (0..entries.len())
        .filter(|&index| {
            matches!(
                entries[index].status(),
                DiffStatus::Regressed
                    | DiffStatus::Improved
                    | DiffStatus::New
                    | DiffStatus::Removed
            )
        })
        .collect();

    if changed.is_empty() {
        return format!(
            "No functions changed relative to {}.",
            escape_controls(git_ref)
        );
    }

    let notes = gate_notes(entries, verdicts);
    let with_gate = changed.iter().any(|&index| !notes[index].is_empty());
    let mut table = Table::new();
    let mut header = vec!["Status", "Location", "Change"];
    if with_gate {
        header.push("Gate");
    }
    table.set_header(header);
    for &index in &changed {
        let entry = &entries[index];
        let mut row = vec![
            Cell::new(status_style(&entry.status())),
            Cell::new(location(entry)),
            Cell::new(format_change(entry)),
        ];
        if with_gate {
            row.push(Cell::new(&notes[index]));
        }
        table.add_row(row);
    }

    let changed: Vec<&DiffEntry> = changed.iter().map(|&index| &entries[index]).collect();
    format!(
        "\n{}\n{}\n\nNet: {}{}",
        rule(&format!(
            "Complexity diff (vs {})",
            escape_controls(git_ref)
        )),
        table,
        build_diff_summary(&changed),
        verdicts.map(gate_summary).unwrap_or_default()
    )
}

fn gate_summary(verdicts: &[Verdict]) -> String {
    let failures = verdicts
        .iter()
        .filter(|verdict| **verdict == Verdict::Fails)
        .count();
    match failures {
        0 => "\nGate: no function fails the gate.".to_string(),
        1 => "\nGate: 1 function fails: REGRESSED or NEW above the threshold.".to_string(),
        _ => format!(
            "\nGate: {} functions fail: REGRESSED or NEW above the threshold.",
            failures
        ),
    }
}

fn location(entry: &DiffEntry) -> String {
    format!(
        "{}::{}",
        escape_controls(&entry.file_path),
        escape_controls(&entry.func_name)
    )
}

/// The gate's note on each entry: FAILS for an entry that fails the gate,
/// and which REMOVED entry a NEW one replaces, and the other way around.
fn gate_notes(entries: &[DiffEntry], verdicts: Option<&[Verdict]>) -> Vec<String> {
    let mut notes = vec![String::new(); entries.len()];
    for (index, verdict) in verdicts.unwrap_or_default().iter().enumerate() {
        match *verdict {
            Verdict::Fails => {
                notes[index] = "FAILS"
                    .if_supports_color(Stream::Stdout, |text| text.style(Style::new().red().bold()))
                    .to_string();
            }
            Verdict::Replaces(removed) => {
                notes[index] = format!("replaces {}", location(&entries[removed]));
                notes[removed] = format!("replaced by {}", location(&entries[index]));
            }
            Verdict::Passes => {}
        }
    }
    notes
}

fn status_style(status: &DiffStatus) -> String {
    match status {
        DiffStatus::Regressed => "REGRESSED"
            .if_supports_color(Stream::Stdout, |text| text.style(Style::new().red().bold()))
            .to_string(),
        DiffStatus::Improved => "IMPROVED"
            .if_supports_color(Stream::Stdout, |text| {
                text.style(Style::new().green().bold())
            })
            .to_string(),
        DiffStatus::New => "NEW"
            .if_supports_color(Stream::Stdout, |text| {
                text.style(Style::new().yellow().bold())
            })
            .to_string(),
        DiffStatus::Removed => "REMOVED"
            .if_supports_color(Stream::Stdout, |text| text.dimmed())
            .to_string(),
        DiffStatus::Unchanged => "UNCHANGED".to_string(),
    }
}

fn format_change(entry: &DiffEntry) -> String {
    match entry.status() {
        DiffStatus::New => format!("{}  (new)", entry.new_complexity.unwrap_or_default())
            .if_supports_color(Stream::Stdout, |text| {
                text.style(Style::new().yellow().bold())
            })
            .to_string(),
        DiffStatus::Removed => format!("{}  (removed)", entry.old_complexity.unwrap_or_default())
            .if_supports_color(Stream::Stdout, |text| text.dimmed())
            .to_string(),
        DiffStatus::Regressed => {
            let delta = entry.delta().unwrap_or_default();
            format!(
                "{} \u{2192} {}  ({:+})",
                entry.old_complexity.unwrap_or_default(),
                entry.new_complexity.unwrap_or_default(),
                delta
            )
            .if_supports_color(Stream::Stdout, |text| text.red())
            .to_string()
        }
        DiffStatus::Improved => {
            let delta = entry.delta().unwrap_or_default();
            format!(
                "{} \u{2192} {}  ({:+})",
                entry.old_complexity.unwrap_or_default(),
                entry.new_complexity.unwrap_or_default(),
                delta
            )
            .if_supports_color(Stream::Stdout, |text| text.green())
            .to_string()
        }
        DiffStatus::Unchanged => {
            let delta = entry.delta().unwrap_or_default();
            format!(
                "{} \u{2192} {}  ({})",
                entry.old_complexity.unwrap_or_default(),
                entry.new_complexity.unwrap_or_default(),
                delta
            )
        }
    }
}

fn build_diff_summary(changed: &[&DiffEntry]) -> String {
    let count_for = |status: DiffStatus| {
        changed
            .iter()
            .filter(|entry| entry.status() == status)
            .count()
    };

    let labels: [(DiffStatus, &str); 4] = [
        (DiffStatus::Regressed, "regressed"),
        (DiffStatus::Improved, "improved"),
        (DiffStatus::New, "new"),
        (DiffStatus::Removed, "removed"),
    ];

    let parts: Vec<String> = labels
        .iter()
        .filter_map(|(status, label)| {
            let count = count_for(*status);
            if count == 0 {
                None
            } else {
                Some(format!(
                    "{} {}",
                    count
                        .to_string()
                        .if_supports_color(Stream::Stdout, |text| text
                            .style(Style::new().green().bold())),
                    label
                ))
            }
        })
        .collect();

    if parts.is_empty() {
        "no changes".to_string()
    } else {
        parts.join(", ")
    }
}

#[cfg(test)]
mod tests;
