use std::collections::{HashMap, HashSet};
use std::io::{self, BufRead, BufReader, Read, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use crate::classes::{FileComplexity, FunctionComplexity};

const GIT_TIMEOUT: Duration = Duration::from_secs(15);
const GIT_ROOT_TIMEOUT: Duration = Duration::from_secs(10);
const MAX_BLOB_BYTES: u64 = 32 * 1024 * 1024;

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
    let mut entries = Vec::new();

    let repository = Repository::locate(invocation_path);
    let regular_files = repository
        .as_ref()
        .map(|_| regular_files_at_ref(git_ref, invocation_path))
        .unwrap_or_default();
    let names: Vec<Option<String>> = current_files
        .iter()
        .map(|file| {
            let path_from_root = repository.as_ref()?.path_of(&file.path)?;
            regular_files
                .contains(&path_from_root)
                .then(|| format!("{}:{}", git_ref, path_from_root))
        })
        .collect();
    let old_functions = analyse_contents(read_objects(invocation_path, &names));

    for (file, old_functions) in current_files.iter().zip(old_functions) {
        let current_map = keyed_complexities(&file.functions);

        let Some(old_functions) = old_functions else {
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

        let Some(old_map) = old_functions else {
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
    let paths = staged_python_files(git_ref, &root);
    let requests: Vec<Option<String>> = paths
        .iter()
        .flat_map(|path_from_root| {
            [
                regular_files
                    .contains(path_from_root)
                    .then(|| format!("{}:{}", git_ref, path_from_root)),
                Some(format!(":0:{}", path_from_root)),
            ]
        })
        .collect();
    let mut contents = analyse_contents(read_objects(&root, &requests)).into_iter();
    for path_from_root in paths {
        let old_map = contents.next().flatten().flatten();
        let new_map = contents.next().flatten().flatten();

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

/// Where the analyzed files sit inside their repository, so that each file
/// is looked up at the ref under its own repository path.
struct Repository {
    root: PathBuf,
    invocation: PathBuf,
}

impl Repository {
    fn locate(invocation_path: &str) -> Option<Self> {
        let root = PathBuf::from(git_root(invocation_path)?);
        let invocation = PathBuf::from(invocation_path);
        Some(Self {
            root: root.canonicalize().unwrap_or(root),
            invocation: invocation.canonicalize().unwrap_or(invocation),
        })
    }

    fn path_of(&self, file_path: &str) -> Option<String> {
        let absolute = self.invocation.join(file_path);
        let absolute = absolute.canonicalize().unwrap_or(absolute);
        let relative = absolute.strip_prefix(&self.root).ok()?.to_str()?;
        Some(relative.replace('\\', "/"))
    }
}

#[derive(Debug, PartialEq, Eq)]
enum GitObject {
    Missing,
    Unreadable,
    Text(String),
}

/// Reads every object that `names` lists through one `git cat-file --batch`
/// process, in order. A `None` name, or one with a control character, which
/// git would split or trim as a line ending, is not requested and reads as
/// missing.
/// A blob reads as its text; a blob over `MAX_BLOB_BYTES` or not valid
/// UTF-8, or an object that does not exist, reads as missing; any other
/// object, such as a tree, reads as unreadable.
fn read_objects(cwd: &str, names: &[Option<String>]) -> Vec<GitObject> {
    let mut objects: Vec<GitObject> = names.iter().map(|_| GitObject::Missing).collect();
    let requested: Vec<(usize, &str)> = names
        .iter()
        .enumerate()
        .filter_map(|(index, name)| Some((index, name.as_deref()?)))
        .filter(|(_, name)| !name.chars().any(char::is_control))
        .collect();
    if requested.is_empty() {
        return objects;
    }

    let Ok(mut child) = Command::new("git")
        .args(["cat-file", "--batch"])
        .current_dir(cwd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
    else {
        return objects;
    };
    let (Some(mut stdin), Some(stdout)) = (child.stdin.take(), child.stdout.take()) else {
        let _ = child.kill();
        let _ = child.wait();
        return objects;
    };

    let input: String = requested
        .iter()
        .map(|(_, name)| format!("{}\n", name))
        .collect();
    let writer = thread::spawn(move || {
        let _ = stdin.write_all(input.as_bytes());
    });
    let count = requested.len();
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let mut reader = BufReader::new(stdout);
        for _ in 0..count {
            let Some(object) = read_object(&mut reader) else {
                break;
            };
            if sender.send(object).is_err() {
                break;
            }
        }
    });

    for (index, _) in requested {
        match receiver.recv_timeout(GIT_TIMEOUT) {
            Ok(object) => objects[index] = object,
            Err(_) => {
                let _ = child.kill();
                break;
            }
        }
    }
    let _ = child.wait();
    let _ = writer.join();
    objects
}

/// Reads one `cat-file --batch` response. Returns `None` when the output
/// ends or cannot be read.
fn read_object(reader: &mut impl BufRead) -> Option<GitObject> {
    let mut header = String::new();
    if reader.read_line(&mut header).ok()? == 0 {
        return None;
    }
    let mut fields = header.trim_end_matches('\n').rsplitn(3, ' ');
    let Some(size) = fields.next().and_then(|size| size.parse::<u64>().ok()) else {
        return Some(GitObject::Missing);
    };
    let kind = fields.next()?;

    let object = if kind == "blob" && size <= MAX_BLOB_BYTES {
        let mut content = Vec::with_capacity(usize::try_from(size).ok()?);
        Read::take(&mut *reader, size)
            .read_to_end(&mut content)
            .ok()?;
        match String::from_utf8(content) {
            Ok(text) => GitObject::Text(text),
            Err(_) => GitObject::Missing,
        }
    } else {
        io::copy(&mut Read::take(&mut *reader, size), &mut io::sink()).ok()?;
        if kind == "blob" {
            GitObject::Missing
        } else {
            GitObject::Unreadable
        }
    };
    let mut terminator = [0u8; 1];
    reader.read_exact(&mut terminator).ok()?;
    Some(object)
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

/// Scores each object: `None` when it is missing, `Some(None)` when it does
/// not parse, and the complexity of each function otherwise.
#[cfg(feature = "runner")]
fn analyse_contents(objects: Vec<GitObject>) -> Vec<Option<Option<HashMap<String, u64>>>> {
    use rayon::prelude::*;

    objects.into_par_iter().map(analyse_object).collect()
}

#[cfg(not(feature = "runner"))]
fn analyse_contents(objects: Vec<GitObject>) -> Vec<Option<Option<HashMap<String, u64>>>> {
    objects.into_iter().map(analyse_object).collect()
}

fn analyse_object(object: GitObject) -> Option<Option<HashMap<String, u64>>> {
    match object {
        GitObject::Missing => None,
        GitObject::Unreadable => Some(None),
        GitObject::Text(content) => Some(analyse_content_to_map(&content)),
    }
}

fn analyse_content_to_map(content: &str) -> Option<HashMap<String, u64>> {
    let parsed = crate::syntax_tree::parse_module(content).ok()?;
    let (functions, _) = crate::cognitive_complexity::function_level_cognitive_complexity_shared(
        parsed.suite(),
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
