use std::borrow::Cow;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::io::{self, BufRead, BufReader, Read, Write};
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use ruff_python_ast::comparable::ComparableStmt;

use crate::classes::{FileComplexity, FunctionComplexity};
use crate::cognitive_complexity::{DefinedFunction, defined_functions};
use crate::syntax_tree::{ParsedModule, parse_module};

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

/// Why the diff could not read a file at the ref. Every function of such a
/// file reports as new.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnresolvedReason {
    /// No git repository holds the file.
    NotInRepository,
    /// The ref names no commit in the repository at `repository`.
    UnknownRef { repository: String },
    /// The repository around the one at `repository` tracks it as a
    /// gitlink but pins no commit for it at the ref.
    NotPinned { repository: String },
    /// The repository at `repository` does not have `commit`, which the
    /// repository that tracks it as a gitlink pins for it at the ref.
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
    pub bodies: Bodies,
}

/// What the gate knows of function bodies, compared structurally, with
/// formatting and comments ignored: the body of each REMOVED entry and of
/// each NEW entry above the threshold, by entry index, and how many
/// functions with each such body the compared files hold at the ref and
/// now, ignore markers disregarded.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Bodies {
    of_entry: HashMap<usize, usize>,
    counts: Vec<[usize; 2]>,
}

/// The analyzed files of one repository, by index and path from its root,
/// and the commit the repository is compared at.
struct RepositoryFiles {
    root: PathBuf,
    commit: String,
    files: Vec<(usize, String)>,
}

pub fn compute_diff(
    current_files: &[FileComplexity],
    git_ref: &str,
    invocation_path: &str,
) -> Vec<DiffEntry> {
    compute_diff_report(current_files, git_ref, invocation_path, &|_| false, None).entries
}

/// Compares each file with its content at `git_ref`, read from the
/// repository that holds the file through one `git cat-file --batch`
/// process per repository, so the working directory does not change the
/// result. A repository that the repository around it tracks as a gitlink
/// is read at the commit that gitlink pins at the outer repository's
/// compared commit, unless it holds `invocation_path`. A file git pairs as renamed since the ref compares with its old
/// path when the run covers that path, and as added otherwise. A Python file
/// deleted since the ref, in a repository that holds a current file,
/// reports its functions as removed when `covers_removed` accepts its
/// absolute path; a file renamed out of the selection reports nothing. The
/// functions of a file that does not parse at the ref report as new. A file
/// outside any repository, or in one without a compared commit, is listed
/// in `unresolved`, and its functions report as new. A path that is not a
/// regular file at the ref, such as a symbolic link, is not read there: the
/// file compares as added, and its deletion reports nothing. When git
/// cannot list a repository's changes, none of its files is read at the
/// ref.
/// When some function is new above `gate`, the gate's threshold, the report
/// counts the bodies of the files that changed since the ref, apart for
/// each repository.
pub fn compute_diff_report(
    current_files: &[FileComplexity],
    git_ref: &str,
    invocation_path: &str,
    covers_removed: &dyn Fn(&Path) -> bool,
    gate: Option<u64>,
) -> DiffReport {
    let mut report = DiffReport::default();

    let mut located = Repositories::new(git_ref, invocation_path);
    let mut repositories: Vec<RepositoryFiles> = Vec::new();
    for (index, file) in current_files.iter().enumerate() {
        let (root, commit, path_from_root) = match located.locate(&file.path) {
            Ok(location) => location,
            Err(reason) => {
                report.unresolved.push(UnresolvedFile {
                    file_path: file.path.clone(),
                    reason,
                });
                continue;
            }
        };
        match repositories
            .iter_mut()
            .find(|repository| repository.root == root)
        {
            Some(repository) => repository.files.push((index, path_from_root)),
            None => repositories.push(RepositoryFiles {
                root,
                commit,
                files: vec![(index, path_from_root)],
            }),
        }
    }

    let mut objects: Vec<GitObject> = current_files.iter().map(|_| GitObject::Missing).collect();
    let mut changed = vec![false; current_files.len()];
    let mut repository_of: Vec<Option<usize>> = vec![None; current_files.len()];
    let mut removed: Vec<String> = Vec::new();
    for (repository, group) in repositories.into_iter().enumerate() {
        let RepositoryFiles {
            root,
            commit,
            files,
        } = group;
        for (index, _) in &files {
            repository_of[*index] = Some(repository);
        }
        let selection = Selection::new(
            &root,
            files
                .iter()
                .map(|(index, path_from_root)| {
                    (path_from_root.clone(), current_files[*index].path.as_str())
                })
                .collect(),
            invocation_path,
            covers_removed,
        );
        let Some(changes) = changed_python_files(&commit, &root, false) else {
            continue;
        };
        let touched: HashSet<&str> = changes
            .iter()
            .flat_map(|change| [change.old.as_deref(), change.new.as_deref()])
            .flatten()
            .collect();
        for (index, path_from_root) in &files {
            changed[*index] = touched.contains(path_from_root.as_str());
        }
        let renamed_from: HashMap<&str, &str> = changes
            .iter()
            .filter(|change| change.old_regular)
            .filter_map(|change| match (&change.old, &change.new) {
                (Some(old), Some(new)) if old != new && selection.removed(old).is_some() => {
                    Some((new.as_str(), old.as_str()))
                }
                _ => None,
            })
            .collect();
        let deleted: Vec<(&str, String)> = changes
            .iter()
            .filter(|change| change.new.is_none() && change.old_regular)
            .filter_map(|change| {
                let old = change.old.as_deref()?;
                Some((old, selection.removed(old)?))
            })
            .collect();
        let by_new: HashMap<&str, &Change> = changes
            .iter()
            .filter_map(|change| Some((change.new.as_deref()?, change)))
            .collect();
        let by_old: HashMap<&str, &Change> = changes
            .iter()
            .filter_map(|change| Some((change.old.as_deref()?, change)))
            .collect();
        let path_at_ref = |path: &str| -> Option<String> {
            if let Some(old) = renamed_from.get(path) {
                return Some(old.to_string());
            }
            let regular = match by_new.get(path).or_else(|| by_old.get(path)) {
                Some(change) => change.old_regular && change.old.as_deref() == Some(path),
                None => root
                    .join(path)
                    .symlink_metadata()
                    .is_ok_and(|metadata| metadata.is_file()),
            };
            regular.then(|| path.to_string())
        };

        let names: Vec<Option<String>> = files
            .iter()
            .map(|(_, path_from_root)| {
                path_at_ref(path_from_root).map(|old_path| format!("{}:{}", commit, old_path))
            })
            .chain(
                deleted
                    .iter()
                    .map(|(old, _)| Some(format!("{}:{}", commit, old))),
            )
            .collect();
        let mut read = read_objects(&root, &names).into_iter();
        for ((index, _), object) in files.iter().zip(read.by_ref()) {
            objects[*index] = object;
        }
        for ((_, display), object) in deleted.into_iter().zip(read) {
            objects.push(object);
            removed.push(display);
            repository_of.push(Some(repository));
        }
    }
    let mut old_functions = analyse_contents(&objects).into_iter();
    let mut ranges: Vec<Range<usize>> = Vec::with_capacity(objects.len());

    for (file, old_functions) in current_files.iter().zip(old_functions.by_ref()) {
        let start = report.entries.len();
        let current_map = keyed_complexities(&file.functions);
        let mut old_map = old_functions.flatten().unwrap_or_default();
        drop_marked(&mut old_map, &current_map, || {
            let content = source_on_disk(&Path::new(invocation_path).join(&file.path))?;
            analyse_content_to_map(&content, true)
        });

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
        ranges.push(start..report.entries.len());
    }

    for (display, old_functions) in removed.into_iter().zip(old_functions) {
        let start = report.entries.len();
        let old_map = old_functions.flatten().unwrap_or_default();
        let mut names: Vec<&String> = old_map.keys().collect();
        names.sort();
        for name in names {
            report.entries.push(DiffEntry {
                file_path: display.clone(),
                func_name: name.clone(),
                old_complexity: old_map.get(name).copied(),
                new_complexity: None,
            });
        }
        ranges.push(start..report.entries.len());
    }

    if let Some(max_complexity) = gate_threshold(&report.entries, gate) {
        let mut by_repository: BTreeMap<Option<usize>, Vec<ComparedFile>> = BTreeMap::new();
        for (index, (object, entries)) in objects.iter().zip(ranges).enumerate() {
            let old = object.text();
            let compared = match current_files.get(index) {
                None => ComparedFile {
                    entries,
                    old,
                    new: None,
                    scored: None,
                },
                Some(_) if old.is_some() && !changed[index] => continue,
                Some(file) => ComparedFile {
                    entries,
                    old,
                    new: source_on_disk(&Path::new(invocation_path).join(&file.path))
                        .map(Cow::Owned),
                    scored: Some(&file.functions),
                },
            };
            by_repository
                .entry(repository_of[index])
                .or_default()
                .push(compared);
        }
        for files in by_repository.values() {
            count_bodies(&report.entries, max_complexity, files, &mut report.bodies);
        }
    }

    report
}

/// Compares the staged content of each file the run selects with
/// `git_ref`, in the repository of `invocation_path`. The index decides
/// the selection, whatever the working tree holds: a staged regular file is
/// selected when `covers` accepts its absolute path, and so is a staged
/// deletion. A file git pairs as renamed compares with its old path when
/// the run covers that path, and as added otherwise; a file renamed out of
/// the selection reports nothing. A path that is not a regular file at the
/// ref compares as added, and its deletion reports nothing. Each entry
/// names its file by the path the run reports, the path of the current file
/// there when there is one. When some function is new above `gate`, the
/// gate's threshold, the report counts the bodies of the compared files.
/// Returns `None` outside a repository.
pub fn compute_staged_diff(
    current_files: &[FileComplexity],
    git_ref: &str,
    invocation_path: &str,
    covers: &dyn Fn(&Path) -> bool,
    gate: Option<u64>,
) -> Option<DiffReport> {
    let root = Path::new(&git_root(Path::new(invocation_path))?)
        .canonicalize()
        .ok()?;
    let invocation = canonical_or_given(invocation_path);
    let files = current_files
        .iter()
        .filter_map(|file| {
            let path = invocation.join(&file.path).canonicalize().ok()?;
            Some((path_from(&root, &path)?, file.path.as_str()))
        })
        .collect();
    let selection = Selection::new(&root, files, invocation_path, covers);

    let mut report = DiffReport::default();
    let compared: Vec<(Option<String>, Option<String>, String)> =
        changed_python_files(git_ref, &root, true)
            .unwrap_or_default()
            .into_iter()
            .filter_map(|change| {
                if let Some(new) = change.new.as_deref() {
                    if !change.new_regular || !selection.covers(new) {
                        return None;
                    }
                    let display = selection.name(new);
                    let old = change
                        .old
                        .filter(|old| change.old_regular && (old == new || selection.covers(old)));
                    return Some((old, change.new, display));
                }
                let old = change
                    .old
                    .filter(|old| change.old_regular && selection.covers(old))?;
                let display = selection.name(&old);
                Some((Some(old), None, display))
            })
            .collect();
    let requests: Vec<Option<String>> = compared
        .iter()
        .flat_map(|(old, new, _)| {
            [
                old.as_ref().map(|old| format!("{}:{}", git_ref, old)),
                new.as_ref().map(|new| format!(":0:{}", new)),
            ]
        })
        .collect();
    let objects = read_objects(&root, &requests);
    let mut contents = analyse_contents(&objects).into_iter();
    let mut ranges: Vec<Range<usize>> = Vec::with_capacity(compared.len());
    for ((_, _, display), sides) in compared.into_iter().zip(objects.chunks(2)) {
        let start = report.entries.len();
        let mut old_map = contents.next().flatten().flatten().unwrap_or_default();
        let new_map = contents.next().flatten().flatten().unwrap_or_default();
        drop_marked(&mut old_map, &new_map, || {
            analyse_content_to_map(sides[1].text()?, true)
        });

        let mut names: Vec<String> = old_map.keys().chain(new_map.keys()).cloned().collect();
        names.sort();
        names.dedup();

        for name in names {
            report.entries.push(DiffEntry {
                file_path: display.clone(),
                func_name: name.clone(),
                old_complexity: old_map.get(&name).copied(),
                new_complexity: new_map.get(&name).copied(),
            });
        }
        ranges.push(start..report.entries.len());
    }

    if let Some(max_complexity) = gate_threshold(&report.entries, gate) {
        let files: Vec<ComparedFile> = objects
            .chunks(2)
            .zip(ranges)
            .map(|(sides, entries)| ComparedFile {
                entries,
                old: sides[0].text(),
                new: sides[1].text().map(Cow::Borrowed),
                scored: None,
            })
            .collect();
        count_bodies(&report.entries, max_complexity, &files, &mut report.bodies);
    }

    Some(report)
}

/// `gate`, when some entry is NEW above it, so the gate needs the bodies.
fn gate_threshold(entries: &[DiffEntry], gate: Option<u64>) -> Option<u64> {
    gate.filter(|&max_complexity| {
        entries
            .iter()
            .any(|entry| is_new_above(entry, max_complexity))
    })
}

fn is_new_above(entry: &DiffEntry, max_complexity: u64) -> bool {
    entry.status() == DiffStatus::New && entry.new_complexity.is_some_and(|c| c > max_complexity)
}

/// A file the diff compares, for the gate's body counts.
struct ComparedFile<'a> {
    /// Its entries in the report.
    entries: Range<usize>,
    /// Its content at the ref, when the diff compares it with one.
    old: Option<&'a str>,
    /// Its content now, unless it was deleted.
    new: Option<Cow<'a, str>>,
    /// The functions the run scored in `new`, whose keys its entries use;
    /// `None` when they use the keys of `new` with ignore markers honored.
    scored: Option<&'a [FunctionComplexity]>,
}

/// Counts the functions of each body on each side of `files`, the compared
/// files of one repository, ignore markers disregarded, and finds the body
/// of each REMOVED entry and of each NEW entry above `max_complexity`. The
/// bodies are added to `bodies` apart from those of any other call, so a
/// function never pairs with or counts against one in another repository.
/// When the current content of a file holds a logical line too long to
/// compare safely, nothing is counted, so no NEW function of `files` above
/// the threshold passes; at the ref, such a file is left out.
fn count_bodies(
    entries: &[DiffEntry],
    max_complexity: u64,
    files: &[ComparedFile],
    bodies: &mut Bodies,
) {
    let texts: Vec<Option<&str>> = files
        .iter()
        .flat_map(|file| [file.old, file.new.as_deref()])
        .collect();
    let modules = map_items(&texts, |text| text.and_then(|text| parse_module(text).ok()));
    if modules.chunks(2).any(|sides| {
        sides[1]
            .as_ref()
            .is_some_and(ParsedModule::has_long_logical_line)
    }) {
        return;
    }

    let mut interned: HashMap<Vec<ComparableStmt>, usize> = HashMap::new();
    for ((file, sides), texts) in files.iter().zip(modules.chunks(2)).zip(texts.chunks(2)) {
        for (side, (module, text)) in sides.iter().zip(texts).enumerate() {
            let (Some(module), Some(text)) = (module, text) else {
                continue;
            };
            if module.has_long_logical_line() {
                continue;
            }
            let functions = defined_functions(module.suite(), text);
            let ids: Vec<usize> = functions
                .iter()
                .map(|function| {
                    let body = function.def.body.iter().map(ComparableStmt::from).collect();
                    let next = bodies.counts.len();
                    let id = *interned.entry(body).or_insert(next);
                    if id == next {
                        bodies.counts.push([0, 0]);
                    }
                    bodies.counts[id][side] += 1;
                    id
                })
                .collect();
            let keyed = match file.scored.filter(|_| side == 1) {
                Some(scored) => keyed_by_line(&functions, &ids, scored),
                None => keyed_visible(&functions, &ids),
            };
            for index in file.entries.clone() {
                let entry = &entries[index];
                let wanted = match side {
                    0 => entry.status() == DiffStatus::Removed,
                    _ => is_new_above(entry, max_complexity),
                };
                if wanted && let Some(&id) = keyed.get(entry.func_name.as_str()) {
                    bodies.of_entry.insert(index, id);
                }
            }
        }
    }
}

/// The body of each function in `scored`, by key, found by its name and
/// first line among `functions`.
fn keyed_by_line(
    functions: &[DefinedFunction],
    ids: &[usize],
    scored: &[FunctionComplexity],
) -> HashMap<String, usize> {
    let by_line: HashMap<(&str, u64), usize> = functions
        .iter()
        .zip(ids)
        .map(|(function, &id)| ((function.name.as_str(), function.line_start), id))
        .collect();
    function_keys(scored)
        .into_iter()
        .zip(scored)
        .filter_map(|(key, function)| {
            Some((
                key,
                *by_line.get(&(function.name.as_str(), function.line_start))?,
            ))
        })
        .collect()
}

/// The body of each function of `functions` that no ignore marker hides,
/// by key.
fn keyed_visible(functions: &[DefinedFunction], ids: &[usize]) -> HashMap<String, usize> {
    let visible: Vec<(&DefinedFunction, usize)> = functions
        .iter()
        .zip(ids.iter().copied())
        .filter(|(function, _)| !function.ignored)
        .collect();
    let lines: Vec<(&str, u64)> = visible
        .iter()
        .map(|(function, _)| (function.name.as_str(), function.line_start))
        .collect();
    keys_in_source_order(&lines)
        .into_iter()
        .zip(visible.into_iter().map(|(_, id)| id))
        .collect()
}

/// How the diff gate judges one entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// A REGRESSED or NEW function above the threshold, which fails the gate.
    Fails,
    /// A NEW function above the threshold that passes as a copy of the
    /// REMOVED entry at this index, whose body is the same.
    Replaces(usize),
    /// Any other entry.
    Passes,
}

/// The gate's verdict on each entry, in order. A REGRESSED or NEW function
/// above `max_complexity` fails, except a NEW one that replaces a REMOVED
/// entry of the same body and of at least its complexity, each REMOVED
/// entry replaced once, when the compared files, as `bodies` counts them,
/// do not hold that body more often now than at the ref: a move, a function
/// rename, or a class rename that leaves the body as it was. Complexity
/// counts because a call to the function's own name scores as recursion.
pub fn gate_verdicts(entries: &[DiffEntry], max_complexity: u64, bodies: &Bodies) -> Vec<Verdict> {
    let mut verdicts: Vec<Verdict> = entries
        .iter()
        .map(|entry| match entry.status() {
            DiffStatus::Regressed | DiffStatus::New
                if entry.new_complexity.is_some_and(|c| c > max_complexity) =>
            {
                Verdict::Fails
            }
            _ => Verdict::Passes,
        })
        .collect();

    let mut by_body: HashMap<usize, (Vec<usize>, Vec<usize>)> = HashMap::new();
    for (index, entry) in entries.iter().enumerate() {
        let Some(&body) = bodies.of_entry.get(&index) else {
            continue;
        };
        let [at_ref, now] = bodies.counts[body];
        if now > at_ref {
            continue;
        }
        let (added, removed) = by_body.entry(body).or_default();
        match entry.status() {
            DiffStatus::New if verdicts[index] == Verdict::Fails => added.push(index),
            DiffStatus::Removed => removed.push(index),
            _ => {}
        }
    }
    for (mut added, mut removed) in by_body.into_values() {
        added.sort_by_key(|&index| std::cmp::Reverse(entries[index].new_complexity));
        for index in added {
            let complexity = entries[index].new_complexity.unwrap_or_default();
            let partner = removed
                .iter()
                .enumerate()
                .map(|(position, &other)| {
                    (position, entries[other].old_complexity.unwrap_or_default())
                })
                .filter(|&(_, old)| old >= complexity)
                .min_by_key(|&(_, old)| old)
                .map(|(position, _)| position);
            if let Some(position) = partner {
                verdicts[index] = Verdict::Replaces(removed.remove(position));
            }
        }
    }
    verdicts
}

/// Whether the gate fails any of `entries` when it knows no function body:
/// every REGRESSED or NEW function above `max_complexity` fails, a moved or
/// renamed one included.
pub fn has_regressions(entries: &[DiffEntry], max_complexity: u64) -> bool {
    gate_verdicts(entries, max_complexity, &Bodies::default()).contains(&Verdict::Fails)
}

/// The files of one repository that a run selects, keyed by their path from
/// the repository root, and what its path arguments cover.
struct Selection<'a> {
    root: &'a Path,
    invocation: PathBuf,
    files: HashMap<String, &'a str>,
    coverage: &'a dyn Fn(&Path) -> bool,
}

impl<'a> Selection<'a> {
    fn new(
        root: &'a Path,
        files: HashMap<String, &'a str>,
        invocation_path: &str,
        coverage: &'a dyn Fn(&Path) -> bool,
    ) -> Self {
        Self {
            root,
            invocation: canonical_or_given(invocation_path),
            files,
            coverage,
        }
    }

    /// Whether the run's path arguments cover the file at `path_from_root`.
    fn covers(&self, path_from_root: &str) -> bool {
        (self.coverage)(&self.root.join(path_from_root))
    }

    /// The path the run reports for the file at `path_from_root`: the path
    /// of the current file there, or else its path from the invocation
    /// directory, or its absolute path outside it.
    fn name(&self, path_from_root: &str) -> String {
        if let Some(path) = self.files.get(path_from_root) {
            return path.to_string();
        }
        let path = self.root.join(path_from_root);
        match path.strip_prefix(&self.invocation) {
            Ok(relative) => slashed(relative),
            Err(_) => slashed(&path),
        }
    }

    /// The path the run reports for `path_from_root` when it is not a
    /// current file, the working tree no longer holds it, and the run covers
    /// it, so its functions may count as removed.
    fn removed(&self, path_from_root: &str) -> Option<String> {
        let gone = !self.files.contains_key(path_from_root)
            && self.root.join(path_from_root).symlink_metadata().is_err();
        (gone && self.covers(path_from_root)).then(|| self.name(path_from_root))
    }
}

fn canonical_or_given(path: &str) -> PathBuf {
    Path::new(path)
        .canonicalize()
        .unwrap_or_else(|_| PathBuf::from(path))
}

fn path_from(root: &Path, path: &Path) -> Option<String> {
    Some(path.strip_prefix(root).ok()?.to_str()?.replace('\\', "/"))
}

fn slashed(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
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
        &[
            "rev-parse",
            "--verify",
            "--quiet",
            "--end-of-options",
            &revision,
        ],
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
    let fields = gitlink_fields(&output, path)?;
    fields.get(2).map(|pinned| pinned.to_string())
}

/// Whether the index of the repository at `cwd` tracks `path` as a gitlink.
fn tracks_gitlink(path: &str, cwd: &Path) -> bool {
    run_git(
        cwd,
        &[
            "--literal-pathspecs",
            "ls-files",
            "--stage",
            "-z",
            "--",
            path,
        ],
        GIT_ROOT_TIMEOUT,
    )
    .is_some_and(|(success, output)| success && gitlink_fields(&output, path).is_some())
}

/// The fields before the tab of the gitlink entry at `path` in the `-z`
/// output of `ls-tree` or `ls-files --stage`.
fn gitlink_fields<'a>(output: &'a str, path: &str) -> Option<Vec<&'a str>> {
    output.split('\0').find_map(|entry| {
        let (fields, entry_path) = entry.split_once('\t')?;
        let fields: Vec<&str> = fields.split(' ').collect();
        (entry_path == path && fields.first() == Some(&"160000")).then_some(fields)
    })
}

/// The repository of each analyzed file, asked of git once for each
/// directory that holds a `.git` entry, and the commit each repository is
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

    /// The root of the repository that holds `directory`, found from the
    /// nearest directory at or above it that holds a `.git` directory or
    /// file.
    fn root_of(&mut self, directory: &Path) -> Option<PathBuf> {
        let nearest = directory
            .ancestors()
            .find(|ancestor| ancestor.join(".git").exists())?;
        self.roots
            .entry(nearest.to_path_buf())
            .or_insert_with(|| {
                git_root(nearest).and_then(|root| Path::new(&root).canonicalize().ok())
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

    /// A repository that the repository around it tracks as a gitlink is
    /// compared at the commit that gitlink pins at the outer repository's
    /// compared commit, unless it holds the invocation directory, and takes
    /// the outer repository's reason when that one has no compared commit.
    /// Any other repository is compared at the ref.
    fn resolve_commit(&mut self, root: &Path) -> Result<String, UnresolvedReason> {
        let repository = root.display().to_string();
        let Some((outer, path)) = self.tracking_outer(root) else {
            return commit_id(self.git_ref, root)
                .ok_or(UnresolvedReason::UnknownRef { repository });
        };
        let outer_commit = self.commit_of(&outer)?;
        let Some(pinned) = pinned_commit(&outer_commit, &path, &outer) else {
            return Err(UnresolvedReason::NotPinned { repository });
        };
        commit_id(&pinned, root).ok_or(UnresolvedReason::PinnedCommitMissing {
            repository,
            commit: pinned,
        })
    }

    /// The root of the repository around `root` and the path of `root` in
    /// it, when that repository tracks `root` as a gitlink and `root` does
    /// not hold the invocation directory.
    fn tracking_outer(&mut self, root: &Path) -> Option<(PathBuf, String)> {
        if self
            .invocation_directory
            .as_deref()
            .is_some_and(|directory| directory.starts_with(root))
        {
            return None;
        }
        let parent = root.parent()?;
        let outer = self.root_of(parent)?;
        if !parent.starts_with(&outer) {
            return None;
        }
        let path = path_from(&outer, root)?;
        tracks_gitlink(&path, &outer).then_some((outer, path))
    }
}

#[derive(Debug, PartialEq, Eq)]
enum GitObject {
    Missing,
    Unreadable,
    Text(String),
}

impl GitObject {
    fn text(&self) -> Option<&str> {
        match self {
            GitObject::Text(text) => Some(text),
            _ => None,
        }
    }
}

/// Reads every object that `names` lists through one `git cat-file --batch`
/// process, in order. A `None` name, or one with a control character, which
/// git would split or trim as a line ending, is not requested and reads as
/// missing.
/// A blob reads as its text; a blob over `MAX_BLOB_BYTES` or not valid
/// UTF-8, or an object that does not exist, reads as missing; any other
/// object, such as a tree, reads as unreadable.
fn read_objects(cwd: &Path, names: &[Option<String>]) -> Vec<GitObject> {
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

/// One Python file that changed since the ref, by its path from the
/// repository root at the ref and on the new side, and whether each side
/// is a regular file rather than a symbolic link or a submodule. An added
/// file has no old path, and a deleted file has no new path.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Change {
    old: Option<String>,
    new: Option<String>,
    old_regular: bool,
    new_regular: bool,
}

/// The Python files that differ between `git_ref` and the working tree, or
/// the index when `staged`, with renames paired by git and type changes
/// listed. `None` when git cannot list them.
fn changed_python_files(git_ref: &str, cwd: &Path, staged: bool) -> Option<Vec<Change>> {
    let mut args = vec![
        "diff",
        "--raw",
        "-z",
        "--no-color",
        "-M",
        "--diff-filter=ACMRDT",
    ];
    if staged {
        args.push("--cached");
    }
    args.extend(["--end-of-options", git_ref, "--", "*.py"]);
    match run_git_bytes(cwd, &args, GIT_TIMEOUT)? {
        (true, output) => Some(parse_raw(&output)),
        (false, _) => None,
    }
}

/// Reads `git diff --raw -z` output. A copy reads as an added file, so it
/// never inherits the history of its source. A change with a path that is
/// not UTF-8 is left out, and the others are still read.
fn parse_raw(output: &[u8]) -> Vec<Change> {
    let mut fields = output.split(|byte| *byte == 0);
    let mut changes = Vec::new();
    while let Some(header) = fields.next().and_then(|header| header.strip_prefix(b":")) {
        let Ok(header) = std::str::from_utf8(header) else {
            break;
        };
        let parts: Vec<&str> = header.split(' ').collect();
        let [old_mode, new_mode, _, _, status] = parts[..] else {
            break;
        };
        let Some(path) = fields.next() else {
            break;
        };
        let renamed = matches!(status.as_bytes().first(), Some(b'R' | b'C'));
        let new_path = if renamed {
            let Some(new) = fields.next() else {
                break;
            };
            Some(new)
        } else {
            None
        };
        let Ok(path) = std::str::from_utf8(path).map(str::to_string) else {
            continue;
        };
        let (old, new) = match (status.as_bytes().first(), new_path) {
            (_, Some(new)) => {
                let Ok(new) = std::str::from_utf8(new).map(str::to_string) else {
                    continue;
                };
                (status.starts_with('R').then_some(path), Some(new))
            }
            (Some(b'A'), None) => (None, Some(path)),
            (Some(b'D'), None) => (Some(path), None),
            _ => (Some(path.clone()), Some(path)),
        };
        changes.push(Change {
            old,
            new,
            old_regular: is_regular(old_mode),
            new_regular: is_regular(new_mode),
        });
    }
    changes
}

fn is_regular(mode: &str) -> bool {
    matches!(mode, "100644" | "100755")
}

/// Scores each object: `None` when it is missing, `Some(None)` when it does
/// not parse, and the complexity of each function otherwise.
fn analyse_contents(objects: &[GitObject]) -> Vec<Option<Option<HashMap<String, u64>>>> {
    map_items(objects, analyse_object)
}

/// Maps each of `items` through `f`, in parallel under the `runner` feature.
#[cfg(feature = "runner")]
fn map_items<T: Sync, U: Send>(items: &[T], f: impl Fn(&T) -> U + Sync + Send) -> Vec<U> {
    use rayon::prelude::*;

    items.par_iter().map(f).collect()
}

#[cfg(not(feature = "runner"))]
fn map_items<T, U>(items: &[T], f: impl Fn(&T) -> U) -> Vec<U> {
    items.iter().map(f).collect()
}

fn analyse_object(object: &GitObject) -> Option<Option<HashMap<String, u64>>> {
    match object {
        GitObject::Missing => None,
        GitObject::Unreadable => Some(None),
        GitObject::Text(content) => Some(analyse_content_to_map(content, false)),
    }
}

/// Scores each function of `content`, honoring ignore markers unless
/// `no_ignore` is set.
fn analyse_content_to_map(content: &str, no_ignore: bool) -> Option<HashMap<String, u64>> {
    let parsed = crate::syntax_tree::parse_module(content).ok()?;
    let (functions, _) = crate::cognitive_complexity::function_level_cognitive_complexity_shared(
        parsed.suite(),
        content,
        &crate::rules::AnalysisOptions {
            no_ignore,
            ..Default::default()
        },
    );
    Some(keyed_complexities(&functions))
}

fn source_on_disk(path: &Path) -> Option<String> {
    let bytes =
        crate::fs_guard::read_bounded(path, None, crate::fs_guard::MAX_SOURCE_BYTES).ok()?;
    crate::encoding::decode_python_source(&bytes).ok()
}

/// Drops each key of `old_map` that `current_map` lacks only because an
/// ignore marker hides the function now, as `unmarked`, the current
/// content scored with every marker disregarded, shows. Such a function was
/// not removed, so it does not report as REMOVED.
fn drop_marked(
    old_map: &mut HashMap<String, u64>,
    current_map: &HashMap<String, u64>,
    unmarked: impl FnOnce() -> Option<HashMap<String, u64>>,
) {
    if old_map.keys().all(|key| current_map.contains_key(key)) {
        return;
    }
    let Some(unmarked) = unmarked() else {
        return;
    };
    old_map.retain(|key, _| current_map.contains_key(key) || !unmarked.contains_key(key));
}

/// One key per function, in input order: the qualified name, then `#N` for
/// the N-th function of that name in source order. A property's getter and
/// setter, or a redefinition, never share a key, in the diff or in the
/// snapshot.
pub fn function_keys(functions: &[FunctionComplexity]) -> Vec<String> {
    let lines: Vec<(&str, u64)> = functions
        .iter()
        .map(|function| (function.name.as_str(), function.line_start))
        .collect();
    keys_in_source_order(&lines)
}

/// The key of each function given by its name and first line, as
/// `function_keys` assigns them.
fn keys_in_source_order(functions: &[(&str, u64)]) -> Vec<String> {
    let mut order: Vec<usize> = (0..functions.len()).collect();
    order.sort_by_key(|&index| functions[index].1);

    let mut keys = vec![String::new(); functions.len()];
    let mut seen: HashMap<&str, usize> = HashMap::new();
    for index in order {
        let name = functions[index].0;
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

fn run_git(cwd: &Path, args: &[&str], timeout: Duration) -> Option<(bool, String)> {
    let (success, output) = run_git_bytes(cwd, args, timeout)?;
    Some((success, String::from_utf8(output).ok()?))
}

fn run_git_bytes(cwd: &Path, args: &[&str], timeout: Duration) -> Option<(bool, Vec<u8>)> {
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
    let mut pause = Duration::from_millis(1);
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => {
                if Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    return None;
                }
                thread::sleep(pause);
                pause = (pause * 2).min(Duration::from_millis(10));
            }
            Err(_) => return None,
        }
    };

    let output = stdout_handle.join().ok()?;
    Some((status.success(), output))
}

#[cfg(test)]
mod tests;
