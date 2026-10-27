use crate::output::diff::format_diff;
use complexipy_core::diff::{DiffEntry, Verdict};

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

fn entry(file_path: &str, func_name: &str, old: Option<u64>, new: Option<u64>) -> DiffEntry {
    DiffEntry {
        file_path: file_path.to_string(),
        func_name: func_name.to_string(),
        old_complexity: old,
        new_complexity: new,
    }
}

#[test]
fn no_entries_renders_no_changes() {
    let output = format_diff(&[], "main", None);

    assert_eq!(output, "No functions changed relative to main.");
}

#[test]
fn unchanged_only_renders_no_changes() {
    let entries = vec![entry("a.py", "f", Some(5), Some(5))];

    let output = format_diff(&entries, "main", None);

    assert_eq!(output, "No functions changed relative to main.");
}

#[test]
fn table_contains_all_statuses() {
    let entries = vec![
        entry("a.py", "reg", Some(3), Some(6)),
        entry("a.py", "imp", Some(6), Some(3)),
        entry("b.py", "new_fn", None, Some(5)),
        entry("b.py", "gone", Some(5), None),
    ];

    let output = strip_ansi(&format_diff(&entries, "main", None));

    assert!(output.contains("Complexity diff (vs main)"));
    assert!(output.contains("a.py::reg"));
    assert!(output.contains("a.py::imp"));
    assert!(output.contains("b.py::new_fn"));
    assert!(output.contains("b.py::gone"));
    assert!(output.contains("3 \u{2192} 6  (+3)"));
    assert!(output.contains("6 \u{2192} 3  (-3)"));
    assert!(output.contains("(new)"));
    assert!(output.contains("(removed)"));
}

#[test]
fn summary_counts() {
    let entries = vec![
        entry("a.py", "r1", Some(3), Some(6)),
        entry("a.py", "r2", Some(3), Some(6)),
        entry("a.py", "i1", Some(6), Some(3)),
        entry("b.py", "n1", None, Some(5)),
        entry("b.py", "g1", Some(5), None),
    ];

    let output = strip_ansi(&format_diff(&entries, "main", None));

    assert!(output.contains("2 regressed"));
    assert!(output.contains("1 improved"));
    assert!(output.contains("1 new"));
    assert!(output.contains("1 removed"));
}

#[test]
fn status_ordering_in_summary() {
    let entries = vec![entry("a.py", "f", Some(3), Some(6))];

    let output = strip_ansi(&format_diff(&entries, "main", None));

    let net_index = output.find("Net:").expect("net line");
    let summary = &output[net_index..];
    assert!(summary.contains("1 regressed"));
}

#[test]
fn the_gate_column_names_both_sides_of_a_replacement() {
    let entries = vec![
        entry("b.py", "f", None, Some(20)),
        entry("a.py", "f", Some(20), None),
    ];
    let verdicts = [Verdict::Replaces(1), Verdict::Passes];

    let output = strip_ansi(&format_diff(&entries, "main", Some(&verdicts)));

    assert!(output.contains("Gate"));
    assert!(output.contains("replaces a.py::f"));
    assert!(output.contains("replaced by b.py::f"));
}

fn header(output: &str) -> &str {
    output
        .lines()
        .find(|line| line.contains("Status"))
        .expect("header row")
}

#[test]
fn no_gate_column_without_a_gate_note() {
    let entries = vec![entry("a.py", "f", Some(3), Some(4))];

    let output = strip_ansi(&format_diff(&entries, "main", Some(&[Verdict::Passes])));

    assert!(!header(&output).contains("Gate"));
    assert!(output.ends_with("Gate: no function fails the gate."));
}

#[test]
fn the_gate_column_marks_each_entry_that_fails() {
    let entries = vec![
        entry("a.py", "worse", Some(31), Some(35)),
        entry("a.py", "small", Some(0), Some(1)),
        entry("b.py", "fresh", None, Some(20)),
    ];
    let verdicts = [Verdict::Fails, Verdict::Passes, Verdict::Fails];

    let output = strip_ansi(&format_diff(&entries, "main", Some(&verdicts)));

    assert!(header(&output).contains("Gate"));
    let failing: Vec<&str> = output
        .lines()
        .filter(|line| line.contains("FAILS"))
        .collect();
    assert_eq!(failing.len(), 2);
    assert!(failing[0].contains("a.py::worse"));
    assert!(failing[1].contains("b.py::fresh"));
    assert!(output.ends_with("Gate: 2 functions fail: REGRESSED or NEW above the threshold."));
}

#[test]
fn a_diff_only_table_has_no_gate() {
    let entries = vec![entry("a.py", "worse", Some(31), Some(35))];

    let output = strip_ansi(&format_diff(&entries, "main", None));

    assert!(!output.contains("Gate"));
    assert!(!output.contains("FAILS"));
}
