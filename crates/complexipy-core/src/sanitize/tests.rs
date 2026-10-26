//! Unit tests for `crate::sanitize`: terminal escaping and CSV formula cells.

use std::borrow::Cow;

use super::{escape_controls, neutralize_csv_cell};

#[test]
fn plain_text_passes_through_unchanged() {
    for text in ["src/app.py", "Svc::value", "    if a:\tpass", "café ✅"] {
        assert!(matches!(escape_controls(text), Cow::Borrowed(_)), "{text}");
    }
}

#[test]
fn escape_sequences_and_line_breaks_become_visible() {
    assert_eq!(escape_controls("a\u{1b}[2Jb"), "a\\u{1b}[2Jb");
    assert_eq!(escape_controls("x\u{7}y\u{9b}z"), "x\\u{7}y\\u{9b}z");
    assert_eq!(
        escape_controls("one\ntwo\rthree"),
        "one\\u{a}two\\u{d}three"
    );
}

#[test]
fn bidirectional_overrides_become_visible() {
    assert_eq!(
        escape_controls("a\u{202e}b\u{2066}c"),
        "a\\u{202e}b\\u{2066}c"
    );
}

#[test]
fn formula_cells_are_neutralized() {
    for cell in ["=1+1", "+cmd", "-2", "@SUM(A1)", "\tx", "\rx"] {
        assert_eq!(neutralize_csv_cell(cell), format!("'{cell}"));
    }
    assert_eq!(neutralize_csv_cell("src/app.py"), "src/app.py");
}
