use std::collections::HashMap;

use owo_colors::OwoColorize;
use std::io::IsTerminal;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::output::refactor::output_refactor_plans;
use crate::output::rows::{build_output_rows, truncate_top_n};
use crate::types::{FileEntry, FunctionRow, Sort};
use complexipy_core::classes::FileComplexity;

const RULE_WIDTH: usize = 80;

pub struct ConsoleSettings {
    pub banner: String,
    pub color_enabled: bool,
}

pub fn handle_console_settings(
    color: &crate::types::Color,
    quiet: bool,
    plain: bool,
) -> ConsoleSettings {
    let color_enabled = match color {
        crate::types::Color::No => false,
        crate::types::Color::Yes => true,
        crate::types::Color::Auto => true,
    };

    let banner = if quiet || plain {
        String::new()
    } else if cfg!(windows) {
        rule("complexipy")
    } else {
        rule("🐙 complexipy")
    };

    ConsoleSettings {
        banner,
        color_enabled,
    }
}

pub fn rule(title: &str) -> String {
    rule_at(title, terminal_width())
}

fn rule_at(title: &str, width: usize) -> String {
    if title.is_empty() {
        return "─".repeat(width).bright_green().to_string();
    }
    if width < 4 {
        return "─".repeat(width).bright_green().to_string();
    }

    let title_cells = UnicodeWidthStr::width(title);
    let side_width = (width - title_cells) / 2;
    let left = side_width.saturating_sub(1);
    let right_length = width - left - title_cells;
    let right = (side_width + 1).min(right_length);

    let plain = format!("{} {} {}", "─".repeat(left), title, "─".repeat(right));
    let plain = set_cell_size(&plain, width);

    if let Some(start) = plain.find(title) {
        let end = start + title.len();
        format!(
            "{}{}{}",
            plain[..start].to_string().bright_green(),
            &plain[start..end],
            plain[end..].to_string().bright_green()
        )
    } else {
        plain.bright_green().to_string()
    }
}

fn set_cell_size(text: &str, width: usize) -> String {
    let mut result = String::new();
    let mut cells = 0;
    for character in text.chars() {
        let character_width = UnicodeWidthChar::width(character).unwrap_or(0);
        if cells + character_width > width {
            break;
        }
        result.push(character);
        cells += character_width;
    }
    while cells < width {
        result.push(' ');
        cells += 1;
    }
    result
}

fn terminal_width() -> usize {
    if std::io::stdout().is_terminal() {
        terminal_size::terminal_size()
            .map(|(width, _)| width.0 as usize)
            .unwrap_or(RULE_WIDTH)
    } else {
        RULE_WIDTH
    }
}

pub struct SummaryOptions<'a> {
    pub files: &'a [FileComplexity],
    pub failed_only: bool,
    pub sort: Sort,
    pub ignore_complexity: bool,
    pub max_complexity: u64,
    pub previous_functions: Option<&'a HashMap<(String, String, String), u64>>,
    pub snapshot_map: Option<&'a HashMap<(String, String, String), u64>>,
    pub plain: bool,
    pub top: Option<u64>,
    pub suggest_refactors: bool,
    pub invocation_path: &'a str,
}

pub fn output_summary(options: SummaryOptions) -> (bool, String) {
    let SummaryOptions {
        files,
        failed_only,
        sort,
        ignore_complexity,
        max_complexity,
        previous_functions,
        snapshot_map,
        plain,
        top,
        suggest_refactors,
        invocation_path,
    } = options;
    let (mut file_entries, total_functions, all_pass) =
        build_output_rows(files, failed_only, sort, max_complexity, snapshot_map);
    let has_success = all_pass || ignore_complexity;

    if let Some(top) = top {
        file_entries = truncate_top_n(file_entries, top);
    }

    if plain {
        return (has_success, output_plain(&file_entries));
    }

    let output = if failed_only && file_entries.is_empty() {
        let plural = if files.len() > 1 { "s" } else { "" };
        format!(
            "No function{} were found with complexity greater than {}.",
            plural, max_complexity
        )
    } else if total_functions == 0 {
        "No files were found with functions. No complexity was calculated.".to_string()
    } else {
        output_file_entries(
            &file_entries,
            previous_functions,
            max_complexity,
            suggest_refactors,
            invocation_path,
        )
    };

    (has_success, output)
}

pub fn output_plain(file_entries: &[FileEntry]) -> String {
    let mut lines = Vec::new();
    for entry in file_entries {
        for function in &entry.functions {
            lines.push(format!(
                "{} {} {}",
                entry.path, function.name, function.complexity
            ));
        }
    }
    lines.join("\n")
}

pub fn output_file_entries(
    file_entries: &[FileEntry],
    previous_functions: Option<&HashMap<(String, String, String), u64>>,
    max_complexity: u64,
    suggest_refactors: bool,
    invocation_path: &str,
) -> String {
    let mut sections = Vec::new();

    for (index, entry) in file_entries.iter().enumerate() {
        let mut lines = vec![entry.path.bold().to_string()];
        for function in &entry.functions {
            let status_text = format_status_text(function.passed);
            let complexity_text = colorize_complexity(function.complexity, max_complexity);
            let delta_text = output_delta_text(previous_functions, function, max_complexity);
            lines.push(format!(
                "    {} {}{}  {}",
                function.name, complexity_text, delta_text, status_text
            ));
            if suggest_refactors {
                let plans = output_refactor_plans(function, invocation_path);
                if !plans.is_empty() {
                    lines.push(plans);
                }
            }
        }
        if index < file_entries.len() - 1 {
            lines.push(String::new());
        }
        sections.push(lines.join("\n"));
    }

    let mut output = sections.join("\n");

    if !file_entries.is_empty()
        && file_entries
            .iter()
            .all(|entry| entry.functions.iter().all(|function| function.passed))
    {
        output.push_str(&format!(
            "\n\n{}",
            "All functions are within the allowed complexity."
                .green()
                .bold()
        ));
    }

    output
}

pub fn format_status_text(passed: bool) -> String {
    if passed {
        format!("✅ {} ", " PASSED ".black().on_green().bold())
    } else {
        format!("❌ {} ", " FAILED ".white().on_red().bold())
    }
}

pub fn output_delta_text(
    previous_functions: Option<&HashMap<(String, String, String), u64>>,
    function: &FunctionRow,
    max_complexity: u64,
) -> String {
    let Some(previous_functions) = previous_functions else {
        return String::new();
    };

    if function.complexity <= max_complexity {
        return String::new();
    }

    let key = (
        function.path.clone(),
        function.file_name.clone(),
        function.name.clone(),
    );
    let previous = previous_functions.get(&key);
    match previous {
        None => format!(" (new, \u{0394} = +{})", function.complexity),
        Some(previous) if *previous != function.complexity => {
            let delta = function.complexity as i64 - *previous as i64;
            format!(" (last: {}, \u{0394} = {:+})", previous, delta)
        }
        Some(_) => String::new(),
    }
}

pub fn colorize_complexity(complexity: u64, max_complexity: u64) -> String {
    if complexity <= max_complexity {
        complexity.to_string().green().to_string()
    } else {
        complexity.to_string().red().to_string()
    }
}

pub fn print_invalid_paths(invalid_paths: &[String]) -> (bool, String) {
    let has_success = invalid_paths.is_empty();
    let mut lines = Vec::new();
    for failed_path in invalid_paths {
        lines.push(format!(
            "{}: Failed to process {} - Please check file/folder exists or check syntax",
            "error".bold().red(),
            failed_path.bold().white()
        ));
    }
    (has_success, lines.join("\n"))
}

#[cfg(test)]
mod tests;
