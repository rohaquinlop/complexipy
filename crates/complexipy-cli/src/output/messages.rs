use owo_colors::{OwoColorize, Stream, Style};

use crate::utils::snapshot::SnapshotEvaluation;
use complexipy_core::classes::RemovableIgnore;
use complexipy_core::diff::{UnresolvedFile, UnresolvedReason};
pub use complexipy_core::{no_active_rule_warning, unknown_rule_warning};

pub fn handle_snapshot_console(snap: &SnapshotEvaluation, output_snapshot_path: &str) -> String {
    if !snap.should_run {
        return String::new();
    }

    if snap.watermark_messages.is_empty() {
        format!(
            "Snapshot watermark passed. Baseline stored at {}",
            output_snapshot_path
        )
    } else {
        snap.watermark_messages
            .iter()
            .map(|message| {
                format!(
                    "{}: {}",
                    "Snapshot watermark".if_supports_color(Stream::Stdout, |text| text
                        .style(Style::new().bold().red())),
                    message
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}

pub fn ignored_summary_output(location_count: usize, no_ignore: bool) -> String {
    let mut output = if location_count == 0 {
        "No ignore comments found.".to_string()
    } else {
        format!("Found {} suppressed location(s).", location_count)
    };
    if location_count > 0 && no_ignore {
        output.push_str("\n(all markers ignored due to --no-ignore)");
    }
    output
}

pub fn ignored_saved_output(path: &str) -> String {
    format!("Ignored locations saved at {}", path)
}

pub fn removable_ignores_output(removable: &[RemovableIgnore]) -> String {
    if removable.is_empty() {
        return String::new();
    }
    let mut lines = vec![
        "\nThe following ignore comment(s) are no longer necessary (complexity is within the allowed limit) and can be removed:".to_string(),
    ];
    for ignore in removable {
        lines.push(format!(
            "{}:{}  function={} complexity={}  {}",
            ignore.path, ignore.line, ignore.function, ignore.complexity, ignore.comment
        ));
    }
    lines.join("\n")
}

pub fn diff_flags_warning() -> String {
    "--diff and --diff-only both set. Using --diff-only (visual only, no enforcement).".to_string()
}

/// One warning for the files outside any git repository, and one per
/// repository where `git_ref` names no commit. The functions of those files
/// report as NEW.
pub fn unresolved_diff_warnings(unresolved: &[UnresolvedFile], git_ref: &str) -> Vec<String> {
    let mut warnings = Vec::new();

    let outside: Vec<&str> = unresolved
        .iter()
        .filter(|file| file.reason == UnresolvedReason::NotInRepository)
        .map(|file| file.file_path.as_str())
        .collect();
    match outside.as_slice() {
        [] => {}
        [path] => warnings.push(format!(
            "{} is not inside a git repository, so all its functions report as NEW.",
            path
        )),
        [path, rest @ ..] => warnings.push(format!(
            "{} and {} other file(s) are not inside a git repository, so all their \
             functions report as NEW.",
            path,
            rest.len()
        )),
    }

    let mut repositories: Vec<(&str, usize)> = Vec::new();
    for file in unresolved {
        let UnresolvedReason::UnknownRef { repository } = &file.reason else {
            continue;
        };
        match repositories
            .iter_mut()
            .find(|(seen, _)| *seen == repository.as_str())
        {
            Some((_, count)) => *count += 1,
            None => repositories.push((repository, 1)),
        }
    }
    for (repository, count) in repositories {
        warnings.push(format!(
            "{:?} does not name a commit in {}, so all functions in its {} analyzed \
             file(s) report as NEW.",
            git_ref, repository, count
        ));
    }

    warnings
}

#[cfg(test)]
mod tests;
