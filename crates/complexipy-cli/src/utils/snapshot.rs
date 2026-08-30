use std::collections::HashMap;
use std::path::Path;

use complexipy_core::classes::FileComplexity;
use complexipy_core::utils::{ExportError, create_snapshot_file_shared, load_snapshot_file_shared};

pub struct SnapshotEvaluation {
    pub should_run: bool,
    pub active_snapshot_map: Option<HashMap<(String, String, String), u64>>,
    pub watermark_success: bool,
    pub watermark_messages: Vec<String>,
    pub snapshot_result: bool,
}

pub fn evaluate_snapshot(
    snapshot_create: bool,
    snapshot_ignore: bool,
    output_snapshot_path: &str,
    max_complexity_allowed: u64,
    files_complexities: &[FileComplexity],
) -> Result<SnapshotEvaluation, ExportError> {
    handle_snapshot_file_creation(
        snapshot_create,
        output_snapshot_path,
        max_complexity_allowed,
        files_complexities,
    )?;

    let snapshot_file_exists = Path::new(output_snapshot_path).exists();
    let snapshot_files = handle_snapshot_functions_load(output_snapshot_path)?;
    let should_run = snapshot_file_exists && !snapshot_ignore;

    let active_snapshot_map = if should_run {
        Some(build_snapshot_map(&snapshot_files))
    } else {
        None
    };

    let (watermark_success, watermark_messages) = handle_snapshot_watermark(
        should_run,
        snapshot_file_exists,
        output_snapshot_path,
        files_complexities,
        &snapshot_files,
        max_complexity_allowed,
    )?;

    let snapshot_result = if should_run { watermark_success } else { true };

    Ok(SnapshotEvaluation {
        should_run,
        active_snapshot_map,
        watermark_success,
        watermark_messages,
        snapshot_result,
    })
}

fn handle_snapshot_file_creation(
    create_snapshot: bool,
    snapshot_file_path: &str,
    max_complexity_allowed: u64,
    files_complexities: &[FileComplexity],
) -> Result<(), ExportError> {
    if create_snapshot {
        let snapshot_files = handle_snapshot_functions_load(snapshot_file_path)?;
        create_snapshot_file_shared(
            snapshot_file_path,
            max_complexity_allowed,
            merge_snapshot_files(snapshot_files, files_complexities),
        )?;
    }
    Ok(())
}

fn handle_snapshot_functions_load(
    snapshot_file_path: &str,
) -> Result<Vec<FileComplexity>, ExportError> {
    if Path::new(snapshot_file_path).exists() {
        load_snapshot_file_shared(snapshot_file_path)
    } else {
        Ok(Vec::new())
    }
}

pub fn merge_snapshot_files(
    snapshot_files: Vec<FileComplexity>,
    files_complexities: &[FileComplexity],
) -> Vec<FileComplexity> {
    let current_entries: HashMap<(String, String), &FileComplexity> = files_complexities
        .iter()
        .map(|file_complexity| {
            (
                (
                    file_complexity.path.clone(),
                    file_complexity.file_name.clone(),
                ),
                file_complexity,
            )
        })
        .collect();

    let mut merged = Vec::new();
    let mut merged_keys = std::collections::HashSet::new();

    for file_complexity in snapshot_files {
        let key = (
            file_complexity.path.clone(),
            file_complexity.file_name.clone(),
        );
        if let Some(current) = current_entries.get(&key) {
            if merged_keys.insert(key) {
                merged.push((*current).clone());
            }
        } else if merged_keys.insert(key) {
            merged.push(file_complexity);
        }
    }

    for file_complexity in files_complexities {
        let key = (
            file_complexity.path.clone(),
            file_complexity.file_name.clone(),
        );
        if merged_keys.insert(key) {
            merged.push(file_complexity.clone());
        }
    }

    merged
}

fn handle_snapshot_watermark(
    snapshot_watermark: bool,
    snapshot_exists: bool,
    output_snapshot_path: &str,
    files_complexities: &[FileComplexity],
    snapshot_files: &[FileComplexity],
    max_complexity_allowed: u64,
) -> Result<(bool, Vec<String>), ExportError> {
    if !snapshot_watermark {
        return Ok((true, Vec::new()));
    }

    if !snapshot_exists {
        return Ok((
            false,
            vec![
                "Snapshot watermark requested but no snapshot file was found. Run complexipy with --snapshot-create first.".to_string(),
            ],
        ));
    }

    let snapshot_map = build_snapshot_map(snapshot_files);
    let mut violations = Vec::new();

    for file_complexity in files_complexities {
        for function in file_complexity
            .functions
            .iter()
            .filter(|function| function.complexity > max_complexity_allowed)
        {
            let key = build_function_key(
                &file_complexity.path,
                &file_complexity.file_name,
                &function.name,
            );
            let previous_complexity = snapshot_map.get(&key);

            if let Some(previous_complexity) = previous_complexity {
                if function.complexity > *previous_complexity {
                    violations.push(format!(
                        "{} increased from {} to {}.",
                        format_function_location(
                            &file_complexity.path,
                            &file_complexity.file_name,
                            &function.name
                        ),
                        previous_complexity,
                        function.complexity
                    ));
                }
            } else {
                violations.push(format!(
                    "{} exceeds {} but was not part of the snapshot.",
                    format_function_location(
                        &file_complexity.path,
                        &file_complexity.file_name,
                        &function.name
                    ),
                    max_complexity_allowed
                ));
            }
        }
    }

    if !violations.is_empty() {
        return Ok((false, violations));
    }

    create_snapshot_file_shared(
        output_snapshot_path,
        max_complexity_allowed,
        merge_snapshot_files(snapshot_files.to_vec(), files_complexities),
    )?;

    Ok((true, Vec::new()))
}

pub fn build_snapshot_map(
    snapshot_files: &[FileComplexity],
) -> HashMap<(String, String, String), u64> {
    let mut snapshot_map = HashMap::new();
    for file_complexity in snapshot_files {
        for function in &file_complexity.functions {
            let key = build_function_key(
                &file_complexity.path,
                &file_complexity.file_name,
                &function.name,
            );
            snapshot_map.insert(key, function.complexity);
        }
    }
    snapshot_map
}

fn build_function_key(
    path: &str,
    file_name: &str,
    function_name: &str,
) -> (String, String, String) {
    (
        path.to_string(),
        file_name.to_string(),
        function_name.to_string(),
    )
}

fn format_function_location(path: &str, file_name: &str, function_name: &str) -> String {
    let location = if path.is_empty() {
        file_name.to_string()
    } else {
        Path::new(path)
            .join(file_name)
            .to_string_lossy()
            .into_owned()
    };
    format!("{}:{}", location, function_name)
}

#[cfg(test)]
mod tests;
