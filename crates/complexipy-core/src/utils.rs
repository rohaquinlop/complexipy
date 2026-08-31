use regex::Regex;
use ruff_python_ast::{self as ast, Stmt};
use std::sync::OnceLock;

mod export_deps {
    pub use crate::classes::{FileComplexity, FunctionComplexity};
    pub use csv::Writer;
    pub use serde_json;
    pub use std::fs::File;
    pub use std::io::Write;
}

use export_deps::*;

use std::fmt;

#[derive(Debug, PartialEq)]
pub enum ExportError {
    Io(String),
    Serialize(String),
    InvalidSort(String),
}

impl fmt::Display for ExportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(message) => write!(f, "{}", message),
            Self::Serialize(message) => write!(f, "{}", message),
            Self::InvalidSort(message) => write!(f, "Invalid sort value: {}", message),
        }
    }
}

impl std::error::Error for ExportError {}

pub fn output_csv_shared(
    invocation_path: &str,
    functions_complexity: Vec<FileComplexity>,
    sort: &str,
    show_detailed_results: bool,
    max_complexity: u64,
) -> Result<(), ExportError> {
    match sort {
        "asc" | "desc" | "name" | "file_name" => {}
        other => return Err(ExportError::InvalidSort(other.to_string())),
    }

    let mut writer = Writer::from_path(invocation_path).map_err(|e| {
        ExportError::Io(format!(
            "Failed to create CSV at {}: {}",
            invocation_path, e
        ))
    })?;

    writer
        .write_record(["Path", "File Name", "Function Name", "Cognitive Complexity"])
        .map_err(|e| {
            ExportError::Io(format!(
                "Failed to write CSV header at {}: {}",
                invocation_path, e
            ))
        })?;

    let mut all_functions: Vec<(String, String, FunctionComplexity)> = vec![];
    for file in functions_complexity {
        for function in file.functions {
            if show_detailed_results || function.complexity > max_complexity {
                all_functions.push((file.path.clone(), file.file_name.clone(), function));
            }
        }
    }

    match sort {
        "desc" => {
            all_functions.sort_by_key(|f| f.2.complexity);
            all_functions.reverse();
        }
        "asc" => all_functions.sort_by_key(|f| f.2.complexity),
        "name" | "file_name" => {
            all_functions.sort_by(|left, right| left.0.cmp(&right.0).then(left.1.cmp(&right.1)))
        }
        other => return Err(ExportError::InvalidSort(other.to_string())),
    }

    for (path, file_name, function) in all_functions {
        writer
            .write_record([
                &path,
                &file_name,
                &function.name,
                &function.complexity.to_string(),
            ])
            .map_err(|e| ExportError::Io(format!("Failed to write CSV row: {}", e)))?;
    }

    writer
        .flush()
        .map_err(|e| ExportError::Io(format!("Failed to flush CSV to disk: {}", e)))?;

    Ok(())
}

pub fn output_json_shared(
    invocation_path: &str,
    functions_complexity: Vec<FileComplexity>,
    show_detailed_results: bool,
    max_complexity: u64,
    suggest_refactors: bool,
) -> Result<(), ExportError> {
    let mut json_data = Vec::new();

    for file in functions_complexity {
        for function in file.functions {
            if show_detailed_results || function.complexity > max_complexity {
                let refactor_plans = if suggest_refactors {
                    function.refactor_plans
                } else {
                    Vec::new()
                };
                let entry = serde_json::json!({
                    "path": file.path,
                    "file_name": file.file_name,
                    "function_name": function.name,
                    "complexity": function.complexity,
                    "refactor_plans": refactor_plans
                });
                json_data.push(entry);
            }
        }
    }

    let json_string = serde_json::to_string_pretty(&json_data)
        .map_err(|e| ExportError::Serialize(format!("Failed to serialize JSON: {}", e)))?;
    let mut file = File::create(invocation_path).map_err(|e| {
        ExportError::Io(format!(
            "Failed to create JSON file at {}: {}",
            invocation_path, e
        ))
    })?;
    file.write_all(json_string.as_bytes()).map_err(|e| {
        ExportError::Io(format!(
            "Failed to write JSON to {}: {}",
            invocation_path, e
        ))
    })?;
    file.write_all(b"\n").map_err(|e| {
        ExportError::Io(format!(
            "Failed to write JSON to {}: {}",
            invocation_path, e
        ))
    })?;

    Ok(())
}

pub fn create_snapshot_file_shared(
    snapshot_file_path: &str,
    max_complexity: u64,
    files_complexities: Vec<FileComplexity>,
) -> Result<(), ExportError> {
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
        .map_err(|e| ExportError::Serialize(format!("Failed to serialize JSON: {}", e)))?;
    let mut file = File::create(snapshot_file_path).map_err(|e| {
        ExportError::Io(format!(
            "Failed to create snapshot file at {}: {}",
            snapshot_file_path, e
        ))
    })?;
    file.write_all(json_string.as_bytes()).map_err(|e| {
        ExportError::Io(format!(
            "Failed to write snapshot file at {}: {}",
            snapshot_file_path, e
        ))
    })?;

    Ok(())
}

pub fn load_snapshot_file_shared(
    snapshot_file_path: &str,
) -> Result<Vec<FileComplexity>, ExportError> {
    let snapshot_content = std::fs::read_to_string(snapshot_file_path).map_err(|e| {
        ExportError::Io(format!(
            "Failed to read snapshot file {}: {}",
            snapshot_file_path, e
        ))
    })?;
    serde_json::from_str(snapshot_content.as_str())
        .map_err(|e| ExportError::Serialize(format!("Failed to parse snapshot JSON: {}", e)))
}

pub struct LineIndex {
    newlines: Vec<usize>,
}

impl LineIndex {
    pub fn new(code: &str) -> Self {
        Self {
            newlines: code
                .bytes()
                .enumerate()
                .filter(|(_, b)| *b == b'\n')
                .map(|(i, _)| i)
                .collect(),
        }
    }

    pub fn line_of(&self, byte: usize) -> u64 {
        (self.newlines.partition_point(|&offset| offset < byte) + 1) as u64
    }

    pub fn column_of(&self, byte: usize, code: &str) -> u64 {
        let idx = self.newlines.partition_point(|&offset| offset < byte);
        let line_start = match idx {
            0 => 0,
            _ => self.newlines[idx - 1] + 1,
        };
        (code[line_start..byte].chars().count() + 1) as u64
    }

    pub fn byte_of_line(&self, line: u64) -> Option<usize> {
        match line {
            0 => None,
            1 => Some(0),
            _ => self.newlines.get((line - 2) as usize).map(|&i| i + 1),
        }
    }
}

pub fn is_decorator(statement: &Stmt) -> bool {
    if let Stmt::FunctionDef(f) = statement
        && f.body.len() == 2
    {
        return matches!(&f.body[0], Stmt::FunctionDef(..))
            && matches!(&f.body[1], Stmt::Return(..));
    }
    false
}

pub fn count_bool_ops(expr: &ast::Expr, nesting_level: u64) -> u64 {
    let mut complexity: u64 = 0;

    match expr {
        ast::Expr::BoolOp(b) => {
            complexity += 1;
            for value in b.values.iter() {
                complexity += count_different_childs_type(value, expr);
            }
        }
        ast::Expr::UnaryOp(u) => {
            complexity += count_different_childs_type(&u.operand, expr);
        }
        ast::Expr::Compare(c) => {
            complexity += count_bool_ops(&c.left, nesting_level);
            for comparator in c.comparators.iter() {
                complexity += count_bool_ops(comparator, nesting_level);
            }
        }
        ast::Expr::If(i) => {
            complexity += 1 + nesting_level;
            complexity += count_bool_ops(&i.test, nesting_level);
            complexity += count_bool_ops(&i.body, nesting_level + 1);
            complexity += count_bool_ops(&i.orelse, nesting_level + 1);
        }
        ast::Expr::Lambda(l) => {
            complexity += count_bool_ops(&l.body, nesting_level + 1);
        }
        ast::Expr::ListComp(c) => {
            complexity += count_comprehension(&[&c.elt], &c.generators, nesting_level);
        }
        ast::Expr::SetComp(c) => {
            complexity += count_comprehension(&[&c.elt], &c.generators, nesting_level);
        }
        ast::Expr::Generator(c) => {
            complexity += count_comprehension(&[&c.elt], &c.generators, nesting_level);
        }
        ast::Expr::DictComp(c) => {
            complexity += count_comprehension(&[&c.key, &c.value], &c.generators, nesting_level);
        }
        ast::Expr::Call(c) => {
            for arg in c.arguments.args.iter() {
                complexity += count_bool_ops(arg, nesting_level);
            }
        }
        ast::Expr::Tuple(t) => {
            for element in t.elts.iter() {
                complexity += count_bool_ops(element, nesting_level);
            }
        }
        ast::Expr::List(l) => {
            for element in l.elts.iter() {
                complexity += count_bool_ops(element, nesting_level);
            }
        }
        ast::Expr::Set(s) => {
            for element in s.elts.iter() {
                complexity += count_bool_ops(element, nesting_level);
            }
        }
        ast::Expr::Dict(d) => {
            for value in d.iter_values() {
                complexity += count_bool_ops(value, nesting_level);
            }
        }
        _ => {}
    }

    complexity
}

fn count_comprehension(
    elements: &[&ast::Expr],
    generators: &[ast::Comprehension],
    nesting_level: u64,
) -> u64 {
    let mut complexity: u64 = 0;
    let inner_nesting = nesting_level + 1;

    for generator in generators.iter() {
        complexity += 1 + nesting_level;
        complexity += count_bool_ops(&generator.iter, nesting_level);
        for filter in generator.ifs.iter() {
            complexity += 1;
            complexity += count_bool_ops(filter, inner_nesting);
        }
    }

    for element in elements.iter() {
        complexity += count_bool_ops(element, inner_nesting);
    }

    complexity
}

fn count_different_childs_type(expr: &ast::Expr, prev_pr: &ast::Expr) -> u64 {
    let mut complexity: u64 = 0;

    match expr {
        ast::Expr::BoolOp(b) => match prev_pr {
            ast::Expr::BoolOp(p) => {
                if b.op != p.op {
                    complexity += 1;
                }
                for value in p.values.iter() {
                    complexity += count_different_childs_type(value, expr);
                }
            }
            ast::Expr::UnaryOp(p) => {
                complexity = 1 + count_different_childs_type(&p.operand, expr);
            }
            _ => {}
        },
        ast::Expr::UnaryOp(..) => match prev_pr {
            ast::Expr::BoolOp(p) => {
                for value in p.values.iter() {
                    complexity += count_different_childs_type(value, expr);
                }
            }
            ast::Expr::UnaryOp(p) => {
                complexity = count_different_childs_type(&p.operand, expr);
            }
            _ => {}
        },
        _ => {}
    }

    complexity
}

fn line_start_of(code: &str, offset: usize) -> usize {
    code[..offset].rfind('\n').map_or(0, |i| i + 1)
}

fn line_end_of(code: &str, offset: usize) -> usize {
    code[offset..].find('\n').map_or(code.len(), |i| offset + i)
}

fn line_at(code: &str, line_start: usize) -> &str {
    &code[line_start..line_end_of(code, line_start)]
}

fn next_line_start(code: &str, line_start: usize) -> Option<usize> {
    let end = line_end_of(code, line_start);
    (end < code.len()).then_some(end + 1)
}

pub fn collect_def_names(code: &str) -> std::collections::HashSet<String> {
    code.lines()
        .filter_map(|line| {
            let trimmed = line.trim_start();
            let rest = trimmed
                .strip_prefix("def ")
                .or_else(|| trimmed.strip_prefix("async def "))?;
            rest.split('(').next().map(|name| name.trim().to_string())
        })
        .collect()
}

/// Extract a canonical ignore comment marker from a line.
///
/// Returns `Some("# complexipy: ignore")` or `Some("# noqa: complexipy")`
/// when the line contains the corresponding pattern (case-insensitive).
/// Returns `None` if neither marker is found.
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
pub fn find_noqa_comment(byte_offset: usize, code: &str) -> Option<String> {
    let line_start = line_start_of(code, byte_offset);
    let current_line = line_at(code, line_start);

    let signature_has_marker = |def_line_start: usize| -> Option<String> {
        let mut pos = def_line_start;
        for _ in 0..20 {
            let line = line_at(code, pos);
            if let Some(marker) = extract_comment_marker(line) {
                return Some(marker);
            }
            if line.contains(':') {
                break;
            }
            pos = next_line_start(code, pos)?;
        }
        None
    };

    if let Some(marker) = extract_comment_marker(current_line) {
        return Some(marker);
    }

    if line_start > 0 {
        let prev_start = line_start_of(code, line_start - 1);
        if let Some(marker) = extract_comment_marker(line_at(code, prev_start)) {
            return Some(marker);
        }
    }

    if current_line.trim_start().starts_with("def ") {
        return signature_has_marker(line_start);
    }

    if current_line.trim_start().starts_with('@') {
        let mut pos = line_start;
        for _ in 0..10 {
            pos = next_line_start(code, pos)?;
            let line = line_at(code, pos);
            let trimmed = line.trim();
            if trimmed.starts_with("def ") {
                if let Some(marker) = signature_has_marker(pos) {
                    return Some(marker);
                }
                if pos > 0 {
                    let prev_start = line_start_of(code, pos - 1);
                    if let Some(marker) = extract_comment_marker(line_at(code, prev_start)) {
                        return Some(marker);
                    }
                }
                break;
            }
            if !trimmed.is_empty() && !line.trim_start().starts_with('@') {
                break;
            }
        }
    }

    None
}

pub fn has_noqa_complexipy(byte_offset: usize, code: &str) -> bool {
    find_noqa_comment(byte_offset, code).is_some()
}

/// Collect ignored locations from code, only reporting markers that
/// actually suppress a function definition (i.e., are adjacent to `def`
/// or `@decorator` lines).
pub fn collect_ignored_locations(code: &str) -> Vec<(u64, String)> {
    let index = LineIndex::new(code);
    let mut results = Vec::new();
    let mut line_start = 0;

    while line_start < code.len() {
        let line = line_at(code, line_start);
        let trimmed = line.trim_start();

        if trimmed.starts_with("def ") {
            if let Some(comment) = find_noqa_comment(line_start, code) {
                results.push((index.line_of(line_start), comment));
            }
            line_start = next_line_start(code, line_start).unwrap_or(code.len());
        } else if trimmed.starts_with('@') {
            let mut def_offset = None;
            let mut pos = line_start;
            for _ in 0..10 {
                let Some(next) = next_line_start(code, pos) else {
                    break;
                };
                pos = next;
                let inner = line_at(code, pos).trim();
                if inner.starts_with("def ") {
                    def_offset = Some(pos);
                    break;
                }
                if !inner.is_empty() && !inner.starts_with('@') {
                    break;
                }
            }
            let mut reported = false;
            if let Some(def_line_start) = def_offset
                && let Some(comment) = find_noqa_comment(def_line_start, code)
            {
                results.push((index.line_of(def_line_start), comment));
                reported = true;
            }
            while let Some(next) = next_line_start(code, line_start) {
                let next_line = line_at(code, next);
                if next_line.trim_start().starts_with('@') || next_line.trim_start().is_empty() {
                    line_start = next;
                } else {
                    break;
                }
            }
            if reported
                && let Some(next) = next_line_start(code, line_start)
                && line_at(code, next).trim_start().starts_with("def ")
            {
                line_start = next;
            }
            line_start = next_line_start(code, line_start).unwrap_or(code.len());
        } else {
            line_start = next_line_start(code, line_start).unwrap_or(code.len());
        }
    }

    results
}

/// Keep only the ignored locations whose containing function no longer
/// exceeds the allowed complexity threshold.
///
/// Returns `(line, comment, function_name, complexity)` for each removable
/// marker; the function's complexity is measured without the marker applied.
pub fn filter_removable_ignores(
    locations: &[(u64, String)],
    functions: &[FunctionComplexity],
    max_complexity_allowed: u64,
) -> Vec<(u64, String, String, u64)> {
    let mut removable = Vec::new();
    for (line, comment) in locations {
        if let Some(function) = functions
            .iter()
            .find(|f| *line >= f.line_start && *line <= f.line_end)
            && function.complexity <= max_complexity_allowed
        {
            removable.push((
                *line,
                comment.clone(),
                function.name.clone(),
                function.complexity,
            ));
        }
    }
    removable
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod export_tests;
