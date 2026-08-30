use std::fmt;
use std::fs;
use std::path::Path;

use crate::types::OutputFormat;

#[derive(Debug, PartialEq)]
pub enum PathsError {
    StdoutNotSupported,
    MultiFormatNotDirectory,
    MultiFormatNoTrailingSeparator,
    Io(String),
}

impl fmt::Display for PathsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::StdoutNotSupported => write!(
                f,
                "Writing machine-readable output to stdout is not supported."
            ),
            Self::MultiFormatNotDirectory => write!(
                f,
                "When multiple output formats are selected, --output must point to a directory."
            ),
            Self::MultiFormatNoTrailingSeparator => write!(
                f,
                "When multiple output formats are selected, --output must point to a directory or end with a path separator."
            ),
            Self::Io(message) => write!(f, "{}", message),
        }
    }
}

impl std::error::Error for PathsError {}

pub fn resolve_output_paths(
    output_formats: &[OutputFormat],
    output: Option<&str>,
    invocation_path: &Path,
) -> Result<Vec<(OutputFormat, String)>, PathsError> {
    if output_formats.is_empty() {
        return Ok(Vec::new());
    }

    let Some(output) = output else {
        return Ok(build_output_paths(invocation_path, output_formats));
    };

    if output == "-" {
        return Err(PathsError::StdoutNotSupported);
    }

    let destination = std::path::absolute(output).map_err(|e| PathsError::Io(e.to_string()))?;
    let is_directory_hint = is_directory_output_hint(output);

    if output_formats.len() > 1 {
        ensure_directory_destination(&destination, is_directory_hint)?;
        return Ok(build_output_paths(&destination, output_formats));
    }

    let output_format = &output_formats[0];
    if destination.is_dir() || is_directory_hint {
        fs::create_dir_all(&destination).map_err(|e| PathsError::Io(e.to_string()))?;
        return Ok(build_output_paths(&destination, output_formats));
    }

    if let Some(parent) = destination
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent).map_err(|e| PathsError::Io(e.to_string()))?;
    }

    Ok(vec![(
        output_format.clone(),
        destination.to_string_lossy().into_owned(),
    )])
}

pub fn build_output_paths(
    destination: &Path,
    output_formats: &[OutputFormat],
) -> Vec<(OutputFormat, String)> {
    output_formats
        .iter()
        .map(|output_format| {
            (
                output_format.clone(),
                destination
                    .join(output_format.default_output_filename())
                    .to_string_lossy()
                    .into_owned(),
            )
        })
        .collect()
}

pub fn is_directory_output_hint(output: &str) -> bool {
    output.ends_with('/') || (cfg!(windows) && output.ends_with('\\'))
}

pub fn normalize_path(path: &str, file_name: &str) -> String {
    let cleaned = path.trim_end_matches('/');
    if cleaned.ends_with(file_name) {
        cleaned.to_string()
    } else if !cleaned.is_empty() {
        format!("{}/{}", cleaned, file_name)
    } else {
        file_name.to_string()
    }
}

pub fn ensure_directory_destination(
    destination: &Path,
    is_directory_hint: bool,
) -> Result<(), PathsError> {
    if destination.exists() && !destination.is_dir() {
        return Err(PathsError::MultiFormatNotDirectory);
    } else if !is_directory_hint {
        return Err(PathsError::MultiFormatNoTrailingSeparator);
    }

    fs::create_dir_all(destination).map_err(|e| PathsError::Io(e.to_string()))
}

#[cfg(test)]
mod tests;
