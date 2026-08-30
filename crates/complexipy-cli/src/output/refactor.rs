use std::fs;

use owo_colors::OwoColorize;
use syntect::easy::HighlightLines;
use syntect::highlighting::{Color as SyntaxColor, ThemeSet};
use syntect::parsing::SyntaxSet;

use crate::types::FunctionRow;
use crate::utils::paths::normalize_path;
use complexipy_core::classes::{Applicability, CodeSuggestion, RefactorPlan, RuleCategory};

pub fn output_refactor_plans(function: &FunctionRow, invocation_path: &str) -> String {
    if function.refactor_plans.is_empty() {
        return String::new();
    }

    let display_path = normalize_path(&function.path, &function.file_name);
    let source_lines = read_source_lines(invocation_path, &function.path, &function.file_name);

    let mut sections = vec![format!("\n      {}", "Refactor Suggestions:".bold())];
    if function.refactor_plans.len() > 1 {
        sections.push(format!(
            "      {}",
            "Each estimate is independent and assumes applying that suggestion alone -- they don't sum."
                .dimmed()
        ));
    }
    for (index, plan) in function.refactor_plans.iter().enumerate() {
        sections.push(output_single_plan(
            plan,
            index + 1,
            &display_path,
            source_lines.as_deref(),
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
            .dimmed()
        ));
    }

    sections.join("\n")
}

fn read_source_lines(invocation_path: &str, path: &str, file_name: &str) -> Option<Vec<String>> {
    let full_path = normalize_path(path, file_name);
    let full_path = if !invocation_path.is_empty() && !full_path.starts_with('/') {
        format!("{}/{}", invocation_path.trim_end_matches('/'), full_path)
    } else {
        full_path
    };
    fs::read_to_string(full_path)
        .ok()
        .map(|content| content.lines().map(str::to_string).collect())
}

fn output_single_plan(
    plan: &RefactorPlan,
    index: usize,
    display_path: &str,
    source_lines: Option<&[String]>,
) -> String {
    let category_icon = get_category_icon(&plan.category);
    let category_name = get_category_name(&plan.category);
    let applicability_icon = get_applicability_icon(&plan.applicability);
    let applicability_name = get_applicability_name(&plan.applicability);

    let mut sections = vec![format!(
        "\n      [{}] {} {}",
        index,
        plan.rule_id.cyan().bold(),
        plan.title
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
        format!("-{}{}", qualifier, plan.estimated_reduction).green(),
        plan.current_complexity,
        plan.estimated_complexity_after
    ));

    if !plan.description.is_empty() {
        sections.push(format!("\n          {}", plan.description.dimmed()));
    }
    if !plan.explanation.is_empty() {
        sections.push(format!("\n          {} {}", ">".bold(), plan.explanation));
    }

    if let Some(suggestion) = &plan.suggestion {
        sections.push(output_suggestion(plan, suggestion, source_lines));
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
    let mut lines = vec![format!("\n          {}", "References:".dimmed())];
    if !doc_url.is_empty() {
        lines.push(format!("            {}", doc_url.underline().blue()));
    }
    for reference in references {
        lines.push(format!("            {}", reference.underline().blue()));
    }
    lines.join("\n")
}

fn output_suggestion(
    plan: &RefactorPlan,
    suggestion: &CodeSuggestion,
    source_lines: Option<&[String]>,
) -> String {
    let applicability_icon = get_applicability_icon(&suggestion.applicability);
    let applicability_name = get_applicability_name(&suggestion.applicability);

    let mut sections = vec![format!(
        "\n          {} {} {}",
        "Suggestion:".bold(),
        applicability_icon,
        applicability_name
    )];
    if !suggestion.description.is_empty() {
        sections.push(format!("          {}", suggestion.description.dimmed()));
    }

    if let Some(source_lines) = source_lines {
        let original_start = plan.line_start as usize;
        let original_end = (plan.line_end as usize).min(source_lines.len());
        if original_start <= original_end && original_start >= 1 {
            let original_code = source_lines[original_start - 1..original_end].join("\n");
            if !original_code.is_empty() {
                sections.push(format!("\n          {}", "Original:".dimmed()));
                sections.push(output_code_snippet(&original_code, original_start));
            }
        }
    }

    if !suggestion.replacement.is_empty() {
        sections.push(format!("\n          {}", "Replacement:".dimmed()));
        sections.push(output_code_snippet(
            &suggestion.replacement,
            plan.line_start as usize,
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
    if column_index >= source_line.chars().count() {
        return String::new();
    }

    let caret_width = source_line[column_index..].trim_end().chars().count();
    if caret_width == 0 {
        return String::new();
    }

    let gutter = format!("{:>5} | ", plan.line_start);
    let blank_gutter = format!("{}| ", " ".repeat(gutter.len() - 2));

    format!(
        "          {}\n          {}{}\n          {}{}{}\n          {}",
        blank_gutter,
        gutter,
        source_line,
        blank_gutter,
        " ".repeat(column_index),
        "^".repeat(caret_width),
        blank_gutter
    )
}

fn output_help(help_text: &str) -> String {
    format!("\n          {}\n          {}", "Help:".bold(), help_text)
}

fn output_code_snippet(code: &str, start_line: usize) -> String {
    if code.is_empty() {
        return String::new();
    }

    let syntax_set = SyntaxSet::load_defaults_newlines();
    let theme_set = ThemeSet::load_defaults();
    let syntax = syntax_set
        .find_syntax_by_extension("py")
        .unwrap_or_else(|| syntax_set.find_syntax_plain_text());
    let theme = &theme_set.themes["base16-ocean.dark"];

    let mut highlight = HighlightLines::new(syntax, theme);
    let mut lines = Vec::new();
    for (offset, line) in code.lines().enumerate() {
        let regions = highlight
            .highlight_line(line, &syntax_set)
            .unwrap_or_default();
        let mut rendered = String::new();
        for (style, text) in regions {
            let color = style.foreground;
            rendered.push_str(&paint(text, color));
        }
        lines.push(format!(
            "{:>12}{:>4} | {}",
            "",
            start_line + offset,
            rendered
        ));
    }
    lines.join("\n")
}

fn paint(text: &str, color: SyntaxColor) -> String {
    if color.a == 0 {
        return text.to_string();
    }
    text.color(owo_colors::Rgb(color.r, color.g, color.b))
        .to_string()
}

#[cfg(test)]
pub fn output_single_plan_for_test(
    plan: &RefactorPlan,
    index: usize,
    display_path: &str,
    source_lines: Option<&[String]>,
) -> String {
    output_single_plan(plan, index, display_path, source_lines)
}

#[cfg(test)]
mod tests;
