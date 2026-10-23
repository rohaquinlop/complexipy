use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use crate::classes::{FileComplexity, FunctionComplexity};

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

pub fn compute_diff(
    current_files: &[FileComplexity],
    git_ref: &str,
    invocation_path: &str,
) -> Vec<DiffEntry> {
    let regular_files = regular_files_at_ref(git_ref, invocation_path);
    let mut entries = Vec::new();

    for file in current_files {
        let path_from_root = resolve_git_path(&file.path, git_ref, invocation_path);
        let current_map = keyed_complexities(&file.functions);

        let Some(old_content) =
            regular_file_content_at_ref(git_ref, &path_from_root, invocation_path, &regular_files)
        else {
            let mut names: Vec<&String> = current_map.keys().collect();
            names.sort();
            for name in names {
                entries.push(DiffEntry {
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
            entries.push(DiffEntry {
                file_path: file.path.clone(),
                func_name: name.clone(),
                old_complexity: old_map.get(&name).copied(),
                new_complexity: current_map.get(&name).copied(),
            });
        }
    }

    entries
}

pub fn compute_staged_diff(git_ref: &str, invocation_path: &str) -> Option<Vec<DiffEntry>> {
    let root = git_root(invocation_path)?;
    let regular_files = regular_files_at_ref(git_ref, &root);

    let mut entries = Vec::new();
    for path_from_root in staged_python_files(git_ref, &root) {
        let old_content =
            regular_file_content_at_ref(git_ref, &path_from_root, &root, &regular_files);
        let new_content = file_content_at_index(&path_from_root, &root);

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

fn git_root(cwd: &str) -> Option<String> {
    let (success, output) = run_git(cwd, &["rev-parse", "--show-toplevel"], GIT_ROOT_TIMEOUT)?;
    let root = output.trim();
    if success && !root.is_empty() {
        Some(root.to_string())
    } else {
        None
    }
}

fn file_content_at_ref(git_ref: &str, path_from_root: &str, cwd: &str) -> Option<String> {
    let argument = format!("{}:{}", git_ref, path_from_root);
    let (success, output) = run_git(cwd, &["show", "--end-of-options", &argument], GIT_TIMEOUT)?;
    if success { Some(output) } else { None }
}

fn regular_file_content_at_ref(
    git_ref: &str,
    path_from_root: &str,
    cwd: &str,
    regular_files: &HashSet<String>,
) -> Option<String> {
    if !regular_files.contains(path_from_root) {
        return None;
    }
    file_content_at_ref(git_ref, path_from_root, cwd)
}

/// The paths from the repository root that are regular files at `git_ref`,
/// mode 100644 or 100755, leaving out symbolic links and submodules.
fn regular_files_at_ref(git_ref: &str, cwd: &str) -> HashSet<String> {
    let args = [
        "ls-tree",
        "-r",
        "-z",
        "--full-tree",
        "--end-of-options",
        git_ref,
    ];
    let Some((true, output)) = run_git_bytes(cwd, &args, GIT_TIMEOUT) else {
        return HashSet::new();
    };
    output
        .split(|byte| *byte == 0)
        .filter_map(|entry| std::str::from_utf8(entry).ok()?.split_once('\t'))
        .filter(|(meta, _)| matches!(meta.split(' ').next(), Some("100644" | "100755")))
        .map(|(_, path)| path.to_string())
        .collect()
}

fn file_content_at_index(path_from_root: &str, cwd: &str) -> Option<String> {
    let argument = format!(":{}", path_from_root);
    let (success, output) = run_git(cwd, &["show", "--end-of-options", &argument], GIT_TIMEOUT)?;
    if success { Some(output) } else { None }
}

fn staged_python_files(git_ref: &str, cwd: &str) -> Vec<String> {
    let (success, output) = run_git(
        cwd,
        &[
            "diff",
            "--name-only",
            "--cached",
            "--no-renames",
            "--diff-filter=ACMRDT",
            "--end-of-options",
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

fn git_tracked_paths(cwd: &str) -> Vec<String> {
    let (success, output) =
        run_git(cwd, &["ls-files", "--full-name"], GIT_TIMEOUT).unwrap_or((false, String::new()));
    if success {
        output.lines().map(str::to_string).collect()
    } else {
        Vec::new()
    }
}

fn resolve_git_path(file_path: &str, git_ref: &str, invocation_path: &str) -> String {
    let normalized = file_path.replace('\\', "/");
    let parts: Vec<&str> = normalized.split('/').collect();

    for i in 0..parts.len() {
        let candidate = parts[i..].join("/");
        if file_content_at_ref(git_ref, &candidate, invocation_path).is_some() {
            return candidate;
        }
    }

    let basename = parts.last().copied().unwrap_or("");
    let tracked_paths = git_tracked_paths(invocation_path);
    let matches: Vec<&String> = tracked_paths
        .iter()
        .filter(|tracked| {
            tracked.as_str() == basename || tracked.ends_with(&format!("/{}", basename))
        })
        .collect();
    if matches.len() == 1 {
        matches[0].clone()
    } else {
        normalized
    }
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
    Some(keyed_complexities(&functions))
}

/// One key per function, in input order: the qualified name, then `#N` for
/// the N-th function of that name in source order. A property's getter and
/// setter, or a redefinition, never share a key, in the diff or in the
/// snapshot.
pub fn function_keys(functions: &[FunctionComplexity]) -> Vec<String> {
    let mut order: Vec<usize> = (0..functions.len()).collect();
    order.sort_by_key(|&index| functions[index].line_start);

    let mut keys = vec![String::new(); functions.len()];
    let mut seen: HashMap<&str, usize> = HashMap::new();
    for index in order {
        let name = functions[index].name.as_str();
        let ordinal = seen.entry(name).or_insert(0);
        *ordinal += 1;
        keys[index] = if *ordinal == 1 {
            name.to_string()
        } else {
            format!("{}#{}", name, ordinal)
        };
    }
    keys
}

fn keyed_complexities(functions: &[FunctionComplexity]) -> HashMap<String, u64> {
    function_keys(functions)
        .into_iter()
        .zip(functions.iter().map(|function| function.complexity))
        .collect()
}

fn run_git(cwd: &str, args: &[&str], timeout: Duration) -> Option<(bool, String)> {
    let (success, output) = run_git_bytes(cwd, args, timeout)?;
    Some((success, String::from_utf8(output).ok()?))
}

fn run_git_bytes(cwd: &str, args: &[&str], timeout: Duration) -> Option<(bool, Vec<u8>)> {
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
    Some((status.success(), output))
}

#[cfg(test)]
mod tests;
