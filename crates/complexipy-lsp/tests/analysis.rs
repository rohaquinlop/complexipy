use complexipy_core::config::{InlayHints, LspConfig, LspSection, StringOrList};
use complexipy_lsp::analysis::{
    DIAGNOSTIC_CODE, DIAGNOSTIC_SOURCE, FUNCTION_HINT_LABEL, LineBounds, analyze, is_stale,
    is_within,
};
use lsp_types::{
    DiagnosticSeverity, HoverContents, InlayHint, InlayHintLabel, NumberOrString, Position, Range,
};

const SAMPLE: &str = "def light():\n    return 1\n\n\ndef heavy(a, b):\n    if a:\n        if b:\n            return 1\n    return 0\n";
const SUPPRESSED: &str = "def heavy(a, b):  # noqa: complexipy\n    if a:\n        if b:\n            return 1\n    return 0\n";
const EVERYTHING: &str = "def unused():\n    pass\n";
const DECORATED: &str =
    "@cached\ndef heavy(a, b):\n    if a:\n        if b:\n            return 1\n    return 0\n";
const MULTILINE: &str =
    "def heavy(\n    a,\n    b,\n):\n    if a:\n        return 1\n    return 0\n";
const ASYNC: &str = "async def heavy(a, b):\n    if a:\n        return 1\n    return 0\n";
const BACKSLASH: &str = "def heavy(a, \\\n          b):\n    if a:\n        if b:\n            return 1\n    return 0\n";
const TRAILING_COMMENT: &str =
    "def heavy(a, b):  # note\n    if a:\n        if b:\n            return 1\n    return 0\n";
const DECORATOR_STRING: &str = "@wraps(\n    \"\"\"\ndef fake():\n\"\"\"\n)\ndef heavy(a, b):\n    if a:\n        if b:\n            return 1\n    return 0\n";
const ONE_LINER: &str = "def heavy(a, b): return 1 if a and b else 0\n";
const NESTED: &str = "def outer(a, b):\n    if a and b:\n        def inner(x):\n            if x:\n                if x:\n                    return 1\n            return 0\n        return inner(a)\n    return 0\n";
const WIDE_FUNCTIONS: usize = 2000;

fn config(max_complexity_allowed: u64) -> LspConfig {
    LspConfig {
        max_complexity_allowed,
        ..LspConfig::default()
    }
}

fn config_with(
    max_complexity_allowed: u64,
    inlay_hints: InlayHints,
    per_line_hints: bool,
    diagnostics: bool,
) -> LspConfig {
    LspConfig {
        max_complexity_allowed,
        lsp: LspSection {
            inlay_hints,
            per_line_hints,
            diagnostics,
        },
        ..LspConfig::default()
    }
}

fn analyzed(source: &str, config: &LspConfig) -> complexipy_lsp::analysis::DocumentAnalysis {
    analyze(source, 1, config).unwrap()
}

const COLLAPSIBLE_IF: &str = "def heavy(a, b, c, d):\n    if a:\n        if b:\n            if c and d:\n                return 1\n    return 0\n";

#[test]
fn inactive_rules_leave_no_plans() {
    let default = analyzed(COLLAPSIBLE_IF, &config(15));
    assert!(
        default.functions[0]
            .refactor_plans
            .iter()
            .any(|plan| plan.rule_id == "C007")
    );

    let config = LspConfig {
        ignore: StringOrList::Many(vec!["C007".to_string()]),
        ..LspConfig::default()
    };
    let narrowed = analyzed(COLLAPSIBLE_IF, &config);
    let plans = &narrowed.functions[0].refactor_plans;

    assert!(plans.iter().all(|plan| plan.rule_id != "C007"));
    assert!(plans.iter().any(|plan| plan.rule_id == "C001"));
}

#[test]
fn inline_rule_list_keeps_the_other_rules() {
    let source = COLLAPSIBLE_IF.replace(
        "def heavy(a, b, c, d):",
        "def heavy(a, b, c, d):  # noqa: complexipy[C007]",
    );

    let analysis = analyzed(&source, &config(15));
    let plans = &analysis.functions[0].refactor_plans;

    assert!(plans.iter().all(|plan| plan.rule_id != "C007"));
    assert!(plans.iter().any(|plan| plan.rule_id == "C001"));
}

fn label(hint: &InlayHint) -> String {
    match &hint.label {
        InlayHintLabel::String(value) => value.clone(),
        InlayHintLabel::LabelParts(parts) => parts.iter().map(|part| part.value.clone()).collect(),
    }
}

fn wide_source() -> String {
    let mut source = String::new();

    for index in 0..WIDE_FUNCTIONS {
        source.push_str(&format!("def wide_{}(a, b):\n", index));
        source.push_str("    if a and b:\n");
        source.push_str("        return 1\n");
        source.push_str("    return 0\n\n");
    }

    source
}

#[test]
fn computes_function_complexity_from_source() {
    let analysis = analyzed(SAMPLE, &config(15));

    assert_eq!(analysis.functions.len(), 2);
    assert_eq!(analysis.functions[0].name, "light");
    assert_eq!(analysis.functions[0].complexity, 0);
    assert_eq!(analysis.functions[1].name, "heavy");
    assert_eq!(analysis.functions[1].complexity, 3);
}

#[test]
fn threshold_hint_is_hidden_below_the_limit() {
    let analysis = analyzed(SAMPLE, &config(15));

    assert!(analysis.hints(&config(15), None).is_empty());
}

#[test]
fn threshold_hint_is_shown_above_the_limit() {
    let analysis = analyzed(SAMPLE, &config(2));
    let hints = analysis.hints(&config(2), None);

    assert_eq!(hints.len(), 1);
    assert_eq!(hints[0].position, Position::new(4, 16));
    assert_eq!(label(&hints[0]), "cognitive: 3");
}

#[test]
fn always_mode_hints_every_function() {
    let analysis = analyzed(SAMPLE, &config(15));
    let hints = analysis.hints(&config_with(15, InlayHints::Always, false, true), None);

    assert_eq!(hints.len(), 2);
}

#[test]
fn never_mode_suppresses_every_hint() {
    let analysis = analyzed(SAMPLE, &config(2));
    let hints = analysis.hints(&config_with(2, InlayHints::Never, true, true), None);

    assert!(hints.is_empty());
}

#[test]
fn per_line_hints_are_off_by_default() {
    let analysis = analyzed(SAMPLE, &config(2));
    let hints = analysis.hints(&config(2), None);

    assert_eq!(hints.len(), 1);
}

#[test]
fn per_line_hints_appear_when_enabled() {
    let analysis = analyzed(SAMPLE, &config(2));
    let config = config_with(2, InlayHints::Threshold, true, true);
    let hints = analysis.hints(&config, None);

    assert_eq!(hints.len(), 3);
    assert_eq!(hints[1].position, Position::new(5, 9));
    assert_eq!(label(&hints[1]), "+1");
    assert_eq!(hints[2].position, Position::new(6, 13));
    assert_eq!(label(&hints[2]), "+2");
}

#[test]
fn hints_outside_the_requested_range_are_dropped() {
    let analysis = analyzed(SAMPLE, &config(2));
    let without_heavy = Range::new(Position::new(0, 0), Position::new(1, 100));
    let heavy_line = Range::new(Position::new(4, 0), Position::new(4, 100));

    assert!(analysis.hints(&config(2), Some(without_heavy)).is_empty());
    assert_eq!(analysis.hints(&config(2), Some(heavy_line)).len(), 1);
}

#[test]
fn per_line_hints_skip_lines_without_an_increment() {
    let analysis = analyzed(SAMPLE, &config(2));
    let config = config_with(2, InlayHints::Threshold, true, true);
    let hints = analysis.hints(&config, None);

    assert!(hints.iter().all(|hint| label(hint) != "+0"));
}

#[test]
fn one_line_functions_show_a_single_hint() {
    let config = config_with(1, InlayHints::Threshold, true, true);
    let analysis = analyzed(ONE_LINER, &config);
    let hints = analysis.hints(&config, None);

    assert_eq!(analysis.functions[0].complexity, 2);
    assert_eq!(hints.len(), 1);
    assert_eq!(label(&hints[0]), "cognitive: 2");
}

#[test]
fn diagnostics_report_the_function_and_the_limit() {
    let analysis = analyzed(SAMPLE, &config(2));
    let diagnostics = analysis.diagnostics(&config(2));

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(
        diagnostics[0].message,
        "cognitive complexity 3 exceeds the allowed 2"
    );
    assert_eq!(diagnostics[0].source.as_deref(), Some(DIAGNOSTIC_SOURCE));
    assert_eq!(
        diagnostics[0].code,
        Some(NumberOrString::String(DIAGNOSTIC_CODE.to_string()))
    );
    assert_eq!(diagnostics[0].severity, Some(DiagnosticSeverity::WARNING));
    assert_eq!(diagnostics[0].range.start, Position::new(4, 0));
    assert_eq!(diagnostics[0].range.end, Position::new(8, 12));
}

#[test]
fn a_function_at_the_limit_is_not_a_diagnostic() {
    let analysis = analyzed(SAMPLE, &config(3));

    assert!(analysis.diagnostics(&config(3)).is_empty());
}

#[test]
fn diagnostics_can_be_disabled() {
    let analysis = analyzed(SAMPLE, &config(2));
    let config = config_with(2, InlayHints::Threshold, false, false);

    assert!(analysis.diagnostics(&config).is_empty());
}

#[test]
fn ignored_functions_are_filtered_by_default() {
    let analysis = analyzed(SUPPRESSED, &config(2));

    assert!(analysis.functions.is_empty());
    assert!(analysis.diagnostics(&config(2)).is_empty());
}

#[test]
fn no_ignore_includes_suppressed_functions() {
    let config = LspConfig {
        no_ignore: true,
        ..config(2)
    };
    let analysis = analyzed(SUPPRESSED, &config);

    assert_eq!(analysis.diagnostics(&config).len(), 1);
}

#[test]
fn hover_reports_the_total_and_the_limit() {
    let analysis = analyzed(SAMPLE, &config(2));
    let hover = analysis.hover(&config(2), Position::new(4, 0)).unwrap();

    let HoverContents::Markup(content) = hover.contents else {
        panic!("expected markup content");
    };

    assert!(content.value.contains("**heavy**: cognitive complexity 3"));
    assert!(content.value.contains("Exceeds the allowed 2"));
}

#[test]
fn hover_outside_a_function_is_empty() {
    let analysis = analyzed(SAMPLE, &config(2));

    assert!(analysis.hover(&config(2), Position::new(2, 0)).is_none());
}

#[test]
fn hover_includes_the_top_refactor_plan() {
    let analysis = analyzed(SAMPLE, &config(2));
    let hover = analysis.hover(&config(2), Position::new(5, 0)).unwrap();

    let HoverContents::Markup(content) = hover.contents else {
        panic!("expected markup content");
    };

    assert!(content.value.contains("Top refactor"));
}

#[test]
fn analysis_without_functions_is_empty() {
    let analysis = analyzed(EVERYTHING, &config(15));

    assert!(analysis.hints(&config(15), None).is_empty());
    assert!(analysis.diagnostics(&config(15)).is_empty());
}

#[test]
fn decorated_functions_hint_on_the_def_line() {
    let analysis = analyzed(DECORATED, &config(0));
    let hints = analysis.hints(&config(0), None);

    assert_eq!(hints.len(), 1);
    assert_eq!(hints[0].position, Position::new(1, 16));
}

#[test]
fn wrapped_definitions_hint_on_the_closing_line() {
    let analysis = analyzed(MULTILINE, &config(0));
    let hints = analysis.hints(&config(0), None);

    assert_eq!(hints.len(), 1);
    assert_eq!(hints[0].position, Position::new(3, 2));
}

#[test]
fn backslash_continuations_hint_on_the_closing_line() {
    let analysis = analyzed(BACKSLASH, &config(0));
    let hints = analysis.hints(&config(0), None);

    assert_eq!(hints.len(), 1);
    assert_eq!(hints[0].position, Position::new(1, 13));
}

#[test]
fn trailing_comments_keep_the_def_line() {
    let analysis = analyzed(TRAILING_COMMENT, &config(0));
    let hints = analysis.hints(&config(0), None);

    assert_eq!(hints.len(), 1);
    assert_eq!(hints[0].position, Position::new(0, 24));
}

#[test]
fn decorator_strings_do_not_receive_the_hint() {
    let analysis = analyzed(DECORATOR_STRING, &config(0));
    let hints = analysis.hints(&config(0), None);

    assert_eq!(hints.len(), 1);
    assert_eq!(hints[0].position, Position::new(5, 16));
}

#[test]
fn nested_functions_keep_the_parent_hint() {
    let analysis = analyzed(NESTED, &config(2));
    let hints = analysis.hints(&config(2), None);

    assert_eq!(analysis.functions.len(), 1);
    assert_eq!(analysis.functions[0].name, "outer");
    assert_eq!(hints.len(), 1);
    assert_eq!(hints[0].position, Position::new(0, 16));
}

#[test]
fn async_definitions_hint_on_the_def_line() {
    let analysis = analyzed(ASYNC, &config(0));
    let hints = analysis.hints(&config(0), None);

    assert_eq!(hints.len(), 1);
    assert_eq!(hints[0].position, Position::new(0, 22));
}

#[test]
fn wide_documents_answer_hints_without_a_per_function_scan() {
    let source = wide_source();
    let analysis = analyzed(&source, &config(15));

    assert_eq!(analysis.functions.len(), WIDE_FUNCTIONS);
    assert!(analysis.hints(&config(15), None).is_empty());

    let config = config_with(15, InlayHints::Always, true, true);
    let hints = analysis.hints(&config, None);
    let function_hints = hints
        .iter()
        .filter(|hint| label(hint).starts_with(FUNCTION_HINT_LABEL))
        .count();

    assert_eq!(function_hints, WIDE_FUNCTIONS);
    assert!(hints.len() > function_hints);
}

#[test]
fn stale_versions_are_detected() {
    assert!(is_stale(4, Some(5)));
    assert!(is_stale(4, None));
    assert!(!is_stale(5, Some(5)));
}

#[test]
fn line_bounds_use_utf16_units() {
    assert_eq!(
        LineBounds::new("a\u{1F600}b").end_position(1),
        Position::new(0, 4)
    );
    assert_eq!(
        LineBounds::new("short\r\nlonger line").end_position(2),
        Position::new(1, 11)
    );
    assert_eq!(LineBounds::new("\tif a:").length(1), 6);
    assert_eq!(LineBounds::new("only").end_position(9), Position::new(8, 0));
    assert_eq!(LineBounds::new("").end_position(1), Position::new(0, 0));
}

#[test]
fn requested_range_membership() {
    let range = Range::new(Position::new(2, 4), Position::new(6, 8));

    assert!(is_within(Position::new(2, 4), Some(&range)));
    assert!(is_within(Position::new(4, 0), Some(&range)));
    assert!(is_within(Position::new(6, 8), Some(&range)));
    assert!(!is_within(Position::new(1, 0), Some(&range)));
    assert!(!is_within(Position::new(7, 0), Some(&range)));
    assert!(is_within(Position::new(99, 0), None));
}

#[test]
fn syntax_errors_carry_a_parse_diagnostic() {
    let Err(failure) = analyze("def broken(:\n", 4, &config(15)) else {
        panic!("expected a syntax error");
    };

    assert!(failure.message.contains("Failed to parse"));
    assert_eq!(failure.version, 4);

    let diagnostic = failure.diagnostic();

    assert_eq!(
        diagnostic.code,
        Some(NumberOrString::String(
            complexipy_lsp::analysis::PARSE_ERROR_CODE.to_string()
        ))
    );
    assert_eq!(diagnostic.severity, Some(DiagnosticSeverity::WARNING));
    assert_eq!(diagnostic.source.as_deref(), Some(DIAGNOSTIC_SOURCE));
    assert_eq!(
        diagnostic.range,
        Range::new(Position::new(0, 0), Position::new(0, 12))
    );
}
