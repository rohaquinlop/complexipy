mod utils;

use crate::classes::FileComplexity;
use pyo3::prelude::*;
use rustpython_parser::{
    ast::{self, Stmt},
    Parse,
};
use std::{path, time::Instant};
use utils::{count_bool_ops, has_recursive_calls, is_decorator};

/// Calculate the cognitive complexity of a python file.
#[pyfunction]
pub fn file_cognitive_complexity(
    file_path: &str,
    max_complexity: usize,
) -> PyResult<FileComplexity> {
    let code = std::fs::read_to_string(file_path)?;
    let ast = ast::Suite::parse(&code, "<embedded>").unwrap();

    let mut complexity: u64 = 0;

    let start_time = Instant::now();

    for node in ast.iter() {
        // println!("{:#?}", node);
        complexity += statement_cognitive_complexity(node.clone(), 0)?;
    }

    if max_complexity > 0 && complexity > max_complexity as u64 {
        return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
            "Cognitive complexity too high",
        ));
    }

    let path = path::Path::new(file_path);
    let file_name = path.file_name().unwrap().to_str().unwrap();

    let elapsed_time = start_time.elapsed();
    println!(
        "{}: Analysis time: {:} ms.",
        file_name,
        elapsed_time.as_millis()
    );

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
