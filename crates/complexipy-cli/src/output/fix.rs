use std::collections::HashSet;

use complexipy_core::fix::{AppliedFix, FixReport, LineMap, SkipReason};
use complexipy_core::sanitize::escape_controls;
use complexipy_core::utils::split_lines;
use owo_colors::OwoColorize;
use unicode_width::UnicodeWidthChar;

use crate::output::refactor::highlight_lines_where;
use crate::output::render::terminal_width;

const REMOVED_BACKGROUND: (u8, u8, u8) = (58, 25, 29);
const ADDED_BACKGROUND: (u8, u8, u8) = (21, 48, 34);

enum Text {
    Old(usize),
    New(usize),
}

enum Row {
    Header(String),
    Code {
        old: Option<usize>,
        new: Option<usize>,
        marker: char,
        text: Text,
    },
}

pub fn format_fix_diff(path: &str, source: &str, report: &FixReport, colored: bool) -> String {
    if report.applied.is_empty() {
        return String::new();
    }

    let old_lines = split_lines(source).count();
    let new_lines = split_lines(&report.patched).count();
    let rows = diff_rows(report, old_lines, colored);
    let (old_rendered, new_rendered) = if colored {
        let removed = spans_of(&report.applied);
        let added = added_spans(&report.applied);
        let shown_old: HashSet<usize> = rows.iter().filter_map(old_text_index).collect();
        let shown_new: HashSet<usize> = rows.iter().filter_map(new_text_index).collect();
        (
            highlight_lines_where(
                source,
                &|index| in_ranges(&removed, index + 1).then_some(REMOVED_BACKGROUND),
                &|index| shown_old.contains(&index),
            ),
            highlight_lines_where(
                &report.patched,
                &|index| in_ranges(&added, index + 1).then_some(ADDED_BACKGROUND),
                &|index| shown_new.contains(&index),
            ),
        )
    } else {
        (
            split_lines(source)
                .map(|line| escape_controls(line).into_owned())
                .collect(),
            split_lines(&report.patched)
                .map(|line| escape_controls(line).into_owned())
                .collect(),
        )
    };

    let width = old_lines.max(new_lines).to_string().len();
    let code_cells = terminal_width().saturating_sub(width * 2 + 3);
    let display = escape_controls(path.trim_start_matches('/'));
    let mut lines = vec![
        colored_line(&format!("--- a/{display}"), colored, Tone::Removed),
        colored_line(&format!("+++ b/{display}"), colored, Tone::Added),
    ];
    for row in rows {
        lines.push(match row {
            Row::Header(header) => header,
            Row::Code {
                old,
                new,
                marker,
                text,
            } => {
                let code = match text {
                    Text::Old(line) => &old_rendered[line - 1],
                    Text::New(line) => &new_rendered[line - 1],
                };
                diff_row(old, new, marker, code, width, code_cells, colored)
            }
        });
    }

    lines.join("\n")
}

fn old_text_index(row: &Row) -> Option<usize> {
    match row {
        Row::Code {
            text: Text::Old(line),
            ..
        } => Some(line - 1),
        _ => None,
    }
}

fn new_text_index(row: &Row) -> Option<usize> {
    match row {
        Row::Code {
            text: Text::New(line),
            ..
        } => Some(line - 1),
        _ => None,
    }
}

fn diff_rows(report: &FixReport, old_lines: usize, colored: bool) -> Vec<Row> {
    let mut lines = Vec::new();
    let mut delta: i64 = 0;
    let mut emitted_to = 0usize;
    for (index, applied) in report.applied.iter().enumerate() {
        let old_start = applied.line_start as usize;
        let old_end = applied.line_end as usize;
        let removed_count = old_end - old_start + 1;
        let added_count = applied.replacement.lines().count();

        let pre_start = (emitted_to + 1)
            .max(old_start.saturating_sub(1))
            .min(old_start);
        let pre_count = old_start - pre_start;
        let mut post_end = (old_end + 1).min(old_lines);
        if let Some(next) = report.applied.get(index + 1) {
            post_end = post_end.min(next.line_start as usize - 1);
        }
        let post_count = post_end.saturating_sub(old_end);

        let old_hunk_start = old_start - pre_count;
        lines.push(Row::Header(hunk_header(
            old_hunk_start,
            pre_count + removed_count + post_count,
            (old_hunk_start as i64 + delta) as usize,
            pre_count + added_count + post_count,
            applied,
            colored,
        )));

        for line in pre_start..old_start {
            lines.push(Row::Code {
                old: Some(line),
                new: Some((line as i64 + delta) as usize),
                marker: ' ',
                text: Text::Old(line),
            });
        }
        for line in old_start..=old_end {
            lines.push(Row::Code {
                old: Some(line),
                new: None,
                marker: '-',
                text: Text::Old(line),
            });
        }
        let new_start = old_start as i64 + delta;
        for offset in 0..added_count {
            let line = new_start as usize + offset;
            lines.push(Row::Code {
                old: None,
                new: Some(line),
                marker: '+',
                text: Text::New(line),
            });
        }
        let delta_after = delta + added_count as i64 - removed_count as i64;
        for line in old_end + 1..=old_end + post_count {
            lines.push(Row::Code {
                old: Some(line),
                new: Some((line as i64 + delta_after) as usize),
                marker: ' ',
                text: Text::Old(line),
            });
        }

        delta = delta_after;
        emitted_to = old_end + post_count;
    }

    lines
}

fn spans_of(applied: &[AppliedFix]) -> Vec<(usize, usize)> {
    applied
        .iter()
        .map(|fix| (fix.line_start as usize, fix.line_end as usize))
        .collect()
}

fn added_spans(applied: &[AppliedFix]) -> Vec<(usize, usize)> {
    let mut spans = Vec::with_capacity(applied.len());
    let mut delta: i64 = 0;
    for fix in applied {
        let start = fix.line_start as i64 + delta;
        let count = fix.replacement.lines().count() as i64;
        spans.push((start as usize, (start + count - 1) as usize));
        delta += count - (fix.line_end - fix.line_start + 1) as i64;
    }
    spans
}

fn in_ranges(ranges: &[(usize, usize)], line: usize) -> bool {
    ranges
        .iter()
        .any(|(start, end)| line >= *start && line <= *end)
}

fn hunk_header(
    old_start: usize,
    old_count: usize,
    new_start: usize,
    new_count: usize,
    applied: &AppliedFix,
    colored: bool,
) -> String {
    let ranges = format!(
        "@@ -{},{} +{},{} @@",
        old_start, old_count, new_start, new_count
    );
    let label = format!("{} {}", applied.rule_id, escape_controls(&applied.title));
    if colored {
        format!("{} {}", ranges.cyan(), label.trim().dimmed())
    } else {
        format!("{} {}", ranges, label.trim())
    }
}

fn diff_row(
    old_number: Option<usize>,
    new_number: Option<usize>,
    marker: char,
    code: &str,
    width: usize,
    code_cells: usize,
    colored: bool,
) -> String {
    let old = old_number.map_or_else(String::new, |number| number.to_string());
    let new = new_number.map_or_else(String::new, |number| number.to_string());
    let mut numbers = format!("{:>width$} {:>width$}", old, new, width = width);
    if colored {
        numbers = numbers.dimmed().to_string();
    }
    let code = truncate_cells(code, code_cells);
    let marker = if colored {
        match marker {
            '-' => "-".red().bold().to_string(),
            '+' => "+".green().bold().to_string(),
            other => other.to_string(),
        }
    } else {
        marker.to_string()
    };
    format!("{} {}{}", numbers, marker, code)
}

fn truncate_cells(text: &str, max_cells: usize) -> String {
    if display_cells(text) <= max_cells {
        return text.to_string();
    }
    let budget = max_cells.saturating_sub(3);
    let mut result = String::new();
    let mut cells = 0;
    let mut chars = text.chars();
    while let Some(ch) = chars.next() {
        if ch == '\x1b' {
            result.push(ch);
            for next in chars.by_ref() {
                result.push(next);
                if next == 'm' {
                    break;
                }
            }
            continue;
        }
        let width = UnicodeWidthChar::width(ch).unwrap_or(0);
        if cells + width > budget {
            break;
        }
        cells += width;
        result.push(ch);
    }
    if text.contains('\x1b') {
        result.push_str("\x1b[0m");
    }
    result.push_str("...");
    result
}

fn display_cells(text: &str) -> usize {
    let mut cells = 0;
    let mut chars = text.chars();
    while let Some(ch) = chars.next() {
        if ch == '\x1b' {
            for next in chars.by_ref() {
                if next == 'm' {
                    break;
                }
            }
            continue;
        }
        cells += UnicodeWidthChar::width(ch).unwrap_or(0);
    }
    cells
}

fn colored_line(text: &str, colored: bool, tone: Tone) -> String {
    if !colored {
        return text.to_string();
    }
    match tone {
        Tone::Removed => text.red().to_string(),
        Tone::Added => text.green().to_string(),
    }
}

enum Tone {
    Removed,
    Added,
}

pub fn format_fix_summary(path: &str, report: &FixReport, colored: bool, map: &LineMap) -> String {
    let mut lines: Vec<String> = report
        .applied
        .iter()
        .map(|applied| {
            let (line_start, line_end) = map.original_span(applied.line_start, applied.line_end);
            let qualifier = if applied.reduction_is_measured {
                ""
            } else {
                " ~"
            };
            format!(
                "{} {} at {}:{}-{} (-{}{} complexity)",
                fixed_label(colored),
                applied.rule_id,
                escape_controls(path),
                line_start,
                line_end,
                qualifier,
                applied.reduction
            )
        })
        .collect();
    lines.extend(report.skipped.iter().map(|skipped| {
        let (line_start, line_end) = map.original_span(skipped.line_start, skipped.line_end);
        format!(
            "{} {} at {}:{}-{} ({})",
            skipped_label(colored),
            skipped.rule_id,
            escape_controls(path),
            line_start,
            line_end,
            reason_text(skipped.reason)
        )
    }));
    lines.join("\n")
}

fn fixed_label(colored: bool) -> String {
    if colored {
        "Fixed".green().bold().to_string()
    } else {
        "Fixed".to_string()
    }
}

fn skipped_label(colored: bool) -> String {
    if colored {
        "Skipped".yellow().bold().to_string()
    } else {
        "Skipped".to_string()
    }
}

fn reason_text(reason: SkipReason) -> &'static str {
    match reason {
        SkipReason::NotFixable => "not safe to auto-apply",
        SkipReason::Overlap => "overlaps another fix",
        SkipReason::ParseFailure => "would break the parse",
    }
}

pub fn pass_label(pass: usize, colored: bool) -> String {
    let label = format!("pass {}:", pass);
    if colored {
        label.dimmed().to_string()
    } else {
        label
    }
}

pub fn no_fixes_output() -> String {
    "No fixes to apply.".to_string()
}

#[cfg(test)]
mod tests;
