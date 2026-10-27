use crate::classes::{FileComplexity, IgnoredLocation, RemovableIgnore};
use crate::cognitive_complexity::function_level_cognitive_complexity_shared;
use crate::encoding::decode_python_source;
use crate::fs_guard::{MAX_SOURCE_BYTES, read_bounded};
use crate::helpers::exclude::{
    ExcludeMatcher, get_paths_to_process, global_gitignore, ignore_rules_hide,
};
use crate::rules::{AnalysisOptions, RuleSet};
use crate::syntax_tree::parse_module;
use crate::utils::{collect_ignored_locations, filter_removable_ignores};
use rayon::prelude::*;
use ruff_python_ast::Suite;
use std::collections::HashSet;
use std::path::{self, PathBuf};

use crate::cognitive_complexity::code_complexity_shared;

type ComplexitiesAndFailedPaths = (Vec<FileComplexity>, Vec<String>);

/// The ignore-comment reports a run collects while it reads each file:
/// every ignored location, as `--report-ignored` lists them, and the ignore
/// comments whose function is at or under `removable_above` without them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LocationReports {
    pub ignored: bool,
    pub removable_above: Option<u64>,
}

impl LocationReports {
    fn any(&self) -> bool {
        self.ignored || self.removable_above.is_some()
    }
}

struct RunContext {
    invocation: path::PathBuf,
    analysis: AnalysisOptions,
    reports: LocationReports,
}

/// One file the run read: a file found under a directory argument, or a
/// file argument itself. `location` is the path its ignore comments are
/// reported at, when the run collects them.
struct SourceRecord {
    path: String,
    resolved: bool,
    location: Option<String>,
    display: Option<String>,
    ignored: Vec<IgnoredLocation>,
    removable: Vec<RemovableIgnore>,
}

/// A file that a path argument names, found before any file is read so
/// that a file reached through several arguments is analyzed once. `key` is
/// its resolved path with forward slashes.
struct PendingSource {
    key: String,
    path: String,
    resolved: bool,
    location: Option<String>,
}

struct Discovery {
    failure: Option<String>,
    files: Vec<PendingSource>,
    skipped: Vec<SkippedPath>,
}

struct ArgumentRecord {
    failure: Option<String>,
    sources: Vec<(SourceRecord, Option<FileComplexity>)>,
    skipped: Vec<SkippedPath>,
}

/// The results of analyzing every path argument, with the ignore-comment
/// reports gathered in the same pass over each file. `skipped` holds the
/// entries discovery left out; the files an ignore rule hides are listed
/// when the run reports ignored locations, or when it selected nothing.
pub struct Analysis {
    pub files: Vec<FileComplexity>,
    pub failed_paths: Vec<String>,
    pub skipped: Vec<SkippedPath>,
    sources: Vec<SourceRecord>,
    context: RunContext,
}

impl Analysis {
    pub fn run(
        paths: &[String],
        exclude: &[String],
        opts: &AnalysisOptions,
        invocation_path: &str,
        reports: LocationReports,
    ) -> Self {
        let context = RunContext {
            invocation: canonical(path::Path::new(invocation_path)),
            analysis: opts.clone(),
            reports,
        };
        let mut discoveries: Vec<Discovery> = paths
            .par_iter()
            .map(|path| discover_argument(path, exclude, &context))
            .collect();
        let mut seen: HashSet<String> = HashSet::new();
        for discovery in &mut discoveries {
            discovery.files.retain(|file| seen.insert(file.key.clone()));
        }
        let arguments: Vec<ArgumentRecord> = discoveries
            .into_par_iter()
            .map(|discovery| ArgumentRecord {
                failure: discovery.failure,
                sources: discovery
                    .files
                    .into_par_iter()
                    .map(|file| analyze_source(file.path, file.resolved, file.location, &context))
                    .collect(),
                skipped: discovery.skipped,
            })
            .collect();

        let mut files = Vec::new();
        let mut failed_paths = Vec::new();
        let mut sources = Vec::new();
        let mut skipped = Vec::new();
        for mut argument in arguments {
            failed_paths.extend(argument.failure);
            skipped.append(&mut argument.skipped);
            let mut complexities = Vec::new();
            for (source, complexity) in argument.sources {
                match complexity {
                    Some(complexity) => complexities.push(complexity),
                    None => failed_paths.push(source.path.clone()),
                }
                sources.push(source);
            }
            sort_files(&mut complexities);
            files.append(&mut complexities);
        }
        if files.is_empty() && failed_paths.is_empty() && !context.reports.ignored {
            skipped.extend(hidden_files(paths, exclude, &context.invocation));
        }

        Self {
            files,
            failed_paths,
            skipped,
            sources,
            context,
        }
    }

    /// Analyzes again the files whose reported path is in `paths`, after
    /// they were rewritten, and updates their results and ignore reports in
    /// place. A file that no longer reads or parses leaves the results.
    pub fn reanalyze(&mut self, paths: &HashSet<String>) {
        let mut fresh: Vec<(String, Option<FileComplexity>)> = Vec::new();
        for source in self.sources.iter_mut() {
            let Some(display) = source.display.clone() else {
                continue;
            };
            if !paths.contains(&display) {
                continue;
            }
            let (record, complexity) = analyze_source(
                source.path.clone(),
                source.resolved,
                source.location.clone(),
                &self.context,
            );
            *source = record;
            fresh.push((display, complexity));
        }
        for (display, complexity) in fresh {
            match complexity {
                Some(complexity) => {
                    for file in self.files.iter_mut().filter(|file| file.path == display) {
                        *file = complexity.clone();
                    }
                }
                None => self.files.retain(|file| file.path != display),
            }
        }
    }

    /// Every ignored location, sorted by path and line.
    pub fn ignored_locations(&self) -> Vec<IgnoredLocation> {
        let mut locations: Vec<IgnoredLocation> = self
            .sources
            .iter()
            .flat_map(|source| source.ignored.iter().cloned())
            .collect();
        locations.sort_by_key(Located::sort_key);
        locations
    }

    /// Every removable ignore comment, sorted by path and line.
    pub fn removable_ignores(&self) -> Vec<RemovableIgnore> {
        let mut removable: Vec<RemovableIgnore> = self
            .sources
            .iter()
            .flat_map(|source| source.removable.iter().cloned())
            .collect();
        removable.sort_by_key(Located::sort_key);
        removable
    }
}

/// The path arguments of a run, resolved once, to decide whether the run
/// covers a file that the working tree no longer holds, such as a staged
/// deletion. A file argument covers the file it names. A directory argument
/// covers a `*.py` path under it that discovery would select: outside
/// hidden entries, not hidden by `.ignore`, `.gitignore`, git's exclude
/// file, or the global gitignore, and matched by no exclude pattern,
/// relative to the argument or to the invocation directory.
pub struct ArgumentCoverage {
    files: Vec<PathBuf>,
    directories: Vec<PathBuf>,
    exclude: ExcludeMatcher,
    anchor: String,
    global_gitignore: ignore::gitignore::Gitignore,
}

impl ArgumentCoverage {
    pub fn new(paths: &[String], exclude: &[String], invocation_path: &str) -> Self {
        let mut coverage = Self {
            files: Vec::new(),
            directories: Vec::new(),
            exclude: ExcludeMatcher::new(exclude),
            anchor: slashed(&canonical(path::Path::new(invocation_path))),
            global_gitignore: global_gitignore(),
        };
        for path in paths {
            let resolved = resolved(path);
            if resolved.is_dir() {
                coverage.directories.push(resolved);
            } else {
                coverage.files.push(resolved);
            }
        }
        coverage
    }

    /// Whether the run covers `file`, an absolute path with its existing
    /// directories resolved.
    pub fn covers(&self, file: &path::Path) -> bool {
        if self.files.iter().any(|candidate| candidate == file) {
            return true;
        }
        if file.extension().is_none_or(|extension| extension != "py") {
            return false;
        }
        let file_text = slashed(file);
        self.directories.iter().any(|directory| {
            let Ok(relative) = file.strip_prefix(directory) else {
                return false;
            };
            !relative
                .components()
                .any(|component| component.as_os_str().to_string_lossy().starts_with('.'))
                && !self.exclude.is_excluded(&file_text, &slashed(directory))
                && !self.exclude.is_excluded_inside(&file_text, &self.anchor)
                && !ignore_rules_hide(directory, file, &self.global_gitignore)
        })
    }
}

/// `path` resolved, or, when it does not exist, its resolved parent joined
/// with its name.
fn resolved(path: &str) -> PathBuf {
    let path = path::Path::new(path);
    if let Ok(resolved) = path.canonicalize() {
        return resolved;
    }
    let parent = match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent,
        _ => path::Path::new("."),
    };
    match (parent.canonicalize(), path.file_name()) {
        (Ok(parent), Some(name)) => parent.join(name),
        _ => path.to_path_buf(),
    }
}

/// Why discovery left a `*.py` entry out of the analysis.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkipReason {
    NotRegularFile,
    OutsideRoot,
    IgnoreRule,
    Unreadable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkippedPath {
    pub path: String,
    pub reason: SkipReason,
}

pub fn run_analysis_shared(
    paths: &[String],
    exclude: &[String],
    opts: &AnalysisOptions,
    invocation_path: &str,
) -> Result<ComplexitiesAndFailedPaths, String> {
    let analysis = Analysis::run(
        paths,
        exclude,
        opts,
        invocation_path,
        LocationReports::default(),
    );
    Ok((analysis.files, analysis.failed_paths))
}

fn hidden_files(paths: &[String], exclude: &[String], invocation: &path::Path) -> Vec<SkippedPath> {
    paths
        .iter()
        .filter(|path| path::Path::new(path).is_dir())
        .filter_map(|path| get_paths_to_process(path, exclude.to_vec(), invocation, true).ok())
        .flat_map(|discovery| discovery.skipped)
        .filter(|skipped| skipped.reason == SkipReason::IgnoreRule)
        .collect()
}

fn sort_files(complexities: &mut [FileComplexity]) {
    complexities.sort_by(|a, b| {
        a.path
            .cmp(&b.path)
            .then(a.file_name.cmp(&b.file_name))
            .then(a.complexity.cmp(&b.complexity))
    });
}

fn canonical(path: &path::Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

fn slashed(path: &path::Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn discover_argument(path: &str, exclude: &[String], context: &RunContext) -> Discovery {
    let path_obj = path::Path::new(path);
    if !path_obj.exists() {
        return Discovery {
            failure: Some(path.to_string()),
            files: Vec::new(),
            skipped: Vec::new(),
        };
    }

    if !path_obj.is_dir() {
        let location = (context.reports.any() && path_obj.is_file())
            .then(|| file_location(path, &context.invocation));
        return Discovery {
            failure: None,
            files: vec![PendingSource {
                key: slashed(&canonical(path_obj)),
                path: path.to_string(),
                resolved: false,
                location,
            }],
            skipped: Vec::new(),
        };
    }

    let discovery = match get_paths_to_process(
        path,
        exclude.to_vec(),
        &context.invocation,
        context.reports.ignored,
    ) {
        Ok(discovery) => discovery,
        Err(e) => {
            return Discovery {
                failure: Some(format!("{}: {}", path, e)),
                files: Vec::new(),
                skipped: Vec::new(),
            };
        }
    };
    let location_base = context
        .reports
        .any()
        .then(|| directory_location_base(path_obj, &context.invocation));
    Discovery {
        failure: None,
        files: discovery
            .files
            .into_iter()
            .map(|file_path| PendingSource {
                key: file_path.clone(),
                location: location_base
                    .as_deref()
                    .map(|base| relative_to_base(&file_path, base).to_string()),
                path: file_path,
                resolved: true,
            })
            .collect(),
        skipped: discovery.skipped,
    }
}

fn analyze_source(
    path: String,
    resolved: bool,
    location: Option<String>,
    context: &RunContext,
) -> (SourceRecord, Option<FileComplexity>) {
    let mut record = SourceRecord {
        path,
        resolved,
        location,
        display: None,
        ignored: Vec::new(),
        removable: Vec::new(),
    };
    let Ok(code) = read_source(&record.path) else {
        return (record, None);
    };
    let parsed = parse_module(&code).ok();
    let suite = parsed.as_ref().map(|parsed| parsed.suite());

    if let Some(relative) = record.location.as_deref() {
        let locations = collect_ignored_locations(&code);
        if let (Some(max_complexity_allowed), Some(suite)) =
            (context.reports.removable_above, suite)
            && !locations.is_empty()
        {
            record.removable =
                removable_ignores(&locations, suite, &code, relative, max_complexity_allowed);
        }
        if context.reports.ignored {
            record.ignored = ignored_locations(locations, relative);
        }
    }

    let complexity = suite.and_then(|suite| analyze_file(&record, suite, &code, context));
    record.display = complexity
        .as_ref()
        .map(|complexity| complexity.path.clone());
    (record, complexity)
}

fn analyze_file(
    record: &SourceRecord,
    suite: &Suite,
    code: &str,
    context: &RunContext,
) -> Option<FileComplexity> {
    let file_path = path::Path::new(&record.path);
    let file_name = file_path.file_name().and_then(|n| n.to_str())?;
    let file_abs = if record.resolved {
        file_path.to_path_buf()
    } else {
        file_path
            .canonicalize()
            .unwrap_or_else(|_| file_path.to_path_buf())
    };
    let relative = file_abs
        .strip_prefix(&context.invocation)
        .ok()
        .and_then(|p| p.to_str())
        .unwrap_or(&record.path);
    let file_name = path::Path::new(relative)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(file_name);
    let (mut functions, complexity) =
        function_level_cognitive_complexity_shared(suite, code, &context.analysis);
    functions.sort_by(|a, b| a.complexity.cmp(&b.complexity).then(a.name.cmp(&b.name)));
    Some(FileComplexity {
        path: relative.to_string(),
        file_name: file_name.to_string(),
        complexity,
        functions,
    })
}

/// The base that the ignore comments under the directory `path_obj` are
/// reported relative to: the invocation directory when the directory is
/// inside it, like the analysis results, and the directory's parent
/// otherwise.
fn directory_location_base(path_obj: &path::Path, invocation: &path::Path) -> String {
    let root = canonical(path_obj);
    if root.starts_with(invocation) {
        slashed(invocation)
    } else {
        slashed(root.parent().unwrap_or(path::Path::new(".")))
    }
}

fn file_location_base(path_obj: &path::Path) -> &str {
    path_obj.parent().and_then(|p| p.to_str()).unwrap_or(".")
}

/// The path that the ignore comments of the file argument `path` are
/// reported at: its resolved path relative to the invocation directory when
/// the file is inside it, like the analysis results, and the path as given
/// relative to its own directory otherwise.
fn file_location(path: &str, invocation: &path::Path) -> String {
    let path_obj = path::Path::new(path);
    let file_abs = canonical(path_obj);
    if file_abs.starts_with(invocation) {
        relative_to_base(&slashed(&file_abs), &slashed(invocation)).to_string()
    } else {
        relative_to_base(path, file_location_base(path_obj)).to_string()
    }
}

fn relative_to_base<'a>(file_path: &'a str, base_path: &str) -> &'a str {
    path::Path::new(file_path)
        .strip_prefix(base_path)
        .ok()
        .and_then(|p| p.to_str())
        .unwrap_or(file_path)
}

fn ignored_locations(locations: Vec<(u64, String)>, relative_path: &str) -> Vec<IgnoredLocation> {
    locations
        .into_iter()
        .map(|(line, comment)| IgnoredLocation {
            path: relative_path.to_string(),
            line,
            comment,
        })
        .collect()
}

fn removable_ignores(
    locations: &[(u64, String)],
    suite: &Suite,
    code: &str,
    relative_path: &str,
    max_complexity_allowed: u64,
) -> Vec<RemovableIgnore> {
    let (functions, _) = function_level_cognitive_complexity_shared(
        suite,
        code,
        &AnalysisOptions {
            no_ignore: true,
            rules: RuleSet::default(),
            ..Default::default()
        },
    );
    filter_removable_ignores(locations, &functions, max_complexity_allowed)
        .into_iter()
        .map(|(line, comment, function, complexity)| RemovableIgnore {
            path: relative_path.to_string(),
            line,
            comment,
            function,
            complexity,
        })
        .collect()
}

pub fn file_complexity_shared(
    file_path: &str,
    base_path: &str,
    opts: &AnalysisOptions,
) -> Result<FileComplexity, String> {
    let path = path::Path::new(file_path);
    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| format!("Invalid file name: {}", file_path))?;
    let relative_path = path
        .strip_prefix(base_path)
        .ok()
        .and_then(|p| p.to_str())
        .unwrap_or(file_path);
    let code = read_source(file_path)?;
    let code_complexity = code_complexity_shared(&code, opts)
        .map_err(|e| format!("Failed to process file '{}': {}", file_path, e))?;
    Ok(FileComplexity {
        path: relative_path.to_string(),
        file_name: file_name.to_string(),
        complexity: code_complexity.complexity,
        functions: code_complexity.functions,
    })
}

pub fn collect_file_ignored_locations(
    file_path: &str,
    base_path: &str,
) -> Result<Vec<IgnoredLocation>, String> {
    let relative_path = relative_to_base(file_path, base_path);
    let code = read_source(file_path)?;
    Ok(ignored_locations(
        collect_ignored_locations(&code),
        relative_path,
    ))
}

pub fn collect_all_ignored_locations_shared(
    paths: &[String],
    exclude: &[String],
    invocation_path: &str,
) -> Result<(Vec<IgnoredLocation>, Vec<String>), String> {
    collect_locations(
        paths,
        exclude,
        invocation_path,
        collect_file_ignored_locations,
    )
}

pub fn collect_removable_ignored_locations_shared(
    paths: &[String],
    exclude: &[String],
    max_complexity_allowed: u64,
    invocation_path: &str,
) -> Result<(Vec<RemovableIgnore>, Vec<String>), String> {
    collect_locations(paths, exclude, invocation_path, |file_path, base_dir| {
        collect_removable_ignores_from_file(file_path, base_dir, max_complexity_allowed)
    })
}

fn read_source(file_path: &str) -> Result<String, String> {
    let bytes = read_bounded(path::Path::new(file_path), None, MAX_SOURCE_BYTES)
        .map_err(|e| format!("Failed to read file '{}': {}", file_path, e))?;
    decode_python_source(&bytes).map_err(|e| format!("File '{}' {}", file_path, e))
}

trait Located {
    fn sort_key(&self) -> (String, u64);
}

impl Located for IgnoredLocation {
    fn sort_key(&self) -> (String, u64) {
        (self.path.clone(), self.line)
    }
}

impl Located for RemovableIgnore {
    fn sort_key(&self) -> (String, u64) {
        (self.path.clone(), self.line)
    }
}

/// Collects per-file locations under `paths`, each file once. A path inside
/// `invocation_path` is reported relative to it, like the analysis results;
/// a directory outside it relative to the directory's parent, and a file
/// outside it relative to the file's own directory.
fn collect_locations<T, F>(
    paths: &[String],
    exclude: &[String],
    invocation_path: &str,
    collect_file: F,
) -> Result<(Vec<T>, Vec<String>), String>
where
    T: Located + Send,
    F: Fn(&str, &str) -> Result<Vec<T>, String> + Copy + Sync,
{
    let mut all_locations = Vec::new();
    let mut failed_paths = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    let invocation = canonical(path::Path::new(invocation_path));

    for path_str in paths {
        let path_obj = path::Path::new(path_str);

        if path_obj.is_dir() {
            let mut files =
                match get_paths_to_process(path_str, exclude.to_vec(), &invocation, false) {
                    Ok(discovery) => discovery.files,
                    Err(e) => {
                        failed_paths.push(format!("{}: {}", path_str, e));
                        continue;
                    }
                };
            files.retain(|file_path| seen.insert(file_path.clone()));
            let base_dir = directory_location_base(path_obj, &invocation);
            let results: Vec<Result<Vec<T>, String>> = files
                .par_iter()
                .map(|file_path| collect_file(file_path, &base_dir))
                .collect();
            for locs in results.into_iter().flatten() {
                all_locations.extend(locs);
            }
        } else if path_obj.is_file() {
            let file_abs = canonical(path_obj);
            if !seen.insert(slashed(&file_abs)) {
                continue;
            }
            let collected = if file_abs.starts_with(&invocation) {
                collect_file(&slashed(&file_abs), &slashed(&invocation))
            } else {
                collect_file(path_str, file_location_base(path_obj))
            };
            if let Ok(locs) = collected {
                all_locations.extend(locs)
            }
        } else {
            failed_paths.push(path_str.to_string());
        }
    }

    all_locations.sort_by_key(Located::sort_key);
    Ok((all_locations, failed_paths))
}

fn collect_removable_ignores_from_file(
    file_path: &str,
    base_path: &str,
    max_complexity_allowed: u64,
) -> Result<Vec<RemovableIgnore>, String> {
    let relative_path = relative_to_base(file_path, base_path);
    let code = read_source(file_path)?;
    let locations = collect_ignored_locations(&code);
    if locations.is_empty() {
        return Ok(vec![]);
    }
    let parsed = parse_module(&code).map_err(|e| format!("Failed to parse code: {}", e))?;
    Ok(removable_ignores(
        &locations,
        parsed.suite(),
        &code,
        relative_path,
        max_complexity_allowed,
    ))
}
