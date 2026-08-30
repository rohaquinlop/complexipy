use std::process::ExitCode;

use owo_colors::OwoColorize;

use crate::args::CliArgs;
use crate::output::messages::{
    diff_flags_warning, handle_snapshot_console, ignored_saved_output, ignored_summary_output,
    removable_ignores_output,
};
use crate::output::render::{handle_console_settings, print_invalid_paths, rule};
use crate::output::{DisplayOptions, StorageOptions, handle_display, handle_results_storage};
use crate::types::ExitReport;
use crate::utils::config::resolve_config;
use crate::utils::ignored::{handle_removable_ignores, handle_report_ignored};
use crate::utils::snapshot::evaluate_snapshot;
use crate::utils::toml::get_complexipy_toml_config;
use complexipy_core::diff::{
    compute_diff, compute_staged_diff, has_regressions, resolve_diff_flags,
};
use complexipy_core::runner::run_analysis_shared;

pub fn run_at(cli: CliArgs, invocation_path: &str) -> ExitCode {
    let toml_config = get_complexipy_toml_config(invocation_path);

    let config = match resolve_config(toml_config, cli) {
        Ok(config) => config,
        Err(error) => {
            eprintln!("{}", error);
            return ExitCode::FAILURE;
        }
    };

    let settings = handle_console_settings(&config.color, config.quiet, config.plain);
    if !settings.banner.is_empty() {
        println!("{}", settings.banner);
    }

    let (diff, diff_only) = resolve_diff_flags(config.diff, config.diff_only, config.staged);
    if !config.quiet && diff_only.is_some() && diff.is_none() {
        println!("{} {}", "Warning:".yellow(), diff_flags_warning());
    }

    let (files_complexities, failed_paths) = match run_analysis_shared(
        &config.paths,
        &config.exclude,
        config.check_script,
        config.no_ignore,
        invocation_path,
    ) {
        Ok(result) => result,
        Err(error) => {
            eprintln!("{}", error);
            return ExitCode::FAILURE;
        }
    };

    let output_snapshot_path = format!("{}/complexipy-snapshot.json", invocation_path);
    let snap = match evaluate_snapshot(
        config.snapshot_create,
        config.snapshot_ignore,
        &output_snapshot_path,
        config.max_complexity_allowed,
        &files_complexities,
    ) {
        Ok(snap) => snap,
        Err(error) => {
            eprintln!("{}", error);
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
        suggest_refactors: config.suggest_refactors,
    }) {
        Ok(saved_lines) => {
            for line in saved_lines {
                println!("{}", line);
            }
        }
        Err(error) => {
            eprintln!("{}", error);
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
        top: config.top,
        suggest_refactors: config.suggest_refactors,
    });
    if !display_output.is_empty() {
        println!("{}", display_output);
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
            eprintln!("{}", error);
            return ExitCode::FAILURE;
        }
    };
    if config.report_ignored {
        if !config.quiet {
            println!(
                "{}",
                ignored_summary_output(ignored_locations.len(), config.no_ignore)
            );
        }
        if let Some(path) = ignored_json_path {
            println!("{}", ignored_saved_output(&path));
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
                            "Warning:".yellow()
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

fn format_diff_for(entries: &[complexipy_core::diff::DiffEntry], git_ref: &str) -> String {
    crate::output::diff::format_diff(entries, git_ref)
}

#[cfg(test)]
mod tests;
