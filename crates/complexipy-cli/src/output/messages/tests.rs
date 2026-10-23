use crate::output::messages::{
    diff_flags_warning, handle_snapshot_console, ignored_saved_output, ignored_summary_output,
    removable_ignores_output, unknown_rule_warning, unresolved_diff_warnings,
};
use crate::utils::snapshot::SnapshotEvaluation;
use complexipy_core::classes::RemovableIgnore;
use complexipy_core::diff::{UnresolvedFile, UnresolvedReason};

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
fn diff_flags_warning_needs_both_flags() {
    assert_eq!(
        diff_flags_warning(Some("main"), Some("dev")).as_deref(),
        Some("--diff and --diff-only both set. Using --diff-only (visual only, no enforcement).")
    );
    assert_eq!(diff_flags_warning(None, Some("dev")), None);
    assert_eq!(diff_flags_warning(Some("main"), None), None);
    assert_eq!(diff_flags_warning(None, None), None);
}

fn unresolved(file_path: &str, reason: UnresolvedReason) -> UnresolvedFile {
    UnresolvedFile {
        file_path: file_path.to_string(),
        reason,
    }
}

#[test]
fn unresolved_diff_warnings_name_each_cause_once() {
    let unknown_ref = || UnresolvedReason::UnknownRef {
        repository: "/repo".to_string(),
    };
    let files = vec![
        unresolved("/tmp/a.py", UnresolvedReason::NotInRepository),
        unresolved("x.py", unknown_ref()),
        unresolved("/tmp/b.py", UnresolvedReason::NotInRepository),
        unresolved("y.py", unknown_ref()),
        unresolved("/tmp/c.py", UnresolvedReason::NotInRepository),
    ];

    assert_eq!(
        unresolved_diff_warnings(&files, "v2"),
        vec![
            "/tmp/a.py and 2 other file(s) are not inside a git repository, so all their \
             functions report as NEW."
                .to_string(),
            "\"v2\" does not name a commit in /repo, so all functions in its 2 analyzed \
             file(s) report as NEW."
                .to_string(),
        ]
    );
}

#[test]
fn unresolved_diff_warnings_name_a_single_file() {
    let files = vec![unresolved("a.py", UnresolvedReason::NotInRepository)];

    assert_eq!(
        unresolved_diff_warnings(&files, "main"),
        vec!["a.py is not inside a git repository, so all its functions report as NEW."]
    );
    assert!(unresolved_diff_warnings(&[], "main").is_empty());
}

#[test]
fn unresolved_diff_warnings_name_a_nested_repository_without_a_usable_pin() {
    let files = vec![
        unresolved(
            "sub/a.py",
            UnresolvedReason::NotPinned {
                repository: "/repo/sub".to_string(),
            },
        ),
        unresolved(
            "lib/b.py",
            UnresolvedReason::PinnedCommitMissing {
                repository: "/repo/lib".to_string(),
                commit: "abc123".to_string(),
            },
        ),
    ];

    assert_eq!(
        unresolved_diff_warnings(&files, "main"),
        vec![
            "/repo/sub sits inside another repository but is not pinned there at \"main\", \
             so all functions in its 1 analyzed file(s) report as NEW."
                .to_string(),
            "/repo/lib is pinned at \"main\" to commit abc123, which it does not have, so \
             all functions in its 1 analyzed file(s) report as NEW."
                .to_string(),
        ]
    );
}
