use std::collections::{HashMap, HashSet};

use crate::types::{FileEntry, FunctionRow, Sort};
use crate::utils::paths::normalize_path;
use complexipy_core::classes::{FileComplexity, FunctionComplexity};
use complexipy_core::diff::function_keys;

/// The functions a diff gate fails, by file path and function key.
pub type DiffFailures = HashSet<(String, String)>;

/// The rows of a run's report.
pub struct Rows {
    pub files: Vec<FileEntry>,
    pub total_functions: u64,
    pub all_pass: bool,
    /// The functions above the threshold that pass, because a gate finds
    /// them no worse than its baseline.
    pub allowed: u64,
}

pub fn build_output_rows(
    files: &[FileComplexity],
    failed_only: bool,
    sort: Sort,
    max_complexity: u64,
    snapshot_map: Option<&HashMap<(String, String, String), u64>>,
    diff_failures: Option<&DiffFailures>,
) -> Rows {
    let mut rows = Rows {
        files: Vec::new(),
        total_functions: 0,
        all_pass: true,
        allowed: 0,
    };

    for file in files {
        let keys: HashMap<(&str, u64), String> = file
            .functions
            .iter()
            .zip(function_keys(&file.functions))
            .map(|(function, key)| ((function.name.as_str(), function.line_start), key))
            .collect();
        let sorted_functions = sort_functions(&file.functions, &sort);
        let mut displayable_functions = Vec::new();

        for function in sorted_functions {
            rows.total_functions += 1;
            let passed = is_function_passing(
                &function,
                &keys[&(function.name.as_str(), function.line_start)],
                &file.path,
                &file.file_name,
                max_complexity,
                snapshot_map,
                diff_failures,
            );

            if !passed {
                rows.all_pass = false;
            } else if function.complexity > max_complexity {
                rows.allowed += 1;
            }

            if failed_only && passed {
                continue;
            }

            displayable_functions.push(FunctionRow {
                name: function.name.clone(),
                complexity: function.complexity,
                passed,
                path: file.path.clone(),
                file_name: file.file_name.clone(),
                refactor_plans: function.refactor_plans.clone(),
                additional_refactor_plans: function.additional_refactor_plans,
            });
        }

        if !displayable_functions.is_empty() {
            rows.files.push(FileEntry {
                path: normalize_path(&file.path, &file.file_name),
                functions: displayable_functions,
            });
        }
    }

    rows
}

pub fn sort_functions(functions: &[FunctionComplexity], sort: &Sort) -> Vec<FunctionComplexity> {
    let mut sorted = functions.to_vec();
    match sort {
        Sort::FileName => sorted.sort_by_key(|function| function.name.to_lowercase()),
        Sort::Asc => sorted.sort_by_key(|function| function.complexity),
        Sort::Desc => {
            sorted.sort_by_key(|function| function.complexity);
            sorted.reverse();
        }
    }
    sorted
}

/// Whether `function` passes, looked up by `key` from `function_keys`.
/// When a diff decides the exit code, `diff_failures` holds the functions
/// its gate fails and every other function passes the diff. Otherwise a
/// function passes at or under the threshold. With a snapshot, a function
/// also needs to be at or under the threshold or no worse than its entry.
pub fn is_function_passing(
    function: &FunctionComplexity,
    key: &str,
    file_path: &str,
    file_name: &str,
    max_complexity: u64,
    snapshot_map: Option<&HashMap<(String, String, String), u64>>,
    diff_failures: Option<&DiffFailures>,
) -> bool {
    if let Some(diff_failures) = diff_failures {
        if diff_failures.contains(&(file_path.to_string(), key.to_string())) {
            return false;
        }
        if snapshot_map.is_none() {
            return true;
        }
    }
    if function.complexity <= max_complexity {
        return true;
    }
    let Some(snapshot_map) = snapshot_map else {
        return false;
    };
    let previous = snapshot_map.get(&(
        file_path.to_string(),
        file_name.to_string(),
        key.to_string(),
    ));
    previous.is_some_and(|previous| function.complexity <= *previous)
}

/// The functions that fail, by file path and function key, judged as
/// `is_function_passing` judges them.
pub fn failing_functions(
    files: &[FileComplexity],
    max_complexity: u64,
    snapshot_map: Option<&HashMap<(String, String, String), u64>>,
    diff_failures: Option<&DiffFailures>,
) -> DiffFailures {
    files
        .iter()
        .flat_map(|file| {
            file.functions
                .iter()
                .zip(function_keys(&file.functions))
                .filter(|(function, key)| {
                    !is_function_passing(
                        function,
                        key,
                        &file.path,
                        &file.file_name,
                        max_complexity,
                        snapshot_map,
                        diff_failures,
                    )
                })
                .map(|(_, key)| (file.path.clone(), key))
        })
        .collect()
}

pub fn has_success_functions(
    files: &[FileComplexity],
    max_complexity: u64,
    snapshot_map: Option<&HashMap<(String, String, String), u64>>,
    diff_failures: Option<&DiffFailures>,
) -> bool {
    files.iter().all(|file| {
        file.functions
            .iter()
            .zip(function_keys(&file.functions))
            .all(|(function, key)| {
                is_function_passing(
                    function,
                    &key,
                    &file.path,
                    &file.file_name,
                    max_complexity,
                    snapshot_map,
                    diff_failures,
                )
            })
    })
}

pub fn truncate_top_n(file_entries: Vec<FileEntry>, n: u64) -> Vec<FileEntry> {
    let mut all_functions: Vec<(String, FunctionRow)> = Vec::new();
    for entry in &file_entries {
        for function in &entry.functions {
            all_functions.push((entry.path.clone(), function.clone()));
        }
    }

    all_functions.sort_by_key(|right| std::cmp::Reverse(right.1.complexity));
    all_functions.truncate(n as usize);

    let mut result: Vec<FileEntry> = Vec::new();
    for (path, function) in all_functions {
        if let Some(last) = result.last_mut()
            && last.path == path
        {
            last.functions.push(function);
        } else {
            result.push(FileEntry {
                path,
                functions: vec![function],
            });
        }
    }
    result
}

#[cfg(test)]
mod tests;
