mod utils;

use crate::classes::FileComplexity;
use ignore::Walk;
use pyo3::prelude::*;
use rayon::prelude::*;
use rustpython_parser::{
    ast::{self, Stmt},
    Parse,
};
use std::path;
use utils::{count_bool_ops, has_recursive_calls, is_decorator};

// Main function
#[pyfunction]
pub fn main(path: &str, is_dir: bool, max_complexity: usize) -> PyResult<Vec<FileComplexity>> {
    let mut ans: Vec<FileComplexity> = Vec::new();
    if is_dir {
        match evaluate_dir(path, max_complexity) {
            Ok(files_complexity) => ans = files_complexity,
            Err(e) => return Err(e),
        }
    } else {
        match file_cognitive_complexity(path, max_complexity) {
            Ok(file_complexity) => ans.push(file_complexity),
            Err(e) => return Err(e),
        }
    }

    ans.sort_by_key(|f| f.path.clone());
    Ok(ans)
}

#[pyfunction]
pub fn evaluate_dir(path: &str, max_complexity: usize) -> PyResult<Vec<FileComplexity>> {
    let mut files_paths: Vec<String> = Vec::new();

    // Get all the python files in the directory
    for entry in Walk::new(path) {
        let entry = entry.unwrap();
        let file_path_str = entry.path().to_str().unwrap();

        if entry.path().extension().and_then(|s| s.to_str()) == Some("py") {
            files_paths.push(file_path_str.to_string());
        }
    }

    let files_complexity_result: Result<Vec<FileComplexity>, PyErr> = files_paths
        .par_iter()
        .map(
            |file_path| match file_cognitive_complexity(file_path, max_complexity) {
                Ok(file_complexity) => Ok(file_complexity),
                Err(e) => Err(e),
            },
        )
        .collect();

    match files_complexity_result {
        Ok(files_complexity) => Ok(files_complexity),
        Err(e) => Err(e),
    }
}

/// Calculate the cognitive complexity of a python file.
#[pyfunction]
pub fn file_cognitive_complexity(
    file_path: &str,
    _max_complexity: usize,
) -> PyResult<FileComplexity> {
    let code = std::fs::read_to_string(file_path)?;
    let ast = ast::Suite::parse(&code, "<embedded>").unwrap();

    let mut complexity: u64 = 0;
    let path = path::Path::new(file_path);
    let file_name = path.file_name().unwrap().to_str().unwrap();

    for node in ast.iter() {
        complexity += statement_cognitive_complexity(node.clone(), 0)?;
    }

    println!("{}", file_name);

    Ok(FileComplexity {
        path: file_path.to_string(),
        file_name: file_name.to_string(),
        complexity: complexity,
    })
}

/// Calculate the cognitive complexity of a python statement
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

    if has_recursive_calls(statement.clone()) {
        complexity += 1;
    }

    match statement {
        Stmt::FunctionDef(f) => {
            for node in f.body.iter() {
                complexity += statement_cognitive_complexity(node.clone(), nesting_level)?;
            }
        }
        Stmt::AsyncFunctionDef(f) => {
            for node in f.body.iter() {
                complexity += statement_cognitive_complexity(node.clone(), nesting_level)?;
            }
        }
        Stmt::ClassDef(c) => {
            for node in c.body.iter() {
                complexity += statement_cognitive_complexity(node.clone(), nesting_level)?;
            }
        }
        // breaks in the linear flow
        Stmt::For(f) => {
            complexity += 1;
            for node in f.body.iter() {
                complexity += statement_cognitive_complexity(node.clone(), nesting_level + 1)?;
            }
        }
        Stmt::While(w) => {
            complexity += 1;
            for node in w.body.iter() {
                complexity += statement_cognitive_complexity(node.clone(), nesting_level + 1)?;
            }
        }
        Stmt::If(i) => {
            complexity += 1;
            complexity += count_bool_ops(*i.test);
            for node in i.body.iter() {
                complexity += statement_cognitive_complexity(node.clone(), nesting_level + 1)?;
            }
        }
        Stmt::Try(t) => {
            complexity += 1;
            for node in t.body.iter() {
                complexity += statement_cognitive_complexity(node.clone(), nesting_level + 1)?;
            }

            for handler in t.handlers.iter() {
                match handler {
                    ast::ExceptHandler::ExceptHandler(e) => {
                        for node in e.body.iter() {
                            complexity +=
                                statement_cognitive_complexity(node.clone(), nesting_level + 1)?;
                        }
                    }
                }
            }
        }
        Stmt::Match(m) => {
            complexity += 1;
            for case in m.cases.iter() {
                for node in case.body.iter() {
                    complexity += statement_cognitive_complexity(node.clone(), nesting_level)?;
                }
            }
        }
        Stmt::Break(..) => {
            complexity += 1;
        }
        Stmt::Continue(..) => {
            complexity += 1;
        }
        _ => {}
    };

    if complexity > 0 {
        complexity += nesting_level;
    }

    Ok(complexity)
}
