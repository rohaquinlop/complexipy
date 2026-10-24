use crate::types::OutputFormat;
use crate::utils::paths::resolve_output_paths;
use complexipy_core::classes::IgnoredLocation;
use complexipy_core::fs_guard::write_atomic;
use complexipy_core::runner::Analysis;
use complexipy_core::utils::ExportError;

pub fn handle_report_ignored(
    report_ignored: bool,
    analysis: &Analysis,
    output_formats: &[OutputFormat],
    output: Option<&str>,
    invocation_path: &str,
) -> Result<(Vec<IgnoredLocation>, Option<String>), ExportError> {
    if !report_ignored {
        return Ok((Vec::new(), None));
    }

    let ignored_locations = analysis.ignored_locations();

    let ignored_json_path = if output_formats.contains(&OutputFormat::Json)
        && !ignored_locations.is_empty()
    {
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
        write_atomic(&ignored_json_path, format!("{}\n", serialized).as_bytes()).map_err(|e| {
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

#[cfg(test)]
mod tests;
