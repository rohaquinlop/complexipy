use std::collections::HashMap;

use crate::types::{FileEntry, FunctionRow, Sort};
use crate::utils::paths::normalize_path;
use complexipy_core::classes::{FileComplexity, FunctionComplexity};

pub fn build_output_rows(
    files: &[FileComplexity],
    failed_only: bool,
    sort: Sort,
    max_complexity: u64,
    snapshot_map: Option<&HashMap<(String, String, String), u64>>,
) -> (Vec<FileEntry>, u64, bool) {
    let mut file_entries = Vec::new();
    let mut total_functions = 0u64;
    let mut all_pass = true;

    for file in files {
        let sorted_functions = sort_functions(&file.functions, &sort);
        let mut displayable_functions = Vec::new();

        for function in sorted_functions {
            total_functions += 1;
            let passed = is_function_passing(
                &function,
                &file.path,
                &file.file_name,
                max_complexity,
                snapshot_map,
            );

            if !passed {
                all_pass = false;
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
            file_entries.push(FileEntry {
                path: normalize_path(&file.path, &file.file_name),
                functions: displayable_functions,
            });
        }
    }

    (file_entries, total_functions, all_pass)
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

pub fn is_function_passing(
    function: &FunctionComplexity,
    file_path: &str,
    file_name: &str,
    max_complexity: u64,
    snapshot_map: Option<&HashMap<(String, String, String), u64>>,
) -> bool {
    if function.complexity <= max_complexity {
        return true;
    }
    let Some(snapshot_map) = snapshot_map else {
        return false;
    };
    let previous = snapshot_map.get(&(
        file_path.to_string(),
        file_name.to_string(),
        function.name.clone(),
    ));
    previous.is_some_and(|previous| function.complexity <= *previous)
}

pub fn has_success_functions(
    files: &[FileComplexity],
    max_complexity: u64,
    snapshot_map: Option<&HashMap<(String, String, String), u64>>,
) -> bool {
    files.iter().all(|file| {
        file.functions.iter().all(|function| {
            is_function_passing(
                function,
                &file.path,
                &file.file_name,
                max_complexity,
                snapshot_map,
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
