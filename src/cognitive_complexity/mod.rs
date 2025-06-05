pub mod utils;

#[cfg(feature = "python")]
use crate::classes::{CodeComplexity, FileComplexity, FunctionComplexity, LineComplexity};
#[cfg(feature = "wasm")]
use crate::classes::{FunctionComplexity, LineComplexity};
#[cfg(feature = "python")]
use ignore::Walk;
#[cfg(feature = "python")]
use indicatif::ProgressBar;
#[cfg(feature = "python")]
use indicatif::ProgressStyle;
#[cfg(feature = "python")]
use pyo3::exceptions::PyValueError;
#[cfg(feature = "python")]
use pyo3::prelude::*;
#[cfg(feature = "python")]
use rayon::prelude::*;
#[cfg(feature = "python")]
use regex::Regex;

// Import from ruff crates
#[cfg(feature = "python")]
use ruff_python_ast::{self as ast, Stmt};
#[cfg(feature = "python")]
use ruff_python_parser::parse_module;

// Import ruff crates for WASM
#[cfg(feature = "wasm")]
use ruff_python_ast::{self as ast, Stmt};
use utils::get_line_number;
#[cfg(feature = "wasm")]
use utils::{count_bool_ops, is_decorator};

#[cfg(feature = "python")]
use std::env;
#[cfg(feature = "python")]
use std::fs::metadata;
#[cfg(feature = "python")]
use std::path;
#[cfg(feature = "python")]
use std::process;
#[cfg(feature = "python")]
use std::sync::{Arc, Mutex};
#[cfg(feature = "python")]
use std::thread;
#[cfg(feature = "python")]
use tempfile::tempdir;
#[cfg(feature = "python")]
use utils::{count_bool_ops, get_repo_name, is_decorator};

// Main function
#[cfg(feature = "python")]
#[pyfunction]
pub fn main(paths: Vec<&str>) -> PyResult<Vec<FileComplexity>> {
    let re = Regex::new(r"^(https:\/\/|http:\/\/|www\.|git@)(github|gitlab)\.com(\/[\w.-]+){2,}$")
        .unwrap();

    let all_files_paths: Vec<(&str, bool, bool)> = paths
        .par_iter()
        .map(|&path| {
            let is_url = re.is_match(path);

            if is_url {
                return (path, false, true);
            } else if metadata(path).unwrap().is_dir() {
                return (path, true, false);
            } else {
                return (path, false, false);
            }
        })
        .collect();

    let all_files_processed: Vec<Result<Vec<FileComplexity>, PyErr>> = all_files_paths
        .iter()
        .map(|(path, is_dir, is_url)| process_path(path, *is_dir, *is_url))
        .collect();

    if all_files_processed.iter().all(|x| !x.is_err()) {
        let ans: Vec<FileComplexity> = all_files_processed
            .iter()
            .flat_map(|file_complexities| file_complexities.as_ref().unwrap().iter().cloned())
            .collect();
        return Ok(ans);
    } else {
        return Err(PyValueError::new_err("Failed to process the paths"));
    }
}

#[cfg(feature = "python")]
#[pyfunction]
pub fn process_path(path: &str, is_dir: bool, is_url: bool) -> PyResult<Vec<FileComplexity>> {
    let mut ans: Vec<FileComplexity> = Vec::new();

    if is_url {
        let dir = tempdir()?;
        let repo_name = get_repo_name(path);

        env::set_current_dir(&dir)?;

        let cloning_done = Arc::new(Mutex::new(false));
        let cloning_done_clone = Arc::clone(&cloning_done);
        let path_clone = path.to_owned(); // Clone the path variable

        thread::spawn(move || {
            let _output = process::Command::new("git")
                .args(&["clone", &path_clone]) // Use the cloned path variable
                .output()
                .expect("failed to execute process");

            let mut done = cloning_done_clone.lock().unwrap();
            *done = true;
        });

        let pb = ProgressBar::new_spinner();
        pb.set_style(ProgressStyle::default_spinner());
        pb.set_message("Cloning repository...");

        while !*cloning_done.lock().unwrap() {
            pb.tick();
            thread::sleep(std::time::Duration::from_millis(100));
        }

        pb.finish_with_message("Repository cloned!");

        let repo_path = dir.path().join(&repo_name).to_str().unwrap().to_string();

        match evaluate_dir(&repo_path) {
            Ok(files_complexity) => ans = files_complexity,
            Err(e) => return Err(e),
        }

        dir.close()?;
    } else if is_dir {
        match evaluate_dir(path) {
            Ok(files_complexity) => ans = files_complexity,
            Err(e) => return Err(e),
        }
    } else {
        let parent_dir = path::Path::new(path).parent().unwrap().to_str().unwrap();

        match file_complexity(path, parent_dir) {
            Ok(file_complexity) => ans.push(file_complexity),
            Err(e) => return Err(e),
        }
    }

    ans.iter_mut()
        .for_each(|f| f.functions.sort_by_key(|f| (f.complexity, f.name.clone())));

    ans.sort_by_key(|f| (f.path.clone(), f.file_name.clone(), f.complexity));
    Ok(ans)
}

#[cfg(feature = "python")]
fn evaluate_dir(path: &str) -> PyResult<Vec<FileComplexity>> {
    let mut files_paths: Vec<String> = Vec::new();

    let parent_dir = path::Path::new(path).parent().unwrap().to_str().unwrap();

    // Get all the python files in the directory
    for entry in Walk::new(path) {
        let entry = entry.unwrap();
        let file_path_str = entry.path().to_str().unwrap();

        if entry.path().extension().and_then(|s| s.to_str()) == Some("py") {
            files_paths.push(file_path_str.to_string());
        }
    }

    let pb = ProgressBar::new(files_paths.len() as u64);
    pb.set_style(
        indicatif::ProgressStyle::default_bar()
            .template(
                "{spiner:.green} [{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}",
            )
            .unwrap()
            .progress_chars("##-"),
    );

    let files_complexity_result: Result<Vec<FileComplexity>, PyErr> = files_paths
        .par_iter()
        .map(|file_path| {
            pb.inc(1);
            match file_complexity(file_path, parent_dir) {
                Ok(file_complexity) => Ok(file_complexity),
                Err(e) => Err(e), // Propagate the error
            }
        })
        .collect();

    pb.finish_with_message("Done!");

    files_complexity_result // Return the collected result
}

/// Calculate the cognitive complexity of a python file.
#[cfg(feature = "python")]
#[pyfunction]
pub fn file_complexity(file_path: &str, base_path: &str) -> PyResult<FileComplexity> {
    let path = path::Path::new(file_path);
    let file_name = path.file_name().unwrap().to_str().unwrap();
    let relative_path = path.strip_prefix(base_path).unwrap().to_str().unwrap();

    let code = std::fs::read_to_string(file_path)?;

    let code_complexity = match code_complexity(&code) {
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

/// Calculate the cognitive complexity of a string of python code.
#[cfg(feature = "python")]
#[pyfunction]
pub fn code_complexity(code: &str) -> PyResult<CodeComplexity> {
    let parsed = match parse_module(code) {
        Ok(parsed) => parsed,
        Err(e) => {
            return Err(PyValueError::new_err(format!(
                "Failed to parse code: {}",
                e
            )));
        }
    };

    // The ruff parser returns a Program, which contains a body (Suite) of statements.
    let ast_body = &parsed.suite();

    let (functions, complexity) = function_level_cognitive_complexity_shared(ast_body, Some(code));

    Ok(CodeComplexity {
        functions,
        complexity,
    })
}

#[cfg(any(feature = "python", feature = "wasm"))]
pub fn function_level_cognitive_complexity_shared(
    ast_body: &ast::Suite,
    code: Option<&str>,
) -> (Vec<FunctionComplexity>, u64) {
    let mut functions: Vec<FunctionComplexity> = Vec::new();
    let mut complexity: u64 = 0;

    for node in ast_body.iter() {
        match node {
            Stmt::FunctionDef(f) => {
                let (func_complexity, line_complexities) =
                    statement_cognitive_complexity_shared(node.clone(), 0, code);

                let function = FunctionComplexity {
                    name: f.name.to_string(),
                    complexity: func_complexity,
                    line_start: get_line_number(usize::from(f.range.start()), code.unwrap()),
                    line_end: get_line_number(usize::from(f.range.end()), code.unwrap()),
                    line_complexities,
                };

                functions.push(function);
            }
            Stmt::ClassDef(c) => {
                for node in c.body.iter() {
                    match node {
                        Stmt::FunctionDef(f) => {
                            let (func_complexity, line_complexities) =
                                statement_cognitive_complexity_shared(node.clone(), 0, code);

                            let function = FunctionComplexity {
                                name: format!("{}::{}", c.name.to_string(), f.name.to_string()),
                                complexity: func_complexity,
                                line_start: get_line_number(
                                    usize::from(f.range.start()),
                                    code.unwrap(),
                                ),
                                line_end: get_line_number(
                                    usize::from(f.range.end()),
                                    code.unwrap(),
                                ),
                                line_complexities,
                            };

                            functions.push(function);
                        }
                        _ => {}
                    }
                }
            }
            _ => {
                let (stmt_complexity, _) =
                    statement_cognitive_complexity_shared(node.clone(), 0, code);
                complexity += stmt_complexity;
            }
        }
    }

    for function in functions.iter() {
        complexity += function.complexity;
    }

    (functions, complexity)
}

fn statement_cognitive_complexity_shared(
    statement: Stmt,
    nesting_level: u64,
    code: Option<&str>,
) -> (u64, Vec<LineComplexity>) {
    let mut complexity: u64 = 0;
    let mut line_complexities: Vec<LineComplexity> = Vec::new();

    if is_decorator(statement.clone()) {
        match statement {
            Stmt::FunctionDef(f) => {
                return statement_cognitive_complexity_shared(
                    f.body[0].clone(),
                    nesting_level,
                    code,
                );
            }
            _ => {}
        }
    }

    match statement {
        Stmt::FunctionDef(f) => {
            for node in f.body.iter() {
                match node {
                    Stmt::FunctionDef(..) => {
                        let (stmt_complexity, stmt_line_complexities) =
                            statement_cognitive_complexity_shared(
                                node.clone(),
                                nesting_level + 1,
                                code,
                            );
                        complexity += stmt_complexity;
                        line_complexities.extend(stmt_line_complexities);
                    }
                    _ => {
                        let (stmt_complexity, stmt_line_complexities) =
                            statement_cognitive_complexity_shared(
                                node.clone(),
                                nesting_level,
                                code,
                            );
                        complexity += stmt_complexity;
                        line_complexities.extend(stmt_line_complexities);
                    }
                }
            }
        }
        Stmt::ClassDef(c) => {
            for node in c.body.iter() {
                match node {
                    Stmt::FunctionDef(..) => {
                        let (stmt_complexity, stmt_line_complexities) =
                            statement_cognitive_complexity_shared(
                                node.clone(),
                                nesting_level,
                                code,
                            );
                        complexity += stmt_complexity;
                        line_complexities.extend(stmt_line_complexities);
                    }
                    _ => {}
                }
            }
        }
        Stmt::Assign(a) => {
            let bool_ops_complexity = count_bool_ops(*a.value, nesting_level);
            complexity += bool_ops_complexity;
            if let Some(code_str) = code {
                let line = get_line_number(usize::from(a.range.start()), code_str);
                line_complexities.push(LineComplexity {
                    line,
                    complexity: bool_ops_complexity,
                });
            }
        }
        Stmt::AnnAssign(a) => {
            if let Some(value) = a.value {
                let bool_ops_complexity = count_bool_ops(*value, nesting_level);
                complexity += bool_ops_complexity;
                if let Some(code_str) = code {
                    let line = get_line_number(usize::from(a.range.start()), code_str);
                    line_complexities.push(LineComplexity {
                        line,
                        complexity: bool_ops_complexity,
                    });
                }
            }
        }
        Stmt::AugAssign(a) => {
            let bool_ops_complexity = count_bool_ops(*a.value, nesting_level);
            complexity += bool_ops_complexity;
            if let Some(code_str) = code {
                let line = get_line_number(usize::from(a.range.start()), code_str);
                line_complexities.push(LineComplexity {
                    line,
                    complexity: bool_ops_complexity,
                });
            }
        }
        Stmt::For(f) => {
            let stmt_complexity = 1 + nesting_level;
            complexity += stmt_complexity;
            if let Some(code_str) = code {
                let line = get_line_number(usize::from(f.range.start()), code_str);
                line_complexities.push(LineComplexity {
                    line,
                    complexity: stmt_complexity,
                });
            }
            for node in f.body.iter() {
                let (stmt_complexity, stmt_line_complexities) =
                    statement_cognitive_complexity_shared(node.clone(), nesting_level + 1, code);
                complexity += stmt_complexity;
                line_complexities.extend(stmt_line_complexities);
            }
            for node in f.orelse.iter() {
                let (stmt_complexity, stmt_line_complexities) =
                    statement_cognitive_complexity_shared(node.clone(), nesting_level + 1, code);
                complexity += stmt_complexity;
                line_complexities.extend(stmt_line_complexities);
            }
        }
        Stmt::While(w) => {
            let stmt_complexity = 1 + nesting_level + count_bool_ops(*w.test, nesting_level);
            complexity += stmt_complexity;
            if let Some(code_str) = code {
                let line = get_line_number(usize::from(w.range.start()), code_str);
                line_complexities.push(LineComplexity {
                    line,
                    complexity: stmt_complexity,
                });
            }
            for node in w.body.iter() {
                let (stmt_complexity, stmt_line_complexities) =
                    statement_cognitive_complexity_shared(node.clone(), nesting_level + 1, code);
                complexity += stmt_complexity;
                line_complexities.extend(stmt_line_complexities);
            }
            for node in w.orelse.iter() {
                let (stmt_complexity, stmt_line_complexities) =
                    statement_cognitive_complexity_shared(node.clone(), nesting_level + 1, code);
                complexity += stmt_complexity;
                line_complexities.extend(stmt_line_complexities);
            }
        }
        Stmt::If(i) => {
            let stmt_complexity = 1 + nesting_level + count_bool_ops(*i.test, nesting_level);
            complexity += stmt_complexity;
            if let Some(code_str) = code {
                let line = get_line_number(usize::from(i.range.start()), code_str);
                line_complexities.push(LineComplexity {
                    line,
                    complexity: stmt_complexity,
                });
            }
            for node in i.body.iter() {
                let (stmt_complexity, stmt_line_complexities) =
                    statement_cognitive_complexity_shared(node.clone(), nesting_level + 1, code);
                complexity += stmt_complexity;
                line_complexities.extend(stmt_line_complexities);
            }
            for clause in i.elif_else_clauses {
                if let Some(test) = clause.test {
                    let clause_complexity = count_bool_ops(test, nesting_level);
                    complexity += clause_complexity;
                    if let Some(code_str) = code {
                        let line = get_line_number(usize::from(clause.range.start()), code_str);
                        line_complexities.push(LineComplexity {
                            line,
                            complexity: clause_complexity,
                        });
                    }
                }
                for node in clause.body.iter() {
                    let (stmt_complexity, stmt_line_complexities) =
                        statement_cognitive_complexity_shared(
                            node.clone(),
                            nesting_level + 1,
                            code,
                        );
                    complexity += stmt_complexity;
                    line_complexities.extend(stmt_line_complexities);
                }
            }
        }
        Stmt::Try(t) => {
            for node in t.body.iter() {
                let (stmt_complexity, stmt_line_complexities) =
                    statement_cognitive_complexity_shared(node.clone(), nesting_level + 1, code);
                complexity += stmt_complexity;
                line_complexities.extend(stmt_line_complexities);
            }
            for handler in t.handlers.iter() {
                complexity += 1;
                if let Some(code_str) = code {
                    let handler = handler.clone().expect_except_handler();
                    let line = get_line_number(usize::from(handler.range.start()), code_str);
                    line_complexities.push(LineComplexity {
                        line,
                        complexity: 1,
                    });
                }
                for node in handler.clone().expect_except_handler().body.iter() {
                    let (stmt_complexity, stmt_line_complexities) =
                        statement_cognitive_complexity_shared(
                            node.clone(),
                            nesting_level + 1,
                            code,
                        );
                    complexity += stmt_complexity;
                    line_complexities.extend(stmt_line_complexities);
                }
            }
            for node in t.orelse.iter() {
                let (stmt_complexity, stmt_line_complexities) =
                    statement_cognitive_complexity_shared(node.clone(), nesting_level + 1, code);
                complexity += stmt_complexity;
                line_complexities.extend(stmt_line_complexities);
            }
            for node in t.finalbody.iter() {
                let (stmt_complexity, stmt_line_complexities) =
                    statement_cognitive_complexity_shared(node.clone(), nesting_level + 1, code);
                complexity += stmt_complexity;
                line_complexities.extend(stmt_line_complexities);
            }
        }
        Stmt::Match(m) => {
            for case in m.cases.iter() {
                for node in case.body.iter() {
                    let (stmt_complexity, stmt_line_complexities) =
                        statement_cognitive_complexity_shared(
                            node.clone(),
                            nesting_level + 1,
                            code,
                        );
                    complexity += stmt_complexity;
                    line_complexities.extend(stmt_line_complexities);
                }
            }
        }
        Stmt::Return(r) => {
            if let Some(value) = r.value {
                let bool_ops_complexity = count_bool_ops(*value, nesting_level);
                complexity += bool_ops_complexity;
                if let Some(code_str) = code {
                    let line = get_line_number(usize::from(r.range.start()), code_str);
                    line_complexities.push(LineComplexity {
                        line,
                        complexity: bool_ops_complexity,
                    });
                }
            }
        }
        Stmt::Raise(r) => {
            let mut raise_complexity = 0;
            if let Some(exc) = r.exc {
                raise_complexity += count_bool_ops(*exc, nesting_level);
            }
            if let Some(cause) = r.cause {
                raise_complexity += count_bool_ops(*cause, nesting_level);
            }
            complexity += raise_complexity;
            if let Some(code_str) = code {
                let line = get_line_number(usize::from(r.range.start()), code_str);
                line_complexities.push(LineComplexity {
                    line,
                    complexity: raise_complexity,
                });
            }
        }
        Stmt::Assert(a) => {
            let mut assert_complexity = count_bool_ops(*a.test, nesting_level);
            if let Some(msg) = a.msg {
                assert_complexity += count_bool_ops(*msg, nesting_level);
            }
            complexity += assert_complexity;
            if let Some(code_str) = code {
                let line = get_line_number(usize::from(a.range.start()), code_str);
                line_complexities.push(LineComplexity {
                    line,
                    complexity: assert_complexity,
                });
            }
        }
        Stmt::With(w) => {
            let mut with_complexity = 0;
            for item in w.items.iter() {
                with_complexity += count_bool_ops(item.context_expr.clone(), nesting_level);
            }
            complexity += with_complexity;
            if let Some(code_str) = code {
                let line = get_line_number(usize::from(w.range.start()), code_str);
                line_complexities.push(LineComplexity {
                    line,
                    complexity: with_complexity,
                });
            }
            for node in w.body.iter() {
                let (stmt_complexity, stmt_line_complexities) =
                    statement_cognitive_complexity_shared(node.clone(), nesting_level + 1, code);
                complexity += stmt_complexity;
                line_complexities.extend(stmt_line_complexities);
            }
        }
        _ => {}
    }

    (complexity, line_complexities)
}
