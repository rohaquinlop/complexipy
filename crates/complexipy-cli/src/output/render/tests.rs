use std::collections::HashMap;

use crate::output::render::{
    SummaryOptions, colorize_complexity, format_status_text, handle_console_settings,
    output_delta_text, output_plain, output_summary, print_invalid_paths, rule_at,
};
use crate::types::{Color, FileEntry, FunctionRow, Sort};
use unicode_width::UnicodeWidthStr;

fn strip_ansi(input: &str) -> String {
    let mut result = String::new();
    let mut chars = input.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            for next in chars.by_ref() {
                if next == 'm' {
                    break;
                }
            }
        } else {
            result.push(c);
        }
    }
    result
}

fn row(name: &str, complexity: u64, passed: bool, path: &str) -> FunctionRow {
    FunctionRow {
        name: name.to_string(),
        complexity,
        passed,
        path: path.to_string(),
        file_name: path.split('/').next_back().unwrap_or(path).to_string(),
        refactor_plans: vec![],
        additional_refactor_plans: 0,
    }
}

#[test]
fn status_text_contains_labels() {
    assert!(format_status_text(true).contains("PASSED"));
    assert!(format_status_text(false).contains("FAILED"));
}

#[test]
fn status_text_emoji_outside_color() {
    let passed = format_status_text(true);
    assert!(passed.starts_with("✅ "));
    let colored = passed.trim_start_matches("✅ ");
    assert!(colored.starts_with("\u{1b}["));
    assert!(colored.contains(" PASSED "));

    let failed = format_status_text(false);
    assert!(failed.starts_with("❌ "));
    let colored = failed.trim_start_matches("❌ ");
    assert!(colored.starts_with("\u{1b}["));
    assert!(colored.contains(" FAILED "));
}

#[test]
fn colorize_marks_over_threshold_red() {
    let ok = colorize_complexity(5, 5);
    let bad = colorize_complexity(6, 5);
    assert!(!ok.contains("\u{1b}[31m"));
    assert!(bad.contains("\u{1b}[31m"));
}

#[test]
fn delta_text_cases() {
    let mut previous = HashMap::new();
    let f_new = row("f", 10, false, "a.py");

    assert_eq!(output_delta_text(None, &f_new, 5), "");
    assert_eq!(
        output_delta_text(Some(&previous), &f_new, 5),
        " (new, \u{0394} = +10)"
    );

    previous.insert(("a.py".to_string(), "a.py".to_string(), "f".to_string()), 7);
    assert_eq!(
        output_delta_text(Some(&previous), &f_new, 5),
        " (last: 7, \u{0394} = +3)"
    );

    previous.insert(
        ("a.py".to_string(), "a.py".to_string(), "f".to_string()),
        10,
    );
    assert_eq!(output_delta_text(Some(&previous), &f_new, 5), "");

    let within = row("f", 3, true, "a.py");
    assert_eq!(output_delta_text(Some(&previous), &within, 5), "");
}

#[test]
fn plain_output_lines() {
    let entries = vec![FileEntry {
        path: "src/a.py".to_string(),
        functions: vec![row("f", 5, true, "src/a.py")],
    }];

    let output = output_plain(&entries);

    assert_eq!(output, "src/a.py f 5");
}

#[test]
fn summary_no_files_message() {
    let (has_success, output) = output_summary(SummaryOptions {
        files: &[],
        failed_only: false,
        sort: Sort::Asc,
        ignore_complexity: false,
        max_complexity: 5,
        previous_functions: None,
        snapshot_map: None,
        plain: false,
        top: None,
        suggest_refactors: false,
        invocation_path: "",
    });

    assert!(has_success);
    assert_eq!(
        output,
        "No files were found with functions. No complexity was calculated."
    );
}

#[test]
fn summary_failed_only_empty_message() {
    let files = vec![complexipy_core::classes::FileComplexity {
        path: "a.py".to_string(),
        file_name: "a.py".to_string(),
        functions: vec![complexipy_core::classes::FunctionComplexity {
            name: "f".to_string(),
            complexity: 2,
            line_start: 1,
            line_end: 2,
            line_complexities: vec![],
            refactor_plans: vec![],
            additional_refactor_plans: 0,
        }],
        complexity: 0,
    }];

    let (_, output) = output_summary(SummaryOptions {
        files: &files,
        failed_only: true,
        sort: Sort::Asc,
        ignore_complexity: false,
        max_complexity: 5,
        previous_functions: None,
        snapshot_map: None,
        plain: false,
        top: None,
        suggest_refactors: false,
        invocation_path: "",
    });

    assert_eq!(
        output,
        "No function were found with complexity greater than 5."
    );
}

#[test]
fn summary_plain_mode_skips_messages() {
    let (_, output) = output_summary(SummaryOptions {
        files: &[],
        failed_only: true,
        sort: Sort::Asc,
        ignore_complexity: false,
        max_complexity: 5,
        previous_functions: None,
        snapshot_map: None,
        plain: true,
        top: None,
        suggest_refactors: false,
        invocation_path: "",
    });

    assert_eq!(output, "");
}

#[test]
fn invalid_paths_rendering() {
    let (has_success, raw) = print_invalid_paths(&["nope.py".to_string()]);
    let output = strip_ansi(&raw);

    assert!(!has_success);
    assert!(output.contains("error"));
    assert!(output.contains("Failed to process nope.py"));
    assert!(output.contains("Please check file/folder exists or check syntax"));

    let (has_success, output) = print_invalid_paths(&[]);
    assert!(has_success);
    assert_eq!(output, "");
}

#[test]
fn rule_renders_title_with_padding() {
    let output = strip_ansi(&rule_at("complexipy", 80));
    assert!(output.contains("complexipy"));
    assert!(output.starts_with('─'));
    assert!(output.ends_with('─'));
    assert_eq!(output.chars().count(), 80);
}

#[test]
fn rule_matches_rich_layout_at_width() {
    let output = strip_ansi(&rule_at("🎉 Analysis completed! 🎉", 100));
    // Rich: side = (width - title_cells) / 2; left = side - 1; clamp to width.
    // title cells = 2+1+19+1+2 = 25; width 100 -> side 37 -> left 36, right 37.
    let stripped = output.trim_end_matches(' ');
    let left = stripped.chars().take_while(|c| *c == '─').count();
    let right = stripped.chars().rev().take_while(|c| *c == '─').count();
    assert_eq!(left, 36);
    assert_eq!(right, 37);
    assert_eq!(output.chars().count(), 98);
    assert_eq!(UnicodeWidthStr::width(output.as_str()), 100);
}

#[test]
fn console_settings_banner() {
    let settings = handle_console_settings(&Color::Auto, false, false);
    assert!(!settings.banner.is_empty());
    assert!(settings.color_enabled);

    let quiet = handle_console_settings(&Color::Auto, true, false);
    assert_eq!(quiet.banner, "");

    let plain = handle_console_settings(&Color::Auto, false, true);
    assert_eq!(plain.banner, "");

    let no_color = handle_console_settings(&Color::No, false, false);
    assert!(!no_color.color_enabled);
}
