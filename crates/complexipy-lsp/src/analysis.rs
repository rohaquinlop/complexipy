use complexipy_core::AnalysisOptions;
use complexipy_core::classes::FunctionComplexity;
use complexipy_core::cognitive_complexity::code_complexity_shared;
use complexipy_core::config::{InlayHints, LspConfig};
use lsp_types::{
    Diagnostic, DiagnosticSeverity, Hover, HoverContents, InlayHint, InlayHintKind, InlayHintLabel,
    MarkupContent, MarkupKind, NumberOrString, Position, Range,
};

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
            lengths: text
                .lines()
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
    bounds: LineBounds,
}

impl ParseFailure {
    pub fn diagnostic(&self) -> Diagnostic {
        Diagnostic {
            range: Range::new(Position::new(0, 0), self.bounds.end_position(1)),
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

#[derive(Clone)]
pub struct DocumentAnalysis {
    pub version: i32,
    pub functions: Vec<FunctionComplexity>,
    bounds: LineBounds,
    declaration_lines: Vec<u64>,
}

impl DocumentAnalysis {
    pub fn new(version: i32, functions: Vec<FunctionComplexity>, source: &str) -> Self {
        let lines: Vec<&str> = source.lines().collect();
        let declaration_lines = functions
            .iter()
            .map(|function| {
                let start = declaration_start_line(&lines, function);

                declaration_end_line(&lines, start, function.line_end)
            })
            .collect();

        Self {
            version,
            functions,
            bounds: LineBounds::new(source),
            declaration_lines,
        }
    }

    pub fn empty(version: i32) -> Self {
        Self {
            version,
            functions: Vec::new(),
            bounds: LineBounds::default(),
            declaration_lines: Vec::new(),
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
            .filter(|function| is_over_threshold(function.complexity, config))
            .map(|function| diagnostic(&self.bounds, function, config))
            .collect()
    }

    pub fn hover(&self, config: &LspConfig, position: Position) -> Option<Hover> {
        let line = u64::from(position.line) + 1;
        let function = self
            .functions
            .iter()
            .find(|function| line >= function.line_start && line <= function.line_end)?;

        Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: hover_value(function, config),
            }),
            range: Some(function_range(&self.bounds, function)),
        })
    }

    fn declaration_position(&self, index: usize) -> Position {
        let line = self
            .declaration_lines
            .get(index)
            .copied()
            .unwrap_or_else(|| self.functions[index].line_start);

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
    text: &str,
    version: i32,
    config: &LspConfig,
) -> Result<DocumentAnalysis, ParseFailure> {
    match code_complexity_shared(
        text,
        &AnalysisOptions {
            no_ignore: config.no_ignore,
            with_plans: true,
            rules: config.rule_set(),
            ..Default::default()
        },
    ) {
        Ok(result) => Ok(DocumentAnalysis::new(version, result.functions, text)),
        Err(message) => Err(ParseFailure {
            version,
            message,
            bounds: LineBounds::new(text),
        }),
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
        padding_left: None,
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
        padding_left: None,
        padding_right: Some(true),
        data: None,
    }
}

fn diagnostic(
    bounds: &LineBounds,
    function: &FunctionComplexity,
    config: &LspConfig,
) -> Diagnostic {
    Diagnostic {
        range: function_range(bounds, function),
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

fn hover_value(function: &FunctionComplexity, config: &LspConfig) -> String {
    let mut value = format!(
        "**{}**: cognitive complexity {}",
        function.name, function.complexity
    );

    if is_over_threshold(function.complexity, config) {
        value.push_str(&format!(
            "\n\nExceeds the allowed {}",
            config.max_complexity_allowed
        ));
    }

    if let Some(plan) = function.refactor_plans.first() {
        value.push_str(&format!(
            "\n\nTop refactor (`{}`): {}",
            plan.rule_id, plan.title
        ));
    }

    value
}
