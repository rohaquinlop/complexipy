use ignore::Walk;
use std::collections::HashSet;
use wax::walk::{Entry, FileIterator};
use wax::{Glob, Program, any};

pub fn is_path_excluded(path: &str, root: &str, patterns: &[String]) -> bool {
    if patterns.is_empty() {
        return false;
    }

    let normalized_path = path.replace('\\', "/");
    let normalized_root = root.replace('\\', "/");
    let relative = normalized_path
        .strip_prefix(normalized_root.trim_end_matches('/'))
        .map(|rest| rest.trim_start_matches('/'))
        .unwrap_or(normalized_path.as_str());
    let pattern_refs: Vec<&str> = patterns.iter().map(|s| s.as_str()).collect();

    match any(pattern_refs) {
        Ok(any) => any.is_match(relative),
        Err(_) => false,
    }
}

pub fn validate_exclude_patterns(patterns: &[String]) -> Result<(), String> {
    if patterns.is_empty() {
        return Ok(());
    }

    let pattern_refs: Vec<&str> = patterns.iter().map(|s| s.as_str()).collect();

    any(pattern_refs)
        .map(|_| ())
        .map_err(|error| format!("invalid exclude pattern: {}", error))
}

pub fn get_paths_to_process(
    root_path: &str,
    to_exclude_paths: Vec<String>,
) -> Result<Vec<String>, String> {
    let mut files_paths: Vec<String> = Vec::new();
    let glob = Glob::new("**/*.py").unwrap();

    let normalized_root = root_path.replace('\\', "/");
    let normalized_excludes: Vec<String> = to_exclude_paths
        .iter()
        .map(|s| s.replace('\\', "/"))
        .collect();

    let gitignore_walk = Walk::new(&normalized_root);
    let non_ignored: HashSet<String> = gitignore_walk
        .filter_map(|result| match result {
            Ok(entry) => entry.path().to_str().map(|s| s.replace('\\', "/")),
            Err(_) => None,
        })
        .collect();

    let exclude_refs: Vec<&str> = normalized_excludes.iter().map(|s| s.as_str()).collect();

    let non_excluded_entries = glob
        .walk(&normalized_root)
        .not(any(exclude_refs))
        .map_err(|e| format!("Failed to apply exclude patterns: {}", e))?;

    for entry in non_excluded_entries
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry
                .path()
                .to_str()
                .map(|s| non_ignored.contains(&s.replace('\\', "/")))
                .unwrap_or(false)
        })
    {
        let entry_path = entry.path();
        let file_abs = match entry_path.canonicalize() {
            Ok(p) => p,
            Err(_) => entry_path.to_path_buf(),
        };
        let file_abs_str = file_abs.to_string_lossy().replace('\\', "/");
        files_paths.push(file_abs_str);
    }

    Ok(files_paths)
}
