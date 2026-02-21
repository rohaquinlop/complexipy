#[cfg(any(feature = "python", feature = "wasm"))]
use ruff_python_ast::{self as ast, Stmt};

#[cfg(feature = "python")]
mod python_deps {
    pub use crate::classes::{FileComplexity, FunctionComplexity};
    pub use csv::Writer;
    pub use pyo3::exceptions::{PyIOError, PyValueError};
    pub use pyo3::prelude::*;
    pub use serde_json;
    pub use std::fs::{File, read_to_string};
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
) -> PyResult<()> {
    let mut writer = Writer::from_path(invocation_path).map_err(|e| {
        PyIOError::new_err(format!(
            "Failed to create CSV at {}: {}",
            invocation_path, e
        ))
    })?;

    writer
        .write_record(["Path", "File Name", "Function Name", "Cognitive Complexity"])
        .map_err(|e| {
            PyIOError::new_err(format!(
                "Failed to write CSV header at {}: {}",
                invocation_path, e
            ))
        })?;

    let max_complexity_limit = u64::try_from(max_complexity)
        .map_err(|_| PyValueError::new_err("max_complexity must be non-negative"))?;

    if sort != "name" {
        let mut all_functions: Vec<(String, String, FunctionComplexity)> = vec![];

        for file in functions_complexity {
            for function in file.functions {
                if show_detailed_results || function.complexity > max_complexity_limit {
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
                .map_err(|e| PyIOError::new_err(format!("Failed to write CSV row: {}", e)))?;
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
                    .map_err(|e| PyIOError::new_err(format!("Failed to write CSV row: {}", e)))?;
            }
        }
    }

    writer
        .flush()
        .map_err(|e| PyIOError::new_err(format!("Failed to flush CSV to disk: {}", e)))?;

    Ok(())
}

#[cfg(feature = "python")]
#[pyfunction]
pub fn output_json(
    invocation_path: &str,
    functions_complexity: Vec<FileComplexity>,
    show_detailed_results: bool,
    max_complexity: i32,
) -> PyResult<()> {
    let mut json_data = Vec::new();
    let max_complexity_limit = u64::try_from(max_complexity)
        .map_err(|_| PyValueError::new_err("max_complexity must be non-negative"))?;

    for file in functions_complexity {
        for function in file.functions {
            if show_detailed_results || function.complexity > max_complexity_limit {
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

    let json_string = serde_json::to_string_pretty(&json_data)
        .map_err(|e| PyValueError::new_err(format!("Failed to serialize JSON: {}", e)))?;
    let mut file = File::create(invocation_path).map_err(|e| {
        PyIOError::new_err(format!(
            "Failed to create JSON file at {}: {}",
            invocation_path, e
        ))
    })?;
    file.write_all(json_string.as_bytes()).map_err(|e| {
        PyIOError::new_err(format!(
            "Failed to write JSON to {}: {}",
            invocation_path, e
        ))
    })?;

    Ok(())
}

#[cfg(feature = "python")]
#[pyfunction]
pub fn create_snapshot_file(
    snapshot_file_path: &str,
    max_complexity: u64,
    files_complexities: Vec<FileComplexity>,
) -> PyResult<()> {
    let files_snapshot: Vec<FileComplexity> = files_complexities
        .into_iter()
        .filter_map(|file_complexity| {
            let functions: Vec<FunctionComplexity> = file_complexity
                .functions
                .into_iter()
                .filter(|function| function.complexity > max_complexity)
                .collect();

            if functions.is_empty() {
                None
            } else {
                Some(FileComplexity {
                    functions,
                    ..file_complexity
                })
            }
        })
        .collect();

    let json_string = serde_json::to_string_pretty(&files_snapshot)
        .map_err(|e| PyValueError::new_err(format!("Failed to serialize JSON: {}", e)))?;
    let mut file = File::create(snapshot_file_path).map_err(|e| {
        PyIOError::new_err(format!(
            "Failed to create snapshot file at {}: {}",
            snapshot_file_path, e
        ))
    })?;
    file.write_all(json_string.as_bytes()).map_err(|e| {
        PyIOError::new_err(format!(
            "Failed to write snapshot file at {}: {}",
            snapshot_file_path, e
        ))
    })?;

    Ok(())
}

#[cfg(feature = "python")]
#[pyfunction]
pub fn load_snapshot_file(snapshot_file_path: &str) -> PyResult<Vec<FileComplexity>> {
    let snapshot_content = read_to_string(snapshot_file_path).map_err(|e| {
        PyIOError::new_err(format!(
            "Failed to read snapshot file {}: {}",
            snapshot_file_path, e
        ))
    })?;
    serde_json::from_str(snapshot_content.as_str())
        .map_err(|e| PyValueError::new_err(format!("Failed to parse snapshot JSON: {}", e)))
}

#[cfg(feature = "python")]
pub fn get_repo_name(url: &str) -> PyResult<String> {
    let url = url.trim_end_matches('/');

    let repo_name = url
        .split('/')
        .next_back()
        .filter(|name| !name.is_empty())
        .ok_or_else(|| PyValueError::new_err("Repository URL is missing a final path segment"))?;

    Ok(repo_name
        .strip_suffix(".git")
        .unwrap_or(repo_name)
        .to_string())
}

#[cfg(any(feature = "python", feature = "wasm"))]
pub fn is_decorator(statement: Stmt) -> bool {
    let mut ans = false;
    if let Stmt::FunctionDef(f) = statement
        && f.body.len() == 2
    {
        ans = matches!(f.body[0].clone(), Stmt::FunctionDef(..))
            && matches!(f.body[1].clone(), Stmt::Return(..));
    }

    ans
}

#[cfg(any(feature = "python", feature = "wasm"))]
pub fn count_bool_ops(expr: ast::Expr, nesting_level: u64) -> u64 {
    let mut complexity: u64 = 0;

    match expr {
        ast::Expr::BoolOp(..) => {
            complexity += 1;
            if let Some(b) = expr.clone().bool_op_expr() {
                for value in b.values.iter() {
                    complexity += count_different_childs_type(value.clone(), expr.clone());
                }
            }
        }
        ast::Expr::UnaryOp(..) => {
            if let Some(u) = expr.clone().unary_op_expr() {
                complexity += count_different_childs_type(*u.operand, expr.clone());
            }
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
                    if let Some(inter) = element.as_interpolation() {
                        complexity += count_bool_ops(*inter.expression.clone(), nesting_level);
                    }
                }
            }
        }
        ast::Expr::ListComp(l) => {
            complexity += count_comprehension_complexity(
                &l.generators,
                *l.elt.clone(),
                nesting_level,
            );
        }
        ast::Expr::SetComp(s) => {
            complexity += count_comprehension_complexity(
                &s.generators,
                *s.elt.clone(),
                nesting_level,
            );
        }
        ast::Expr::Generator(g) => {
            complexity += count_comprehension_complexity(
                &g.generators,
                *g.elt.clone(),
                nesting_level,
            );
        }
        ast::Expr::DictComp(d) => {
            // Score the generators+key together; then scan the value expression.
            complexity += count_comprehension_complexity(
                &d.generators,
                *d.key.clone(),
                nesting_level,
            );
            complexity += count_bool_ops(*d.value.clone(), nesting_level + 1);
        }
        _ => {}
    }

    complexity
}

/// Score a comprehension (list / set / generator / dict) expression.
///
/// Rules applied:
/// - The comprehension itself: `+1 + nesting_level` (consistent with how
///   other control-flow constructs are penalised for depth).
/// - Each additional `for` clause beyond the first: `+1`.
/// - Each `if` filter inside any generator: `+1`, plus any boolean-operator
///   complexity within that condition.
/// - The element expression (`elt`) and each iterator expression are
///   recursed into at `nesting_level + 1` so that nested comprehensions
///   receive an appropriate depth penalty.
#[cfg(any(feature = "python", feature = "wasm"))]
fn count_comprehension_complexity(
    generators: &[ast::Comprehension],
    elt: ast::Expr,
    nesting_level: u64,
) -> u64 {
    let mut complexity: u64 = 1 + nesting_level;

    for (i, clause) in generators.iter().enumerate() {
        if i > 0 {
            // Each additional `for` clause makes the comprehension harder to read.
            complexity += 1;
        }
        for if_expr in clause.ifs.iter() {
            // The `if` filter itself costs 1, plus any boolean operators inside.
            complexity += 1 + count_bool_ops(if_expr.clone(), nesting_level + 1);
        }
        // Check the iterator expression for nested comprehensions or bool ops.
        complexity += count_bool_ops(clause.iter.clone(), nesting_level + 1);
    }

    // Recurse into the element expression to catch nested comprehensions.
    complexity += count_bool_ops(elt, nesting_level + 1);

    complexity
}

#[cfg(any(feature = "python", feature = "wasm"))]
fn count_different_childs_type(expr: ast::Expr, prev_pr: ast::Expr) -> u64 {
    let mut complexity: u64 = 0;

    match expr {
        ast::Expr::BoolOp(..) => match prev_pr {
            ast::Expr::BoolOp(p) => {
                if let Some(b) = expr.clone().bool_op_expr() {
                    if b.op != p.op {
                        complexity += 1;
                    }
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

#[cfg(any(feature = "python", feature = "wasm"))]
pub fn has_noqa_complexipy(line_number: u64, code: &str) -> bool {
    if line_number == 0 {
        return false;
    }

    let lines: Vec<&str> = code.lines().collect();
    let idx = (line_number as usize).saturating_sub(1);

    let contains_marker = |s: &str| -> bool {
        let lower = s.to_lowercase();
        lower.contains("noqa: complexipy")
    };

    if idx < lines.len() && contains_marker(lines[idx]) {
        return true;
    }

    if idx > 0 && contains_marker(lines[idx - 1]) {
        return true;
    }


    // handle decorated functions
    if idx < lines.len() && lines[idx].trim_start().starts_with('@') {
        let max_scan = (idx + 10).min(lines.len());
        for i in (idx + 1)..max_scan {
            let line = lines[i].trim();
            if line.starts_with("def ") {
                if contains_marker(lines[i]) {
                    return true;
                }
                if i > 0 && contains_marker(lines[i - 1]) {
                    return true;
                }
                break;
            }
            if !line.is_empty() && !line.trim_start().starts_with('@') {
                break;
            }
        }
    }

    false
}
