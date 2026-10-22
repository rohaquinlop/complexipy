use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use crate::classes::FileComplexity;

const GIT_TIMEOUT: Duration = Duration::from_secs(15);
const GIT_ROOT_TIMEOUT: Duration = Duration::from_secs(10);

pub use complexipy_types::DiffStatus;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffEntry {
    pub file_path: String,
    pub func_name: String,
    pub old_complexity: Option<u64>,
    pub new_complexity: Option<u64>,
}

impl DiffEntry {
    pub fn status(&self) -> DiffStatus {
        match (self.old_complexity, self.new_complexity) {
            (None, ..) => DiffStatus::New,
            (.., None) => DiffStatus::Removed,
            (Some(old), Some(new)) => {
                if new > old {
                    DiffStatus::Regressed
                } else if new < old {
                    DiffStatus::Improved
                } else {
                    DiffStatus::Unchanged
                }
            }
        }
    }

    pub fn delta(&self) -> Option<i64> {
        match (self.old_complexity, self.new_complexity) {
            (None, ..) => None,
            (.., None) => None,
            (Some(old), Some(new)) => Some(new as i64 - old as i64),
        }
    }
}

/// Why the diff could not read a file at the ref. Every function of such a
/// file reports as new.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnresolvedReason {
    /// No git repository holds the file.
    NotInRepository,
    /// The ref names no commit in the repository at `repository`.
    UnknownRef { repository: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnresolvedFile {
    pub file_path: String,
    pub reason: UnresolvedReason,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DiffReport {
    pub entries: Vec<DiffEntry>,
    pub unresolved: Vec<UnresolvedFile>,
}

pub fn compute_diff(
    current_files: &[FileComplexity],
    git_ref: &str,
    invocation_path: &str,
) -> Vec<DiffEntry> {
    compute_diff_report(current_files, git_ref, invocation_path).entries
}

/// Compares each file with its content at `git_ref`, read from the
/// repository that holds the file, so the working directory does not
/// change the result. A file the diff cannot read at the ref is listed in
/// `unresolved`, and its functions report as new.
pub fn compute_diff_report(
    current_files: &[FileComplexity],
    git_ref: &str,
    invocation_path: &str,
) -> DiffReport {
    let mut report = DiffReport::default();
    let mut repositories = Repositories::new(git_ref);

    for file in current_files {
        let current_map: HashMap<String, u64> = file
            .functions
            .iter()
            .map(|function| (function.name.clone(), function.complexity))
            .collect();

        let old_content = match repositories.locate(invocation_path, &file.path) {
            Ok((root, path_from_root)) => file_content_at_ref(git_ref, &path_from_root, &root),
            Err(reason) => {
                report.unresolved.push(UnresolvedFile {
                    file_path: file.path.clone(),
                    reason,
                });
                None
            }
        };
        let Some(old_content) = old_content else {
            let mut names: Vec<&String> = current_map.keys().collect();
            names.sort();
            for name in names {
                report.entries.push(DiffEntry {
                    file_path: file.path.clone(),
                    func_name: name.clone(),
                    old_complexity: None,
                    new_complexity: current_map.get(name).copied(),
                });
            }
            continue;
        };

        let Some(old_map) = analyse_content_to_map(Some(&old_content)) else {
            continue;
        };

        let mut all_names: Vec<String> =
            old_map.keys().chain(current_map.keys()).cloned().collect();
        all_names.sort();
        all_names.dedup();

        for name in all_names {
            report.entries.push(DiffEntry {
                file_path: file.path.clone(),
                func_name: name.clone(),
                old_complexity: old_map.get(&name).copied(),
                new_complexity: current_map.get(&name).copied(),
            });
        }
    }

    report
}

pub fn compute_staged_diff(git_ref: &str, invocation_path: &str) -> Option<Vec<DiffEntry>> {
    let root = git_root(Path::new(invocation_path))?;
    let root = Path::new(&root);

    let mut entries = Vec::new();
    for path_from_root in staged_python_files(git_ref, root) {
        let old_content = file_content_at_ref(git_ref, &path_from_root, root);
        let new_content = file_content_at_index(&path_from_root, root);

        let old_map = analyse_content_to_map(old_content.as_deref());
        let new_map = analyse_content_to_map(new_content.as_deref());

        if old_map.is_none() && new_map.is_none() {
            continue;
        }

        let old_map = old_map.unwrap_or_default();
        let new_map = new_map.unwrap_or_default();

        let mut names: Vec<String> = old_map.keys().chain(new_map.keys()).cloned().collect();
        names.sort();
        names.dedup();

        for name in names {
            entries.push(DiffEntry {
                file_path: path_from_root.clone(),
                func_name: name.clone(),
                old_complexity: old_map.get(&name).copied(),
                new_complexity: new_map.get(&name).copied(),
            });
        }
    }

    Some(entries)
}

pub fn has_regressions(entries: &[DiffEntry], max_complexity: u64) -> bool {
    entries.iter().any(|entry| match entry.status() {
        DiffStatus::Regressed => entry.new_complexity.is_some_and(|c| c > max_complexity),
        DiffStatus::New => entry.new_complexity.is_some_and(|c| c > max_complexity),
        _ => false,
    })
}

pub fn resolve_diff_flags(
    diff: Option<String>,
    diff_only: Option<String>,
    staged: bool,
) -> (Option<String>, Option<String>) {
    let mut diff = diff;
    if staged && diff.is_none() && diff_only.is_none() {
        diff = Some("HEAD".to_string());
    }
    if diff_only.is_some() && diff.is_some() {
        diff = None;
    }
    (diff, diff_only)
}

fn git_root(cwd: &Path) -> Option<String> {
    let (success, output) = run_git(cwd, &["rev-parse", "--show-toplevel"], GIT_ROOT_TIMEOUT)?;
    let root = output.trim();
    if success && !root.is_empty() {
        Some(root.to_string())
    } else {
        None
    }
}

fn names_commit(git_ref: &str, cwd: &Path) -> bool {
    let revision = format!("{}^{{commit}}", git_ref);
    run_git(
        cwd,
        &["rev-parse", "--verify", "--quiet", &revision],
        GIT_ROOT_TIMEOUT,
    )
    .is_some_and(|(success, _)| success)
}

/// The repository of each analyzed file, found from the file's own
/// directory and cached per directory, with whether the ref names a commit
/// there.
struct Repositories<'a> {
    git_ref: &'a str,
    roots: HashMap<PathBuf, Option<PathBuf>>,
    has_ref: HashMap<PathBuf, bool>,
}

impl<'a> Repositories<'a> {
    fn new(git_ref: &'a str) -> Self {
        Self {
            git_ref,
            roots: HashMap::new(),
            has_ref: HashMap::new(),
        }
    }

    /// The root of the repository that holds `file_path`, and the file's
    /// path from that root. A relative `file_path` is taken from
    /// `invocation_path`.
    fn locate(
        &mut self,
        invocation_path: &str,
        file_path: &str,
    ) -> Result<(PathBuf, String), UnresolvedReason> {
        let joined = Path::new(invocation_path).join(file_path);
        let (Some(directory), Some(name)) = (joined.parent(), joined.file_name()) else {
            return Err(UnresolvedReason::NotInRepository);
        };
        let Ok(directory) = directory.canonicalize() else {
            return Err(UnresolvedReason::NotInRepository);
        };
        let root = self
            .roots
            .entry(directory.clone())
            .or_insert_with(|| {
                git_root(&directory).and_then(|root| Path::new(&root).canonicalize().ok())
            })
            .clone();
        let Some(root) = root else {
            return Err(UnresolvedReason::NotInRepository);
        };
        let git_ref = self.git_ref;
        let has_ref = *self
            .has_ref
            .entry(root.clone())
            .or_insert_with(|| names_commit(git_ref, &root));
        if !has_ref {
            return Err(UnresolvedReason::UnknownRef {
                repository: root.display().to_string(),
            });
        }
        let path_from_root = directory
            .join(name)
            .strip_prefix(&root)
            .ok()
            .and_then(Path::to_str)
            .map(|path| path.replace('\\', "/"))
            .ok_or(UnresolvedReason::NotInRepository)?;
        Ok((root, path_from_root))
    }
}

fn file_content_at_ref(git_ref: &str, path_from_root: &str, cwd: &Path) -> Option<String> {
    let argument = format!("{}:{}", git_ref, path_from_root);
    let (success, output) = run_git(cwd, &["show", &argument], GIT_TIMEOUT)?;
    if success { Some(output) } else { None }
}

fn file_content_at_index(path_from_root: &str, cwd: &Path) -> Option<String> {
    let argument = format!(":{}", path_from_root);
    let (success, output) = run_git(cwd, &["show", &argument], GIT_TIMEOUT)?;
    if success { Some(output) } else { None }
}

fn staged_python_files(git_ref: &str, cwd: &Path) -> Vec<String> {
    let (success, output) = run_git(
        cwd,
        &[
            "diff",
            "--name-only",
            "--cached",
            "--no-renames",
            "--diff-filter=ACMRD",
            git_ref,
            "--",
            "*.py",
        ],
        GIT_TIMEOUT,
    )
    .unwrap_or((false, String::new()));
    if !success {
        return Vec::new();
    }
    output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(str::to_string)
        .collect()
}

fn analyse_content_to_map(content: Option<&str>) -> Option<HashMap<String, u64>> {
    let content = content?;
    let parsed = ruff_python_parser::parse_module(content).ok()?;
    let ast_body = parsed.into_suite();
    let (functions, _) = crate::cognitive_complexity::function_level_cognitive_complexity_shared(
        &ast_body,
        content,
        &crate::rules::AnalysisOptions::default(),
    );
    Some(
        functions
            .into_iter()
            .map(|function| (function.name, function.complexity))
            .collect(),
    )
}

fn run_git(cwd: &Path, args: &[&str], timeout: Duration) -> Option<(bool, String)> {
    let mut child = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;

    let stdout = child.stdout.take()?;
    let stdout_handle = thread::spawn(move || {
        let mut buffer = Vec::new();
        let mut reader = stdout;
        let _ = reader.read_to_end(&mut buffer);
        buffer
    });

    let deadline = Instant::now() + timeout;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => {
                if Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    return None;
                }
                thread::sleep(Duration::from_millis(10));
            }
            Err(_) => return None,
        }
    };

    let output = stdout_handle.join().ok()?;
    let output = String::from_utf8(output).ok()?;
    Some((status.success(), output))
}

#[cfg(test)]
mod tests;
