//! Tests for document analysis: diagnostics, inlay hints, and hover.

use complexipy_core::config::{
    DiagnosticRange, HoverScope, InlayHints, LspConfig, LspSection, StringOrList,
};
use complexipy_lsp::analysis::{
    DIAGNOSTIC_CODE, DIAGNOSTIC_SOURCE, FUNCTION_HINT_LABEL, FunctionLines, LineBounds,
    LineIncrement, analyze, is_stale, is_within,
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
            ..LspSection::default()
        },
        ..LspConfig::default()
    }
}

fn scoped(hover: HoverScope, diagnostic_range: DiagnosticRange) -> LspConfig {
    LspConfig {
        max_complexity_allowed: 2,
        lsp: LspSection {
            hover,
            diagnostic_range,
            ..LspSection::default()
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
    let mut default = analyzed(COLLAPSIBLE_IF, &config(15));
    assert!(
        default
            .plans(1, &config(15))
            .iter()
            .any(|plan| plan.rule_id == "C007")
    );

    let config = LspConfig {
        ignore: StringOrList::Many(vec!["C007".to_string()]),
        ..LspConfig::default()
    };
    let mut narrowed = analyzed(COLLAPSIBLE_IF, &config);
    let plans = narrowed.plans(1, &config);

    assert!(plans.iter().all(|plan| plan.rule_id != "C007"));
    assert!(plans.iter().any(|plan| plan.rule_id == "C001"));
}

#[test]
fn inline_rule_list_keeps_the_other_rules() {
    let source = COLLAPSIBLE_IF.replace(
        "def heavy(a, b, c, d):",
        "def heavy(a, b, c, d):  # noqa: complexipy[C007]",
    );

    let mut analysis = analyzed(&source, &config(15));
    let plans = analysis.plans(1, &config(15));

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
    assert_eq!(hints[0].padding_left, Some(true));
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
    assert!(hints.iter().all(|hint| hint.padding_left == Some(true)));
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
    assert_eq!(
        diagnostics[0].range,
        Range::new(Position::new(4, 4), Position::new(4, 9))
    );
}

#[test]
fn diagnostics_underline_the_function_name() {
    for (source, expected) in [
        (
            DECORATED,
            Range::new(Position::new(1, 4), Position::new(1, 9)),
        ),
        (
            ASYNC,
            Range::new(Position::new(0, 10), Position::new(0, 15)),
        ),
        (
            MULTILINE,
            Range::new(Position::new(0, 4), Position::new(0, 9)),
        ),
        (
            DECORATOR_STRING,
            Range::new(Position::new(5, 4), Position::new(5, 9)),
        ),
    ] {
        let analysis = analyzed(source, &config(0));
        let diagnostics = analysis.diagnostics(&config(0));

        assert_eq!(diagnostics.len(), 1, "{:?}", source);
        assert_eq!(diagnostics[0].range, expected, "{:?}", source);
    }
}

#[test]
fn diagnostics_can_cover_the_whole_function() {
    let config = scoped(HoverScope::Signature, DiagnosticRange::Function);
    let analysis = analyzed(SAMPLE, &config);
    let diagnostics = analysis.diagnostics(&config);

    assert_eq!(
        diagnostics[0].range,
        Range::new(Position::new(4, 0), Position::new(8, 12))
    );
}

#[test]
fn cr_line_breaks_place_diagnostics_on_the_same_lines() {
    for newline in ["\r", "\r\n"] {
        let source = SAMPLE.replace('\n', newline);
        for (diagnostic_range, expected) in [
            (
                DiagnosticRange::Name,
                Range::new(Position::new(4, 4), Position::new(4, 9)),
            ),
            (
                DiagnosticRange::Function,
                Range::new(Position::new(4, 0), Position::new(8, 12)),
            ),
        ] {
            let config = scoped(HoverScope::Signature, diagnostic_range);
            let analysis = analyzed(&source, &config);
            let diagnostics = analysis.diagnostics(&config);

            assert_eq!(diagnostics.len(), 1, "{newline:?}");
            assert_eq!(diagnostics[0].range, expected, "{newline:?}");
        }
    }
}

#[test]
fn hover_builds_plans_for_a_cr_source() {
    let source = "def f(xs, a, b, c):\n    if c:\r        for x in xs:\r            if a:\r                if b:\r                    print(x)\n";
    let config = scoped(HoverScope::Function, DiagnosticRange::Name);
    let mut analysis = analyzed(source, &config);

    assert!(analysis.hover(&config, Position::new(3, 0)).is_some());
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
    let mut analysis = analyzed(SAMPLE, &config(2));
    let hover = analysis.hover(&config(2), Position::new(4, 0)).unwrap();

    let HoverContents::Markup(content) = hover.contents else {
        panic!("expected markup content");
    };

    assert!(content.value.contains("**heavy**: cognitive complexity 3"));
    assert!(content.value.contains("Exceeds the allowed 2"));
    assert!(content.value.contains("Top refactor"));
}

#[test]
fn hover_shows_the_function_name_literally() {
    let source = "class Box:\n    def __init__(self, a, b):\n        if a:\n            if b:\n                return\n";
    let mut analysis = analyzed(source, &config(2));
    let hover = analysis.hover(&config(2), Position::new(1, 0)).unwrap();

    let HoverContents::Markup(content) = hover.contents else {
        panic!("expected markup content");
    };

    assert!(content.value.starts_with("**Box\\:\\:\\_\\_init\\_\\_**"));
}

#[test]
fn hover_outside_a_function_is_empty() {
    let mut analysis = analyzed(SAMPLE, &config(2));

    assert!(analysis.hover(&config(2), Position::new(2, 0)).is_none());
}

#[test]
fn hover_answers_only_on_the_signature_by_default() {
    let mut analysis = analyzed(SAMPLE, &config(2));
    let hover = analysis.hover(&config(2), Position::new(4, 0)).unwrap();

    assert_eq!(
        hover.range,
        Some(Range::new(Position::new(4, 0), Position::new(4, 16)))
    );
    assert!(
        analysis
            .plans_needed(&config(2), Position::new(5, 0))
            .is_none()
    );
    assert!(analysis.hover(&config(2), Position::new(5, 0)).is_none());
}

#[test]
fn hover_covers_every_line_of_a_multiline_signature() {
    let mut analysis = analyzed(MULTILINE, &config(2));

    for line in 0..4 {
        assert!(analysis.hover(&config(2), Position::new(line, 0)).is_some());
    }
    assert!(analysis.hover(&config(2), Position::new(4, 0)).is_none());
}

#[test]
fn hover_can_cover_the_whole_function() {
    let config = scoped(HoverScope::Function, DiagnosticRange::Name);
    let mut analysis = analyzed(SAMPLE, &config);
    let hover = analysis.hover(&config, Position::new(5, 0)).unwrap();

    assert_eq!(
        hover.range,
        Some(Range::new(Position::new(4, 0), Position::new(8, 12)))
    );
}

#[test]
fn hover_can_be_disabled() {
    let config = scoped(HoverScope::Never, DiagnosticRange::Name);
    let mut analysis = analyzed(SAMPLE, &config);

    assert!(
        analysis
            .plans_needed(&config, Position::new(4, 0))
            .is_none()
    );
    assert!(analysis.hover(&config, Position::new(4, 0)).is_none());
}

#[test]
fn line_complexities_ignore_the_hint_settings() {
    let config = config_with(15, InlayHints::Never, false, false);
    let result = analyzed(SAMPLE, &config).line_complexities(&config);

    assert_eq!(result.version, 1);
    assert_eq!(result.max_complexity_allowed, 15);
    assert_eq!(
        result.functions,
        vec![
            FunctionLines {
                name: "light".to_string(),
                complexity: 0,
                line: 0,
                lines: Vec::new(),
            },
            FunctionLines {
                name: "heavy".to_string(),
                complexity: 3,
                line: 4,
                lines: vec![
                    LineIncrement {
                        line: 5,
                        complexity: 1,
                    },
                    LineIncrement {
                        line: 6,
                        complexity: 2,
                    },
                ],
            },
        ]
    );
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

    assert!(!failure.message.is_empty());
    assert!(!failure.message.contains("Failed to parse"));
    assert!(!failure.message.contains("byte range"));
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
        Range::new(Position::new(0, 11), Position::new(0, 12))
    );
}

fn parse_error_range(source: &str) -> Range {
    let Err(failure) = analyze(source, 1, &config(15)) else {
        panic!("expected a syntax error");
    };

    failure.diagnostic().range
}

#[test]
fn a_syntax_error_sits_on_its_own_line() {
    let source = format!("{}def broken(:\n", "x = 1\n".repeat(39));

    assert_eq!(parse_error_range(&source).start, Position::new(39, 11));
}

#[test]
fn the_error_column_counts_utf16_code_units() {
    let source = "name = \"\u{1F600}\u{e9}\"; def broken(:\n";
    let ascii = "name = \"ab\"; def broken(:\n";

    assert_eq!(
        parse_error_range(source).start.character,
        parse_error_range(ascii).start.character + 1
    );
}

#[test]
fn an_error_at_the_end_stays_inside_the_document() {
    let source = "values = [1, 2\n\n";
    let range = parse_error_range(source);

    assert_eq!(range.start.line, 0);
    assert!(range.start <= range.end);
}

#[test]
fn analysis_builds_no_plans() {
    let analysis = analyzed(COLLAPSIBLE_IF, &config(2));

    assert!(
        analysis
            .functions
            .iter()
            .all(|function| function.refactor_plans.is_empty())
    );
}

#[test]
fn hover_names_the_top_plan_rule() {
    let mut analysis = analyzed(COLLAPSIBLE_IF, &config(2));
    let hover = analysis.hover(&config(2), Position::new(0, 0)).unwrap();

    let HoverContents::Markup(content) = hover.contents else {
        panic!("expected markup content");
    };

    assert!(content.value.contains("Top refactor (`C007`)"));
}

#[test]
fn plans_are_built_once_per_analysis() {
    let mut analysis = analyzed(COLLAPSIBLE_IF, &config(2));
    let first: Vec<String> = analysis
        .plans(1, &config(2))
        .iter()
        .map(|plan| plan.rule_id.clone())
        .collect();
    let narrowed = LspConfig {
        ignore: StringOrList::Many(vec!["C007".to_string()]),
        ..config(2)
    };

    let second: Vec<String> = analysis
        .plans(1, &narrowed)
        .iter()
        .map(|plan| plan.rule_id.clone())
        .collect();

    assert!(first.contains(&"C007".to_string()));
    assert_eq!(first, second);
}

#[test]
fn a_bracketed_ignore_hides_the_plan_from_hover() {
    let source = COLLAPSIBLE_IF.replace(
        "def heavy(a, b, c, d):",
        "def heavy(a, b, c, d):  # complexipy: ignore[C007, C001]",
    );
    let mut analysis = analyzed(&source, &config(2));
    let hover = analysis.hover(&config(2), Position::new(0, 0)).unwrap();

    let HoverContents::Markup(content) = hover.contents else {
        panic!("expected markup content");
    };

    assert!(!content.value.contains("Top refactor"));
}

#[test]
fn a_comma_separated_ignore_string_drops_each_rule() {
    let config = LspConfig {
        ignore: StringOrList::One("C007, C001".to_string()),
        ..config(2)
    };
    let mut analysis = analyzed(COLLAPSIBLE_IF, &config);
    let plans = analysis.plans(1, &config);

    assert!(
        plans
            .iter()
            .all(|plan| plan.rule_id != "C007" && plan.rule_id != "C001")
    );
}
