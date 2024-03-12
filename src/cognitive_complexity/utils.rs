use crate::classes::{FileComplexity, FunctionComplexity};
use csv::Writer;
use pyo3::prelude::*;
use rustpython_parser::ast::{self, Stmt};

#[pyfunction]
pub fn output_csv_file_level(
    invocation_path: &str,
    files_complexity: Vec<FileComplexity>,
    sort: &str,
) {
    let mut writer = Writer::from_path(invocation_path).unwrap();

    writer
        .write_record(&["Path", "File Name", "Cognitive Complexity"])
        .unwrap();

    if sort != "name" {
        let mut files_complexity = files_complexity;

        files_complexity.sort_by_key(|f| f.complexity);

        if sort == "desc" {
            files_complexity.reverse();
        }

        for file in files_complexity.into_iter() {
            writer
                .write_record(&[&file.path, &file.file_name, &file.complexity.to_string()])
                .unwrap();
        }
    } else {
        for file in &files_complexity {
            writer
                .write_record(&[&file.path, &file.file_name, &file.complexity.to_string()])
                .unwrap();
        }
    }

    writer.flush().unwrap();
}

#[pyfunction]
pub fn output_csv_function_level(
    invocation_path: &str,
    functions_complexity: Vec<FileComplexity>,
    sort: &str,
) {
    let mut writer = Writer::from_path(invocation_path).unwrap();

    writer
        .write_record(&["Path", "File Name", "Function Name", "Cognitive Complexity"])
        .unwrap();

    if sort != "name" {
        let mut all_functions: Vec<(String, String, FunctionComplexity)> = vec![];

        for file in functions_complexity {
            for function in file.functions {
                all_functions.push((file.path.clone(), file.file_name.clone(), function));
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
        ast::Expr::BoolOp(b) => {
            complexity += 1;
            for value in b.values.iter() {
                complexity += count_bool_ops(value.clone());
            }
        }
        ast::Expr::UnaryOp(u) => {
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
