use complexipy_core::fix::{AppliedFix, FixReport, SkipReason, SkippedFix};

use super::{format_fix_diff, format_fix_summary, no_fixes_output, pass_label};

const SOURCE: &str = "def a():\n    if x:\n        return 1\n";

fn applied_fix(line_start: u64, line_end: u64, replacement: &str) -> AppliedFix {
    AppliedFix {
        rule_id: "C007".to_string(),
        title: "Merge nested if statements".to_string(),
        line_start,
        line_end,
        replacement: replacement.to_string(),
        reduction: 2,
        reduction_is_measured: true,
    }
}

fn report() -> FixReport {
    FixReport {
        patched: "def a():\n    pass\n".to_string(),
        applied: vec![applied_fix(2, 3, "    pass")],
        skipped: vec![SkippedFix {
            rule_id: "C002".to_string(),
            line_start: 5,
            line_end: 6,
            reason: SkipReason::ParseFailure,
        }],
    }
}

#[test]
fn diff_renders_numbered_rows_with_context_and_the_rule_label() {
    let rendered = format_fix_diff("pkg/a.py", SOURCE, &report(), false);

    assert_eq!(
        rendered,
        concat!(
            "--- a/pkg/a.py\n",
            "+++ b/pkg/a.py\n",
            "@@ -1,3 +1,2 @@ C007 Merge nested if statements\n",
            "1 1  def a():\n",
            "2   -    if x:\n",
            "3   -        return 1\n",
            "  2 +    pass",
        )
    );
}

#[test]
fn diff_numbers_the_second_hunk_on_the_patched_side() {
    let source = "def a():\n    if x:\n        return 1\n\n\ndef b():\n    if y:\n        return 2";
    let report = FixReport {
        patched: "def a():\n    pass\n\n\ndef b():\n    pass".to_string(),
        applied: vec![applied_fix(2, 3, "    pass"), applied_fix(7, 8, "    pass")],
        skipped: vec![],
    };

    let rendered = format_fix_diff("pkg/a.py", source, &report, false);

    assert!(rendered.contains("@@ -1,4 +1,3 @@"));
    assert!(rendered.contains("@@ -6,3 +5,2 @@"));
    assert!(rendered.contains("4 3  \n"));
    assert!(rendered.contains("6 5  def b():"));
    assert!(rendered.contains("  6 +    pass"));
}

#[test]
fn colored_diff_carries_syntax_colors_and_background_tints() {
    let rendered = format_fix_diff("pkg/a.py", SOURCE, &report(), true);

    assert!(rendered.contains("48;2;58;25;29m"));
    assert!(rendered.contains("48;2;21;48;34m"));
    assert!(rendered.contains("\x1b[38;2;"));
    assert!(rendered.contains("--- a/pkg/a.py"));
}

#[test]
fn long_lines_truncate_to_the_terminal_width() {
    let long_line = format!("    value = \"{}\"", "x".repeat(200));
    let source = format!("def a():\n{long_line}\n    return 0\n");
    let report = FixReport {
        patched: "def a():\n    pass\n    return 0\n".to_string(),
        applied: vec![applied_fix(2, 2, "    pass")],
        skipped: vec![],
    };

    let rendered = format_fix_diff("pkg/a.py", &source, &report, false);

    let removed = rendered
        .lines()
        .find(|line| line.contains("value"))
        .expect("a removal row exists");
    assert!(removed.ends_with("..."));
    assert!(removed.chars().count() <= 80);
}

#[test]
fn summary_lists_applied_then_skipped_with_reasons() {
    let rendered = format_fix_summary("pkg/a.py", &report(), false);

    assert_eq!(
        rendered,
        concat!(
            "Fixed C007 at pkg/a.py:2-3 (-2 complexity)\n",
            "Skipped C002 at pkg/a.py:5-6 (would break the parse)",
        )
    );
}

#[test]
fn summary_marks_an_unmeasured_reduction_with_a_tilde() {
    let mut applied = applied_fix(2, 3, "    pass");
    applied.reduction = 3;
    applied.reduction_is_measured = false;
    let report = FixReport {
        patched: "def a():\n    pass\n".to_string(),
        applied: vec![applied],
        skipped: vec![],
    };

    let rendered = format_fix_summary("pkg/a.py", &report, false);

    assert_eq!(rendered, "Fixed C007 at pkg/a.py:2-3 (-~3 complexity)");
}

#[test]
fn colored_summary_labels_use_green_and_yellow() {
    let rendered = format_fix_summary("pkg/a.py", &report(), true);

    assert!(rendered.contains("\x1b[32m"));
    assert!(rendered.contains("\x1b[33m"));
}

#[test]
fn pass_label_is_plain_without_color_and_dimmed_with_color() {
    assert_eq!(pass_label(2, false), "pass 2:");
    assert!(pass_label(2, true).contains("\x1b["));
}

#[test]
fn empty_report_renders_empty_summary_and_no_fixes_message() {
    let empty = FixReport {
        patched: SOURCE.to_string(),
        applied: vec![],
        skipped: vec![],
    };

    assert_eq!(format_fix_summary("pkg/a.py", &empty, false), "");
    assert_eq!(format_fix_diff("pkg/a.py", SOURCE, &empty, false), "");
    assert_eq!(no_fixes_output(), "No fixes to apply.");
}
