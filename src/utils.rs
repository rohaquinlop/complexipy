#[cfg(any(feature = "python", feature = "wasm"))]
use regex::Regex;
#[cfg(any(feature = "python", feature = "wasm"))]
use ruff_python_ast::{self as ast, Stmt};
#[cfg(any(feature = "python", feature = "wasm"))]
use std::sync::OnceLock;

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
                    "complexity": function.complexity,
                    "refactor_plans": function.refactor_plans
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
    file.write_all(b"\n").map_err(|e| {
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
                if let Some(b) = expr.clone().bool_op_expr()
                    && b.op != p.op
                {
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

/// Extract a canonical ignore comment marker from a line.
///
/// Returns `Some("# complexipy: ignore")` or `Some("# noqa: complexipy")`
/// when the line contains the corresponding pattern (case-insensitive).
/// Returns `None` if neither marker is found.
#[cfg(any(feature = "python", feature = "wasm"))]
pub fn extract_comment_marker(line: &str) -> Option<String> {
    static IGNORE_RE: OnceLock<Regex> = OnceLock::new();
    static NOQA_RE: OnceLock<Regex> = OnceLock::new();

    let ignore_re =
        IGNORE_RE.get_or_init(|| Regex::new(r"(?i)#\s*complexipy\s*:\s*ignore.*").unwrap());
    let noqa_re = NOQA_RE.get_or_init(|| Regex::new(r"(?i)#\s*noqa\s*:\s*complexipy.*").unwrap());

    if ignore_re.is_match(line) {
        return Some("# complexipy: ignore".to_string());
    } else if noqa_re.is_match(line) {
        return Some("# noqa: complexipy".to_string());
    }

    None
}

/// Find a noqa/ignore comment near a `def` or decorator line.
///
/// Returns `Some(comment_text)` when a marker is found that would
/// trigger suppression, `None` otherwise.
#[cfg(any(feature = "python", feature = "wasm"))]
pub fn find_noqa_comment(line_number: u64, code: &str) -> Option<String> {
    if line_number == 0 {
        return None;
    }

    let lines: Vec<&str> = code.lines().collect();
    let idx = (line_number as usize).saturating_sub(1);

    let signature_has_marker = |def_idx: usize| -> Option<String> {
        let max_scan = (def_idx + 20).min(lines.len());
        for line in lines.iter().skip(def_idx).take(max_scan - def_idx) {
            if let Some(marker) = extract_comment_marker(line) {
                return Some(marker);
            }
            if line.contains(':') {
                break;
            }
        }
        None
    };

    if idx < lines.len()
        && let Some(marker) = extract_comment_marker(lines[idx])
    {
        return Some(marker);
    }

    if idx > 0
        && let Some(marker) = extract_comment_marker(lines[idx - 1])
    {
        return Some(marker);
    }

    if idx < lines.len() && lines[idx].trim_start().starts_with("def ") {
        return signature_has_marker(idx);
    }

    if idx < lines.len() && lines[idx].trim_start().starts_with('@') {
        let max_scan = (idx + 10).min(lines.len());
        for i in (idx + 1)..max_scan {
            let line = lines[i].trim();
            if line.starts_with("def ") {
                if let Some(marker) = signature_has_marker(i) {
                    return Some(marker);
                }
                if i > 0
                    && let Some(marker) = extract_comment_marker(lines[i - 1])
                {
                    return Some(marker);
                }
                break;
            }
            if !line.is_empty() && !line.trim_start().starts_with('@') {
                break;
            }
        }
    }

    None
}

#[cfg(any(feature = "python", feature = "wasm"))]
pub fn has_noqa_complexipy(line_number: u64, code: &str) -> bool {
    find_noqa_comment(line_number, code).is_some()
}

/// Collect ignored locations from code, only reporting markers that
/// actually suppress a function definition (i.e., are adjacent to `def`
/// or `@decorator` lines).
#[cfg(any(feature = "python", feature = "wasm"))]
pub fn collect_ignored_locations(code: &str) -> Vec<(u64, String)> {
    let mut results = Vec::new();
    let lines: Vec<&str> = code.lines().collect();
    #[allow(clippy::needless_range_loop)]
    let mut idx = 0;
    while idx < lines.len() {
        let trimmed = lines[idx].trim_start();

        if trimmed.starts_with("def ") {
            let line_number = (idx + 1) as u64;
            if let Some(comment) = find_noqa_comment(line_number, code) {
                results.push((line_number, comment));
            }
            idx += 1;
        } else if trimmed.starts_with('@') {
            // Scan forward to find the def line, use its line number for
            // both the comment lookup and the reported location.
            let max_scan = (idx + 10).min(lines.len());
            let mut def_line_number = None;
            for (inner_idx, inner_line) in lines
                .iter()
                .enumerate()
                .skip(idx + 1)
                .take(max_scan - (idx + 1))
            {
                let inner = inner_line.trim();
                if inner.starts_with("def ") {
                    def_line_number = Some((inner_idx + 1) as u64);
                    break;
                }
                if !inner.is_empty() && !inner.starts_with('@') {
                    break;
                }
            }
            let mut reported = false;
            if let Some(dln) = def_line_number
                && let Some(comment) = find_noqa_comment(dln, code)
            {
                results.push((dln, comment));
                reported = true;
            }
            // Skip remaining decorators in the chain so they don't
            // re-scan and produce duplicate entries.
            while idx + 1 < lines.len() {
                let next = lines[idx + 1].trim_start();
                if next.starts_with('@') || next.is_empty() {
                    idx += 1;
                } else {
                    break;
                }
            }
            // Skip the def line too if we already reported from this chain
            if reported && idx + 1 < lines.len() && lines[idx + 1].trim_start().starts_with("def ")
            {
                idx += 1;
            }
            idx += 1;
        } else {
            idx += 1;
        }
    }

    results
}
