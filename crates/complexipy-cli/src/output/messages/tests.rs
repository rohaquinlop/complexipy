use crate::output::messages::{
    config_gate_change_warning, diff_flags_warning, handle_snapshot_console, ignored_saved_output,
    ignored_summary_output, removable_ignores_output, unknown_rule_warning,
    unmatchable_exclude_warning,
};
use crate::types::ConfigGateChange;
use crate::utils::snapshot::SnapshotEvaluation;
use complexipy_core::classes::{IgnoredLocation, RemovableIgnore};

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
fn unknown_rule_warning_names_the_rule() {
    assert_eq!(
        unknown_rule_warning("C999"),
        "Unknown rule id 'C999'; it matches no registered rule."
    );
}

#[test]
fn unknown_rule_warning_handles_an_empty_id() {
    assert_eq!(
        unknown_rule_warning(""),
        "Empty rule id in select or ignore; it matches no rule."
    );
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
    let location = |path: &str, line: u64| IgnoredLocation {
        path: path.to_string(),
        line,
        comment: "# complexipy: ignore".to_string(),
    };
    assert_eq!(
        ignored_summary_output(&[], false),
        "No ignore comments found."
    );
    assert_eq!(
        ignored_summary_output(&[location("a.py", 3), location("b.py", 7)], false),
        "a.py:3  # complexipy: ignore\nb.py:7  # complexipy: ignore\nFound 2 suppressed location(s)."
    );
    let with_flag = ignored_summary_output(&[location("a.py", 3)], true);
    assert!(with_flag.contains("Found 1 suppressed location(s)."));
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

#[test]
fn config_gate_change_warning_names_the_file_and_the_pin() {
    let warning = config_gate_change_warning(
        "/repo/complexipy.toml",
        &ConfigGateChange::DiffRef("HEAD".to_string()),
    );

    assert!(warning.starts_with("/repo/complexipy.toml sets [diff] branch = \"HEAD\""));
    assert!(warning.contains("--isolated"));
    for change in [
        ConfigGateChange::StagedDiff,
        ConfigGateChange::SnapshotCreate,
    ] {
        assert!(config_gate_change_warning("cfg.toml", &change).starts_with("cfg.toml sets "));
    }
}

#[test]
fn unmatchable_pattern_warning_names_the_pattern_and_a_fix() {
    let warning = unmatchable_exclude_warning("./legacy/**");

    assert!(warning.contains("\"./legacy/**\""));
    assert!(warning.contains("legacy/**`"));
}
