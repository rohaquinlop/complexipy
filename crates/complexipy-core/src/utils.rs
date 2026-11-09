use regex::Regex;
use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast, Stmt};
use std::sync::OnceLock;

mod export_deps {
    pub use crate::classes::{FileComplexity, FunctionComplexity, LineComplexity};
    pub use crate::fs_guard::write_atomic;
    pub use crate::sanitize::neutralize_csv_cell;
    pub use csv::Writer;
    pub use serde_json;
    pub use std::collections::BTreeMap;
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
    let bytes = render_csv(
        functions_complexity,
        sort,
        show_detailed_results,
        max_complexity,
    )?;
    write_atomic(Path::new(invocation_path), &bytes).map_err(|e| {
        ExportError::Io(format!(
            "Failed to create CSV at {}: {}",
            invocation_path, e
        ))
    })
}

pub fn render_csv(
    functions_complexity: Vec<FileComplexity>,
    sort: &str,
    show_detailed_results: bool,
    max_complexity: u64,
) -> Result<Vec<u8>, ExportError> {
    match sort {
        "asc" | "desc" | "name" | "file_name" => {}
        other => return Err(ExportError::InvalidSort(other.to_string())),
    }

    let mut writer = Writer::from_writer(Vec::new());

    writer
        .write_record(["Path", "File Name", "Function Name", "Cognitive Complexity"])
        .map_err(|e| ExportError::Io(format!("Failed to write CSV header: {}", e)))?;

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

    writer
        .into_inner()
        .map_err(|e| ExportError::Io(format!("Failed to write CSV: {}", e)))
}

pub fn output_json_shared(
    invocation_path: &str,
    functions_complexity: Vec<FileComplexity>,
    show_detailed_results: bool,
    max_complexity: u64,
    suggest_refactors: bool,
) -> Result<(), ExportError> {
    let bytes = render_json(
        functions_complexity,
        show_detailed_results,
        max_complexity,
        suggest_refactors,
    )?;
    write_atomic(Path::new(invocation_path), &bytes).map_err(|e| {
        ExportError::Io(format!(
            "Failed to write JSON to {}: {}",
            invocation_path, e
        ))
    })
}

fn scoring_lines(line_complexities: &[LineComplexity]) -> Vec<serde_json::Value> {
    let mut per_line: BTreeMap<u64, u64> = BTreeMap::new();
    for line in line_complexities {
        *per_line.entry(line.line).or_default() += line.complexity;
    }
    per_line
        .into_iter()
        .filter(|&(_, complexity)| complexity > 0)
        .map(|(line, complexity)| serde_json::json!({"line": line, "complexity": complexity}))
        .collect()
}

pub fn render_json(
    functions_complexity: Vec<FileComplexity>,
    show_detailed_results: bool,
    max_complexity: u64,
    suggest_refactors: bool,
) -> Result<Vec<u8>, ExportError> {
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
                    "line_start": function.line_start,
                    "line_end": function.line_end,
                    "line_complexities": scoring_lines(&function.line_complexities),
                    "refactor_plans": refactor_plans
                });
                json_data.push(entry);
            }
        }
    }

    let json_string = serde_json::to_string_pretty(&json_data)
        .map_err(|e| ExportError::Serialize(format!("Failed to serialize JSON: {}", e)))?;
    Ok(format!("{}\n", json_string).into_bytes())
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

/// Returns the nested function of a decorator: a function whose body, after
/// an optional docstring, is only a nested `def` and a `return` of its name or
/// of a call that takes it as a direct argument, such as
/// `functools.wraps(f)(inner)`. A call of the nested function does not count.
pub fn decorator_inner(statement: &Stmt) -> Option<(&Stmt, &Stmt)> {
    let Stmt::FunctionDef(f) = statement else {
        return None;
    };
    let body = match f.body.as_slice() {
        [Stmt::Expr(docstring), rest @ ..] if docstring.value.is_string_literal_expr() => rest,
        body => body,
    };
    let [inner @ Stmt::FunctionDef(def), returned @ Stmt::Return(ret)] = body else {
        return None;
    };
    let is_inner =
        |expr: &ast::Expr| matches!(expr, ast::Expr::Name(name) if name.id == def.name.id);
    let returns_inner = match ret.value.as_deref() {
        Some(ast::Expr::Call(call)) => {
            !is_inner(&call.func)
                && (call.arguments.args.iter().any(is_inner)
                    || call
                        .arguments
                        .keywords
                        .iter()
                        .any(|kw| kw.arg.is_some() && is_inner(&kw.value)))
        }
        Some(value) => is_inner(value),
        None => false,
    };
    returns_inner.then_some((inner, returned))
}

pub fn count_bool_ops(expr: &ast::Expr, nesting_level: u64) -> u64 {
    own_score(expr, nesting_level).unwrap_or_else(|| count_child_expressions(expr, nesting_level))
}

/// Scores `expr` when it has a rule of its own, or returns `None` for an
/// expression that is only crossed to reach the scored expressions below it.
fn own_score(expr: &ast::Expr, nesting_level: u64) -> Option<u64> {
    let score = match expr {
        ast::Expr::BoolOp(b) => operator_sequences(b, nesting_level),
        ast::Expr::UnaryOp(u) => count_bool_ops(&u.operand, nesting_level),
        ast::Expr::Compare(c) => {
            count_bool_ops(&c.left, nesting_level)
                + c.comparators
                    .iter()
                    .map(|comparator| count_bool_ops(comparator, nesting_level))
                    .sum::<u64>()
        }
        ast::Expr::If(i) => {
            1 + nesting_level
                + count_bool_ops(&i.test, nesting_level)
                + count_bool_ops(&i.body, nesting_level + 1)
                + count_bool_ops(&i.orelse, nesting_level + 1)
        }
        ast::Expr::Lambda(l) => {
            l.parameters
                .iter()
                .flat_map(|p| p.iter_non_variadic_params())
                .filter_map(|parameter| parameter.default.as_deref())
                .map(|default| count_bool_ops(default, nesting_level))
                .sum::<u64>()
                + count_bool_ops(&l.body, nesting_level + 1)
        }
        ast::Expr::ListComp(c) => count_comprehension(&[&c.elt], &c.generators, nesting_level),
        ast::Expr::SetComp(c) => count_comprehension(&[&c.elt], &c.generators, nesting_level),
        ast::Expr::Generator(c) => count_comprehension(&[&c.elt], &c.generators, nesting_level),
        ast::Expr::DictComp(c) => {
            let elements: Vec<&ast::Expr> =
                c.key.as_deref().into_iter().chain([&*c.value]).collect();
            count_comprehension(&elements, &c.generators, nesting_level)
        }
        _ => return None,
    };
    Some(score)
}

/// Sums the scores of the scored expressions below `expr`, crossing the
/// others with an explicit stack instead of recursion.
fn count_child_expressions(expr: &ast::Expr, nesting_level: u64) -> u64 {
    let mut complexity: u64 = 0;
    let mut pending = direct_children(expr);
    while let Some(child) = pending.pop() {
        match own_score(child, nesting_level) {
            Some(score) => complexity += score,
            None => pending.extend(direct_children(child)),
        }
    }
    complexity
}

fn direct_children(expr: &ast::Expr) -> Vec<&ast::Expr> {
    let mut children = ChildExpressions::default();
    visitor::walk_expr(&mut children, expr);
    children.exprs
}

#[derive(Default)]
struct ChildExpressions<'a> {
    exprs: Vec<&'a ast::Expr>,
}

impl<'a> Visitor<'a> for ChildExpressions<'a> {
    fn visit_expr(&mut self, expr: &'a ast::Expr) {
        self.exprs.push(expr);
    }
}

fn count_comprehension(
    elements: &[&ast::Expr],
    generators: &[ast::Comprehension],
    nesting_level: u64,
) -> u64 {
    generators
        .iter()
        .flat_map(|generator| std::iter::once(&generator.iter).chain(generator.ifs.iter()))
        .chain(elements.iter().copied())
        .map(|expr| count_bool_ops(expr, nesting_level))
        .sum()
}

enum ChainItem<'a> {
    Operand(&'a ast::Expr),
    Operator(ast::BoolOp),
}

fn push_chain<'a>(stack: &mut Vec<ChainItem<'a>>, bool_op: &'a ast::ExprBoolOp) {
    for (position, value) in bool_op.values.iter().enumerate().rev() {
        stack.push(ChainItem::Operand(value));
        if position > 0 {
            stack.push(ChainItem::Operator(bool_op.op));
        }
    }
}

/// Counts the sequences of like boolean operators in the chain rooted at
/// `bool_op`, read left to right: the operators of nested boolean operations
/// sit between the operators around them, and each change of operator starts
/// a new sequence. Operands that are not boolean operations are scored on
/// their own, so the operand of a `not` starts a chain of its own.
fn operator_sequences(bool_op: &ast::ExprBoolOp, nesting_level: u64) -> u64 {
    let mut complexity: u64 = 0;
    let mut previous: Option<ast::BoolOp> = None;
    let mut stack = Vec::new();
    push_chain(&mut stack, bool_op);
    while let Some(item) = stack.pop() {
        match item {
            ChainItem::Operand(ast::Expr::BoolOp(nested)) => push_chain(&mut stack, nested),
            ChainItem::Operand(operand) => complexity += count_bool_ops(operand, nesting_level),
            ChainItem::Operator(operator) => {
                if previous != Some(operator) {
                    complexity += 1;
                }
                previous = Some(operator);
            }
        }
    }
    complexity
}

/// The line of the `else` keyword that opens the suite whose first statement
/// starts at `suite_start`. A clause keyword opens its own line, so the search
/// looks at that statement's line (`else: pass`) and then back over blank and
/// comment lines only. Falls back to the statement's own line.
pub fn else_keyword_line(code: &str, suite_start: usize, index: &LineIndex) -> u64 {
    let statement_line = line_start_of(code, suite_start);
    let mut line_start = statement_line;
    loop {
        let text = line_at(code, line_start).trim_start();
        let opens_else = text
            .strip_prefix("else")
            .is_some_and(|rest| rest.starts_with(':') || rest.starts_with(char::is_whitespace));
        if opens_else {
            return index.line_of(line_start);
        }
        let skippable = text.is_empty() || text.starts_with('#');
        if line_start != statement_line && !skippable {
            break;
        }
        match previous_line_start(code, line_start) {
            Some(previous) => line_start = previous,
            None => break,
        }
    }
    index.line_of(statement_line)
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

/// Whether `code` holds ignore marker text anywhere, comment or not.
pub fn has_marker_text(code: &str) -> bool {
    ignore_re().is_match(code) || noqa_re().is_match(code)
}

/// `code` with every ignore marker and the trailing spaces of each line
/// removed, so two sources that differ only in their markers compare equal.
pub fn without_markers(code: &str) -> String {
    code.lines()
        .map(|line| {
            let line = ignore_re().replace_all(line, "");
            noqa_re().replace_all(&line, "").trim_end().to_string()
        })
        .collect::<Vec<_>>()
        .join("\n")
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
        if !has_marker_text(code) {
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
