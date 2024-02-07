use crate::classes::{FileComplexity, FunctionComplexity};
use pyo3::prelude::*;
use rustpython_parser::{
    ast::{self, Stmt},
    Parse,
};

/// Calculate the cognitive complexity of a python file.
#[pyfunction]
pub fn file_cognitive_complexity(
    file_path: &str,
    max_complexity: usize,
) -> PyResult<FileComplexity> {
    let code = std::fs::read_to_string(file_path)?;
    let ast = ast::Suite::parse(&code, "<embedded>").unwrap();

    let mut complexity: u64 = 0;

    for node in ast.iter() {
        println!("{:#?}", node);
        complexity += statement_cognitive_complexity(node.clone())?;
    }

    if max_complexity > 0 && complexity > max_complexity as u64 {
        return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
            "Cognitive complexity too high",
        ));
    }

    Ok(FileComplexity {
        path: file_path.to_string(),
        functions: vec![FunctionComplexity {
            name: "foo".to_string(),
            complexity: 42,
        }],
        complexity: complexity,
    })
}

/// Calculate the cognitive complexity of a python statement
fn statement_cognitive_complexity(statement: Stmt) -> PyResult<u64> {
    let mut complexity: u64 = 0;

    match statement {
        Stmt::FunctionDef(f) => {
            // Add code here if needed
            complexity += 1;
            for node in f.body.iter() {
                complexity += statement_cognitive_complexity(node.clone())?;
            }
        }
        Stmt::ClassDef(c) => {
            // Add code here if needed
            complexity += 1;
            for node in c.body.iter() {
                complexity += statement_cognitive_complexity(node.clone())?;
            }
        }
        Stmt::If(i) => {
            // Add code here if needed
            complexity += 1;
            complexity += count_bool_ops(*i.test);
            for node in i.body.iter() {
                complexity += statement_cognitive_complexity(node.clone())?;
            }
            for node in i.orelse.iter() {
                complexity += statement_cognitive_complexity(node.clone())?;
            }
        }
        Stmt::While(w) => {
            // Add code here if needed
            complexity += 1;
            for node in w.body.iter() {
                complexity += statement_cognitive_complexity(node.clone())?;
            }
            for node in w.orelse.iter() {
                complexity += statement_cognitive_complexity(node.clone())?;
            }
        }
        Stmt::For(f) => {
            // Add code here if needed
            complexity += 1;
            for node in f.body.iter() {
                complexity += statement_cognitive_complexity(node.clone())?;
            }
            for node in f.orelse.iter() {
                complexity += statement_cognitive_complexity(node.clone())?;
            }
        }
        Stmt::Try(t) => {
            // Add code here if needed
            complexity += 1;
            for node in t.body.iter() {
                complexity += statement_cognitive_complexity(node.clone())?;
            }
            for node in t.orelse.iter() {
                complexity += statement_cognitive_complexity(node.clone())?;
            }
            for node in t.finalbody.iter() {
                complexity += statement_cognitive_complexity(node.clone())?;
            }
        }
        Stmt::With(w) => {
            // Add code here if needed
            complexity += 1;
            for node in w.body.iter() {
                complexity += statement_cognitive_complexity(node.clone())?;
            }
        }
        _ => {}
    };

    Ok(complexity)
}

fn count_bool_ops(expr: ast::Expr) -> u64 {
    let mut complexity: u64 = 0;

    match expr {
        ast::Expr::BoolOp(b) => {
            complexity += b.values.len() as u64 - 1;
            for value in b.values.iter() {
                complexity += count_bool_ops(value.clone());
            }
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
