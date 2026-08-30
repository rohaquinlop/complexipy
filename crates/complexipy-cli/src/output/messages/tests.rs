use crate::output::messages::{
    diff_flags_warning, handle_snapshot_console, ignored_saved_output, ignored_summary_output,
    removable_ignores_output,
};
use crate::utils::snapshot::SnapshotEvaluation;
use complexipy_core::classes::RemovableIgnore;

fn evaluation(should_run: bool, messages: Vec<String>) -> SnapshotEvaluation {
    SnapshotEvaluation {
        should_run,
        active_snapshot_map: None,
        watermark_success: messages.is_empty(),
        watermark_messages: messages,
        snapshot_result: true,
    }
}

#[test]
fn snapshot_not_running_renders_empty() {
    let snap = evaluation(false, vec![]);
    assert_eq!(handle_snapshot_console(&snap, "snap.json"), "");
}

#[test]
fn snapshot_pass_line() {
    let snap = evaluation(true, vec![]);
    assert_eq!(
        handle_snapshot_console(&snap, "snap.json"),
        "Snapshot watermark passed. Baseline stored at snap.json"
    );
}

#[test]
fn snapshot_messages_rendered() {
    let snap = evaluation(true, vec!["a.py:f increased from 3 to 6.".to_string()]);
    let output = handle_snapshot_console(&snap, "snap.json");

    assert!(output.contains("Snapshot watermark"));
    assert!(output.contains("a.py:f increased from 3 to 6."));
}

#[test]
fn ignored_summary_variants() {
    assert_eq!(
        ignored_summary_output(0, false),
        "No ignore comments found."
    );
    assert_eq!(
        ignored_summary_output(3, false),
        "Found 3 suppressed location(s)."
    );
    let with_flag = ignored_summary_output(2, true);
    assert!(with_flag.contains("Found 2 suppressed location(s)."));
    assert!(with_flag.contains("(all markers ignored due to --no-ignore)"));
}

#[test]
fn ignored_saved_line() {
    assert_eq!(
        ignored_saved_output("/tmp/x/complexipy-ignored.json"),
        "Ignored locations saved at /tmp/x/complexipy-ignored.json"
    );
}

#[test]
fn removable_ignores_block() {
    let removable = vec![RemovableIgnore {
        path: "a.py".to_string(),
        line: 1,
        comment: "# complexipy: ignore".to_string(),
        function: "f".to_string(),
        complexity: 2,
    }];

    let output = removable_ignores_output(&removable);

    assert!(output.contains("no longer necessary"));
    assert!(output.contains("a.py:1  function=f complexity=2  # complexipy: ignore"));

    assert_eq!(removable_ignores_output(&[]), "");
}

#[test]
fn diff_flags_warning_text() {
    assert_eq!(
        diff_flags_warning(),
        "--diff and --diff-only both set. Using --diff-only (visual only, no enforcement)."
    );
}
