use std::path::Path;
use std::sync::OnceLock;

use owo_colors::{OwoColorize, Stream, Style};
use syntect::easy::HighlightLines;
use syntect::highlighting::{Color as SyntaxColor, ThemeSet};
use syntect::parsing::SyntaxSet;

use crate::types::FunctionRow;
use crate::utils::paths::normalize_path;
use complexipy_core::classes::{Applicability, CodeSuggestion, RefactorPlan, RuleCategory};
use complexipy_core::fs_guard::{MAX_SOURCE_BYTES, read_bounded_text};
use complexipy_core::sanitize::escape_controls;
use complexipy_core::utils::split_lines;

/// Renders the plans of one function. `source_lines` holds the lines of
/// the function's file, read once per file by the caller.
pub fn output_refactor_plans(
    function: &FunctionRow,
    source_lines: Option<&[String]>,
    color: bool,
) -> String {
    if function.refactor_plans.is_empty() {
        return String::new();
    }

    let display_path =
        escape_controls(&normalize_path(&function.path, &function.file_name)).into_owned();

    let mut sections = vec![format!(
        "\n      {}",
        "Refactor Suggestions:".if_supports_color(Stream::Stdout, |text| text.bold())
    )];
    if function.refactor_plans.len() > 1 {
        sections.push(format!(
            "      {}",
            "Each estimate is independent and assumes applying that suggestion alone -- they don't sum.".if_supports_color(Stream::Stdout, |text| text.dimmed())
        ));
    }
    for (index, plan) in function.refactor_plans.iter().enumerate() {
        sections.push(output_single_plan(
            plan,
            index + 1,
            &display_path,
            source_lines,
            color,
        ));
    }

    if function.additional_refactor_plans > 0 {
        let suffix = if function.additional_refactor_plans != 1 {
            "s"
        } else {
            ""
        };
        sections.push(format!(
            "\n      {}",
            format!(
                "... and {} more suggestion{}",
                function.additional_refactor_plans, suffix
            )
            .if_supports_color(Stream::Stdout, |text| text.dimmed())
        ));
    }

    sections.join("\n")
}

pub fn read_source_lines(
    invocation_path: &str,
    path: &str,
    file_name: &str,
) -> Option<Vec<String>> {
    let full_path = normalize_path(path, file_name);
    let full_path = if !invocation_path.is_empty() && !full_path.starts_with('/') {
        format!("{}/{}", invocation_path.trim_end_matches('/'), full_path)
    } else {
        full_path
    };
    read_bounded_text(Path::new(&full_path), None, MAX_SOURCE_BYTES)
        .ok()
        .map(|content| split_lines(&content).map(str::to_string).collect())
}

fn output_single_plan(
    plan: &RefactorPlan,
    index: usize,
    display_path: &str,
    source_lines: Option<&[String]>,
    color: bool,
) -> String {
    let category_icon = get_category_icon(&plan.category);
    let category_name = get_category_name(&plan.category);
    let applicability_icon = get_applicability_icon(&plan.applicability);
    let applicability_name = get_applicability_name(&plan.applicability);

    let mut sections = vec![format!(
        "\n      [{}] {} {}",
        index,
        plan.rule_id.if_supports_color(Stream::Stdout, |text| text
            .style(Style::new().cyan().bold())),
        escape_controls(&plan.title)
    )];
    sections.push(format!(
        "          --> {}:{}:{}",
        display_path, plan.line_start, plan.column_start
    ));
    let caret = output_caret_span(plan, source_lines);
    if !caret.is_empty() {
        sections.push(caret);
    }
    sections.push(format!(
        "          Category: {} {} | Applicability: {} {}",
        category_icon, category_name, applicability_icon, applicability_name
    ));
    let reduction_label = if plan.reduction_is_measured {
        "Reduction"
    } else {
        "Estimated reduction"
    };
    let qualifier = if plan.reduction_is_measured { "" } else { "~" };
    sections.push(format!(
        "          Lines {}-{} -> {}: {} complexity ({} -> {})",
        plan.line_start,
        plan.line_end,
        reduction_label,
        format!("-{}{}", qualifier, plan.estimated_reduction)
            .if_supports_color(Stream::Stdout, |text| text.green()),
        plan.current_complexity,
        plan.estimated_complexity_after
    ));

    if !plan.description.is_empty() {
        sections.push(format!(
            "\n          {}",
            escape_controls(&plan.description)
                .if_supports_color(Stream::Stdout, |text| text.dimmed())
        ));
    }
    if !plan.explanation.is_empty() {
        sections.push(format!(
            "\n          {} {}",
            ">".if_supports_color(Stream::Stdout, |text| text.bold()),
            escape_controls(&plan.explanation)
        ));
    }

    if let Some(suggestion) = &plan.suggestion {
        sections.push(output_suggestion(plan, suggestion, source_lines, color));
    } else if let Some(help) = &plan.help {
        sections.push(output_help(help));
    }

    let references = output_plan_references(&plan.doc_url, &plan.references);
    if !references.is_empty() {
        sections.push(references);
    }

    sections.join("\n")
}

pub fn get_category_icon(category: &RuleCategory) -> &'static str {
    match category {
        RuleCategory::Complexity => "\u{25b2}",
        RuleCategory::Readability => "\u{25c6}",
    }
}

pub fn get_category_name(category: &RuleCategory) -> &'static str {
    match category {
        RuleCategory::Complexity => "Complexity",
        RuleCategory::Readability => "Readability",
    }
}

pub fn get_applicability_icon(applicability: &Applicability) -> &'static str {
    match applicability {
        Applicability::MachineApplicable => "*",
        Applicability::MaybeIncorrect => "!",
        Applicability::Informational => "i",
    }
}

pub fn get_applicability_name(applicability: &Applicability) -> &'static str {
    match applicability {
        Applicability::MachineApplicable => "Safe to apply",
        Applicability::MaybeIncorrect => "Needs review",
        Applicability::Informational => "Informational",
    }
}

fn output_plan_references(doc_url: &str, references: &[String]) -> String {
    if doc_url.is_empty() && references.is_empty() {
        return String::new();
    }
    let mut lines = vec![format!(
        "\n          {}",
        "References:".if_supports_color(Stream::Stdout, |text| text.dimmed())
    )];
    if !doc_url.is_empty() {
        lines.push(format!(
            "            {}",
            doc_url.if_supports_color(Stream::Stdout, |text| text
                .style(Style::new().underline().blue()))
        ));
    }
    for reference in references {
        lines.push(format!(
            "            {}",
            reference.if_supports_color(Stream::Stdout, |text| text
                .style(Style::new().underline().blue()))
        ));
    }
    lines.join("\n")
}

fn output_suggestion(
    plan: &RefactorPlan,
    suggestion: &CodeSuggestion,
    source_lines: Option<&[String]>,
    color: bool,
) -> String {
    let applicability_icon = get_applicability_icon(&suggestion.applicability);
    let applicability_name = get_applicability_name(&suggestion.applicability);

    let mut sections = vec![format!(
        "\n          {} {} {}",
        "Suggestion:".if_supports_color(Stream::Stdout, |text| text.bold()),
        applicability_icon,
        applicability_name
    )];
    if !suggestion.description.is_empty() {
        sections.push(format!(
            "          {}",
            escape_controls(&suggestion.description)
                .if_supports_color(Stream::Stdout, |text| text.dimmed())
        ));
    }

    if let Some(source_lines) = source_lines {
        let original_start = plan.line_start as usize;
        let original_end = (plan.line_end as usize).min(source_lines.len());
        if original_start <= original_end && original_start >= 1 {
            let original_code = source_lines[original_start - 1..original_end].join("\n");
            if !original_code.is_empty() {
                sections.push(format!(
                    "\n          {}",
                    "Original:".if_supports_color(Stream::Stdout, |text| text.dimmed())
                ));
                sections.push(output_code_snippet(&original_code, original_start, color));
            }
        }
    }

    if !suggestion.replacement.is_empty() {
        sections.push(format!(
            "\n          {}",
            "Replacement:".if_supports_color(Stream::Stdout, |text| text.dimmed())
        ));
        sections.push(output_code_snippet(
            &suggestion.replacement,
            plan.line_start as usize,
            color,
        ));
    }

    sections.join("\n")
}

fn output_caret_span(plan: &RefactorPlan, source_lines: Option<&[String]>) -> String {
    let Some(source_lines) = source_lines else {
        return String::new();
    };
    if plan.column_start == 0 {
        return String::new();
    }

    let line_index = plan.line_start as usize - 1;
    let Some(source_line) = source_lines.get(line_index) else {
        return String::new();
    };

    let column_index = plan.column_start as usize - 1;
    let Some((byte_index, _)) = source_line.char_indices().nth(column_index) else {
        return String::new();
    };

    let caret_width = source_line[byte_index..].trim_end().chars().count();
    if caret_width == 0 {
        return String::new();
    }

    let gutter = format!("{:>5} | ", plan.line_start);
    let blank_gutter = format!("{}| ", " ".repeat(gutter.len() - 2));

    format!(
        "          {}\n          {}{}\n          {}{}{}\n          {}",
        blank_gutter,
        gutter,
        escape_controls(source_line),
        blank_gutter,
        " ".repeat(column_index),
        "^".repeat(caret_width),
        blank_gutter
    )
}

fn output_help(help_text: &str) -> String {
    format!(
        "\n          {}\n          {}",
        "Help:".if_supports_color(Stream::Stdout, |text| text.bold()),
        escape_controls(help_text)
    )
}

fn output_code_snippet(code: &str, start_line: usize, color: bool) -> String {
    if code.is_empty() {
        return String::new();
    }

    let rendered = if color {
        highlight_regions(code, &|_| None)
    } else {
        split_lines(code).map(str::to_string).collect()
    };
    rendered
        .into_iter()
        .enumerate()
        .map(|(offset, rendered)| format!("{:>12}{:>4} | {}", "", start_line + offset, rendered))
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn highlight_regions(code: &str, tint: &dyn Fn(usize) -> Option<(u8, u8, u8)>) -> Vec<String> {
    highlight_lines_where(code, tint, &|_| true)
}

/// Highlights the lines that `shown` selects and leaves the others empty.
/// A line's colors depend on the lines before it, so every line up to the
/// last shown one is still parsed; none after it is.
pub fn highlight_lines_where(
    code: &str,
    tint: &dyn Fn(usize) -> Option<(u8, u8, u8)>,
    shown: &dyn Fn(usize) -> bool,
) -> Vec<String> {
    if code.is_empty() {
        return Vec::new();
    }
    let last_shown = split_lines(code)
        .enumerate()
        .filter(|(index, _)| shown(*index))
        .map(|(index, _)| index)
        .last();

    static SYNTAX_SET: OnceLock<SyntaxSet> = OnceLock::new();
    static THEME_SET: OnceLock<ThemeSet> = OnceLock::new();

    let syntax_set = SYNTAX_SET.get_or_init(SyntaxSet::load_defaults_newlines);
    let theme_set = THEME_SET.get_or_init(ThemeSet::load_defaults);
    let syntax = syntax_set
        .find_syntax_by_extension("py")
        .unwrap_or_else(|| syntax_set.find_syntax_plain_text());
    let theme = &theme_set.themes["base16-ocean.dark"];

    let mut highlight = HighlightLines::new(syntax, theme);
    split_lines(code)
        .enumerate()
        .map(|(index, line)| {
            if last_shown.is_none_or(|last| index > last) {
                return String::new();
            }
            let terminated = format!("{}\n", escape_controls(line));
            let regions = highlight
                .highlight_line(&terminated, syntax_set)
                .unwrap_or_default();
            if !shown(index) {
                return String::new();
            }
            let background = tint(index);
            regions
                .into_iter()
                .map(|(style, text)| {
                    paint(text.trim_end_matches('\n'), style.foreground, background)
                })
                .collect()
        })
        .collect()
}

fn paint(text: &str, color: SyntaxColor, background: Option<(u8, u8, u8)>) -> String {
    if text.is_empty() {
        return String::new();
    }
    let mut codes: Vec<String> = Vec::new();
    if color.a != 0 {
        codes.push(format!("38;2;{};{};{}", color.r, color.g, color.b));
    }
    if let Some((red, green, blue)) = background {
        codes.push(format!("48;2;{};{};{}", red, green, blue));
    }
    if codes.is_empty() {
        return text.to_string();
    }
    format!("\x1b[{}m{}\x1b[0m", codes.join(";"), text)
}

#[cfg(test)]
pub fn output_single_plan_for_test(
    plan: &RefactorPlan,
    index: usize,
    display_path: &str,
    source_lines: Option<&[String]>,
    color: bool,
) -> String {
    output_single_plan(plan, index, display_path, source_lines, color)
}

#[cfg(test)]
mod tests;
