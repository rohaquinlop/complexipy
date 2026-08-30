use std::path::Path;

use crate::classes::{CodeComplexity, FileComplexity};
use crate::cognitive_complexity::code_complexity_shared;
use crate::runner::file_complexity_shared;

pub fn code_complexity(
    code: &str,
    check_script: bool,
    no_ignore: bool,
) -> Result<CodeComplexity, String> {
    code_complexity_shared(code, check_script, no_ignore)
}

pub fn file_complexity(
    file_path: &str,
    check_script: bool,
    no_ignore: bool,
) -> Result<FileComplexity, String> {
    let path = std::fs::canonicalize(file_path)
        .map_err(|e| format!("Failed to resolve file '{}': {}", file_path, e))?;
    let cwd = std::env::current_dir()
        .map_err(|e| format!("Failed to resolve the current directory: {}", e))?;

    let base_path = if path.starts_with(&cwd) {
        cwd
    } else {
        path.parent()
            .map(|parent| parent.to_path_buf())
            .unwrap_or_else(|| Path::new(".").to_path_buf())
    };

    file_complexity_shared(
        &to_posix(&path),
        &to_posix(&base_path),
        check_script,
        no_ignore,
    )
}

fn to_posix(path: &Path) -> String {
    let value = path.to_string_lossy();
    if cfg!(windows) {
        value.replace('\\', "/")
    } else {
        value.into_owned()
    }
}

#[cfg(test)]
mod tests;
