use std::path::Path;
use std::process::ExitCode;

macro_rules! emit {
    ($($arguments:tt)*) => {
        crate::output::emit(format_args!($($arguments)*))
    };
}

macro_rules! emit_error {
    ($($arguments:tt)*) => {
        crate::output::emit_error(format_args!($($arguments)*))
    };
}

use owo_colors::{OwoColorize, Stream};

use crate::args::CliArgs;
use crate::output::diff::format_diff;
use crate::output::messages::{
    config_gate_change_warning, diff_flags_warning, handle_snapshot_console,
    ignore_rule_skips_output, ignored_saved_output, ignored_summary_output, no_active_rule_warning,
    nothing_selected_warning, removable_ignores_output, skipped_path_warning, unknown_rule_warning,
    unmatchable_exclude_warning, unresolved_diff_warnings,
};
use crate::output::render::{
    color_enabled_for, handle_console_settings, print_invalid_paths, rule,
};
use crate::output::rows::{DiffFailures, failing_functions};
use crate::output::{DisplayOptions, StorageOptions, handle_display, handle_results_storage};
use crate::types::{DiffBaseline, ExitReport, OutputFormat, RunConfig};
use crate::utils::config::resolve_config;
use crate::utils::fix::run_fix_pass;
use crate::utils::ignored::handle_report_ignored;
use crate::utils::snapshot::evaluate_snapshot;
use crate::utils::toml::load_run_config;
use complexipy_core::classes::FileComplexity;
use complexipy_core::diff::{
    DiffReport, Verdict, compute_diff_report, compute_staged_diff, gate_verdicts,
    resolve_diff_flags,
};
use complexipy_core::runner::{Analysis, ArgumentCoverage, LocationReports};
use complexipy_core::sanitize::escape_controls;
use complexipy_core::{
    AnalysisOptions, RuleSet, registered_rule_ids, unmatchable_exclude_patterns,
};

pub fn run_at(cli: CliArgs, invocation_path: &str) -> ExitCode {
    let source = match load_run_config(&cli, invocation_path) {
        Ok(source) => source,
        Err(error) => {
            emit_error!("{}", escape_controls(&error.to_string()));
            return ExitCode::FAILURE;
        }
    };
    let config_path = source
        .as_ref()
        .map(|source| source.path.display().to_string());

    let config = match resolve_config(source.map(|source| source.value), cli, invocation_path) {
        Ok(config) => config,
        Err(error) => {
            emit_error!("{}", escape_controls(&error.to_string()));
            return ExitCode::FAILURE;
        }
    };

    let settings = handle_console_settings(&config.color, config.quiet, config.plain);
    let label = if color_enabled_for(&config.color, config.plain, supports_color::Stream::Stderr) {
        "Warning:".yellow().to_string()
    } else {
        "Warning:".to_string()
    };
    if let Some(config_path) = &config_path {
        for change in &config.gate_changes_from_config {
            emit_error!(
                "{} {}",
                label,
                escape_controls(&config_gate_change_warning(config_path, change))
            );
        }
    }
    if !settings.banner.is_empty() {
        emit!("{}", settings.banner);
    }

    if !config.quiet
        && let Some(warning) =
            diff_flags_warning(config.diff.as_deref(), config.diff_only.as_deref())
    {
        emit!(
            "{} {}",
            "Warning:".if_supports_color(Stream::Stdout, |text| text.yellow()),
            warning
        );
    }
    let (diff, diff_only) =
        resolve_diff_flags(config.diff.clone(), config.diff_only.clone(), config.staged);

    let known_rules = registered_rule_ids();
    let (rules, unknown_rules) = RuleSet::resolve(&config.select, &config.ignore, &known_rules);
    if !config.quiet {
        let mut warnings: Vec<String> = unknown_rules
            .iter()
            .map(|rule_id| unknown_rule_warning(rule_id))
            .collect();
        if !rules.has_active_rule(&known_rules) {
            warnings.push(no_active_rule_warning());
        }
        warnings.extend(
            unmatchable_exclude_patterns(&config.exclude)
                .iter()
                .map(|pattern| unmatchable_exclude_warning(pattern)),
        );
        for warning in warnings {
            emit_error!("{} {}", label, escape_controls(&warning));
        }
    }
    let analysis = analysis_options(&config, rules);

    let mut run = Analysis::run(
        &config.paths,
        &config.exclude,
        &analysis,
        invocation_path,
        LocationReports {
            ignored: config.report_ignored,
            removable_above: (!config.quiet).then_some(config.max_complexity_allowed),
        },
    );
    if !config.quiet {
        for warning in run.skipped.iter().filter_map(skipped_path_warning) {
            emit_error!("{} {}", label, escape_controls(&warning));
        }
    }
    if run.files.is_empty() && run.failed_paths.is_empty() {
        emit_error!("{} {}", label, nothing_selected_warning(&run.skipped));
    }
    let skipped = std::mem::take(&mut run.skipped);

    let mut fix_console = String::new();
    if config.fix || config.dry_run {
        let pass = run_fix_pass(
            &run.files,
            config.dry_run,
            invocation_path,
            settings.color_enabled,
            &analysis,
            config.max_complexity_allowed,
        );
        fix_console = pass.console;
        if !pass.written.is_empty() {
            run.reanalyze(&pass.written);
        }
    }
    let files_complexities = std::mem::take(&mut run.files);
    let failed_paths = std::mem::take(&mut run.failed_paths);

    let output_snapshot_path = format!("{}/complexipy-snapshot.json", invocation_path);
    let snap = match evaluate_snapshot(
        config.snapshot_create,
        config.snapshot_ignore,
        !config.dry_run,
        &output_snapshot_path,
        config.max_complexity_allowed,
        &files_complexities,
    ) {
        Ok(snap) => snap,
        Err(error) => {
            emit_error!("{}", escape_controls(&error.to_string()));
            return ExitCode::FAILURE;
        }
    };

    let run_diff = diff
        .as_deref()
        .or(diff_only.as_deref())
        .and_then(|git_ref| {
            compute_run_diff(
                &config,
                diff.is_some(),
                git_ref,
                &files_complexities,
                invocation_path,
            )
        });
    let compared = run_diff
        .as_ref()
        .filter(|run_diff| run_diff.report.is_some());
    let diff_failures = compared.filter(|_| diff.is_some()).map(RunDiff::failures);
    let diff_baseline = compared.map(|run_diff| {
        run_diff.baseline(diff_failures.as_ref().map(|diff_failures| {
            failing_functions(
                &files_complexities,
                config.max_complexity_allowed,
                snap.active_snapshot_map.as_ref(),
                Some(diff_failures),
            )
        }))
    });

    match handle_results_storage(StorageOptions {
        output_formats: &config.output_format,
        output: config.output.as_deref(),
        files_complexities: &files_complexities,
        sort: config.sort.clone(),
        show_details: !config.failed,
        max_complexity: config.max_complexity_allowed,
        invocation_path,
        suggest_refactors: config.suggest_refactors && !config.dry_run,
        diff_baseline: diff_baseline.as_ref(),
    }) {
        Ok(saved_lines) => {
            for line in saved_lines {
                emit!("{}", line);
            }
        }
        Err(error) => {
            emit_error!("{}", escape_controls(&error.to_string()));
            return ExitCode::FAILURE;
        }
    }

    let (display_ok, display_output) = handle_display(DisplayOptions {
        files_complexities: &files_complexities,
        paths: &config.paths,
        failed: config.failed,
        sort: config.sort.clone(),
        ignore_complexity: config.ignore_complexity,
        max_complexity_allowed: config.max_complexity_allowed,
        active_snapshot_map: snap.active_snapshot_map.as_ref(),
        diff_failures: diff_failures.as_ref(),
        quiet: config.quiet,
        plain: config.plain,
        invocation_path,
        cache_dir: config.cache_dir.as_deref(),
        write_cache: !config.dry_run,
        top: config.top,
        suggest_refactors: config.suggest_refactors && !config.dry_run,
        color: settings.color_enabled,
    });
    if !display_output.is_empty() {
        emit!("{}", display_output);
    }

    if !config.quiet && !fix_console.is_empty() {
        emit!("{}", fix_console);
    }

    let (ignored_locations, ignored_json_path) = match handle_report_ignored(
        config.report_ignored,
        &run,
        &config.output_format,
        config.output.as_deref(),
        invocation_path,
    ) {
        Ok(result) => result,
        Err(error) => {
            emit_error!("{}", escape_controls(&error.to_string()));
            return ExitCode::FAILURE;
        }
    };
    if config.report_ignored {
        if !config.quiet {
            emit!(
                "{}",
                ignored_summary_output(&ignored_locations, config.no_ignore)
            );
            emit!("{}", ignore_rule_skips_output(&skipped));
        }
        if let Some(path) = ignored_json_path {
            emit!("{}", escape_controls(&ignored_saved_output(&path)));
        }
    }

    if !config.quiet {
        let removable_output = removable_ignores_output(&run.removable_ignores());
        if !removable_output.is_empty() {
            emit!("{}", removable_output);
        }
    }

    let snapshot_ok = if config.quiet {
        if snap.should_run {
            snap.watermark_success
        } else {
            true
        }
    } else {
        let snapshot_output = handle_snapshot_console(&snap, &output_snapshot_path);
        if !snapshot_output.is_empty() {
            emit!("{}", snapshot_output);
        }
        snap.watermark_success
    };

    let (paths_ok, invalid_paths_output) = print_invalid_paths(&failed_paths);
    if !invalid_paths_output.is_empty() {
        emit!("{}", invalid_paths_output);
    }

    if !config.quiet
        && let Some(run_diff) = &run_diff
    {
        match &run_diff.report {
            Some(report) => {
                emit!(
                    "{}",
                    format_diff(
                        &report.entries,
                        &run_diff.label,
                        run_diff.verdicts.as_deref()
                    )
                );
                for warning in unresolved_diff_warnings(&report.unresolved, &run_diff.git_ref) {
                    emit!(
                        "{} {}",
                        "Warning:".if_supports_color(Stream::Stdout, |text| text.yellow()),
                        escape_controls(&warning)
                    );
                }
            }
            None => emit!(
                "{} --staged requires a git repository; skipping the staged diff, so the threshold does not decide the exit code.",
                "Warning:".if_supports_color(Stream::Stdout, |text| text.yellow())
            ),
        }
    }
    let diff_ok = run_diff
        .as_ref()
        .and_then(|run_diff| run_diff.verdicts.as_ref())
        .is_none_or(|verdicts| !verdicts.contains(&Verdict::Fails));

    if !config.quiet && !config.plain {
        if cfg!(windows) {
            emit!("{}", rule("Analysis completed!"));
        } else {
            emit!("{}", rule("🎉 Analysis completed! 🎉"));
        }
    }

    let report = ExitReport {
        display_ok,
        snapshot_ok,
        paths_ok,
        diff_ok,
        enforce_diff: diff.is_some(),
    };
    if report.success() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn analysis_options(config: &RunConfig, rules: RuleSet) -> AnalysisOptions {
    AnalysisOptions {
        check_script: config.check_script,
        no_ignore: config.no_ignore,
        with_plans: config.suggest_refactors,
        plans_above: (!reports_passing_plans(config)).then_some(config.max_complexity_allowed),
        rules,
    }
}

/// Returns whether some output of the run shows the plans of a function at
/// or under the threshold: the console and JSON list passing functions
/// unless `--failed` is set, and SARIF and GitLab report every plan. The fix
/// pass reads only the plans of functions over the threshold.
fn reports_passing_plans(config: &RunConfig) -> bool {
    let shows_plans = config.suggest_refactors && !config.dry_run;
    let lists_passing =
        !config.failed && (!config.quiet || config.output_format.contains(&OutputFormat::Json));
    let reports_every_plan = config
        .output_format
        .iter()
        .any(|format| matches!(format, OutputFormat::Sarif | OutputFormat::Gitlab));
    shows_plans && (lists_passing || reports_every_plan)
}

/// The comparison a run makes against a git ref.
struct RunDiff {
    git_ref: String,
    label: String,
    /// `None` when `--staged` runs outside a repository.
    report: Option<DiffReport>,
    /// The gate's verdict on each entry, when the diff decides the exit code.
    verdicts: Option<Vec<Verdict>>,
}

impl RunDiff {
    /// The functions the gate fails, by file path and function key.
    fn failures(&self) -> DiffFailures {
        let (Some(report), Some(verdicts)) = (&self.report, &self.verdicts) else {
            return DiffFailures::new();
        };
        report
            .entries
            .iter()
            .zip(verdicts)
            .filter(|(_, verdict)| **verdict == Verdict::Fails)
            .map(|(entry, _)| (entry.file_path.clone(), entry.func_name.clone()))
            .collect()
    }

    /// The complexity each current function compares with, the old
    /// function's for one that replaces it, and `failing`, the functions
    /// that fail the run when the diff decides the exit code.
    fn baseline(&self, failing: Option<DiffFailures>) -> DiffBaseline {
        let mut baseline = DiffBaseline {
            failing,
            ..DiffBaseline::default()
        };
        let Some(report) = &self.report else {
            return baseline;
        };
        for (index, entry) in report.entries.iter().enumerate() {
            if entry.new_complexity.is_none() {
                continue;
            }
            let complexity = match self.verdicts.as_ref().map(|verdicts| verdicts[index]) {
                Some(Verdict::Replaces(removed)) => report.entries[removed].old_complexity,
                _ => entry.old_complexity,
            };
            baseline.complexities.insert(
                (entry.file_path.clone(), entry.func_name.clone()),
                complexity,
            );
        }
        baseline
    }
}

fn compute_run_diff(
    config: &RunConfig,
    enforce: bool,
    git_ref: &str,
    files: &[FileComplexity],
    invocation_path: &str,
) -> Option<RunDiff> {
    let coverage = ArgumentCoverage::new(&config.paths, &config.exclude, invocation_path);
    let covers = |path: &Path| coverage.covers(path);
    let gate = enforce.then_some(config.max_complexity_allowed);
    let (label, report) = if config.staged {
        (
            format!("{} (staged)", git_ref),
            compute_staged_diff(files, git_ref, invocation_path, &covers, gate),
        )
    } else if !files.is_empty() {
        (
            git_ref.to_string(),
            Some(compute_diff_report(
                files,
                git_ref,
                invocation_path,
                &covers,
                gate,
            )),
        )
    } else {
        return None;
    };
    let verdicts = report.as_ref().filter(|_| enforce).map(|report| {
        gate_verdicts(
            &report.entries,
            config.max_complexity_allowed,
            &report.bodies,
        )
    });
    Some(RunDiff {
        git_ref: git_ref.to_string(),
        label,
        report,
        verdicts,
    })
}

#[cfg(test)]
mod tests;
