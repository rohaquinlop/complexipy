pub mod utils;

use crate::classes::{CodeComplexity, FileComplexity, FunctionComplexity};
use ignore::Walk;
use indicatif::ProgressBar;
use indicatif::ProgressStyle;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use rayon::prelude::*;
use regex::Regex;

// Import from ruff crates
use ruff_python_ast::{self as ast, Stmt};
use ruff_python_parser::parse_program;

use std::env;
use std::fs::metadata;
use std::path;
use std::process;
use std::sync::{Arc, Mutex};
use std::thread;
use tempfile::tempdir;
use utils::{count_bool_ops, get_repo_name, is_decorator};

// Main function
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
#[pyfunction]
pub fn code_complexity(code: &str) -> PyResult<CodeComplexity> {
    let parsed = match parse_program(code) {
        Ok(parsed) => parsed,
        Err(e) => {
            return Err(PyValueError::new_err(format!(
                "Failed to parse code: {}",
                e
            )));
        }
    };

    // The ruff parser returns a Program, which contains a body (Suite) of statements.
    let ast_body = &parsed.body;

    let (functions, complexity) = function_level_cognitive_complexity(ast_body)?;

    Ok(CodeComplexity {
        functions,
        complexity,
    })
}

fn function_level_cognitive_complexity(
    ast_body: &ast::Suite, // Use ruff_python_ast::Suite
) -> PyResult<(Vec<FunctionComplexity>, u64)> {
    let mut functions: Vec<FunctionComplexity> = Vec::new();
    let mut complexity: u64 = 0;

    for node in ast_body.iter() {
        match node {
            Stmt::FunctionDef(f) => {
                // Pass the function statement itself to calculate its complexity
                let func_complexity = statement_cognitive_complexity(node.clone(), 0)?;
                functions.push(FunctionComplexity {
                    name: f.name.to_string(), // Use .to_string() for Identifier
                    complexity: func_complexity,
                });
            }
            Stmt::ClassDef(c) => {
                for node in c.body.iter() {
                    match node {
                        Stmt::FunctionDef(f) => {
                            // Methods inside classes start at nesting level 1 relative to the class
                            // Pass the method statement itself to calculate its complexity
                            let func_complexity = statement_cognitive_complexity(node.clone(), 0)?;
                            functions.push(FunctionComplexity {
                                // Use .to_string() for Identifiers in format!
                                name: format!("{}::{}", c.name.to_string(), f.name.to_string()),
                                complexity: func_complexity,
                            });
                        }
                        _ => {} // Ignore other statements inside classes for complexity calculation
                    }
                }
                // Don't add complexity for the class definition itself, only for its methods and top-level code
            }
            _ => {
                complexity += statement_cognitive_complexity(node.clone(), 0)?;
            }
        }
    }

    for function in functions.iter() {
        complexity += function.complexity;
    }

    Ok((functions, complexity))
}

/// Calculate the cognitive complexity of a python statement using ruff_python_ast
fn statement_cognitive_complexity(statement: Stmt, nesting_level: u64) -> PyResult<u64> {
    let mut complexity: u64 = 0;

    if is_decorator(statement.clone()) {
        match statement {
            Stmt::FunctionDef(f) => {
                return statement_cognitive_complexity(f.body[0].clone(), nesting_level);
            }
            _ => {}
        }
    }

    match statement {
        Stmt::FunctionDef(f) => {
            for node in f.body.iter() {
                match node {
                    Stmt::FunctionDef(..) => {
                        complexity +=
                            statement_cognitive_complexity(node.clone(), nesting_level + 1)?;
                    }
                    _ => {
                        complexity += statement_cognitive_complexity(node.clone(), nesting_level)?;
                    }
                }
            }
        }
        Stmt::ClassDef(c) => {
            for node in c.body.iter() {
                match node {
                    Stmt::FunctionDef(..) => {
                        // Nesting level for methods starts at the class level + 1
                        complexity += statement_cognitive_complexity(node.clone(), nesting_level)?;
                    }
                    _ => {}
                }
            }
        }
        Stmt::Assign(a) => {
            complexity += count_bool_ops(*a.value, nesting_level);
        }
        Stmt::AnnAssign(a) => {
            if let Some(value) = a.value {
                complexity += count_bool_ops(*value, nesting_level);
            }
        }
        Stmt::AugAssign(a) => {
            complexity += count_bool_ops(*a.value, nesting_level);
        }
        Stmt::For(f) => {
            complexity += 1 + nesting_level;
            for node in f.body.iter() {
                complexity += statement_cognitive_complexity(node.clone(), nesting_level + 1)?;
            }
            for node in f.orelse.iter() {
                // Handle orelse in For loops
                complexity += statement_cognitive_complexity(node.clone(), nesting_level + 1)?;
            }
        }
        Stmt::While(w) => {
            complexity += 1 + nesting_level;
            complexity += count_bool_ops(*w.test, nesting_level);
            for node in w.body.iter() {
                complexity += statement_cognitive_complexity(node.clone(), nesting_level + 1)?;
            }
            for node in w.orelse.iter() {
                // Handle orelse in While loops
                complexity += statement_cognitive_complexity(node.clone(), nesting_level + 1)?;
            }
        }
        Stmt::If(i) => {
            complexity += 1 + nesting_level;
            complexity += count_bool_ops(*i.test, nesting_level);
            for node in i.body.iter() {
                complexity += statement_cognitive_complexity(node.clone(), nesting_level + 1)?;
            }

            for clause in i.elif_else_clauses {
                for node in clause.body.iter() {
                    complexity += statement_cognitive_complexity(node.clone(), nesting_level)?;
                }
            }
        }
        Stmt::Try(t) => {
            for node in t.body.iter() {
                complexity += statement_cognitive_complexity(node.clone(), nesting_level + 1)?;
            }
            for handler in t.handlers.iter() {
                complexity += 1; // Increment for each except handler
                for node in handler.clone().expect_except_handler().body.iter() {
                    complexity += statement_cognitive_complexity(node.clone(), nesting_level + 1)?;
                }
            }
            for node in t.orelse.iter() {
                complexity += statement_cognitive_complexity(node.clone(), nesting_level + 1)?;
            }
            for node in t.finalbody.iter() {
                complexity += statement_cognitive_complexity(node.clone(), nesting_level + 1)?;
            }
        }
        Stmt::Match(m) => {
            for case in m.cases.iter() {
                for node in case.body.iter() {
                    complexity += statement_cognitive_complexity(node.clone(), nesting_level + 1)?;
                }
            }
        }
        Stmt::Return(r) => {
            if let Some(value) = r.value {
                complexity += count_bool_ops(*value, nesting_level);
            }
        }
        Stmt::Raise(r) => {
            if let Some(exc) = r.exc {
                complexity += count_bool_ops(*exc, nesting_level);
            }
            if let Some(cause) = r.cause {
                complexity += count_bool_ops(*cause, nesting_level);
            }
        }
        Stmt::Assert(a) => {
            complexity += count_bool_ops(*a.test, nesting_level);
            if let Some(msg) = a.msg {
                complexity += count_bool_ops(*msg, nesting_level);
            }
        }
        Stmt::With(w) => {
            for item in w.items.iter() {
                complexity += count_bool_ops(item.context_expr.clone(), nesting_level);
            }
            for node in w.body.iter() {
                complexity += statement_cognitive_complexity(node.clone(), nesting_level)?;
            }
        }
        _ => {}
    };

    Ok(complexity)
}
