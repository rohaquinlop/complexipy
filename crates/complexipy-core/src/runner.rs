use crate::classes::{FileComplexity, IgnoredLocation, RemovableIgnore};
use crate::cognitive_complexity::function_level_cognitive_complexity_shared;
use crate::helpers::exclude::get_paths_to_process;
use crate::rules::{AnalysisOptions, RuleSet};
use crate::syntax_tree::parse_module;
use crate::utils::{collect_ignored_locations, filter_removable_ignores};
use rayon::prelude::*;
use std::collections::HashSet;
use std::path::{self, PathBuf};

use crate::cognitive_complexity::code_complexity_shared;

struct ProcessOptions {
    exclude: Vec<String>,
    analysis: AnalysisOptions,
}

type ComplexitiesAndFailedPaths = (Vec<FileComplexity>, Vec<String>);

pub fn run_analysis_shared(
    paths: &[String],
    exclude: &[String],
    opts: &AnalysisOptions,
    invocation_path: &str,
) -> Result<ComplexitiesAndFailedPaths, String> {
    let mut successful = Vec::new();
    let mut failed_paths = Vec::new();
    let mut seen: HashSet<PathBuf> = HashSet::new();
    let inv_abs = canonical(invocation_path);
    let opts = ProcessOptions {
        exclude: exclude.to_vec(),
        analysis: opts.clone(),
    };

    for path in paths {
        let path_obj = path::Path::new(path);
        if !path_obj.exists() {
            failed_paths.push(path.to_string());
            continue;
        }

        let (mut complexities, mut f_paths) = if path_obj.is_dir() {
            evaluate_dir_shared(path, &opts, &inv_abs, &mut seen)
        } else if !seen.insert(canonical(path)) {
            continue;
        } else {
            match analyze_file_shared(path, &opts, &inv_abs) {
                Ok(file_complexity) => (vec![file_complexity], vec![]),
                Err(_) => (vec![], vec![path.to_string()]),
            }
        };
        complexities.iter_mut().for_each(|f| {
            f.functions
                .sort_by(|a, b| a.complexity.cmp(&b.complexity).then(a.name.cmp(&b.name)))
        });
        complexities.sort_by(|a, b| {
            a.path
                .cmp(&b.path)
                .then(a.file_name.cmp(&b.file_name))
                .then(a.complexity.cmp(&b.complexity))
        });
        successful.append(&mut complexities);
        failed_paths.append(&mut f_paths);
    }

    Ok((successful, failed_paths))
}

fn canonical(path: &str) -> PathBuf {
    path::Path::new(path)
        .canonicalize()
        .unwrap_or_else(|_| path::Path::new(path).to_path_buf())
}

fn evaluate_dir_shared(
    path: &str,
    opts: &ProcessOptions,
    invocation_path: &path::Path,
    seen: &mut HashSet<PathBuf>,
) -> ComplexitiesAndFailedPaths {
    let mut files_paths_to_process =
        match get_paths_to_process(path, opts.exclude.clone(), invocation_path) {
            Ok(paths) => paths,
            Err(e) => return (vec![], vec![format!("{}: {}", path, e)]),
        };
    files_paths_to_process.retain(|file_path| seen.insert(PathBuf::from(file_path)));

    let results: Vec<Result<FileComplexity, String>> = files_paths_to_process
        .par_iter()
        .map(|file_path| analyze_file_shared(file_path, opts, invocation_path))
        .collect();

    let mut complexities = Vec::new();
    let mut failed_paths = Vec::new();
    for (file_path, result) in files_paths_to_process.into_iter().zip(results) {
        match result {
            Ok(file_complexity) => complexities.push(file_complexity),
            Err(_) => failed_paths.push(file_path),
        }
    }
    (complexities, failed_paths)
}

fn analyze_file_shared(
    path: &str,
    opts: &ProcessOptions,
    invocation_path: &path::Path,
) -> Result<FileComplexity, String> {
    let inv_str = invocation_path.to_string_lossy().replace('\\', "/");
    let file_abs = path::Path::new(path)
        .canonicalize()
        .unwrap_or_else(|_| path::Path::new(path).to_path_buf());
    let rel = file_abs
        .strip_prefix(invocation_path)
        .ok()
        .and_then(|p| p.to_str())
        .unwrap_or(path);
    let mut complexity = file_complexity_shared(path, &inv_str, &opts.analysis)?;
    if let Some(file_name) = path::Path::new(rel)
        .file_name()
        .and_then(|name| name.to_str())
    {
        complexity.file_name = file_name.to_string();
    }
    complexity.path = rel.to_string();
    Ok(complexity)
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
    let code = std::fs::read_to_string(file_path)
        .map_err(|e| format!("Failed to read file '{}': {}", file_path, e))?;
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
    let path = path::Path::new(file_path);
    let relative_path = path
        .strip_prefix(base_path)
        .ok()
        .and_then(|p| p.to_str())
        .unwrap_or(file_path);
    let code = std::fs::read_to_string(file_path)
        .map_err(|e| format!("Failed to read file '{}': {}", file_path, e))?;
    let locations = collect_ignored_locations(&code);
    Ok(locations
        .into_iter()
        .map(|(line, comment)| IgnoredLocation {
            path: relative_path.to_string(),
            line,
            comment,
        })
        .collect())
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
    let mut seen: HashSet<PathBuf> = HashSet::new();
    let inv_abs = canonical(invocation_path);
    let inv_str = inv_abs.to_string_lossy().replace('\\', "/");

    for path_str in paths {
        let path_obj = path::Path::new(path_str);

        if path_obj.is_dir() {
            let mut files = match get_paths_to_process(path_str, exclude.to_vec(), &inv_abs) {
                Ok(paths) => paths,
                Err(e) => {
                    failed_paths.push(format!("{}: {}", path_str, e));
                    continue;
                }
            };
            files.retain(|file_path| seen.insert(PathBuf::from(file_path)));
            let root = canonical(path_str);
            let base_dir = if root.starts_with(&inv_abs) {
                inv_str.clone()
            } else {
                root.parent()
                    .unwrap_or(path::Path::new("."))
                    .to_string_lossy()
                    .replace('\\', "/")
            };
            let results: Vec<Result<Vec<T>, String>> = files
                .par_iter()
                .map(|file_path| collect_file(file_path, &base_dir))
                .collect();
            for locs in results.into_iter().flatten() {
                all_locations.extend(locs);
            }
        } else if path_obj.is_file() {
            let file_abs = canonical(path_str);
            if !seen.insert(file_abs.clone()) {
                continue;
            }
            let collected = if file_abs.starts_with(&inv_abs) {
                collect_file(&file_abs.to_string_lossy().replace('\\', "/"), &inv_str)
            } else {
                let parent_dir = path_obj.parent().and_then(|p| p.to_str()).unwrap_or(".");
                collect_file(path_str, parent_dir)
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
    let path = path::Path::new(file_path);
    let relative_path = path
        .strip_prefix(base_path)
        .ok()
        .and_then(|p| p.to_str())
        .unwrap_or(file_path);
    let code = std::fs::read_to_string(file_path)
        .map_err(|e| format!("Failed to read file '{}': {}", file_path, e))?;
    let locations = collect_ignored_locations(&code);
    if locations.is_empty() {
        return Ok(vec![]);
    }
    let parsed = parse_module(&code).map_err(|e| format!("Failed to parse code: {}", e))?;
    let (functions, _) = function_level_cognitive_complexity_shared(
        parsed.suite(),
        &code,
        &AnalysisOptions {
            no_ignore: true,
            rules: RuleSet::default(),
            ..Default::default()
        },
    );
    let removable = filter_removable_ignores(&locations, &functions, max_complexity_allowed);
    Ok(removable
        .into_iter()
        .map(|(line, comment, function, complexity)| RemovableIgnore {
            path: relative_path.to_string(),
            line,
            comment,
            function,
            complexity,
        })
        .collect())
}
