use crate::classes::FileComplexity;
use csv::Writer;
use pyo3::prelude::*;
use rustpython_parser::ast::{self, Stmt};

#[pyfunction]
pub fn output_csv_file_level(invocation_path: &str, files_complexity: Vec<FileComplexity>) {
    let mut writer = Writer::from_path(invocation_path).unwrap();

    writer
        .write_record(&["Path", "File Name", "Cognitive Complexity"])
        .unwrap();

    for file in files_complexity {
        writer
            .write_record(&[&file.path, &file.file_name, &file.complexity.to_string()])
            .unwrap();
    }

    writer.flush().unwrap();
}

#[pyfunction]
pub fn output_csv_function_level(invocation_path: &str, functions_complexity: Vec<FileComplexity>) {
    let mut writer = Writer::from_path(invocation_path).unwrap();

    writer
        .write_record(&["Path", "File Name", "Function Name", "Cognitive Complexity"])
        .unwrap();

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

    writer.flush().unwrap();
}

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

pub fn count_bool_ops(expr: ast::Expr) -> u64 {
    let mut complexity: u64 = 0;

    match expr {
        ast::Expr::BoolOp(..) => {
            complexity += 1;
        }
        ast::Expr::BinOp(b) => {
            complexity += 1;
            complexity += count_bool_ops(*b.left);
            complexity += count_bool_ops(*b.right);
        }
        ast::Expr::UnaryOp(u) => {
            complexity += 1;
            complexity += count_bool_ops(*u.operand);
        }
        ast::Expr::Compare(c) => {
            complexity += count_bool_ops(*c.left);
            for comparator in c.comparators.iter() {
                complexity += count_bool_ops(comparator.clone());
            }
        }
        ast::Expr::IfExp(i) => {
            complexity += 1;
            complexity += count_bool_ops(*i.test);
            complexity += count_bool_ops(*i.body);
            complexity += count_bool_ops(*i.orelse);
        }
        _ => {}
    }

    complexity
}
