//! Tests that `\r`, `\r\n`, and `\n` line breaks analyze, plan, and fix alike.

use complexipy_core::classes::{FunctionComplexity, RefactorPlan};
use complexipy_core::cognitive_complexity::code_complexity_shared;
use complexipy_core::fix::apply_fixes;
use complexipy_core::utils::{LineIndex, split_lines};
use complexipy_core::{AnalysisOptions, RuleSet};

const NESTED_LOOP: &str = "def f(xs, a, b, c):\n    if c:\n        for x in xs:\n            if a:\n                if b:\n                    print(x)\n";
const IGNORED_LAST: &str = "def f(a, b, c):\n    if a:\n        if b:\n            if c:\n                return 1\ndef g(a, b, c):\n    if a:\n        if b:\n            if c:\n                return 1\ndef h():\n    return 1  # complexipy: ignore\n";
const MULTIBYTE_COMMENT: &str = "def f(a, b, c, d):\n    x = 1\n    # \u{20ac}\u{20ac}\u{20ac}\u{20ac}\u{20ac}\u{20ac}\n    if a:\n        if b:\n            if c:\n                if d:\n                    return 1\n";

fn with_plans() -> AnalysisOptions {
    AnalysisOptions {
        with_plans: true,
        rules: RuleSet::default(),
        ..Default::default()
    }
}

fn analyze(code: &str) -> Vec<FunctionComplexity> {
    code_complexity_shared(code, &with_plans())
        .expect("code should parse")
        .functions
}

type Summary = (String, u64, u64, u64, Vec<(u64, u64)>, Vec<RefactorPlan>);

fn summary(code: &str) -> Vec<Summary> {
    analyze(code)
        .into_iter()
        .map(|function| {
            (
                function.name,
                function.complexity,
                function.line_start,
                function.line_end,
                function
                    .line_complexities
                    .iter()
                    .map(|line| (line.line, line.complexity))
                    .collect(),
                function.refactor_plans,
            )
        })
        .collect()
}

fn converted(code: &str, newline: &str) -> String {
    code.replace('\n', newline)
}

#[test]
fn every_terminator_starts_a_line() {
    let code = "a\rb\r\nc\nd";
    let index = LineIndex::new(code);

    let lines: Vec<&str> = (1..=4)
        .map(|line| index.line_text(code, line).expect("line exists"))
        .collect();

    assert_eq!(lines, ["a", "b", "c", "d"]);
    assert_eq!(index.line_of(code.find('d').unwrap()), 4);
    assert_eq!(index.line_text(code, 5), None);
}

#[test]
fn split_lines_matches_str_lines_and_breaks_on_a_lone_cr() {
    for code in ["", "\n", "a", "a\n", "a\n\nb", "a\r\nb\r\n", "a\nb"] {
        assert_eq!(
            split_lines(code).collect::<Vec<_>>(),
            code.lines().collect::<Vec<_>>(),
            "{code:?}"
        );
    }
    assert_eq!(
        split_lines("a\rb\r\rc\r").collect::<Vec<_>>(),
        ["a", "b", "", "c"]
    );
}

#[test]
fn line_breaks_do_not_change_the_analysis() {
    for code in [NESTED_LOOP, IGNORED_LAST, MULTIBYTE_COMMENT] {
        let expected = summary(code);
        for newline in ["\r", "\r\n"] {
            assert_eq!(summary(&converted(code, newline)), expected, "{newline:?}");
        }
    }
}

#[test]
fn a_marker_in_one_function_does_not_suppress_the_whole_file_with_cr_breaks() {
    let names: Vec<String> = analyze(&converted(IGNORED_LAST, "\r"))
        .into_iter()
        .map(|function| function.name)
        .collect();

    assert_eq!(names, ["f", "g", "h"]);
}

#[test]
fn a_loop_guard_fix_keeps_cr_breaks() {
    let code = converted(NESTED_LOOP, "\r");
    let plans: Vec<_> = analyze(&code)
        .into_iter()
        .flat_map(|function| function.refactor_plans)
        .collect();

    let report = apply_fixes(&code, &plans);

    assert!(!report.applied.is_empty());
    assert!(!report.patched.contains('\n'));
    assert_eq!(
        report.patched.matches('\r').count(),
        split_lines(&report.patched).count()
    );
}
