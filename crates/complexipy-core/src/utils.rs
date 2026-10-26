use regex::Regex;
use ruff_python_ast::{self as ast, Stmt};
use std::sync::OnceLock;

mod export_deps {
    pub use crate::classes::{FileComplexity, FunctionComplexity};
    pub use crate::fs_guard::write_atomic;
    pub use crate::sanitize::neutralize_csv_cell;
    pub use csv::Writer;
    pub use serde_json;
    pub use std::path::Path;
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

    let mut writer = Writer::from_writer(Vec::new());

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
                neutralize_csv_cell(&path).as_ref(),
                neutralize_csv_cell(&file_name).as_ref(),
                neutralize_csv_cell(&function.name).as_ref(),
                &function.complexity.to_string(),
            ])
            .map_err(|e| ExportError::Io(format!("Failed to write CSV row: {}", e)))?;
    }

    let bytes = writer
        .into_inner()
        .map_err(|e| ExportError::Io(format!("Failed to write CSV: {}", e)))?;
    write_atomic(Path::new(invocation_path), &bytes).map_err(|e| {
        ExportError::Io(format!(
            "Failed to create CSV at {}: {}",
            invocation_path, e
        ))
    })
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
    write_atomic(
        Path::new(invocation_path),
        format!("{}\n", json_string).as_bytes(),
    )
    .map_err(|e| {
        ExportError::Io(format!(
            "Failed to write JSON to {}: {}",
            invocation_path, e
        ))
    })
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
    write_atomic(Path::new(snapshot_file_path), json_string.as_bytes()).map_err(|e| {
        ExportError::Io(format!(
            "Failed to write snapshot file at {}: {}",
            snapshot_file_path, e
        ))
    })
}

pub fn load_snapshot_file_shared(
    snapshot_file_path: &str,
) -> Result<Vec<FileComplexity>, ExportError> {
    let snapshot_path = std::path::Path::new(snapshot_file_path);
    let root = snapshot_path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(std::path::Path::new("."));
    let snapshot_content = crate::fs_guard::read_bounded_text(
        snapshot_path,
        Some(root),
        crate::fs_guard::MAX_DATA_BYTES,
    )
    .map_err(|e| {
        ExportError::Io(format!(
            "Failed to read snapshot file {}: {}",
            snapshot_file_path, e
        ))
    })?;
    serde_json::from_str(snapshot_content.as_str())
        .map_err(|e| ExportError::Serialize(format!("Failed to parse snapshot JSON: {}", e)))
}

/// Byte offsets of line starts. A line ends at `\r\n`, `\r`, or `\n`, the
/// terminators the Python tokenizer and LSP 3.17 recognize.
pub struct LineIndex {
    line_starts: Vec<usize>,
}

impl LineIndex {
    pub fn new(code: &str) -> Self {
        let bytes = code.as_bytes();
        let mut line_starts = Vec::new();
        let mut offset = 0;
        while offset < bytes.len() {
            match bytes[offset] {
                b'\n' => line_starts.push(offset + 1),
                b'\r' => {
                    if bytes.get(offset + 1) == Some(&b'\n') {
                        offset += 1;
                    }
                    line_starts.push(offset + 1);
                }
                _ => {}
            }
            offset += 1;
        }
        Self { line_starts }
    }

    pub fn line_of(&self, byte: usize) -> u64 {
        (self.line_starts.partition_point(|&start| start <= byte) + 1) as u64
    }

    pub fn column_of(&self, byte: usize, code: &str) -> u64 {
        let idx = self.line_starts.partition_point(|&start| start <= byte);
        let line_start = match idx {
            0 => 0,
            _ => self.line_starts[idx - 1],
        };
        (code[line_start..byte].chars().count() + 1) as u64
    }

    pub fn byte_of_line(&self, line: u64) -> Option<usize> {
        match line {
            0 => None,
            1 => Some(0),
            _ => self.line_starts.get((line - 2) as usize).copied(),
        }
    }

    /// The text of `line` without its terminator, or `None` past the end.
    pub fn line_text<'a>(&self, code: &'a str, line: u64) -> Option<&'a str> {
        let start = self.byte_of_line(line)?;
        let end = self
            .byte_of_line(line + 1)
            .map_or(code.len(), |next| next - terminator_len_before(code, next));
        code.get(start..end)
    }
}

fn terminator_len_before(code: &str, line_start: usize) -> usize {
    if code[..line_start].ends_with("\r\n") {
        2
    } else {
        1
    }
}

/// Splits `text` into lines like `str::lines`, also breaking on a lone `\r`.
pub fn split_lines(text: &str) -> impl Iterator<Item = &str> {
    let mut rest = text;
    std::iter::from_fn(move || {
        if rest.is_empty() {
            return None;
        }
        let Some(end) = rest.find(['\r', '\n']) else {
            return Some(std::mem::take(&mut rest));
        };
        let line = &rest[..end];
        let terminator = if rest[end..].starts_with("\r\n") {
            2
        } else {
            1
        };
        rest = &rest[end + terminator..];
        Some(line)
    })
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
            complexity += operator_sequences(b, None);
        }
        ast::Expr::UnaryOp(u) => {
            complexity += nested_operator_sequences(&u.operand, None);
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
            let elements: Vec<&ast::Expr> =
                c.key.as_deref().into_iter().chain([&*c.value]).collect();
            complexity += count_comprehension(&elements, &c.generators, nesting_level);
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

/// Counts the sequences of like boolean operators in `bool_op`. The node
/// starts a new sequence unless `enclosing` is the operator of the boolean
/// operation directly around it; `None` means a unary operator or nothing
/// encloses it.
fn operator_sequences(bool_op: &ast::ExprBoolOp, enclosing: Option<ast::BoolOp>) -> u64 {
    let starts_sequence = u64::from(enclosing != Some(bool_op.op));
    starts_sequence
        + bool_op
            .values
            .iter()
            .map(|value| nested_operator_sequences(value, Some(bool_op.op)))
            .sum::<u64>()
}

fn nested_operator_sequences(expr: &ast::Expr, enclosing: Option<ast::BoolOp>) -> u64 {
    match expr {
        ast::Expr::BoolOp(bool_op) => operator_sequences(bool_op, enclosing),
        ast::Expr::UnaryOp(unary) => nested_operator_sequences(&unary.operand, None),
        _ => 0,
    }
}

fn line_start_of(code: &str, offset: usize) -> usize {
    code[..offset].rfind(['\r', '\n']).map_or(0, |i| i + 1)
}

fn line_end_of(code: &str, offset: usize) -> usize {
    code[offset..]
        .find(['\r', '\n'])
        .map_or(code.len(), |i| offset + i)
}

fn line_at(code: &str, line_start: usize) -> &str {
    &code[line_start..line_end_of(code, line_start)]
}

fn next_line_start(code: &str, line_start: usize) -> Option<usize> {
    let end = line_end_of(code, line_start);
    if end == code.len() {
        return None;
    }
    let terminator = if code[end..].starts_with("\r\n") {
        2
    } else {
        1
    };
    Some(end + terminator)
}

fn previous_line_start(code: &str, line_start: usize) -> Option<usize> {
    if line_start == 0 {
        return None;
    }
    Some(line_start_of(
        code,
        line_start - terminator_len_before(code, line_start),
    ))
}

pub fn collect_def_names(code: &str) -> std::collections::HashSet<String> {
    split_lines(code)
        .filter_map(|line| {
            let trimmed = line.trim_start();
            let rest = trimmed
                .strip_prefix("def ")
                .or_else(|| trimmed.strip_prefix("async def "))?;
            rest.split('(').next().map(|name| name.trim().to_string())
        })
        .collect()
}

/// A suppression marker next to a function. `rules` is `None` for a bare
/// marker, which suppresses the whole function, and holds the rule ids named
/// in a bracketed list otherwise.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IgnoreDirective {
    pub text: String,
    pub rules: Option<Vec<String>>,
}

fn ignore_re() -> &'static Regex {
    static IGNORE_RE: OnceLock<Regex> = OnceLock::new();
    IGNORE_RE.get_or_init(|| {
        Regex::new(r"(?i)#[ \t]*complexipy[ \t]*:[ \t]*ignore\b([ \t]*\[([^\]]*)\])?").unwrap()
    })
}

fn noqa_re() -> &'static Regex {
    static NOQA_RE: OnceLock<Regex> = OnceLock::new();
    NOQA_RE.get_or_init(|| {
        Regex::new(r"(?i)#[ \t]*noqa[ \t]*:[ \t]*complexipy\b([ \t]*\[([^\]]*)\])?").unwrap()
    })
}

/// Extract an ignore directive from a line.
///
/// Returns `Some(..)` for `# complexipy: ignore` and `# noqa: complexipy`,
/// case-insensitive, with an optional bracketed rule list after the marker.
/// The marker word must end there, and only spaces and tabs separate its
/// parts. Returns `None` when no marker is found.
pub fn extract_comment_marker(line: &str) -> Option<IgnoreDirective> {
    if let Some(captures) = ignore_re().captures(line) {
        return Some(build_directive("# complexipy: ignore", captures.get(2)));
    }
    if let Some(captures) = noqa_re().captures(line) {
        return Some(build_directive("# noqa: complexipy", captures.get(2)));
    }

    None
}

/// The byte ranges of the comment tokens of a source, the only places an
/// ignore marker counts. A marker inside a string, a docstring, or a
/// decorator argument is data, not a comment.
#[derive(Debug, Default)]
pub struct CommentRanges(Vec<(usize, usize)>);

impl CommentRanges {
    /// Tokenizes `code` only when it holds marker text somewhere, so a file
    /// without markers costs one scan.
    pub fn of(code: &str) -> Self {
        if !ignore_re().is_match(code) && !noqa_re().is_match(code) {
            return Self::default();
        }
        Self(crate::syntax_tree::comment_ranges(code))
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    fn on_line(&self, line_start: usize, line_end: usize) -> impl Iterator<Item = (usize, usize)> {
        let first = self.0.partition_point(|(start, _)| *start < line_start);
        self.0[first..]
            .iter()
            .copied()
            .take_while(move |(start, _)| *start < line_end)
    }

    fn marker_on_line(&self, code: &str, line_start: usize) -> Option<IgnoreDirective> {
        self.on_line(line_start, line_end_of(code, line_start))
            .find_map(|(start, end)| extract_comment_marker(&code[start..end]))
    }

    fn own_line_marker(&self, code: &str, line_start: usize) -> Option<IgnoreDirective> {
        self.on_line(line_start, line_end_of(code, line_start))
            .filter(|(start, _)| code[line_start..*start].trim().is_empty())
            .find_map(|(start, end)| extract_comment_marker(&code[start..end]))
    }
}

fn build_directive(marker: &str, codes: Option<regex::Match<'_>>) -> IgnoreDirective {
    let Some(codes) = codes else {
        return IgnoreDirective {
            text: marker.to_string(),
            rules: None,
        };
    };
    let rules: Vec<String> = codes
        .as_str()
        .split(',')
        .map(|code| code.trim().to_ascii_uppercase())
        .filter(|code| !code.is_empty())
        .collect();
    if !rules.iter().all(|code| is_rule_id(code)) {
        return IgnoreDirective {
            text: marker.to_string(),
            rules: None,
        };
    }
    IgnoreDirective {
        text: format!("{}[{}]", marker, rules.join(",")),
        rules: Some(rules),
    }
}

fn is_rule_id(code: &str) -> bool {
    let mut bytes = code.bytes();
    matches!(bytes.next(), Some(first) if first.is_ascii_alphabetic())
        && bytes.clone().count() > 0
        && bytes.all(|byte| byte.is_ascii_digit())
}

/// Returns whether the code could hold an ignore directive. Both directive
/// patterns contain this text under the same case-insensitive match, so a
/// file without it holds no directive, and the per-function lookups can be
/// skipped.
pub fn may_contain_marker(code: &str) -> bool {
    static MARKER_TEXT: OnceLock<Regex> = OnceLock::new();

    MARKER_TEXT
        .get_or_init(|| Regex::new(r"(?i)complexipy").unwrap())
        .is_match(code)
}

/// Find an ignore directive near a `def` or decorator line: in a comment on
/// the `def` line, on a signature line or a decorator line that follows it,
/// or in an own-line comment directly above it.
///
/// Returns `Some(..)` when a marker is found that would trigger suppression,
/// `None` otherwise.
pub fn find_noqa_comment(
    byte_offset: usize,
    code: &str,
    comments: &CommentRanges,
) -> Option<IgnoreDirective> {
    if comments.is_empty() {
        return None;
    }
    let line_start = line_start_of(code, byte_offset);
    let current_line = line_at(code, line_start);

    let signature_has_marker = |def_line_start: usize| -> Option<IgnoreDirective> {
        let mut pos = def_line_start;
        for _ in 0..20 {
            if let Some(directive) = comments.marker_on_line(code, pos) {
                return Some(directive);
            }
            if line_at(code, pos).contains(':') {
                break;
            }
            pos = next_line_start(code, pos)?;
        }
        None
    };

    if let Some(marker) = comments.marker_on_line(code, line_start) {
        return Some(marker);
    }

    if let Some(prev_start) = previous_line_start(code, line_start)
        && let Some(marker) = comments.own_line_marker(code, prev_start)
    {
        return Some(marker);
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
                if let Some(prev_start) = previous_line_start(code, pos)
                    && let Some(directive) = comments.marker_on_line(code, prev_start)
                {
                    return Some(directive);
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

/// Collect ignored locations from code, only reporting bare markers that
/// suppress a whole function definition (i.e., are adjacent to `def`
/// or `@decorator` lines). A marker with a rule list is not a whole-function
/// suppression and is never reported here.
pub fn collect_ignored_locations(code: &str) -> Vec<(u64, String)> {
    if !may_contain_marker(code) {
        return Vec::new();
    }
    let comments = CommentRanges::of(code);
    if comments.is_empty() {
        return Vec::new();
    }
    let index = LineIndex::new(code);
    let mut results = Vec::new();
    let mut line_start = 0;

    while line_start < code.len() {
        let line = line_at(code, line_start);
        let trimmed = line.trim_start();

        if trimmed.starts_with("def ") {
            if let Some(directive) = find_noqa_comment(line_start, code, &comments)
                && directive.rules.is_none()
            {
                results.push((index.line_of(line_start), directive.text));
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
                && let Some(directive) = find_noqa_comment(def_line_start, code, &comments)
                && directive.rules.is_none()
            {
                results.push((index.line_of(def_line_start), directive.text));
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
