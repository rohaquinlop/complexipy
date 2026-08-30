use std::fs;

use crate::types::OutputFormat;
use crate::utils::paths::resolve_output_paths;
use complexipy_core::classes::{IgnoredLocation, RemovableIgnore};
use complexipy_core::runner::{
    collect_all_ignored_locations_shared, collect_removable_ignored_locations_shared,
};
use complexipy_core::utils::ExportError;

pub fn handle_report_ignored(
    report_ignored: bool,
    paths: &[String],
    exclude: &[String],
    output_formats: &[OutputFormat],
    output: Option<&str>,
    _no_ignore: bool,
    invocation_path: &str,
) -> Result<(Vec<IgnoredLocation>, Option<String>), ExportError> {
    if !report_ignored {
        return Ok((Vec::new(), None));
    }

    let (ignored_locations, _) =
        collect_all_ignored_locations_shared(paths, exclude, invocation_path)
            .map_err(ExportError::Io)?;

    let ignored_json_path =
        if output_formats.contains(&OutputFormat::Json) && !ignored_locations.is_empty() {
            let ignored_output_paths = resolve_output_paths(
                &[OutputFormat::Json],
                output,
                std::path::Path::new(invocation_path),
            )
            .map_err(|e| ExportError::Io(e.to_string()))?;
            let ignored_output_path = &ignored_output_paths[0].1;
            let ignored_dir = std::path::Path::new(ignored_output_path)
                .parent()
                .unwrap_or(std::path::Path::new("."));
            let ignored_json_path = ignored_dir.join("complexipy-ignored.json");

            let ignored_data: Vec<serde_json::Value> = ignored_locations
                .iter()
                .map(|location| {
                    serde_json::json!({
                        "path": location.path,
                        "line": location.line,
                        "comment": location.comment,
                    })
                })
                .collect();
            let serialized = serde_json::to_string_pretty(&ignored_data).map_err(|e| {
                ExportError::Serialize(format!("Failed to serialize ignored locations: {}", e))
            })?;
            fs::write(&ignored_json_path, format!("{}\n", serialized)).map_err(|e| {
                ExportError::Io(format!(
                    "Failed to write ignored locations to {}: {}",
                    ignored_json_path.display(),
                    e
                ))
            })?;

            Some(ignored_json_path.to_string_lossy().into_owned())
        } else {
            None
        };

    Ok((ignored_locations, ignored_json_path))
}

pub fn handle_removable_ignores(
    paths: &[String],
    exclude: &[String],
    max_complexity_allowed: u64,
    invocation_path: &str,
) -> Vec<RemovableIgnore> {
    collect_removable_ignored_locations_shared(
        paths,
        exclude,
        max_complexity_allowed,
        invocation_path,
    )
    .map(|(removable_ignores, _)| removable_ignores)
    .unwrap_or_default()
}

#[cfg(test)]
mod tests;
