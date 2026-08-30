use comfy_table::{Cell, Table};
use owo_colors::OwoColorize;

use crate::output::render::rule;
use complexipy_core::diff::{DiffEntry, DiffStatus};

pub fn format_diff(entries: &[DiffEntry], git_ref: &str) -> String {
    let changed: Vec<&DiffEntry> = entries
        .iter()
        .filter(|entry| {
            matches!(
                entry.status(),
                DiffStatus::Regressed
                    | DiffStatus::Improved
                    | DiffStatus::New
                    | DiffStatus::Removed
            )
        })
        .collect();

    if changed.is_empty() {
        return format!("No functions changed relative to {}.", git_ref);
    }

    let mut table = Table::new();
    table.set_header(vec!["Status", "Location", "Change"]);
    for entry in changed.iter().copied() {
        table.add_row(vec![
            Cell::new(status_style(&entry.status())),
            Cell::new(format!("{}::{}", entry.file_path, entry.func_name)),
            Cell::new(format_change(entry)),
        ]);
    }

    format!(
        "\n{}\n{}\n\nNet: {}",
        rule(&format!("Complexity diff (vs {})", git_ref)),
        table,
        build_diff_summary(&changed)
    )
}

fn status_style(status: &DiffStatus) -> String {
    match status {
        DiffStatus::Regressed => "REGRESSED".red().bold().to_string(),
        DiffStatus::Improved => "IMPROVED".green().bold().to_string(),
        DiffStatus::New => "NEW".yellow().bold().to_string(),
        DiffStatus::Removed => "REMOVED".dimmed().to_string(),
        DiffStatus::Unchanged => "UNCHANGED".to_string(),
    }
}

fn format_change(entry: &DiffEntry) -> String {
    match entry.status() {
        DiffStatus::New => format!("{}  (new)", entry.new_complexity.unwrap_or_default())
            .yellow()
            .bold()
            .to_string(),
        DiffStatus::Removed => format!("{}  (removed)", entry.old_complexity.unwrap_or_default())
            .dimmed()
            .to_string(),
        DiffStatus::Regressed => {
            let delta = entry.delta().unwrap_or_default();
            format!(
                "{} \u{2192} {}  ({:+})",
                entry.old_complexity.unwrap_or_default(),
                entry.new_complexity.unwrap_or_default(),
                delta
            )
            .red()
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
            .green()
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
                Some(format!("{} {}", count.to_string().green().bold(), label))
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
