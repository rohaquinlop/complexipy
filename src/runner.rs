use crate::classes::{FileComplexity, IgnoredLocation};
use crate::cognitive_complexity::code_complexity;
use crate::helpers::exclude::get_paths_to_process;
use crate::utils::{collect_ignored_locations, get_repo_name};
use indicatif::ProgressBar;
use indicatif::ProgressStyle;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use regex::Regex;
use std::env;
use std::path;
use std::process;
use std::sync::{Arc, Mutex};
use std::thread;
use tempfile::tempdir;

struct ProcessOptions {
    quiet: bool,
    exclude: Vec<String>,
    check_script: bool,
    no_ignore: bool,
}

type ComplexitiesAndFailedPaths = (Vec<FileComplexity>, Vec<String>);

#[pyfunction]
#[pyo3(signature = (paths, quiet, exclude, check_script=false, no_ignore=false, invocation_path="."))]
pub fn main(
    paths: Vec<String>,
    quiet: bool,
    exclude: Vec<String>,
    check_script: bool,
    no_ignore: bool,
    invocation_path: &str,
) -> PyResult<ComplexitiesAndFailedPaths> {
    let _ = invocation_path;

    let re = Regex::new(r"^(https:\/\/|http:\/\/|www\.|git@)(github|gitlab)\.com(\/[\w.-]+){2,}$")
        .map_err(|e| PyValueError::new_err(format!("Invalid repository pattern: {}", e)))?;

    let mut successful = Vec::new();
    let mut failed_paths = Vec::new();

    for path in paths {
        let is_url = re.is_match(&path);
        let path_obj = path::Path::new(&path);
        let exists = path_obj.exists();
        let is_dir = path_obj.is_dir();

        if !exists && !is_url {
            failed_paths.push(path.to_string());
            continue;
        }

        let opts = ProcessOptions {
            quiet,
            exclude: exclude.clone(),
            check_script,
            no_ignore,
        };

        match process_path(&path, is_dir, is_url, &opts, invocation_path) {
            Ok((mut complexities, mut f_paths)) => {
                successful.append(&mut complexities);
                failed_paths.append(&mut f_paths);
            }
            Err(_) => failed_paths.push(path.to_string()),
        }
    }

    Ok((successful, failed_paths))
}

fn process_path(
    path: &str,
    is_dir: bool,
    is_url: bool,
    opts: &ProcessOptions,
    invocation_path: &str,
) -> Result<ComplexitiesAndFailedPaths, PyErr> {
    let mut file_complexities = Vec::new();
    let mut failed_paths = Vec::new();

    if is_url {
        let dir = tempdir()?;
        let repo_name = get_repo_name(path)?;
        env::set_current_dir(&dir)?;
        let cloning_done = Arc::new(Mutex::new(false));
        let cloning_done_clone = Arc::clone(&cloning_done);
        let path_clone = path.to_owned();

        thread::spawn(move || {
            let _ = process::Command::new("git")
                .args(["clone", &path_clone])
                .output();
            if let Ok(mut done) = cloning_done_clone.lock() {
                *done = true;
            }
        });

        let mut progress_bar = if opts.quiet {
            None
        } else {
            let pb = ProgressBar::new_spinner();
            pb.set_style(ProgressStyle::default_spinner());
            pb.set_message("Cloning repository...");
            Some(pb)
        };

        loop {
            match cloning_done.lock() {
                Ok(done) if *done => break,
                Ok(_) => {
                    if let Some(pb) = progress_bar.as_ref() {
                        pb.tick();
                    }
                    thread::sleep(std::time::Duration::from_millis(100));
                }
                Err(_) => {
                    if let Some(pb) = progress_bar.take() {
                        pb.finish_and_clear();
                    }
                    return Err(PyValueError::new_err("Failed to track cloning progress"));
                }
            }
        }

        if let Some(pb) = progress_bar {
            pb.finish_and_clear();
        }

        let repo_path = dir.path().join(&repo_name).to_string_lossy().to_string();
        let (complexities, f_paths) = evaluate_dir(&repo_path, opts, invocation_path);
        dir.close()?;
        file_complexities = complexities;
        failed_paths = f_paths;
    } else if is_dir {
        let (complexities, f_paths) = evaluate_dir(path, opts, invocation_path);
        file_complexities = complexities;
        failed_paths = f_paths;
    } else {
        let inv_abs = path::Path::new(invocation_path)
            .canonicalize()
            .unwrap_or_else(|_| path::Path::new(invocation_path).to_path_buf());
        let inv_str = inv_abs.to_string_lossy().replace('\\', "/");
        let file_abs = path::Path::new(path)
            .canonicalize()
            .unwrap_or_else(|_| path::Path::new(path).to_path_buf());
        let rel = file_abs
            .strip_prefix(&inv_abs)
            .ok()
            .and_then(|p| p.to_str())
            .unwrap_or(path);
        if let Ok(complexity) = file_complexity(path, &inv_str, opts.check_script, opts.no_ignore) {
            let mut complexity = complexity;
            complexity.path = rel.to_string();
            file_complexities.push(complexity);
        } else {
            failed_paths.push(path.to_string());
        }
    }

    file_complexities
        .iter_mut()
        .for_each(|f| f.functions.sort_by_key(|f| (f.complexity, f.name.clone())));
    file_complexities.sort_by_key(|f| (f.path.clone(), f.file_name.clone(), f.complexity));
    Ok((file_complexities, failed_paths))
}

fn evaluate_dir(
    path: &str,
    opts: &ProcessOptions,
    invocation_path: &str,
) -> ComplexitiesAndFailedPaths {
    let inv_abs = path::Path::new(invocation_path)
        .canonicalize()
        .unwrap_or_else(|_| path::Path::new(invocation_path).to_path_buf());
    let base_dir = inv_abs.to_string_lossy().replace('\\', "/");
    let files_paths_to_process = match get_paths_to_process(path, opts.exclude.clone()) {
        Ok(paths) => paths,
        Err(e) => return (vec![], vec![format!("{}: {}", path, e)]),
    };

    if opts.quiet {
        let results: Vec<_> = files_paths_to_process
            .iter()
            .map(|file_path| {
                match file_complexity(file_path, &base_dir, opts.check_script, opts.no_ignore) {
                    Ok(file_complexity) => (Some(file_complexity), None),
                    Err(_) => (None, Some(file_path.clone())),
                }
            })
            .collect();
        let mut complexities = Vec::new();
        let mut failed_paths = Vec::new();
        for (success, error_path) in results {
            match (success, error_path) {
                (Some(file_complexity), None) => complexities.push(file_complexity),
                (None, Some(failed_path)) => failed_paths.push(failed_path),
                _ => unreachable!(),
            }
        }
        return (complexities, failed_paths);
    }

    let pb = ProgressBar::new(files_paths_to_process.len() as u64);
    let bar_style = indicatif::ProgressStyle::default_bar()
        .template("{spiner:.green} [{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}")
        .unwrap_or_else(|_| indicatif::ProgressStyle::default_bar())
        .progress_chars("##-");
    pb.set_style(bar_style);

    let results: Vec<_> = files_paths_to_process
        .iter()
        .map(|file_path| {
            pb.inc(1);
            match file_complexity(file_path, &base_dir, opts.check_script, opts.no_ignore) {
                Ok(file_complexity) => (Some(file_complexity), None),
                Err(_) => (None, Some(file_path.clone())),
            }
        })
        .collect();

    let mut complexities = Vec::new();
    let mut failed_paths = Vec::new();
    for (success, error_path) in results {
        match (success, error_path) {
            (Some(file_complexity), None) => complexities.push(file_complexity),
            (None, Some(failed_path)) => failed_paths.push(failed_path),
            _ => unreachable!(),
        }
    }
    pb.finish_and_clear();
    (complexities, failed_paths)
}

#[pyfunction]
#[pyo3(signature = (file_path, base_path, check_script=false, no_ignore=false))]
pub fn file_complexity(
    file_path: &str,
    base_path: &str,
    check_script: bool,
    no_ignore: bool,
) -> PyResult<FileComplexity> {
    let path = path::Path::new(file_path);
    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| PyValueError::new_err(format!("Invalid file name: {}", file_path)))?;
    let relative_path = path
        .strip_prefix(base_path)
        .ok()
        .and_then(|p| p.to_str())
        .unwrap_or(file_path);
    let code = std::fs::read_to_string(file_path)?;
    let code_complexity = match code_complexity(&code, check_script, no_ignore) {
        Ok(v) => v,
        Err(e) => {
            return Err(PyValueError::new_err(format!(
                "Failed to process file '{}': {}",
                file_path, e
            )));
        }
    };
    Ok(FileComplexity {
        path: relative_path.to_string(),
        file_name: file_name.to_string(),
        complexity: code_complexity.complexity,
        functions: code_complexity.functions,
    })
}

#[pyfunction]
#[pyo3(signature = (file_path, base_path))]
pub fn collect_file_ignored_locations(
    file_path: &str,
    base_path: &str,
) -> PyResult<Vec<IgnoredLocation>> {
    let path = path::Path::new(file_path);
    let relative_path = path
        .strip_prefix(base_path)
        .ok()
        .and_then(|p| p.to_str())
        .unwrap_or(file_path);
    let code = std::fs::read_to_string(file_path).map_err(|e| {
        PyValueError::new_err(format!("Failed to read file '{}': {}", file_path, e))
    })?;
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

#[pyfunction]
#[pyo3(signature = (paths, exclude, invocation_path="."))]
pub fn collect_all_ignored_locations(
    paths: Vec<String>,
    exclude: Vec<String>,
    invocation_path: &str,
) -> PyResult<(Vec<IgnoredLocation>, Vec<String>)> {
    let _invocation_dir = path::Path::new(invocation_path)
        .canonicalize()
        .unwrap_or_else(|_| path::Path::new(invocation_path).to_path_buf());

    let mut all_locations = Vec::new();
    let mut failed_paths = Vec::new();

    let re = Regex::new(r"^(https:\/\/|http:\/\/|www\.|git@)(github|gitlab)\.com(\/[\w.-]+){2,}$")
        .map_err(|e| PyValueError::new_err(format!("Invalid repository pattern: {}", e)))?;

    for path_str in &paths {
        let path_obj = path::Path::new(path_str);
        let is_url = re.is_match(path_str);

        if is_url {
            match collect_ignored_locations_from_url(path_str, &exclude) {
                Ok(locs) => all_locations.extend(locs),
                Err(_) => failed_paths.push(path_str.to_string()),
            }
        } else if path_obj.is_dir() {
            let files = match get_paths_to_process(path_str, exclude.clone()) {
                Ok(paths) => paths,
                Err(e) => {
                    failed_paths.push(format!("{}: {}", path_str, e));
                    continue;
                }
            };
            let base_dir = path_obj
                .canonicalize()
                .unwrap_or_else(|_| path_obj.to_path_buf())
                .parent()
                .unwrap_or(path::Path::new("."))
                .to_string_lossy()
                .replace('\\', "/");
            for file_path in &files {
                if let Ok(locs) = collect_file_ignored_locations(file_path, &base_dir) {
                    all_locations.extend(locs)
                }
            }
        } else if path_obj.is_file() {
            let parent_dir = path_obj.parent().and_then(|p| p.to_str()).unwrap_or(".");
            if let Ok(locs) = collect_file_ignored_locations(path_str, parent_dir) {
                all_locations.extend(locs)
            }
        } else {
            failed_paths.push(path_str.to_string());
        }
    }

    all_locations.sort_by_key(|loc| (loc.path.clone(), loc.line));
    Ok((all_locations, failed_paths))
}

fn collect_ignored_locations_from_url(
    url: &str,
    exclude: &[String],
) -> PyResult<Vec<IgnoredLocation>> {
    let dir = tempdir()?;
    let repo_name = get_repo_name(url)?;
    env::set_current_dir(&dir)?;
    let cloning_done = Arc::new(Mutex::new(false));
    let cloning_done_clone = Arc::clone(&cloning_done);
    let path_clone = url.to_owned();

    thread::spawn(move || {
        let _ = process::Command::new("git")
            .args(["clone", &path_clone])
            .output();
        if let Ok(mut done) = cloning_done_clone.lock() {
            *done = true;
        }
    });

    loop {
        match cloning_done.lock() {
            Ok(done) if *done => break,
            Ok(_) => {
                thread::sleep(std::time::Duration::from_millis(100));
            }
            Err(_) => {
                return Err(PyValueError::new_err("Failed to track cloning progress"));
            }
        }
    }

    let repo_path = dir.path().join(&repo_name).to_string_lossy().to_string();
    let files =
        get_paths_to_process(&repo_path, exclude.to_vec()).map_err(PyValueError::new_err)?;
    let base_dir = path::Path::new(&repo_path)
        .canonicalize()
        .unwrap_or_else(|_| path::Path::new(&repo_path).to_path_buf())
        .parent()
        .unwrap_or(path::Path::new("."))
        .to_string_lossy()
        .replace('\\', "/");

    let mut locations = Vec::new();
    for file_path in &files {
        if let Ok(locs) = collect_file_ignored_locations(file_path, &base_dir) {
            locations.extend(locs);
        }
    }

    dir.close()?;
    Ok(locations)
}
