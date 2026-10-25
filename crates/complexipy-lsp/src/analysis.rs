use std::collections::HashMap;
use std::sync::Arc;

use complexipy_core::AnalysisOptions;
use complexipy_core::classes::{FunctionComplexity, RefactorPlan};
use complexipy_core::cognitive_complexity::{
    code_complexity_shared, plans_for_function, syntax_error,
};
use complexipy_core::config::{DiagnosticRange, HoverScope, InlayHints, LspConfig};
use complexipy_core::utils::{LineIndex, split_lines};
use lsp_types::{
    Diagnostic, DiagnosticSeverity, Hover, HoverContents, InlayHint, InlayHintKind, InlayHintLabel,
    MarkupContent, MarkupKind, NumberOrString, Position, Range,
};
use serde::Serialize;

pub const DIAGNOSTIC_CODE: &str = "cognitive-complexity";
pub const PARSE_ERROR_CODE: &str = "complexipy-parse-error";
pub const DIAGNOSTIC_SOURCE: &str = "complexipy";
pub const FUNCTION_HINT_LABEL: &str = "cognitive: ";
pub const LINE_HINT_LABEL: &str = "+";

const CONTINUATION_BYTES: &[u8] = b",([{\\";

#[derive(Clone, Debug, Default)]
pub struct LineBounds {
    lengths: Vec<u32>,
}

impl LineBounds {
    pub fn new(text: &str) -> Self {
        Self {
            lengths: split_lines(text)
                .map(|line| line.encode_utf16().count() as u32)
                .collect(),
        }
    }

    pub fn length(&self, line: u64) -> u32 {
        self.lengths
            .get(line.saturating_sub(1) as usize)
            .copied()
            .unwrap_or(0)
    }

    pub fn end_position(&self, line: u64) -> Position {
        Position::new(line.saturating_sub(1) as u32, self.length(line))
    }
}

#[derive(Clone, Debug)]
pub struct ParseFailure {
    pub version: i32,
    pub message: String,
    range: Range,
}

impl ParseFailure {
    pub fn diagnostic(&self) -> Diagnostic {
        Diagnostic {
            range: self.range,
            severity: Some(DiagnosticSeverity::WARNING),
            code: Some(NumberOrString::String(PARSE_ERROR_CODE.to_string())),
            code_description: None,
            source: Some(DIAGNOSTIC_SOURCE.to_string()),
            message: format!("{}: {}", DIAGNOSTIC_SOURCE, self.message),
            related_information: None,
            tags: None,
            data: None,
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct Declaration {
    start: u64,
    end: u64,
}

#[derive(Serialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LineComplexities {
    pub version: i32,
    pub max_complexity_allowed: u64,
    pub functions: Vec<FunctionLines>,
}

#[derive(Serialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FunctionLines {
    pub name: String,
    pub complexity: u64,
    pub line: u32,
    pub lines: Vec<LineIncrement>,
}

#[derive(Serialize, Debug, PartialEq)]
pub struct LineIncrement {
    pub line: u32,
    pub complexity: u64,
}

#[derive(Clone)]
pub struct DocumentAnalysis {
    pub version: i32,
    pub functions: Vec<FunctionComplexity>,
    bounds: LineBounds,
    declarations: Vec<Declaration>,
    source: Arc<str>,
    plans: HashMap<u64, Vec<RefactorPlan>>,
}

impl DocumentAnalysis {
    pub fn new(version: i32, functions: Vec<FunctionComplexity>, source: Arc<str>) -> Self {
        let lines: Vec<&str> = split_lines(&source).collect();
        let declarations = functions
            .iter()
            .map(|function| {
                let start = declaration_start_line(&lines, function);
                let end = declaration_end_line(&lines, start, function.line_end);

                Declaration { start, end }
            })
            .collect();

        Self {
            version,
            functions,
            bounds: LineBounds::new(&source),
            declarations,
            source,
            plans: HashMap::new(),
        }
    }

    pub fn source(&self) -> Arc<str> {
        Arc::clone(&self.source)
    }

    /// Returns the first line of the function under `position` when its
    /// plans are not built yet, so a hover there needs them first.
    pub fn plans_needed(&self, config: &LspConfig, position: Position) -> Option<u64> {
        let function = &self.functions[self.hovered(config, position)?];

        (!self.plans.contains_key(&function.line_start)).then_some(function.line_start)
    }

    pub fn insert_plans(&mut self, line_start: u64, plans: Vec<RefactorPlan>) {
        self.plans.entry(line_start).or_insert(plans);
    }

    pub fn empty(version: i32) -> Self {
        Self {
            version,
            functions: Vec::new(),
            bounds: LineBounds::default(),
            declarations: Vec::new(),
            source: Arc::from(""),
            plans: HashMap::new(),
        }
    }

    pub fn hints(&self, config: &LspConfig, requested: Option<Range>) -> Vec<InlayHint> {
        if config.lsp.inlay_hints == InlayHints::Never {
            return Vec::new();
        }

        let mut hints = Vec::new();

        for (index, function) in self.functions.iter().enumerate() {
            let mut function_hint_position = None;

            if self.shows_function_hint(function, config) {
                let position = self.declaration_position(index);

                if is_within(position, requested.as_ref()) {
                    hints.push(function_hint(position, function.complexity));
                    function_hint_position = Some(position);
                }
            }

            if config.lsp.per_line_hints {
                for line in function
                    .line_complexities
                    .iter()
                    .filter(|line| line.complexity > 0)
                {
                    let position = self.bounds.end_position(line.line);

                    if function_hint_position == Some(position) {
                        continue;
                    }

                    if is_within(position, requested.as_ref()) {
                        hints.push(line_hint(position, line.complexity));
                    }
                }
            }
        }

        hints
    }

    pub fn diagnostics(&self, config: &LspConfig) -> Vec<Diagnostic> {
        if !config.lsp.diagnostics {
            return Vec::new();
        }

        self.functions
            .iter()
            .enumerate()
            .filter(|(_, function)| is_over_threshold(function.complexity, config))
            .map(|(index, function)| {
                diagnostic(self.diagnostic_range(index, config), function, config)
            })
            .collect()
    }

    pub fn line_complexities(&self, config: &LspConfig) -> LineComplexities {
        LineComplexities {
            version: self.version,
            max_complexity_allowed: config.max_complexity_allowed,
            functions: self
                .functions
                .iter()
                .enumerate()
                .map(|(index, function)| FunctionLines {
                    name: function.name.clone(),
                    complexity: function.complexity,
                    line: self.declaration_position(index).line,
                    lines: function
                        .line_complexities
                        .iter()
                        .filter(|line| line.complexity > 0)
                        .map(|line| LineIncrement {
                            line: line.line.saturating_sub(1) as u32,
                            complexity: line.complexity,
                        })
                        .collect(),
                })
                .collect(),
        }
    }

    pub fn hover(&mut self, config: &LspConfig, position: Position) -> Option<Hover> {
        let index = self.hovered(config, position)?;
        let function = self.functions[index].clone();
        let range = match config.lsp.hover {
            HoverScope::Function => function_range(&self.bounds, &function),
            _ => self.declaration_range(index),
        };
        let top_plan = self.plans(function.line_start, config).first();

        Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: hover_value(&function, top_plan, config),
            }),
            range: Some(range),
        })
    }

    fn hovered(&self, config: &LspConfig, position: Position) -> Option<usize> {
        let line = u64::from(position.line) + 1;

        match config.lsp.hover {
            HoverScope::Never => None,
            HoverScope::Function => self
                .functions
                .iter()
                .position(|function| line >= function.line_start && line <= function.line_end),
            HoverScope::Signature => self
                .declarations
                .iter()
                .position(|declaration| line >= declaration.start && line <= declaration.end),
        }
    }

    fn declaration_range(&self, index: usize) -> Range {
        let declaration = self.declarations[index];

        Range::new(
            Position::new(declaration.start.saturating_sub(1) as u32, 0),
            self.bounds.end_position(declaration.end),
        )
    }

    fn diagnostic_range(&self, index: usize, config: &LspConfig) -> Range {
        let function = &self.functions[index];

        match config.lsp.diagnostic_range {
            DiagnosticRange::Function => function_range(&self.bounds, function),
            DiagnosticRange::Name => {
                let start = self.declarations[index].start;
                let content = split_lines(&self.source).nth(start.saturating_sub(1) as usize);

                name_range(content.unwrap_or(""), start)
                    .unwrap_or_else(|| self.declaration_range(index))
            }
        }
    }

    pub fn plans(&mut self, line_start: u64, config: &LspConfig) -> &[RefactorPlan] {
        self.plans
            .entry(line_start)
            .or_insert_with(|| plans_for(&self.source, line_start, config))
    }

    fn declaration_position(&self, index: usize) -> Position {
        let line = self
            .declarations
            .get(index)
            .map_or(self.functions[index].line_start, |declaration| {
                declaration.end
            });

        self.bounds.end_position(line)
    }

    fn shows_function_hint(&self, function: &FunctionComplexity, config: &LspConfig) -> bool {
        match config.lsp.inlay_hints {
            InlayHints::Never => false,
            InlayHints::Always => true,
            InlayHints::Threshold => is_over_threshold(function.complexity, config),
        }
    }
}

pub fn analyze(
    text: impl Into<Arc<str>>,
    version: i32,
    config: &LspConfig,
) -> Result<DocumentAnalysis, ParseFailure> {
    let text = text.into();
    match code_complexity_shared(&text, &analysis_options(config)) {
        Ok(result) => Ok(DocumentAnalysis::new(version, result.functions, text)),
        Err(message) => Err(parse_failure(&text, version, message)),
    }
}

/// Builds the refactor plans of the function that starts on `line_start`.
pub fn plans_for(source: &str, line_start: u64, config: &LspConfig) -> Vec<RefactorPlan> {
    plans_for_function(source, line_start, &analysis_options(config))
        .ok()
        .flatten()
        .map(|(plans, _)| plans)
        .unwrap_or_default()
}

fn parse_failure(text: &str, version: i32, fallback: String) -> ParseFailure {
    let bounds = LineBounds::new(text);
    let Some(error) = syntax_error(text) else {
        return ParseFailure {
            version,
            message: fallback,
            range: Range::new(Position::new(0, 0), bounds.end_position(1)),
        };
    };

    let start = position_at(text, error.start);
    let end = position_at(text, error.end);
    let end = if start == end {
        bounds.end_position(u64::from(start.line) + 1)
    } else {
        end
    };

    ParseFailure {
        version,
        message: error.message,
        range: Range::new(start, end),
    }
}

fn position_at(text: &str, offset: usize) -> Position {
    let mut offset = offset.min(text.trim_end().len());
    while !text.is_char_boundary(offset) {
        offset -= 1;
    }
    let index = LineIndex::new(text);
    let line = index.line_of(offset);
    let line_start = index.byte_of_line(line).unwrap_or(0);

    Position::new(
        line.saturating_sub(1) as u32,
        text[line_start..offset].encode_utf16().count() as u32,
    )
}

fn analysis_options(config: &LspConfig) -> AnalysisOptions {
    AnalysisOptions {
        no_ignore: config.no_ignore,
        rules: config.rule_set(),
        ..Default::default()
    }
}

pub fn is_stale(computed_version: i32, stored_version: Option<i32>) -> bool {
    stored_version != Some(computed_version)
}

pub fn is_over_threshold(complexity: u64, config: &LspConfig) -> bool {
    complexity > config.max_complexity_allowed
}

pub fn declaration_start_line(lines: &[&str], function: &FunctionComplexity) -> u64 {
    let start = function.line_start;
    let end = function.line_end.max(start);

    (start..=end)
        .find(|line| {
            let Some(content) = lines.get(line.saturating_sub(1) as usize) else {
                return false;
            };

            is_def_line(content.trim_start()) && opens_body(lines, *line, content)
        })
        .unwrap_or(start)
}

pub fn declaration_end_line(lines: &[&str], start: u64, last: u64) -> u64 {
    let mut depth: i64 = 0;

    for line in start..=last {
        let Some(content) = lines.get(line.saturating_sub(1) as usize) else {
            break;
        };

        depth = (depth + bracket_delta(content)).max(0);

        let trimmed = content.trim_end();

        if depth == 0 && (trimmed.ends_with(':') || !continues(trimmed)) {
            return line;
        }
    }

    start
}

pub fn is_within(position: Position, requested: Option<&Range>) -> bool {
    match requested {
        None => true,
        Some(range) => {
            let after_start = position.line > range.start.line
                || (position.line == range.start.line
                    && position.character >= range.start.character);
            let before_end = position.line < range.end.line
                || (position.line == range.end.line && position.character <= range.end.character);
            after_start && before_end
        }
    }
}

fn name_range(content: &str, line: u64) -> Option<Range> {
    let indent = indentation(content);
    let trimmed = &content[indent..];
    let after_async = trimmed
        .strip_prefix("async")
        .filter(|rest| rest.starts_with(char::is_whitespace))
        .map_or(trimmed, str::trim_start);
    let after_def = after_async
        .strip_prefix("def")
        .filter(|rest| rest.starts_with(char::is_whitespace))?
        .trim_start();
    let name_len = after_def
        .find(|character: char| !(character.is_alphanumeric() || character == '_'))
        .unwrap_or(after_def.len());

    if name_len == 0 {
        return None;
    }

    let name_offset = content.len() - after_def.len();
    let start = content[..name_offset].encode_utf16().count() as u32;
    let end = start + after_def[..name_len].encode_utf16().count() as u32;
    let row = line.saturating_sub(1) as u32;

    Some(Range::new(
        Position::new(row, start),
        Position::new(row, end),
    ))
}

fn is_def_line(trimmed: &str) -> bool {
    trimmed.starts_with("def ") || trimmed.starts_with("async def ")
}

fn opens_body(lines: &[&str], line: u64, content: &str) -> bool {
    if content
        .trim_start()
        .split_once(':')
        .is_some_and(|(_, rest)| !rest.trim().is_empty())
    {
        return true;
    }

    lines
        .get(line as usize)
        .is_some_and(|next| indentation(next) > indentation(content))
}

fn indentation(content: &str) -> usize {
    content.len() - content.trim_start().len()
}

fn continues(trimmed: &str) -> bool {
    trimmed
        .as_bytes()
        .last()
        .is_some_and(|byte| CONTINUATION_BYTES.contains(byte))
}

fn bracket_delta(content: &str) -> i64 {
    content.chars().fold(0, |depth, character| match character {
        '(' | '[' | '{' => depth + 1,
        ')' | ']' | '}' => depth - 1,
        _ => depth,
    })
}

fn function_hint(position: Position, complexity: u64) -> InlayHint {
    InlayHint {
        position,
        label: InlayHintLabel::String(format!("{}{}", FUNCTION_HINT_LABEL, complexity)),
        kind: Some(InlayHintKind::TYPE),
        text_edits: None,
        tooltip: None,
        padding_left: Some(true),
        padding_right: Some(true),
        data: None,
    }
}

fn line_hint(position: Position, complexity: u64) -> InlayHint {
    InlayHint {
        position,
        label: InlayHintLabel::String(format!("{}{}", LINE_HINT_LABEL, complexity)),
        kind: None,
        text_edits: None,
        tooltip: None,
        padding_left: Some(true),
        padding_right: Some(true),
        data: None,
    }
}

fn diagnostic(range: Range, function: &FunctionComplexity, config: &LspConfig) -> Diagnostic {
    Diagnostic {
        range,
        severity: Some(DiagnosticSeverity::WARNING),
        code: Some(NumberOrString::String(DIAGNOSTIC_CODE.to_string())),
        code_description: None,
        source: Some(DIAGNOSTIC_SOURCE.to_string()),
        message: format!(
            "cognitive complexity {} exceeds the allowed {}",
            function.complexity, config.max_complexity_allowed
        ),
        related_information: None,
        tags: None,
        data: None,
    }
}

fn function_range(bounds: &LineBounds, function: &FunctionComplexity) -> Range {
    Range::new(
        Position::new(function.line_start.saturating_sub(1) as u32, 0),
        bounds.end_position(function.line_end),
    )
}

fn escape_markdown(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for character in text.chars() {
        if character.is_ascii_punctuation() {
            escaped.push('\\');
        }
        escaped.push(character);
    }
    escaped
}

fn hover_value(
    function: &FunctionComplexity,
    top_plan: Option<&RefactorPlan>,
    config: &LspConfig,
) -> String {
    let mut value = format!(
        "**{}**: cognitive complexity {}",
        escape_markdown(&function.name),
        function.complexity
    );

    if is_over_threshold(function.complexity, config) {
        value.push_str(&format!(
            "\n\nExceeds the allowed {}",
            config.max_complexity_allowed
        ));
    }

    if let Some(plan) = top_plan {
        value.push_str(&format!(
            "\n\nTop refactor (`{}`): {}",
            plan.rule_id, plan.title
        ));
    }

    value
}
