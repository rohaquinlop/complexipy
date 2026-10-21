use ignore::Walk;
use std::borrow::Cow;
use std::collections::HashSet;
use std::path::Path;
use wax::walk::{Entry, FileIterator};
use wax::{Any, Glob, Program, any};

#[derive(Clone, Debug, Default)]
pub struct ExcludeMatcher {
    programs: Vec<Any<'static>>,
    invalid: Vec<String>,
}

impl ExcludeMatcher {
    pub fn new(patterns: &[String]) -> Self {
        let mut matcher = ExcludeMatcher::default();
        for pattern in patterns {
            match compile_pattern(pattern) {
                Some(program) => matcher.programs.push(program),
                None => matcher.invalid.push(pattern.clone()),
            }
        }
        matcher
    }

    pub fn invalid(&self) -> &[String] {
        &self.invalid
    }

    pub fn overflows(&self) -> bool {
        !self.programs.is_empty()
            && self.invalid.is_empty()
            && any(self.programs.iter().cloned()).is_err()
    }

    pub fn is_excluded(&self, path: &str, root: &str) -> bool {
        if self.programs.is_empty() {
            return false;
        }

        let normalized_path = path.replace('\\', "/");
        let normalized_root = root.replace('\\', "/");
        let relative = relative_to(&normalized_path, normalized_root.trim_end_matches('/'))
            .unwrap_or(normalized_path.as_str());

        self.programs
            .iter()
            .any(|program| program.is_match(relative))
    }

    /// Like `is_excluded`, for a `path` inside `root` only.
    pub fn is_excluded_inside(&self, path: &str, root: &str) -> bool {
        if self.programs.is_empty() {
            return false;
        }

        let normalized_path = path.replace('\\', "/");
        let normalized_root = root.replace('\\', "/");
        relative_to(&normalized_path, normalized_root.trim_end_matches('/')).is_some_and(
            |relative| {
                self.programs
                    .iter()
                    .any(|program| program.is_match(relative))
            },
        )
    }
}

fn compile_pattern(pattern: &str) -> Option<Any<'static>> {
    let glob = Glob::new(normalized_pattern(pattern).as_ref())
        .ok()?
        .into_owned();
    any([glob]).ok()
}

pub fn is_path_excluded(path: &str, root: &str, patterns: &[String]) -> bool {
    ExcludeMatcher::new(patterns).is_excluded(path, root)
}

fn normalized_pattern(pattern: &str) -> Cow<'_, str> {
    if pattern.contains('\\') {
        Cow::Owned(pattern.replace('\\', "/"))
    } else {
        Cow::Borrowed(pattern)
    }
}

fn relative_to<'a>(path: &'a str, root: &str) -> Option<&'a str> {
    if root.is_empty() {
        return Some(path.trim_start_matches('/'));
    }

    if path == root {
        return Some("");
    }

    path.strip_prefix(root)?.strip_prefix('/')
}

pub fn invalid_exclude_patterns(patterns: &[String]) -> Vec<String> {
    ExcludeMatcher::new(patterns).invalid().to_vec()
}

pub fn exclude_list_overflows(patterns: &[String]) -> bool {
    ExcludeMatcher::new(patterns).overflows()
}

/// The patterns that match no path: paths are matched relative to a root,
/// so they never start with `/`, `./`, `../`, or a drive, and a file path
/// never ends with `/`.
pub fn unmatchable_exclude_patterns(patterns: &[String]) -> Vec<String> {
    patterns
        .iter()
        .filter(|pattern| {
            let pattern = normalized_pattern(pattern);
            let bytes = pattern.as_bytes();
            let drive = bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':';
            pattern.starts_with('/')
                || pattern.starts_with("./")
                || pattern.starts_with("../")
                || pattern.ends_with('/')
                || drive
        })
        .cloned()
        .collect()
}

/// Lists the Python files under `root_path` that no pattern excludes. A
/// pattern matches a path relative to `root_path`, and also relative to
/// `anchor`, the directory of the configuration, for files inside it.
pub fn get_paths_to_process(
    root_path: &str,
    to_exclude_paths: Vec<String>,
    anchor: &Path,
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
    let anchor_matcher = Path::new(root_path)
        .canonicalize()
        .is_ok_and(|root| root != anchor)
        .then(|| ExcludeMatcher::new(&normalized_excludes));
    let anchor = anchor.to_string_lossy().replace('\\', "/");

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
        if anchor_matcher
            .as_ref()
            .is_some_and(|matcher| matcher.is_excluded_inside(&file_abs_str, &anchor))
        {
            continue;
        }
        files_paths.push(file_abs_str);
    }

    Ok(files_paths)
}

#[cfg(test)]
mod tests;
