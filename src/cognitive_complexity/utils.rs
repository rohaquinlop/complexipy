#[cfg(any(feature = "python", feature = "wasm"))]
use ruff_python_ast::{self as ast, Stmt};

#[cfg(feature = "python")]
mod python_deps {
    pub use crate::classes::{FileComplexity, FunctionComplexity};
    pub use csv::Writer;
    pub use pyo3::prelude::*;
    pub use serde_json;
    pub use std::fs::File;
    pub use std::io::Write;
}

#[cfg(feature = "python")]
use python_deps::*;

#[cfg(feature = "python")]
#[pyfunction]
pub fn output_csv(
    invocation_path: &str,
    functions_complexity: Vec<FileComplexity>,
    sort: &str,
    show_detailed_results: bool,
    max_complexity: i32,
) {
    let mut writer = Writer::from_path(invocation_path).unwrap();

    writer
        .write_record(["Path", "File Name", "Function Name", "Cognitive Complexity"])
        .unwrap();

    if sort != "name" {
        let mut all_functions: Vec<(String, String, FunctionComplexity)> = vec![];

        for file in functions_complexity {
            for function in file.functions {
                if show_detailed_results || function.complexity > max_complexity.try_into().unwrap()
                {
                    all_functions.push((file.path.clone(), file.file_name.clone(), function));
                }
            }
        }

        all_functions.sort_by_key(|f| f.2.complexity);

        if sort == "desc" {
            all_functions.reverse();
        }

        for (path, file_name, function) in all_functions.into_iter() {
            writer
                .write_record([
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
                    .write_record([
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
    max_complexity: i32,
) {
    let mut json_data = Vec::new();

    for file in functions_complexity {
        for function in file.functions {
            if show_detailed_results || function.complexity > max_complexity.try_into().unwrap() {
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

    let json_string = serde_json::to_string_pretty(&json_data).unwrap();
    let mut file = File::create(invocation_path).unwrap();
    file.write_all(json_string.as_bytes()).unwrap();
}

#[cfg(feature = "python")]
pub fn get_repo_name(url: &str) -> String {
    let url = url.trim_end_matches('/');

    let repo_name = url.split('/').next_back().unwrap();

    if let Some(name) = repo_name.strip_suffix(".git") {
        return name.to_string();
    }

    repo_name.to_string()
}

#[cfg(any(feature = "python", feature = "wasm"))]
pub fn is_decorator(statement: Stmt) -> bool {
    let mut ans = false;
    if let Stmt::FunctionDef(f) = statement {
        if f.body.len() == 2 {
            ans = matches!(f.body[0].clone(), Stmt::FunctionDef(..))
                && matches!(f.body[1].clone(), Stmt::Return(..));
        }
    }

    ans
}

#[cfg(any(feature = "python", feature = "wasm"))]
pub fn count_bool_ops(expr: ast::Expr, nesting_level: u64) -> u64 {
    let mut complexity: u64 = 0;

    match expr {
        ast::Expr::BoolOp(..) => {
            complexity += 1;
            let b = expr.clone().bool_op_expr().unwrap();
            for value in b.values.iter() {
                complexity += count_different_childs_type(value.clone(), expr.clone());
            }
        }
        ast::Expr::UnaryOp(..) => {
            let u = expr.clone().unary_op_expr().unwrap();
            complexity += 1 + count_different_childs_type(*u.operand, expr.clone());
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
        ast::Expr::FString(f) => {
            for element in f.value.elements() {
                if element.is_interpolation() {
                    let inter = element.as_interpolation().unwrap();
                    complexity += count_bool_ops(*inter.expression.clone(), nesting_level);
                }
            }
        }
        _ => {}
    }

    complexity
}

#[cfg(any(feature = "python", feature = "wasm"))]
fn count_different_childs_type(expr: ast::Expr, prev_pr: ast::Expr) -> u64 {
    let mut complexity: u64 = 0;

    match expr {
        ast::Expr::BoolOp(..) => match prev_pr {
            ast::Expr::BoolOp(p) => {
                let b = expr.clone().bool_op_expr().unwrap().op;
                if b != p.op {
                    complexity += 1;
                }

                for value in p.values {
                    complexity += count_different_childs_type(value, expr.clone());
                }
            }
            ast::Expr::UnaryOp(p) => {
                complexity = 1 + count_different_childs_type(*p.operand, expr);
            }
            _ => {}
        },
        ast::Expr::UnaryOp(..) => match prev_pr {
            ast::Expr::BoolOp(p) => {
                for value in p.values {
                    complexity += count_different_childs_type(value, expr.clone());
                }
            }
            ast::Expr::UnaryOp(p) => {
                complexity = count_different_childs_type(*p.operand, expr);
            }
            _ => {}
        },
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
