use crate::runner::{SkipReason, SkippedPath};
use ignore::gitignore::{Gitignore, GitignoreBuilder};
use ignore::{DirEntry, Match, WalkBuilder, WalkState};
use std::borrow::Cow;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use wax::query::When;
use wax::{Any, BuildError, CandidatePath, Glob, Program, any};

#[derive(Clone, Debug, Default)]
pub struct ExcludeMatcher {
    programs: Vec<Any<'static>>,
    combined: Option<Any<'static>>,
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
        if !matcher.programs.is_empty() {
            matcher.combined = any(matcher.programs.iter().cloned()).ok();
        }
        matcher
    }

    pub fn invalid(&self) -> &[String] {
        &self.invalid
    }

    pub fn overflows(&self) -> bool {
        !self.programs.is_empty() && self.invalid.is_empty() && self.combined.is_none()
    }

    pub fn is_excluded(&self, path: &str, root: &str) -> bool {
        if self.programs.is_empty() {
            return false;
        }

        let normalized_path = path.replace('\\', "/");
        let normalized_root = root.replace('\\', "/");
        let relative = relative_to(&normalized_path, normalized_root.trim_end_matches('/'))
            .unwrap_or(normalized_path.as_str());

        self.matches(relative)
    }

    /// Like `is_excluded`, for a `path` inside `root` only.
    pub fn is_excluded_inside(&self, path: &str, root: &str) -> bool {
        if self.programs.is_empty() {
            return false;
        }

        let normalized_path = path.replace('\\', "/");
        let normalized_root = root.replace('\\', "/");
        relative_to(&normalized_path, normalized_root.trim_end_matches('/'))
            .is_some_and(|relative| self.matches(relative))
    }

    fn matches(&self, relative: &str) -> bool {
        match &self.combined {
            Some(combined) => combined.is_match(relative),
            None => self
                .programs
                .iter()
                .any(|program| program.is_match(relative)),
        }
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

struct WalkExclusion<'t> {
    any: Any<'t>,
    exhaustive: Option<Any<'t>>,
}

impl<'t> WalkExclusion<'t> {
    fn new(patterns: &'t [String]) -> Result<Option<Self>, BuildError> {
        if patterns.is_empty() {
            return Ok(None);
        }
        let any_pattern = any(patterns.iter().map(String::as_str))?;
        let exhaustive = match any_pattern.is_exhaustive() {
            When::Always => Some(any_pattern.clone()),
            When::Never => None,
            When::Sometimes => exhaustive_subset(patterns)?,
        };
        Ok(Some(Self {
            any: any_pattern,
            exhaustive,
        }))
    }
}

fn exhaustive_subset(patterns: &[String]) -> Result<Option<Any<'_>>, BuildError> {
    let mut exhaustive = Vec::new();
    for pattern in patterns {
        let glob = Glob::new(pattern)?;
        if glob.is_exhaustive().is_always() {
            exhaustive.push(glob);
        }
    }
    if exhaustive.is_empty() {
        return Ok(None);
    }
    any(exhaustive).map(Some)
}

enum Visit {
    Prune,
    Skip,
    Select,
}

fn visit(entry: &DirEntry, python_files: &Glob<'_>, exclusion: Option<&WalkExclusion>) -> Visit {
    let path = entry.path();
    let depth = entry.depth();
    let ancestor = path.ancestors().nth(depth).unwrap_or(Path::new(""));
    let relative = path.strip_prefix(ancestor).unwrap_or(path);

    if let Some(exclusion) = exclusion {
        if exclusion
            .exhaustive
            .as_ref()
            .is_some_and(|program| program.is_match(CandidatePath::from(relative)))
        {
            return Visit::Prune;
        }
        if exclusion.any.is_match(CandidatePath::from(relative)) {
            return Visit::Skip;
        }
    }

    if depth > 0 && python_files.is_match(CandidatePath::from(relative)) {
        Visit::Select
    } else {
        Visit::Skip
    }
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

#[derive(Debug, Default)]
pub struct Discovery {
    pub files: Vec<String>,
    pub skipped: Vec<SkippedPath>,
}

#[derive(Default)]
struct Walked {
    files: Vec<String>,
    skipped: Vec<SkippedPath>,
    visited: HashSet<PathBuf>,
}

/// Selects the `*.py` files under `root_path` that no pattern excludes, in
/// one parallel walk that honors `.gitignore`, `.ignore`, and hidden paths,
/// and prunes every directory an exhaustive pattern covers. A pattern
/// matches a path relative to `root_path`, and also relative to `anchor`,
/// the directory of the configuration, for files inside it. A selected file
/// is a regular file inside the root; a symlink to a file elsewhere, a
/// dangling link, a directory named `*.py`, and a directory the walk cannot
/// read are skipped and reported, never read. With `list_hidden`, a second
/// walk without the ignore rules reports every file they hide. A file whose
/// name is not UTF-8 is selected under a lossy name, so its read fails and
/// the run reports it.
pub fn get_paths_to_process(
    root_path: &str,
    to_exclude_paths: Vec<String>,
    anchor: &Path,
    list_hidden: bool,
) -> Result<Discovery, String> {
    let python_files = Glob::new("**/*.py").unwrap();

    let normalized_root = root_path.replace('\\', "/");
    let canonical_root = Path::new(&normalized_root)
        .canonicalize()
        .map_err(|e| format!("Failed to resolve {}: {}", root_path, e))?;
    let normalized_excludes: Vec<String> = to_exclude_paths
        .iter()
        .map(|s| s.replace('\\', "/"))
        .collect();

    let anchor_matcher =
        (canonical_root != anchor).then(|| ExcludeMatcher::new(&normalized_excludes));
    let anchor = slashed(anchor);
    let excluded_from_anchor = |file_abs: &str| {
        anchor_matcher
            .as_ref()
            .is_some_and(|matcher| matcher.is_excluded_inside(file_abs, &anchor))
    };

    let exclusion = WalkExclusion::new(&normalized_excludes)
        .map_err(|e| format!("Failed to apply exclude patterns: {}", e))?;

    let walked: Mutex<Walked> = Mutex::new(Walked::default());
    WalkBuilder::new(&normalized_root).build_parallel().run(|| {
        Box::new(|result| {
            let entry = match result {
                Ok(entry) => entry,
                Err(error) => {
                    if let Some(path) = unreadable_path(&error) {
                        lock(&walked)
                            .skipped
                            .push(skipped(path, SkipReason::Unreadable));
                    }
                    return WalkState::Continue;
                }
            };
            match visit(&entry, &python_files, exclusion.as_ref()) {
                Visit::Prune => WalkState::Skip,
                Visit::Skip => WalkState::Continue,
                Visit::Select => {
                    let entry_path = entry.path();
                    let selected = selected_file(entry_path, &canonical_root);
                    let mut walked = lock(&walked);
                    if list_hidden {
                        walked.visited.insert(entry_path.to_path_buf());
                    }
                    match selected.map(|file_abs| slashed(&file_abs)) {
                        Ok(file_abs) if excluded_from_anchor(&file_abs) => {}
                        Ok(file_abs) => walked.files.push(file_abs),
                        Err(reason) => walked.skipped.push(skipped(entry_path, reason)),
                    }
                    WalkState::Continue
                }
            }
        })
    });

    let walked = walked
        .into_inner()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let mut discovery = Discovery {
        files: walked.files,
        skipped: walked.skipped,
    };
    if list_hidden {
        let hidden: Mutex<Vec<SkippedPath>> = Mutex::new(Vec::new());
        WalkBuilder::new(&normalized_root)
            .standard_filters(false)
            .build_parallel()
            .run(|| {
                Box::new(|result| {
                    let Ok(entry) = result else {
                        return WalkState::Continue;
                    };
                    match visit(&entry, &python_files, exclusion.as_ref()) {
                        Visit::Prune => WalkState::Skip,
                        Visit::Skip => WalkState::Continue,
                        Visit::Select => {
                            let path = entry.path();
                            if !walked.visited.contains(path)
                                && !path
                                    .canonicalize()
                                    .is_ok_and(|file_abs| excluded_from_anchor(&slashed(&file_abs)))
                            {
                                lock(&hidden).push(skipped(path, SkipReason::IgnoreRule));
                            }
                            WalkState::Continue
                        }
                    }
                })
            });
        discovery.skipped.extend(
            hidden
                .into_inner()
                .unwrap_or_else(|poisoned| poisoned.into_inner()),
        );
    }

    discovery.files.sort();
    discovery.skipped.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(discovery)
}

/// The global gitignore, matched the way discovery's walk matches it.
pub(crate) fn global_gitignore() -> Gitignore {
    GitignoreBuilder::new("").build_global().0
}

/// Whether the ignore rules discovery honors would hide `file` from a walk
/// of `root`: `.ignore` files, and, inside a git repository, `.gitignore`
/// files, git's exclude file, and `global`. Each directory from `root` down
/// is checked as the walk would meet it, then the file, so `file` need not
/// exist.
pub(crate) fn ignore_rules_hide(root: &Path, file: &Path, global: &Gitignore) -> bool {
    let Ok(relative) = file.strip_prefix(root) else {
        return false;
    };
    let mut path = root.to_path_buf();
    let mut components = relative.components().peekable();
    while let Some(component) = components.next() {
        path.push(component);
        if ignore_match(&path, components.peek().is_some(), global).is_ignore() {
            return true;
        }
    }
    false
}

/// The rule that decides `path`, with the precedence the `ignore` crate
/// gives it: the deepest `.ignore` match, then the deepest `.gitignore`
/// match inside the repository, then the exclude file, then `global`.
fn ignore_match(path: &Path, is_dir: bool, global: &Gitignore) -> Match<()> {
    let has_git = |dir: &Path| dir.join(".git").exists() || dir.join(".jj").exists();
    let directories: Vec<&Path> = path.ancestors().skip(1).collect();
    let any_git = directories.iter().any(|dir| has_git(dir));
    let matched = |gitignore: Gitignore| gitignore.matched(path, is_dir).map(|_| ());
    let (mut ignore, mut gitignore, mut exclude) = (Match::None, Match::None, Match::None);
    let mut saw_git = false;
    for directory in directories {
        if ignore.is_none() {
            ignore = matched(Gitignore::new(directory.join(".ignore")).0);
        }
        if any_git && !saw_git {
            if gitignore.is_none() {
                gitignore = matched(Gitignore::new(directory.join(".gitignore")).0);
            }
            if exclude.is_none()
                && let Some(exclude_file) = git_exclude_file(directory)
            {
                let mut builder = GitignoreBuilder::new(directory);
                let _ = builder.add(exclude_file);
                exclude = matched(builder.build().unwrap_or_else(|_| Gitignore::empty()));
            }
        }
        saw_git = saw_git || has_git(directory);
    }
    let global_match = if any_git {
        global.matched(path, is_dir).map(|_| ())
    } else {
        Match::None
    };
    ignore.or(gitignore).or(exclude).or(global_match)
}

/// The `info/exclude` file of the repository whose `.git` sits in
/// `directory`, following a worktree's `.git` file to its common directory.
fn git_exclude_file(directory: &Path) -> Option<PathBuf> {
    let dot_git = directory.join(".git");
    if dot_git.is_dir() {
        return Some(dot_git.join("info/exclude"));
    }
    let first_line = |path: &Path| -> Option<String> {
        let text =
            crate::fs_guard::read_bounded_text(path, None, crate::fs_guard::MAX_CONFIG_BYTES)
                .ok()?;
        Some(text.lines().next()?.to_string())
    };
    let git_dir = directory.join(first_line(&dot_git)?.strip_prefix("gitdir: ")?);
    let common = git_dir.join(first_line(&git_dir.join("commondir"))?);
    Some(common.join("info/exclude"))
}

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn slashed(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn unreadable_path(error: &ignore::Error) -> Option<&Path> {
    match error {
        ignore::Error::WithPath { path, .. } => Some(path),
        ignore::Error::WithDepth { err, .. } | ignore::Error::WithLineNumber { err, .. } => {
            unreadable_path(err)
        }
        _ => None,
    }
}

fn skipped(path: &Path, reason: SkipReason) -> SkippedPath {
    SkippedPath {
        path: slashed(path),
        reason,
    }
}

fn selected_file(entry_path: &Path, canonical_root: &Path) -> Result<PathBuf, SkipReason> {
    let file_abs = entry_path
        .canonicalize()
        .map_err(|_| SkipReason::NotRegularFile)?;
    if !file_abs.starts_with(canonical_root) {
        return Err(SkipReason::OutsideRoot);
    }
    if !file_abs.is_file() {
        return Err(SkipReason::NotRegularFile);
    }
    Ok(file_abs)
}

#[cfg(test)]
mod tests;
