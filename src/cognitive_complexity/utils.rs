#[cfg(feature = "python")]
use crate::classes::{FileComplexity, FunctionComplexity};
#[cfg(feature = "python")]
use csv::Writer;
#[cfg(feature = "python")]
use pyo3::prelude::*;
#[cfg(any(feature = "python", feature = "wasm"))]
use ruff_python_ast::{self as ast, Stmt};
#[cfg(feature = "python")]
use serde_json;
#[cfg(feature = "python")]
use std::fs::File;
#[cfg(feature = "python")]
use std::io::Write;

#[cfg(feature = "python")]
#[pyfunction]
pub fn output_csv(
    invocation_path: &str,
    functions_complexity: Vec<FileComplexity>,
    sort: &str,
    show_detailed_results: bool,
) {
    let mut writer = Writer::from_path(invocation_path).unwrap();

    writer
        .write_record(&["Path", "File Name", "Function Name", "Cognitive Complexity"])
        .unwrap();

    if sort != "name" {
        let mut all_functions: Vec<(String, String, FunctionComplexity)> = vec![];

        for file in functions_complexity {
            for function in file.functions {
                if show_detailed_results {
                    all_functions.push((file.path.clone(), file.file_name.clone(), function));
                } else {
                    if function.complexity >= 15 {
                        all_functions.push((file.path.clone(), file.file_name.clone(), function));
                    }
                }
            }
        }

        all_functions.sort_by_key(|f| f.2.complexity);

        if sort == "desc" {
            all_functions.reverse();
        }

        for (path, file_name, function) in all_functions.into_iter() {
            writer
                .write_record(&[
                    &path,
                    &file_name,
                    &function.name,
                    &function.complexity.to_string(),
                ])
                .unwrap();
        }
    } else {
        for file in functions_complexity {
            for function in file.functions {
                writer
                    .write_record(&[
                        &file.path,
                        &file.file_name,
                        &function.name,
                        &function.complexity.to_string(),
                    ])
                    .unwrap();
            }
        }
    }

    writer.flush().unwrap();
}

#[cfg(feature = "python")]
#[pyfunction]
pub fn output_json(
    invocation_path: &str,
    functions_complexity: Vec<FileComplexity>,
    show_detailed_results: bool,
) {
    let mut json_data = Vec::new();

    for file in functions_complexity {
        for function in file.functions {
            if show_detailed_results {
                let entry = serde_json::json!({
                    "path": file.path,
                    "file_name": file.file_name,
                    "function_name": function.name,
                    "complexity": function.complexity
                });
                json_data.push(entry);
            } else {
                if function.complexity >= 15 {
                    let entry = serde_json::json!({
                        "path": file.path,
                        "file_name": file.file_name,
                        "function_name": function.name,
                        "complexity": function.complexity
                    });
                    json_data.push(entry);
                }
            }
        }
    }

    let json_string = serde_json::to_string_pretty(&json_data).unwrap();
    let mut file = File::create(invocation_path).unwrap();
    file.write_all(json_string.as_bytes()).unwrap();
}

#[cfg(feature = "python")]
pub fn get_repo_name(url: &str) -> String {
    let url = url.trim_end_matches('/');

    let repo_name = url.split('/').last().unwrap();

    let repo_name = if repo_name.ends_with(".git") {
        &repo_name[..repo_name.len() - 4]
    } else {
        repo_name
    };

    repo_name.to_string()
}

#[cfg(any(feature = "python", feature = "wasm"))]
pub fn is_decorator(statement: Stmt) -> bool {
    let mut ans = false;
    match statement {
        Stmt::FunctionDef(f) => {
            if f.body.len() == 2 {
                ans =
                    true && match f.body[0].clone() {
                        Stmt::FunctionDef(..) => true,
                        _ => false,
                    } && match f.body[1].clone() {
                        Stmt::Return(..) => true,
                        _ => false,
                    };
            }
        }
        _ => {}
    }

    ans
}

#[cfg(any(feature = "python", feature = "wasm"))]
pub fn count_bool_ops(expr: ast::Expr, nesting_level: u64) -> u64 {
    let mut complexity: u64 = 0;

    match expr {
        ast::Expr::BoolOp(b) => {
            complexity += 1;
            for value in b.values.iter() {
                complexity += count_bool_ops(value.clone(), nesting_level);
            }
        }
        ast::Expr::UnaryOp(u) => {
            complexity += count_bool_ops(*u.operand, nesting_level);
        }
        ast::Expr::Compare(c) => {
            complexity += count_bool_ops(*c.left, nesting_level);
            for comparator in c.comparators.iter() {
                complexity += count_bool_ops(comparator.clone(), nesting_level);
            }
        }
        ast::Expr::If(i) => {
            complexity += 1 + nesting_level;
            complexity += count_bool_ops(*i.test, nesting_level);
            complexity += count_bool_ops(*i.body, nesting_level);
            complexity += count_bool_ops(*i.orelse, nesting_level);
        }
        ast::Expr::Call(c) => {
            for arg in c.arguments.args.iter() {
                complexity += count_bool_ops(arg.clone(), nesting_level);
            }
        }
        ast::Expr::Tuple(t) => {
            for element in t.elts.iter() {
                complexity += count_bool_ops(element.clone(), nesting_level);
            }
        }
        ast::Expr::List(l) => {
            for element in l.elts.iter() {
                complexity += count_bool_ops(element.clone(), nesting_level);
            }
        }
        ast::Expr::Set(s) => {
            for element in s.elts.iter() {
                complexity += count_bool_ops(element.clone(), nesting_level);
            }
        }
        ast::Expr::Dict(d) => {
            for value in d.iter_values() {
                complexity += count_bool_ops(value.clone(), nesting_level);
            }
        }
        _ => {}
    }

    complexity
}

#[cfg(any(feature = "python", feature = "wasm"))]
pub fn get_line_number(byte_index: usize, code: &str) -> u64 {
    let before_slice = &code[..byte_index];
    let newline_count = before_slice.chars().filter(|&c| c == '\n').count();
    (newline_count + 1) as u64
}
