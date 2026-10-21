use std::process::ExitCode;

use owo_colors::{OwoColorize, Stream};

use crate::args::CliArgs;
use crate::output::messages::{
    config_gate_change_warning, diff_flags_warning, handle_snapshot_console,
    ignore_rule_skips_output, ignored_saved_output, ignored_summary_output, no_active_rule_warning,
    nothing_selected_warning, removable_ignores_output, skipped_path_warning, unknown_rule_warning,
};
use crate::output::render::{handle_console_settings, print_invalid_paths, rule};
use crate::output::{DisplayOptions, StorageOptions, handle_display, handle_results_storage};
use crate::types::{ExitReport, RunConfig};
use crate::utils::config::resolve_config;
use crate::utils::fix::run_fix_pass;
use crate::utils::ignored::{handle_removable_ignores, handle_report_ignored};
use crate::utils::snapshot::evaluate_snapshot;
use crate::utils::toml::load_run_config;
use complexipy_core::diff::{
    compute_diff, compute_staged_diff, has_regressions, resolve_diff_flags,
};
use complexipy_core::runner::{analyze_paths, run_analysis_shared};
use complexipy_core::sanitize::escape_controls;
use complexipy_core::{AnalysisOptions, RuleSet, registered_rule_ids};

pub fn run_at(cli: CliArgs, invocation_path: &str) -> ExitCode {
    let source = match load_run_config(&cli, invocation_path) {
        Ok(source) => source,
        Err(error) => {
            eprintln!("{}", escape_controls(&error.to_string()));
            return ExitCode::FAILURE;
        }
    };
    let config_path = source
        .as_ref()
        .map(|source| source.path.display().to_string());

    let config = match resolve_config(source.map(|source| source.value), cli, invocation_path) {
        Ok(config) => config,
        Err(error) => {
            eprintln!("{}", escape_controls(&error.to_string()));
            return ExitCode::FAILURE;
        }
    };

    let settings = handle_console_settings(&config.color, config.quiet, config.plain);
    if let Some(config_path) = &config_path {
        for change in &config.gate_changes_from_config {
            eprintln!(
                "{} {}",
                "Warning:".if_supports_color(Stream::Stderr, |text| text.yellow()),
                escape_controls(&config_gate_change_warning(config_path, change))
            );
        }
    }
    if !settings.banner.is_empty() {
        println!("{}", settings.banner);
    }

    let (diff, diff_only) =
        resolve_diff_flags(config.diff.clone(), config.diff_only.clone(), config.staged);
    if !config.quiet && diff_only.is_some() && diff.is_none() {
        println!(
            "{} {}",
            "Warning:".if_supports_color(Stream::Stdout, |text| text.yellow()),
            diff_flags_warning()
        );
    }

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
        for warning in warnings {
            eprintln!(
                "{} {}",
                "Warning:".if_supports_color(Stream::Stderr, |text| text.yellow()),
                escape_controls(&warning)
            );
        }
    }
    let analysis = analysis_options(&config, rules);

    let report = match analyze_paths(&config.paths, &config.exclude, &analysis, invocation_path) {
        Ok(report) => report,
        Err(error) => {
            eprintln!("{}", escape_controls(&error.to_string()));
            return ExitCode::FAILURE;
        }
    };
    if !config.quiet {
        for warning in report.skipped.iter().filter_map(skipped_path_warning) {
            eprintln!(
                "{} {}",
                "Warning:".if_supports_color(Stream::Stderr, |text| text.yellow()),
                escape_controls(&warning)
            );
        }
    }
    if report.files.is_empty() && report.failed_paths.is_empty() {
        eprintln!(
            "{} {}",
            "Warning:".if_supports_color(Stream::Stderr, |text| text.yellow()),
            nothing_selected_warning(&report.skipped)
        );
    }
    let mut files_complexities = report.files;
    let failed_paths = report.failed_paths;
    let skipped = report.skipped;

    let mut fix_console = String::new();
    if config.fix || config.dry_run {
        let pass = run_fix_pass(
            &files_complexities,
            config.dry_run,
            invocation_path,
            settings.color_enabled,
            &analysis,
            config.max_complexity_allowed,
        );
        fix_console = pass.console;
        if pass.wrote {
            match run_analysis_shared(&config.paths, &config.exclude, &analysis, invocation_path) {
                Ok((post_fix, _)) => files_complexities = post_fix,
                Err(error) => eprintln!("{}", escape_controls(&error)),
            }
        }
    }

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
            eprintln!("{}", escape_controls(&error.to_string()));
            return ExitCode::FAILURE;
        }
    };

    match handle_results_storage(StorageOptions {
        output_formats: &config.output_format,
        output: config.output.as_deref(),
        files_complexities: &files_complexities,
        sort: config.sort.clone(),
        show_details: !config.failed,
        max_complexity: config.max_complexity_allowed,
        invocation_path,
        suggest_refactors: config.suggest_refactors && !config.dry_run,
    }) {
        Ok(saved_lines) => {
            for line in saved_lines {
                println!("{}", line);
            }
        }
        Err(error) => {
            eprintln!("{}", escape_controls(&error.to_string()));
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
        quiet: config.quiet,
        plain: config.plain,
        invocation_path,
        cache_dir: config.cache_dir.as_deref(),
        write_cache: !config.dry_run,
        top: config.top,
        suggest_refactors: config.suggest_refactors && !config.dry_run,
    });
    if !display_output.is_empty() {
        println!("{}", display_output);
    }

    if !config.quiet && !fix_console.is_empty() {
        println!("{}", fix_console);
    }

    let (ignored_locations, ignored_json_path) = match handle_report_ignored(
        config.report_ignored,
        &config.paths,
        &config.exclude,
        &config.output_format,
        config.output.as_deref(),
        config.no_ignore,
        invocation_path,
    ) {
        Ok(result) => result,
        Err(error) => {
            eprintln!("{}", escape_controls(&error.to_string()));
            return ExitCode::FAILURE;
        }
    };
    if config.report_ignored {
        if !config.quiet {
            println!(
                "{}",
                ignored_summary_output(ignored_locations.len(), config.no_ignore)
            );
            println!("{}", ignore_rule_skips_output(&skipped));
        }
        if let Some(path) = ignored_json_path {
            println!("{}", escape_controls(&ignored_saved_output(&path)));
        }
    }

    if !config.quiet {
        let removable = handle_removable_ignores(
            &config.paths,
            &config.exclude,
            config.max_complexity_allowed,
            invocation_path,
        );
        let removable_output = removable_ignores_output(&removable);
        if !removable_output.is_empty() {
            println!("{}", removable_output);
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
            println!("{}", snapshot_output);
        }
        snap.watermark_success
    };

    let (paths_ok, invalid_paths_output) = print_invalid_paths(&failed_paths);
    if !invalid_paths_output.is_empty() {
        println!("{}", invalid_paths_output);
    }

    let diff_ref = diff.clone().or_else(|| diff_only.clone());
    let diff_entries = if let Some(diff_ref) = diff_ref {
        if config.staged {
            match compute_staged_diff(&diff_ref, invocation_path) {
                Some(entries) => {
                    if !config.quiet {
                        println!(
                            "{}",
                            format_diff_for(&entries, &format!("{} (staged)", diff_ref))
                        );
                    }
                    Some(entries)
                }
                None => {
                    if !config.quiet {
                        println!(
                            "{} --staged requires a git repository; skipping the staged diff.",
                            "Warning:".if_supports_color(Stream::Stdout, |text| text.yellow())
                        );
                    }
                    None
                }
            }
        } else if !files_complexities.is_empty() {
            let entries = compute_diff(&files_complexities, &diff_ref, invocation_path);
            if !config.quiet {
                println!("{}", format_diff_for(&entries, &diff_ref));
            }
            Some(entries)
        } else {
            None
        }
    } else {
        None
    };

    let mut diff_ok = true;
    if diff.is_some()
        && let Some(entries) = diff_entries
    {
        diff_ok = !has_regressions(&entries, config.max_complexity_allowed);
    }

    if !config.quiet && !config.plain {
        if cfg!(windows) {
            println!("{}", rule("Analysis completed!"));
        } else {
            println!("{}", rule("🎉 Analysis completed! 🎉"));
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
        rules,
    }
}

fn format_diff_for(entries: &[complexipy_core::diff::DiffEntry], git_ref: &str) -> String {
    crate::output::diff::format_diff(entries, git_ref)
}

#[cfg(test)]
mod tests;
