pub mod diff;
pub mod messages;
pub mod refactor;
pub mod render;
pub mod rows;

use std::collections::HashMap;

use crate::output::render::{SummaryOptions, output_summary};
use crate::output::rows::has_success_functions;
use crate::types::{OutputFormat, Sort};
use crate::utils::cache::remember_previous_functions;
use crate::utils::gitlab::store_gitlab;
use crate::utils::paths::resolve_output_paths;
use crate::utils::sarif::store_sarif;
use complexipy_core::classes::FileComplexity;
use complexipy_core::utils::{ExportError, output_csv_shared, output_json_shared};

pub struct DisplayOptions<'a> {
    pub files_complexities: &'a [FileComplexity],
    pub paths: &'a [String],
    pub failed: bool,
    pub sort: Sort,
    pub ignore_complexity: bool,
    pub max_complexity_allowed: u64,
    pub active_snapshot_map: Option<&'a HashMap<(String, String, String), u64>>,
    pub quiet: bool,
    pub plain: bool,
    pub invocation_path: &'a str,
    pub cache_dir: Option<&'a str>,
    pub top: Option<u64>,
    pub suggest_refactors: bool,
}

pub fn handle_display(options: DisplayOptions) -> (bool, String) {
    let DisplayOptions {
        files_complexities,
        paths,
        failed,
        sort,
        ignore_complexity,
        max_complexity_allowed,
        active_snapshot_map,
        quiet,
        plain,
        invocation_path,
        cache_dir,
        top,
        suggest_refactors,
    } = options;

    let previous_functions = if !files_complexities.is_empty() {
        remember_previous_functions(invocation_path, paths, files_complexities, cache_dir)
    } else {
        None
    };

    if quiet {
        let has_success = has_success_functions(
            files_complexities,
            max_complexity_allowed,
            active_snapshot_map,
        );
        return (has_success, String::new());
    }

    let effective_sort = if top.is_some() { Sort::Desc } else { sort };
    let (has_success, output) = output_summary(SummaryOptions {
        files: files_complexities,
        failed_only: failed,
        sort: effective_sort,
        ignore_complexity,
        max_complexity: max_complexity_allowed,
        previous_functions: previous_functions.as_ref(),
        snapshot_map: active_snapshot_map,
        plain,
        top,
        suggest_refactors,
        invocation_path,
    });
    (has_success, output)
}

pub struct StorageOptions<'a> {
    pub output_formats: &'a [OutputFormat],
    pub output: Option<&'a str>,
    pub files_complexities: &'a [FileComplexity],
    pub sort: Sort,
    pub show_details: bool,
    pub max_complexity: u64,
    pub invocation_path: &'a str,
    pub suggest_refactors: bool,
}

pub fn handle_results_storage(options: StorageOptions) -> Result<Vec<String>, ExportError> {
    let StorageOptions {
        output_formats,
        output,
        files_complexities,
        sort,
        show_details,
        max_complexity,
        invocation_path,
        suggest_refactors,
    } = options;

    let output_paths = resolve_output_paths(
        output_formats,
        output,
        std::path::Path::new(invocation_path),
    )
    .map_err(|error| ExportError::Io(error.to_string()))?;
    let mut saved_lines = Vec::new();

    for (output_format, output_path) in output_paths {
        match output_format {
            OutputFormat::Csv => output_csv_shared(
                &output_path,
                files_complexities.to_vec(),
                sort_value(&sort),
                show_details,
                max_complexity,
            )?,
            OutputFormat::Json => output_json_shared(
                &output_path,
                files_complexities.to_vec(),
                show_details,
                max_complexity,
                suggest_refactors,
            )?,
            OutputFormat::Gitlab => store_gitlab(
                &output_path,
                files_complexities,
                max_complexity,
                suggest_refactors,
            )?,
            OutputFormat::Sarif => store_sarif(
                &output_path,
                files_complexities,
                max_complexity,
                suggest_refactors,
            )?,
        }
        saved_lines.push(format!("Results saved at {}", output_path));
    }

    Ok(saved_lines)
}

fn sort_value(sort: &Sort) -> &'static str {
    match sort {
        Sort::Asc => "asc",
        Sort::Desc => "desc",
        Sort::FileName => "file_name",
    }
}

pub fn effective_sort_for_display(sort: Sort, top: Option<u64>) -> Sort {
    if top.is_some() { Sort::Desc } else { sort }
}
