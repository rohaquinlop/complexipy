#[cfg(any(feature = "python", feature = "wasm"))]
mod shared_deps {
    pub use crate::classes::{FunctionComplexity, LineComplexity};
    pub use crate::refactor_plans::{
        ComplexityRegion, ComplexityResult, RegionKind, build_refactor_plans,
    };
    pub use crate::utils::{count_bool_ops, get_line_number, has_noqa_complexipy, is_decorator};
    pub use ruff_python_ast::{self as ast, Stmt};
}

#[cfg(feature = "python")]
use crate::classes::{CodeComplexity, FileComplexity};

#[cfg(any(feature = "python", feature = "wasm"))]
use shared_deps::*;

#[cfg(feature = "python")]
mod python_deps {
    pub use crate::utils::get_repo_name;
    pub use globset::{Glob, GlobMatcher};
    pub use ignore::Walk;
    pub use indicatif::ProgressBar;
    pub use indicatif::ProgressStyle;
    pub use pyo3::exceptions::PyValueError;
    pub use pyo3::prelude::*;
    pub use regex::Regex;
    pub use ruff_python_parser::parse_module;
    pub use std::env;
    pub use std::path;
    pub use std::process;
    pub use std::sync::{Arc, Mutex};
    pub use std::thread;
    pub use tempfile::tempdir;
}

#[cfg(feature = "python")]
use python_deps::*;

#[cfg(feature = "python")]
type ComplexitiesAndFailedPaths = (Vec<FileComplexity>, Vec<String>);

#[cfg(feature = "python")]
#[pyfunction]
#[pyo3(signature = (paths, quiet, exclude, check_script=false))]
pub fn main(
    paths: Vec<&str>,
    quiet: bool,
    exclude: Vec<&str>,
    check_script: bool,
) -> PyResult<ComplexitiesAndFailedPaths> {
    let re = Regex::new(r"^(https:\/\/|http:\/\/|www\.|git@)(github|gitlab)\.com(\/[\w.-]+){2,}$")
        .map_err(|e| PyValueError::new_err(format!("Invalid repository pattern: {}", e)))?;

    let mut successful = Vec::new();
    let mut failed_paths = Vec::new();

    for path in paths {
        let is_url = re.is_match(path);
        let path_obj = path::Path::new(path);
        let exists = path_obj.exists();
        let is_dir = path_obj.is_dir();

        if !exists && !is_url {
            failed_paths.push(path.to_string());
            continue;
        }

        match process_path(path, is_dir, is_url, quiet, exclude.clone(), check_script) {
            Ok((mut complexities, mut f_paths)) => {
                successful.append(&mut complexities);
                failed_paths.append(&mut f_paths);
            }
            Err(_) => failed_paths.push(path.to_string()),
        }
    }

    Ok((successful, failed_paths))
}

#[cfg(feature = "python")]
#[pyfunction]
pub fn process_path(
    path: &str,
    is_dir: bool,
    is_url: bool,
    quiet: bool,
    exclude: Vec<&str>,
    check_script: bool,
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

        let mut progress_bar = if quiet {
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
        let (complexities, f_paths) =
            evaluate_dir(&repo_path, quiet, exclude.clone(), check_script);
        dir.close()?;
        file_complexities = complexities;
        failed_paths = f_paths;
    } else if is_dir {
        let (complexities, f_paths) = evaluate_dir(path, quiet, exclude.clone(), check_script);
        file_complexities = complexities;
        failed_paths = f_paths;
    } else {
        let parent_dir = path::Path::new(path)
            .parent()
            .and_then(|p| p.to_str())
            .unwrap_or(".");
        if let Ok(complexity) = file_complexity(path, parent_dir, check_script) {
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

#[cfg(feature = "python")]
fn evaluate_dir(
    path: &str,
    quiet: bool,
    exclude: Vec<&str>,
    check_script: bool,
) -> ComplexitiesAndFailedPaths {
    let mut files_paths: Vec<String> = Vec::new();
    let parent_dir = path::Path::new(path)
        .parent()
        .and_then(|p| p.to_str())
        .unwrap_or(".");

    #[derive(Clone)]
    struct ExcludeSpec {
        abs: String,
        is_dir: bool,
    }

    let root_canon = match path::Path::new(path).canonicalize() {
        Ok(p) => p,
        Err(_) => path::PathBuf::from(path),
    };
    let root_canon_str = root_canon.to_string_lossy().replace('\\', "/");
    let mut exclude_specs: Vec<ExcludeSpec> = Vec::new();
    let mut glob_matchers: Vec<GlobMatcher> = Vec::new();

    for raw in exclude.iter() {
        if raw.contains('*') || raw.contains('?') || raw.contains('[') {
            if let Ok(glob) = Glob::new(raw) {
                glob_matchers.push(glob.compile_matcher());
            }
            continue;
        }

        let p = path::Path::new(raw);
        let candidate = if p.is_absolute() {
            p.to_path_buf()
        } else {
            root_canon.join(p)
        };
        let (exists, is_dir, is_file, abs_str) = match candidate.canonicalize() {
            Ok(canon) => {
                let is_dir = canon.is_dir();
                let is_file = canon.is_file();
                let s = canon.to_string_lossy().replace('\\', "/");
                (true, is_dir, is_file, s)
            }
            Err(_) => (false, false, false, String::new()),
        };

        if exists && (is_dir || is_file) {
            exclude_specs.push(ExcludeSpec {
                abs: abs_str,
                is_dir,
            });
        }
    }

    for entry in Walk::new(path) {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        let entry_path = entry.path();
        if entry_path.extension().and_then(|s| s.to_str()) != Some("py") {
            continue;
        }
        let file_abs = match entry_path.canonicalize() {
            Ok(p) => p,
            Err(_) => entry_path.to_path_buf(),
        };
        let file_abs_str = file_abs.to_string_lossy().replace('\\', "/");
        let relative_path = file_abs_str
            .strip_prefix(&format!("{}/", root_canon_str))
            .unwrap_or(&file_abs_str);
        let is_excluded = exclude_specs.iter().any(|spec| {
            if spec.is_dir {
                file_abs_str.starts_with(&spec.abs)
            } else {
                file_abs_str == spec.abs
            }
        }) || glob_matchers
            .iter()
            .any(|m| m.is_match(relative_path) || m.is_match(&file_abs_str));
        if !is_excluded && let Some(file_path_str) = entry_path.to_str() {
            files_paths.push(file_path_str.to_string());
        }
    }

    if quiet {
        let results: Vec<_> = files_paths
            .iter()
            .map(
                |file_path| match file_complexity(file_path, parent_dir, check_script) {
                    Ok(file_complexity) => (Some(file_complexity), None),
                    Err(_) => (None, Some(file_path.clone())),
                },
            )
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

    let pb = ProgressBar::new(files_paths.len() as u64);
    let bar_style = indicatif::ProgressStyle::default_bar()
        .template("{spiner:.green} [{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}")
        .unwrap_or_else(|_| indicatif::ProgressStyle::default_bar())
        .progress_chars("##-");
    pb.set_style(bar_style);

    let results: Vec<_> = files_paths
        .iter()
        .map(|file_path| {
            pb.inc(1);
            match file_complexity(file_path, parent_dir, check_script) {
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

#[cfg(feature = "python")]
#[pyfunction]
#[pyo3(signature = (file_path, base_path, check_script=false))]
pub fn file_complexity(
    file_path: &str,
    base_path: &str,
    check_script: bool,
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
    let code_complexity = match code_complexity(&code, check_script) {
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

#[cfg(feature = "python")]
#[pyfunction]
#[pyo3(signature = (code, check_script=false))]
pub fn code_complexity(code: &str, check_script: bool) -> PyResult<CodeComplexity> {
    let parsed = match parse_module(code) {
        Ok(parsed) => parsed,
        Err(e) => {
            return Err(PyValueError::new_err(format!(
                "Failed to parse code: {}",
                e
            )));
        }
    };
    let ast_body = parsed.into_suite();
    let (functions, complexity) =
        function_level_cognitive_complexity_shared(&ast_body, code, check_script);
    Ok(CodeComplexity {
        functions,
        complexity,
        #[cfg(feature = "wasm")]
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

#[cfg(any(feature = "python", feature = "wasm"))]
pub fn function_level_cognitive_complexity_shared(
    ast_body: &ast::Suite,
    code: &str,
    check_script: bool,
) -> (Vec<FunctionComplexity>, u64) {
    let mut functions: Vec<FunctionComplexity> = Vec::new();
    let mut complexity: u64 = 0;
    let mut module_complexity: u64 = 0;
    let mut module_line_complexities: Vec<LineComplexity> = Vec::new();
    let mut module_regions: Vec<ComplexityRegion> = Vec::new();

    for node in ast_body.iter() {
        match node {
            Stmt::FunctionDef(f) => {
                let start_line = get_line_number(usize::from(f.range.start()), code);
                if !has_noqa_complexipy(start_line, code) {
                    let result = statement_cognitive_complexity_shared(node, 0, code);
                    functions.push(FunctionComplexity {
                        name: f.name.to_string(),
                        complexity: result.complexity,
                        line_start: start_line,
                        line_end: get_line_number(usize::from(f.range.end()), code),
                        line_complexities: result.line_complexities,
                        refactor_plans: build_refactor_plans(result.complexity, &result.regions),
                    });
                }
            }
            Stmt::ClassDef(c) => {
                for node in c.body.iter() {
                    if let Stmt::FunctionDef(f) = node {
                        let start_line = get_line_number(usize::from(f.range.start()), code);
                        if !has_noqa_complexipy(start_line, code) {
                            let result = statement_cognitive_complexity_shared(node, 0, code);
                            functions.push(FunctionComplexity {
                                name: format!("{}::{}", c.name, f.name),
                                complexity: result.complexity,
                                line_start: start_line,
                                line_end: get_line_number(usize::from(f.range.end()), code),
                                line_complexities: result.line_complexities,
                                refactor_plans: build_refactor_plans(
                                    result.complexity,
                                    &result.regions,
                                ),
                            });
                        }
                    }
                }
            }
            _ => {
                let result = statement_cognitive_complexity_shared(node, 0, code);
                if check_script {
                    module_complexity += result.complexity;
                    module_line_complexities.extend(result.line_complexities);
                    module_regions.extend(result.regions);
                } else {
                    complexity += result.complexity;
                }
            }
        }
    }

    if check_script {
        let total_lines = code.lines().count() as u64;
        functions.push(FunctionComplexity {
            name: "<module>".to_string(),
            complexity: module_complexity,
            line_start: 1,
            line_end: total_lines,
            line_complexities: module_line_complexities,
            refactor_plans: build_refactor_plans(module_complexity, &module_regions),
        });
    }

    for function in functions.iter() {
        complexity += function.complexity;
    }
    (functions, complexity)
}

#[cfg(any(feature = "python", feature = "wasm"))]
fn empty_result() -> ComplexityResult {
    ComplexityResult {
        complexity: 0,
        line_complexities: Vec::new(),
        regions: Vec::new(),
    }
}

#[cfg(any(feature = "python", feature = "wasm"))]
fn merge_child(
    result: &mut ComplexityResult,
    child: ComplexityResult,
    region_children: &mut Vec<ComplexityRegion>,
) {
    result.complexity += child.complexity;
    result.line_complexities.extend(child.line_complexities);
    region_children.extend(child.regions.clone());
    result.regions.extend(child.regions);
}

#[cfg(any(feature = "python", feature = "wasm"))]
fn collect_suite(
    suite: &ast::Suite,
    nesting_level: u64,
    code: &str,
    region_children: &mut Vec<ComplexityRegion>,
) -> ComplexityResult {
    let mut result = empty_result();
    for node in suite.iter() {
        let child = statement_cognitive_complexity_shared(node, nesting_level, code);
        merge_child(&mut result, child, region_children);
    }
    result
}

#[cfg(any(feature = "python", feature = "wasm"))]
fn sum_region_child_totals(children: &[ComplexityRegion]) -> u64 {
    children
        .iter()
        .filter(|child| child.kind != RegionKind::BooleanCondition)
        .map(|child| child.total)
        .sum()
}

#[cfg(any(feature = "python", feature = "wasm"))]
fn push_bool_region(
    regions: &mut Vec<ComplexityRegion>,
    line_start: u64,
    line_end: u64,
    boolean: u64,
) {
    if boolean >= 2 {
        regions.push(ComplexityRegion {
            kind: RegionKind::BooleanCondition,
            line_start,
            line_end,
            structural: 0,
            nesting: 0,
            boolean,
            total: boolean,
            elif_count: 0,
            case_count: 0,
            bool_op_count: boolean,
            children: Vec::new(),
        });
    }
}

#[cfg(any(feature = "python", feature = "wasm"))]
fn statement_cognitive_complexity_shared(
    statement: &Stmt,
    nesting_level: u64,
    code: &str,
) -> ComplexityResult {
    let mut result = empty_result();

    if is_decorator(statement.clone())
        && let Stmt::FunctionDef(f) = statement
    {
        return statement_cognitive_complexity_shared(&f.body[0], nesting_level, code);
    }

    match statement {
        Stmt::FunctionDef(f) => {
            for node in f.body.iter() {
                let next_nesting = if matches!(node, Stmt::FunctionDef(..)) {
                    nesting_level + 1
                } else {
                    nesting_level
                };
                let child = statement_cognitive_complexity_shared(node, next_nesting, code);
                result.complexity += child.complexity;
                result.line_complexities.extend(child.line_complexities);
                result.regions.extend(child.regions);
            }
        }
        Stmt::ClassDef(c) => {
            for node in c.body.iter() {
                if let Stmt::FunctionDef(..) = node {
                    let child = statement_cognitive_complexity_shared(node, nesting_level, code);
                    result.complexity += child.complexity;
                    result.line_complexities.extend(child.line_complexities);
                    result.regions.extend(child.regions);
                }
            }
        }
        Stmt::Assign(a) => {
            let bool_ops_complexity = count_bool_ops(*a.value.clone(), nesting_level);
            result.complexity += bool_ops_complexity;
            let line = get_line_number(usize::from(a.range.start()), code);
            result.line_complexities.push(LineComplexity {
                line,
                complexity: bool_ops_complexity,
            });
        }
        Stmt::AnnAssign(a) => {
            if let Some(value) = a.value.clone() {
                let bool_ops_complexity = count_bool_ops(*value, nesting_level);
                result.complexity += bool_ops_complexity;
                let line = get_line_number(usize::from(a.range.start()), code);
                result.line_complexities.push(LineComplexity {
                    line,
                    complexity: bool_ops_complexity,
                });
            }
        }
        Stmt::AugAssign(a) => {
            let bool_ops_complexity = count_bool_ops(*a.value.clone(), nesting_level);
            result.complexity += bool_ops_complexity;
            let line = get_line_number(usize::from(a.range.start()), code);
            result.line_complexities.push(LineComplexity {
                line,
                complexity: bool_ops_complexity,
            });
        }
        Stmt::For(f) => {
            let structural = 1;
            let nesting = nesting_level;
            let own = structural + nesting;
            result.complexity += own;
            let line_start = get_line_number(usize::from(f.range.start()), code);
            let line_end = get_line_number(usize::from(f.range.end()), code);
            result.line_complexities.push(LineComplexity {
                line: line_start,
                complexity: own,
            });
            let mut children = Vec::new();
            let child_result = collect_suite(&f.body, nesting_level + 1, code, &mut children);
            result.complexity += child_result.complexity;
            result
                .line_complexities
                .extend(child_result.line_complexities);
            let else_result = collect_suite(&f.orelse, nesting_level + 1, code, &mut children);
            result.complexity += else_result.complexity;
            result
                .line_complexities
                .extend(else_result.line_complexities);
            let mut region = ComplexityRegion {
                kind: RegionKind::Loop,
                line_start,
                line_end,
                structural,
                nesting,
                boolean: 0,
                total: own,
                elif_count: 0,
                case_count: 0,
                bool_op_count: 0,
                children,
            };
            region.total += sum_region_child_totals(&region.children);
            result.regions.push(region);
        }
        Stmt::While(w) => {
            let structural = 1;
            let nesting = nesting_level;
            let boolean = count_bool_ops(*w.test.clone(), nesting_level);
            let own = structural + nesting + boolean;
            result.complexity += own;
            let line_start = get_line_number(usize::from(w.range.start()), code);
            let line_end = get_line_number(usize::from(w.range.end()), code);
            result.line_complexities.push(LineComplexity {
                line: line_start,
                complexity: own,
            });
            let mut children = Vec::new();
            push_bool_region(&mut children, line_start, line_start, boolean);
            let child_result = collect_suite(&w.body, nesting_level + 1, code, &mut children);
            result.complexity += child_result.complexity;
            result
                .line_complexities
                .extend(child_result.line_complexities);
            let else_result = collect_suite(&w.orelse, nesting_level + 1, code, &mut children);
            result.complexity += else_result.complexity;
            result
                .line_complexities
                .extend(else_result.line_complexities);
            let mut region = ComplexityRegion {
                kind: RegionKind::Loop,
                line_start,
                line_end,
                structural,
                nesting,
                boolean,
                total: own,
                elif_count: 0,
                case_count: 0,
                bool_op_count: boolean,
                children,
            };
            region.total += sum_region_child_totals(&region.children);
            result.regions.push(region);
        }
        Stmt::If(i) => {
            let structural = 1;
            let nesting = nesting_level;
            let boolean = count_bool_ops(*i.test.clone(), nesting_level);
            let own = structural + nesting + boolean;
            result.complexity += own;
            let line_start = get_line_number(usize::from(i.range.start()), code);
            let line_end = get_line_number(usize::from(i.range.end()), code);
            result.line_complexities.push(LineComplexity {
                line: line_start,
                complexity: own,
            });
            let mut children = Vec::new();
            push_bool_region(&mut children, line_start, line_start, boolean);
            let body_result = collect_suite(&i.body, nesting_level + 1, code, &mut children);
            result.complexity += body_result.complexity;
            result
                .line_complexities
                .extend(body_result.line_complexities);
            let mut elif_count = 0;
            for clause in i.elif_else_clauses.clone() {
                let mut clause_complexity = 1;
                if let Some(test) = clause.test.clone() {
                    elif_count += 1;
                    let clause_bool = count_bool_ops(test, nesting_level);
                    clause_complexity += clause_bool;
                    let line = get_line_number(usize::from(clause.range.start()), code);
                    push_bool_region(&mut children, line, line, clause_bool);
                }
                result.complexity += clause_complexity;
                let line = get_line_number(usize::from(clause.range.start()), code);
                result.line_complexities.push(LineComplexity {
                    line,
                    complexity: clause_complexity,
                });
                let clause_result =
                    collect_suite(&clause.body, nesting_level + 1, code, &mut children);
                result.complexity += clause_result.complexity;
                result
                    .line_complexities
                    .extend(clause_result.line_complexities);
            }
            let kind = if elif_count > 0 {
                RegionKind::ElifChain
            } else {
                RegionKind::If
            };
            let mut region = ComplexityRegion {
                kind,
                line_start,
                line_end,
                structural,
                nesting,
                boolean,
                total: own,
                elif_count,
                case_count: 0,
                bool_op_count: boolean,
                children,
            };
            region.total += sum_region_child_totals(&region.children);
            result.regions.push(region);
        }
        Stmt::Try(t) => {
            let line_start = get_line_number(usize::from(t.range.start()), code);
            let line_end = get_line_number(usize::from(t.range.end()), code);
            let mut children = Vec::new();
            let body_result = collect_suite(&t.body, nesting_level + 1, code, &mut children);
            result.complexity += body_result.complexity;
            result
                .line_complexities
                .extend(body_result.line_complexities);
            let mut structural = 0;
            for handler in t.handlers.iter() {
                structural += 1;
                result.complexity += 1;
                let handler = handler.clone().expect_except_handler();
                let line = get_line_number(usize::from(handler.range.start()), code);
                result.line_complexities.push(LineComplexity {
                    line,
                    complexity: 1,
                });
                let handler_result =
                    collect_suite(&handler.body, nesting_level + 1, code, &mut children);
                result.complexity += handler_result.complexity;
                result
                    .line_complexities
                    .extend(handler_result.line_complexities);
            }
            let else_result = collect_suite(&t.orelse, nesting_level + 1, code, &mut children);
            result.complexity += else_result.complexity;
            result
                .line_complexities
                .extend(else_result.line_complexities);
            let final_result = collect_suite(&t.finalbody, nesting_level + 1, code, &mut children);
            result.complexity += final_result.complexity;
            result
                .line_complexities
                .extend(final_result.line_complexities);
            let mut region = ComplexityRegion {
                kind: RegionKind::Try,
                line_start,
                line_end,
                structural,
                nesting: 0,
                boolean: 0,
                total: structural,
                elif_count: 0,
                case_count: 0,
                bool_op_count: 0,
                children,
            };
            region.total += sum_region_child_totals(&region.children);
            result.regions.push(region);
        }
        Stmt::Match(m) => {
            let line_start = get_line_number(usize::from(m.range.start()), code);
            let line_end = get_line_number(usize::from(m.range.end()), code);
            let mut children = Vec::new();
            for case in m.cases.iter() {
                let case_result = collect_suite(&case.body, nesting_level + 1, code, &mut children);
                result.complexity += case_result.complexity;
                result
                    .line_complexities
                    .extend(case_result.line_complexities);
            }
            let mut region = ComplexityRegion {
                kind: RegionKind::Match,
                line_start,
                line_end,
                structural: 0,
                nesting: 0,
                boolean: 0,
                total: 0,
                elif_count: 0,
                case_count: m.cases.len() as u64,
                bool_op_count: 0,
                children,
            };
            region.total += sum_region_child_totals(&region.children);
            result.regions.push(region);
        }
        Stmt::Return(r) => {
            if let Some(value) = r.value.clone() {
                let bool_ops_complexity = count_bool_ops(*value, nesting_level);
                result.complexity += bool_ops_complexity;
                let line = get_line_number(usize::from(r.range.start()), code);
                result.line_complexities.push(LineComplexity {
                    line,
                    complexity: bool_ops_complexity,
                });
            }
        }
        Stmt::Raise(r) => {
            let mut raise_complexity = 0;
            if let Some(exc) = r.exc.clone() {
                raise_complexity += count_bool_ops(*exc, nesting_level);
            }
            if let Some(cause) = r.cause.clone() {
                raise_complexity += count_bool_ops(*cause, nesting_level);
            }
            result.complexity += raise_complexity;
            let line = get_line_number(usize::from(r.range.start()), code);
            result.line_complexities.push(LineComplexity {
                line,
                complexity: raise_complexity,
            });
        }
        Stmt::Assert(a) => {
            let mut assert_complexity = count_bool_ops(*a.test.clone(), nesting_level);
            if let Some(msg) = a.msg.clone() {
                assert_complexity += count_bool_ops(*msg, nesting_level);
            }
            result.complexity += assert_complexity;
            let line = get_line_number(usize::from(a.range.start()), code);
            result.line_complexities.push(LineComplexity {
                line,
                complexity: assert_complexity,
            });
        }
        Stmt::With(w) => {
            let mut with_complexity = 0;
            for item in w.items.iter() {
                with_complexity += count_bool_ops(item.context_expr.clone(), nesting_level);
            }
            result.complexity += with_complexity;
            let line_start = get_line_number(usize::from(w.range.start()), code);
            let line_end = get_line_number(usize::from(w.range.end()), code);
            result.line_complexities.push(LineComplexity {
                line: line_start,
                complexity: with_complexity,
            });
            let mut children = Vec::new();
            let body_result = collect_suite(&w.body, nesting_level + 1, code, &mut children);
            result.complexity += body_result.complexity;
            result
                .line_complexities
                .extend(body_result.line_complexities);
            let mut region = ComplexityRegion {
                kind: RegionKind::With,
                line_start,
                line_end,
                structural: 0,
                nesting: 0,
                boolean: with_complexity,
                total: with_complexity,
                elif_count: 0,
                case_count: 0,
                bool_op_count: with_complexity,
                children,
            };
            region.total += sum_region_child_totals(&region.children);
            result.regions.push(region);
        }
        _ => {}
    }

    result
}
