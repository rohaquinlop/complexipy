#[cfg(any(feature = "python", feature = "wasm"))]
mod shared_deps {
    pub use crate::classes::{CodeComplexity, FileComplexity, FunctionComplexity, LineComplexity};
    pub use crate::utils::{count_bool_ops, get_line_number, has_noqa_complexipy, is_decorator};
    pub use ruff_python_ast::{self as ast, Stmt};
}

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
            Err(_) => {
                failed_paths.push(path.to_string());
            }
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
        let (complexities, f_paths) = evaluate_dir(&repo_path, quiet, exclude.clone(), check_script);
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
fn evaluate_dir(path: &str, quiet: bool, exclude: Vec<&str>, check_script: bool) -> ComplexitiesAndFailedPaths {
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

    // Get all the python files in the directory
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
            .map(|file_path| match file_complexity(file_path, parent_dir, check_script) {
                Ok(file_complexity) => (Some(file_complexity), None),
                Err(_) => (None, Some(file_path.clone())),
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
pub fn file_complexity(file_path: &str, base_path: &str, check_script: bool) -> PyResult<FileComplexity> {
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

    // println!("{:#?}", ast_body);

    let (functions, complexity) = function_level_cognitive_complexity_shared(&ast_body, code, check_script);

    Ok(CodeComplexity {
        functions,
        complexity,
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

    for node in ast_body.iter() {
        match node {
            Stmt::FunctionDef(f) => {
                let start_line = get_line_number(usize::from(f.range.start()), code);
                if !has_noqa_complexipy(start_line, code) {
                    let (func_complexity, line_complexities) =
                        statement_cognitive_complexity_shared(node, 0, code);

                    let function = FunctionComplexity {
                        name: f.name.to_string(),
                        complexity: func_complexity,
                        cyclomatic_complexity: None,
                        line_start: start_line,
                        line_end: get_line_number(usize::from(f.range.end()), code),
                        line_complexities,
                    };

                    functions.push(function);
                }
            }
            Stmt::ClassDef(c) => {
                for node in c.body.iter() {
                    if let Stmt::FunctionDef(f) = node {
                        let start_line = get_line_number(usize::from(f.range.start()), code);
                        if !has_noqa_complexipy(start_line, code) {
                            let (func_complexity, line_complexities) =
                                statement_cognitive_complexity_shared(node, 0, code);

                            let function = FunctionComplexity {
                                name: format!("{}::{}", c.name, f.name),
                                complexity: func_complexity,
                                cyclomatic_complexity: None,
                                line_start: start_line,
                                line_end: get_line_number(usize::from(f.range.end()), code),
                                line_complexities,
                            };

                            functions.push(function);
                        }
                    }
                }
            }
            _ => {
                let (stmt_complexity, stmt_line_complexities) =
                    statement_cognitive_complexity_shared(node, 0, code);
                if check_script {
                    module_complexity += stmt_complexity;
                    module_line_complexities.extend(stmt_line_complexities);
                } else {
                    complexity += stmt_complexity;
                }
            }
        }
    }

    if check_script {
        let total_lines = code.lines().count() as u64;
        let module_func = FunctionComplexity {
            name: "<module>".to_string(),
            complexity: module_complexity,
            cyclomatic_complexity: None,
            line_start: 1,
            line_end: total_lines,
            line_complexities: module_line_complexities,
        };
        functions.push(module_func);
    }

    for function in functions.iter() {
        complexity += function.complexity;
    }

    (functions, complexity)
}

fn statement_cognitive_complexity_shared(
    statement: &Stmt,
    nesting_level: u64,
    code: &str,
) -> (u64, Vec<LineComplexity>) {
    let mut complexity: u64 = 0;
    let mut line_complexities: Vec<LineComplexity> = Vec::new();

    if is_decorator(statement.clone())
        && let Stmt::FunctionDef(f) = statement
    {
        return statement_cognitive_complexity_shared(&f.body[0], nesting_level, code);
    }

    match statement {
        Stmt::FunctionDef(f) => {
            for node in f.body.iter() {
                match node {
                    Stmt::FunctionDef(..) => {
                        let (stmt_complexity, stmt_line_complexities) =
                            statement_cognitive_complexity_shared(node, nesting_level + 1, code);
                        complexity += stmt_complexity;
                        line_complexities.extend(stmt_line_complexities);
                    }
                    _ => {
                        let (stmt_complexity, stmt_line_complexities) =
                            statement_cognitive_complexity_shared(node, nesting_level, code);
                        complexity += stmt_complexity;
                        line_complexities.extend(stmt_line_complexities);
                    }
                }
            }
        }
        Stmt::ClassDef(c) => {
            for node in c.body.iter() {
                if let Stmt::FunctionDef(..) = node {
                    let (stmt_complexity, stmt_line_complexities) =
                        statement_cognitive_complexity_shared(node, nesting_level, code);
                    complexity += stmt_complexity;
                    line_complexities.extend(stmt_line_complexities);
                }
            }
        }
        Stmt::Assign(a) => {
            let bool_ops_complexity = count_bool_ops(*a.value.clone(), nesting_level);
            complexity += bool_ops_complexity;
            let line = get_line_number(usize::from(a.range.start()), code);
            line_complexities.push(LineComplexity {
                line,
                complexity: bool_ops_complexity,
            });
        }
        Stmt::AnnAssign(a) => {
            if let Some(value) = a.value.clone() {
                let bool_ops_complexity = count_bool_ops(*value, nesting_level);
                complexity += bool_ops_complexity;
                let line = get_line_number(usize::from(a.range.start()), code);
                line_complexities.push(LineComplexity {
                    line,
                    complexity: bool_ops_complexity,
                });
            }
        }
        Stmt::AugAssign(a) => {
            let bool_ops_complexity = count_bool_ops(*a.value.clone(), nesting_level);
            complexity += bool_ops_complexity;
            let line = get_line_number(usize::from(a.range.start()), code);
            line_complexities.push(LineComplexity {
                line,
                complexity: bool_ops_complexity,
            });
        }
        Stmt::For(f) => {
            let stmt_complexity = 1 + nesting_level;
            complexity += stmt_complexity;
            let line = get_line_number(usize::from(f.range.start()), code);
            line_complexities.push(LineComplexity {
                line,
                complexity: stmt_complexity,
            });
            for node in f.body.iter() {
                let (stmt_complexity, stmt_line_complexities) =
                    statement_cognitive_complexity_shared(node, nesting_level + 1, code);
                complexity += stmt_complexity;
                line_complexities.extend(stmt_line_complexities);
            }
            for node in f.orelse.iter() {
                let (stmt_complexity, stmt_line_complexities) =
                    statement_cognitive_complexity_shared(node, nesting_level + 1, code);
                complexity += stmt_complexity;
                line_complexities.extend(stmt_line_complexities);
            }
        }
        Stmt::While(w) => {
            let stmt_complexity =
                1 + nesting_level + count_bool_ops(*w.test.clone(), nesting_level);
            complexity += stmt_complexity;
            let line = get_line_number(usize::from(w.range.start()), code);
            line_complexities.push(LineComplexity {
                line,
                complexity: stmt_complexity,
            });
            for node in w.body.iter() {
                let (stmt_complexity, stmt_line_complexities) =
                    statement_cognitive_complexity_shared(node, nesting_level + 1, code);
                complexity += stmt_complexity;
                line_complexities.extend(stmt_line_complexities);
            }
            for node in w.orelse.iter() {
                let (stmt_complexity, stmt_line_complexities) =
                    statement_cognitive_complexity_shared(node, nesting_level + 1, code);
                complexity += stmt_complexity;
                line_complexities.extend(stmt_line_complexities);
            }
        }
        Stmt::If(i) => {
            let stmt_complexity =
                1 + nesting_level + count_bool_ops(*i.test.clone(), nesting_level);
            complexity += stmt_complexity;
            let line = get_line_number(usize::from(i.range.start()), code);
            line_complexities.push(LineComplexity {
                line,
                complexity: stmt_complexity,
            });
            for node in i.body.iter() {
                let (stmt_complexity, stmt_line_complexities) =
                    statement_cognitive_complexity_shared(node, nesting_level + 1, code);
                complexity += stmt_complexity;
                line_complexities.extend(stmt_line_complexities);
            }
            for clause in i.elif_else_clauses.clone() {
                let mut clause_complexity = 1;
                if let Some(test) = clause.test.clone() {
                    clause_complexity += count_bool_ops(test, nesting_level);
                }

                complexity += clause_complexity;

                let line = get_line_number(usize::from(clause.range.start()), code);
                line_complexities.push(LineComplexity {
                    line,
                    complexity: clause_complexity,
                });

                for node in clause.body.iter() {
                    let (stmt_complexity, stmt_line_complexities) =
                        statement_cognitive_complexity_shared(node, nesting_level + 1, code);
                    complexity += stmt_complexity;
                    line_complexities.extend(stmt_line_complexities);
                }
            }
        }
        Stmt::Try(t) => {
            for node in t.body.iter() {
                let (stmt_complexity, stmt_line_complexities) =
                    statement_cognitive_complexity_shared(node, nesting_level + 1, code);
                complexity += stmt_complexity;
                line_complexities.extend(stmt_line_complexities);
            }
            for handler in t.handlers.iter() {
                complexity += 1;
                let handler = handler.clone().expect_except_handler();
                let line = get_line_number(usize::from(handler.range.start()), code);
                line_complexities.push(LineComplexity {
                    line,
                    complexity: 1,
                });
                for node in handler.clone().body.iter() {
                    let (stmt_complexity, stmt_line_complexities) =
                        statement_cognitive_complexity_shared(node, nesting_level + 1, code);
                    complexity += stmt_complexity;
                    line_complexities.extend(stmt_line_complexities);
                }
            }
            for node in t.orelse.iter() {
                let (stmt_complexity, stmt_line_complexities) =
                    statement_cognitive_complexity_shared(node, nesting_level + 1, code);
                complexity += stmt_complexity;
                line_complexities.extend(stmt_line_complexities);
            }
            for node in t.finalbody.iter() {
                let (stmt_complexity, stmt_line_complexities) =
                    statement_cognitive_complexity_shared(node, nesting_level + 1, code);
                complexity += stmt_complexity;
                line_complexities.extend(stmt_line_complexities);
            }
        }
        Stmt::Match(m) => {
            for case in m.cases.iter() {
                for node in case.body.iter() {
                    let (stmt_complexity, stmt_line_complexities) =
                        statement_cognitive_complexity_shared(node, nesting_level + 1, code);
                    complexity += stmt_complexity;
                    line_complexities.extend(stmt_line_complexities);
                }
            }
        }
        Stmt::Return(r) => {
            if let Some(value) = r.value.clone() {
                let bool_ops_complexity = count_bool_ops(*value, nesting_level);
                complexity += bool_ops_complexity;
                let line = get_line_number(usize::from(r.range.start()), code);
                line_complexities.push(LineComplexity {
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
            complexity += raise_complexity;
            let line = get_line_number(usize::from(r.range.start()), code);
            line_complexities.push(LineComplexity {
                line,
                complexity: raise_complexity,
            });
        }
        Stmt::Assert(a) => {
            let mut assert_complexity = count_bool_ops(*a.test.clone(), nesting_level);
            if let Some(msg) = a.msg.clone() {
                assert_complexity += count_bool_ops(*msg, nesting_level);
            }
            complexity += assert_complexity;
            let line = get_line_number(usize::from(a.range.start()), code);
            line_complexities.push(LineComplexity {
                line,
                complexity: assert_complexity,
            });
        }
        Stmt::With(w) => {
            let mut with_complexity = 0;
            for item in w.items.iter() {
                with_complexity += count_bool_ops(item.context_expr.clone(), nesting_level);
            }
            complexity += with_complexity;
            let line = get_line_number(usize::from(w.range.start()), code);
            line_complexities.push(LineComplexity {
                line,
                complexity: with_complexity,
            });
            for node in w.body.iter() {
                let (stmt_complexity, stmt_line_complexities) =
                    statement_cognitive_complexity_shared(node, nesting_level + 1, code);
                complexity += stmt_complexity;
                line_complexities.extend(stmt_line_complexities);
            }
        }
        _ => {}
    }

    (complexity, line_complexities)
}
