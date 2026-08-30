use crate::output::diff::format_diff;
use complexipy_core::diff::DiffEntry;

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
    let output = format_diff(&[], "main");

    assert_eq!(output, "No functions changed relative to main.");
}

#[test]
fn unchanged_only_renders_no_changes() {
    let entries = vec![entry("a.py", "f", Some(5), Some(5))];

    let output = format_diff(&entries, "main");

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

    let output = strip_ansi(&format_diff(&entries, "main"));

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

    let output = strip_ansi(&format_diff(&entries, "main"));

    assert!(output.contains("2 regressed"));
    assert!(output.contains("1 improved"));
    assert!(output.contains("1 new"));
    assert!(output.contains("1 removed"));
}

#[test]
fn status_ordering_in_summary() {
    let entries = vec![entry("a.py", "f", Some(3), Some(6))];

    let output = strip_ansi(&format_diff(&entries, "main"));

    let net_index = output.find("Net:").expect("net line");
    let summary = &output[net_index..];
    assert!(summary.contains("1 regressed"));
}
