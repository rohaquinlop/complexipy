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
    /// The repository at `repository` sits inside another repository, which
    /// pins no commit for it at the ref.
    NotPinned { repository: String },
    /// The repository at `repository` does not have `commit`, which the
    /// repository around it pins for it at the ref.
    PinnedCommitMissing { repository: String, commit: String },
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
/// repository that holds the file. A repository inside another repository
/// is read at the commit that the outer repository pins for it at its own
/// compared commit, unless it holds `invocation_path`. A file the diff
/// cannot read is listed in `unresolved`, and its functions report as new.
pub fn compute_diff_report(
    current_files: &[FileComplexity],
    git_ref: &str,
    invocation_path: &str,
) -> DiffReport {
    let mut report = DiffReport::default();
    let mut repositories = Repositories::new(git_ref, invocation_path);

    for file in current_files {
        let current_map: HashMap<String, u64> = file
            .functions
            .iter()
            .map(|function| (function.name.clone(), function.complexity))
            .collect();

        let old_content = match repositories.locate(&file.path) {
            Ok((root, commit, path_from_root)) => {
                file_content_at_ref(&commit, &path_from_root, &root)
            }
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

/// The id of the commit that `revision` names in the repository at `cwd`.
fn commit_id(revision: &str, cwd: &Path) -> Option<String> {
    let revision = format!("{}^{{commit}}", revision);
    let (success, output) = run_git(
        cwd,
        &["rev-parse", "--verify", "--quiet", &revision],
        GIT_ROOT_TIMEOUT,
    )?;
    let commit = output.trim();
    (success && !commit.is_empty()).then(|| commit.to_string())
}

/// The commit that the gitlink at `path` in `commit` pins, read from the
/// repository at `cwd`.
fn pinned_commit(commit: &str, path: &str, cwd: &Path) -> Option<String> {
    let (success, output) = run_git(
        cwd,
        &["ls-tree", "-z", commit, "--", path],
        GIT_ROOT_TIMEOUT,
    )?;
    if !success {
        return None;
    }
    output.split('\0').find_map(|entry| {
        let (fields, entry_path) = entry.split_once('\t')?;
        let mut fields = fields.split(' ');
        let (Some("160000"), Some("commit"), Some(pinned)) =
            (fields.next(), fields.next(), fields.next())
        else {
            return None;
        };
        (entry_path == path).then(|| pinned.to_string())
    })
}

/// `path` from `root`, with forward slashes.
fn path_from(root: &Path, path: &Path) -> Option<String> {
    Some(path.strip_prefix(root).ok()?.to_str()?.replace('\\', "/"))
}

/// The repository of each analyzed file, found from the file's own
/// directory and cached per directory, and the commit each repository is
/// compared at, cached per repository.
struct Repositories<'a> {
    git_ref: &'a str,
    invocation_path: &'a str,
    invocation_directory: Option<PathBuf>,
    roots: HashMap<PathBuf, Option<PathBuf>>,
    commits: HashMap<PathBuf, Result<String, UnresolvedReason>>,
}

impl<'a> Repositories<'a> {
    fn new(git_ref: &'a str, invocation_path: &'a str) -> Self {
        Self {
            git_ref,
            invocation_path,
            invocation_directory: Path::new(invocation_path).canonicalize().ok(),
            roots: HashMap::new(),
            commits: HashMap::new(),
        }
    }

    /// The root of the repository that holds `file_path`, the commit to
    /// read the file at, and the file's path from that root. A relative
    /// `file_path` is taken from the invocation path.
    fn locate(&mut self, file_path: &str) -> Result<(PathBuf, String, String), UnresolvedReason> {
        let joined = Path::new(self.invocation_path).join(file_path);
        let (Some(directory), Some(name)) = (joined.parent(), joined.file_name()) else {
            return Err(UnresolvedReason::NotInRepository);
        };
        let Ok(directory) = directory.canonicalize() else {
            return Err(UnresolvedReason::NotInRepository);
        };
        let root = self
            .root_of(&directory)
            .ok_or(UnresolvedReason::NotInRepository)?;
        let commit = self.commit_of(&root)?;
        let path_from_root =
            path_from(&root, &directory.join(name)).ok_or(UnresolvedReason::NotInRepository)?;
        Ok((root, commit, path_from_root))
    }

    fn root_of(&mut self, directory: &Path) -> Option<PathBuf> {
        self.roots
            .entry(directory.to_path_buf())
            .or_insert_with(|| {
                git_root(directory).and_then(|root| Path::new(&root).canonicalize().ok())
            })
            .clone()
    }

    fn commit_of(&mut self, root: &Path) -> Result<String, UnresolvedReason> {
        if let Some(commit) = self.commits.get(root) {
            return commit.clone();
        }
        let commit = self.resolve_commit(root);
        self.commits.insert(root.to_path_buf(), commit.clone());
        commit
    }

    /// A repository that holds the invocation directory, or that sits
    /// inside no other repository, is compared at the ref. Any other
    /// repository is compared at the commit that the repository around it
    /// pins for it at that repository's compared commit.
    fn resolve_commit(&mut self, root: &Path) -> Result<String, UnresolvedReason> {
        let repository = root.display().to_string();
        let Some(outer) = self.outer_root(root) else {
            return commit_id(self.git_ref, root)
                .ok_or(UnresolvedReason::UnknownRef { repository });
        };
        let pinned = self
            .commit_of(&outer)
            .ok()
            .zip(path_from(&outer, root))
            .and_then(|(outer_commit, path)| pinned_commit(&outer_commit, &path, &outer));
        let Some(pinned) = pinned else {
            return Err(UnresolvedReason::NotPinned { repository });
        };
        commit_id(&pinned, root).ok_or(UnresolvedReason::PinnedCommitMissing {
            repository,
            commit: pinned,
        })
    }

    /// The root of the repository around `root`, or `None` when no
    /// repository is around it or `root` holds the invocation directory.
    fn outer_root(&mut self, root: &Path) -> Option<PathBuf> {
        if self
            .invocation_directory
            .as_deref()
            .is_some_and(|directory| directory.starts_with(root))
        {
            return None;
        }
        let parent = root.parent()?;
        let outer = self.root_of(parent)?;
        parent.starts_with(&outer).then_some(outer)
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
