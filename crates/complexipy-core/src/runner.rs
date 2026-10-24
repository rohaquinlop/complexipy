use crate::classes::{FileComplexity, IgnoredLocation, RemovableIgnore};
use crate::cognitive_complexity::function_level_cognitive_complexity_shared;
use crate::encoding::decode_python_source;
use crate::fs_guard::{MAX_SOURCE_BYTES, read_bounded};
use crate::helpers::exclude::get_paths_to_process;
use crate::rules::{AnalysisOptions, RuleSet};
use crate::utils::{collect_ignored_locations, filter_removable_ignores};
use rayon::prelude::*;
use ruff_python_ast::Suite;
use ruff_python_parser::parse_module;
use std::collections::HashSet;
use std::path;

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
/// file argument itself.
struct SourceRecord {
    path: String,
    resolved: bool,
    location_base: Option<String>,
    display: Option<String>,
    ignored: Vec<IgnoredLocation>,
    removable: Vec<RemovableIgnore>,
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
            invocation: path::Path::new(invocation_path)
                .canonicalize()
                .unwrap_or_else(|_| path::Path::new(invocation_path).to_path_buf()),
            analysis: opts.clone(),
            reports,
        };
        let arguments: Vec<ArgumentRecord> = paths
            .par_iter()
            .map(|path| analyze_argument(path, exclude, &context))
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
            skipped.extend(hidden_files(paths, exclude));
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
                source.location_base.clone(),
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

fn hidden_files(paths: &[String], exclude: &[String]) -> Vec<SkippedPath> {
    paths
        .iter()
        .filter(|path| path::Path::new(path).is_dir())
        .filter_map(|path| get_paths_to_process(path, exclude.to_vec(), true).ok())
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

fn analyze_argument(path: &str, exclude: &[String], context: &RunContext) -> ArgumentRecord {
    let path_obj = path::Path::new(path);
    if !path_obj.exists() {
        return ArgumentRecord {
            failure: Some(path.to_string()),
            sources: Vec::new(),
            skipped: Vec::new(),
        };
    }

    if !path_obj.is_dir() {
        let location_base = (context.reports.any() && path_obj.is_file())
            .then(|| file_location_base(path_obj).to_string());
        return ArgumentRecord {
            failure: None,
            sources: vec![analyze_source(
                path.to_string(),
                false,
                location_base,
                context,
            )],
            skipped: Vec::new(),
        };
    }

    let discovery = match get_paths_to_process(path, exclude.to_vec(), context.reports.ignored) {
        Ok(discovery) => discovery,
        Err(e) => {
            return ArgumentRecord {
                failure: Some(format!("{}: {}", path, e)),
                sources: Vec::new(),
                skipped: Vec::new(),
            };
        }
    };
    let location_base = context
        .reports
        .any()
        .then(|| directory_location_base(path_obj));
    ArgumentRecord {
        failure: None,
        sources: discovery
            .files
            .into_par_iter()
            .map(|file_path| analyze_source(file_path, true, location_base.clone(), context))
            .collect(),
        skipped: discovery.skipped,
    }
}

fn analyze_source(
    path: String,
    resolved: bool,
    location_base: Option<String>,
    context: &RunContext,
) -> (SourceRecord, Option<FileComplexity>) {
    let mut record = SourceRecord {
        path,
        resolved,
        location_base,
        display: None,
        ignored: Vec::new(),
        removable: Vec::new(),
    };
    let Ok(code) = read_source(&record.path) else {
        return (record, None);
    };
    let suite = parse_module(&code).ok().map(|parsed| parsed.into_suite());

    if let Some(base) = record.location_base.as_deref() {
        let relative = relative_to_base(&record.path, base).to_string();
        let locations = collect_ignored_locations(&code);
        if let (Some(max_complexity_allowed), Some(suite)) =
            (context.reports.removable_above, suite.as_ref())
            && !locations.is_empty()
        {
            record.removable =
                removable_ignores(&locations, suite, &code, &relative, max_complexity_allowed);
        }
        if context.reports.ignored {
            record.ignored = ignored_locations(locations, &relative);
        }
    }

    let complexity = suite.and_then(|suite| analyze_file(&record, &suite, &code, context));
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

fn directory_location_base(path_obj: &path::Path) -> String {
    path_obj
        .canonicalize()
        .unwrap_or_else(|_| path_obj.to_path_buf())
        .parent()
        .unwrap_or(path::Path::new("."))
        .to_string_lossy()
        .replace('\\', "/")
}

fn file_location_base(path_obj: &path::Path) -> &str {
    path_obj.parent().and_then(|p| p.to_str()).unwrap_or(".")
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
    _invocation_path: &str,
) -> Result<(Vec<IgnoredLocation>, Vec<String>), String> {
    collect_locations(paths, exclude, collect_file_ignored_locations)
}

pub fn collect_removable_ignored_locations_shared(
    paths: &[String],
    exclude: &[String],
    max_complexity_allowed: u64,
    _invocation_path: &str,
) -> Result<(Vec<RemovableIgnore>, Vec<String>), String> {
    collect_locations(paths, exclude, |file_path, base_dir| {
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

fn collect_locations<T, F>(
    paths: &[String],
    exclude: &[String],
    collect_file: F,
) -> Result<(Vec<T>, Vec<String>), String>
where
    T: Located + Send,
    F: Fn(&str, &str) -> Result<Vec<T>, String> + Copy + Sync,
{
    let mut all_locations = Vec::new();
    let mut failed_paths = Vec::new();

    for path_str in paths {
        let path_obj = path::Path::new(path_str);

        if path_obj.is_dir() {
            let files = match get_paths_to_process(path_str, exclude.to_vec(), false) {
                Ok(discovery) => discovery.files,
                Err(e) => {
                    failed_paths.push(format!("{}: {}", path_str, e));
                    continue;
                }
            };
            let base_dir = directory_location_base(path_obj);
            let results: Vec<Result<Vec<T>, String>> = files
                .par_iter()
                .map(|file_path| collect_file(file_path, &base_dir))
                .collect();
            for locs in results.into_iter().flatten() {
                all_locations.extend(locs);
            }
        } else if path_obj.is_file() {
            if let Ok(locs) = collect_file(path_str, file_location_base(path_obj)) {
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
        &parsed.into_suite(),
        &code,
        relative_path,
        max_complexity_allowed,
    ))
}
