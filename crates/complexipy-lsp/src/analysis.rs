use complexipy_core::classes::FunctionComplexity;
use complexipy_core::cognitive_complexity::code_complexity_shared;
use complexipy_core::config::{InlayHints, LspConfig};
use lsp_types::{
    Diagnostic, DiagnosticSeverity, Hover, HoverContents, InlayHint, InlayHintKind, InlayHintLabel,
    MarkupContent, MarkupKind, NumberOrString, Position, Range,
};

pub const DIAGNOSTIC_CODE: &str = "cognitive-complexity";
pub const DIAGNOSTIC_SOURCE: &str = "complexipy";
pub const FUNCTION_HINT_LABEL: &str = "cognitive: ";
pub const LINE_HINT_LABEL: &str = "+";

#[derive(Clone)]
pub struct DocumentAnalysis {
    pub version: i32,
    pub functions: Vec<FunctionComplexity>,
}

impl DocumentAnalysis {
    pub fn new(version: i32, functions: Vec<FunctionComplexity>) -> Self {
        Self { version, functions }
    }

    pub fn empty(version: i32) -> Self {
        Self::new(version, Vec::new())
    }

    pub fn hints(
        &self,
        text: &str,
        config: &LspConfig,
        requested: Option<Range>,
    ) -> Vec<InlayHint> {
        if config.lsp.inlay_hints == InlayHints::Never {
            return Vec::new();
        }

        let mut hints = Vec::new();

        for function in &self.functions {
            let definition = line_end_position(text, definition_line(text, function));

            if self.shows_function_hint(function, config)
                && is_within(definition, requested.as_ref())
            {
                hints.push(function_hint(definition, function.complexity));
            }

            if config.lsp.per_line_hints {
                for line in function
                    .line_complexities
                    .iter()
                    .filter(|line| line.complexity > 0)
                {
                    let position = line_end_position(text, line.line);
                    if is_within(position, requested.as_ref()) {
                        hints.push(line_hint(position, line.complexity));
                    }
                }
            }
        }

        hints
    }

    pub fn diagnostics(&self, text: &str, config: &LspConfig) -> Vec<Diagnostic> {
        if !config.lsp.diagnostics {
            return Vec::new();
        }

        self.functions
            .iter()
            .filter(|function| is_over_threshold(function.complexity, config))
            .map(|function| diagnostic(text, function, config))
            .collect()
    }

    pub fn hover(&self, text: &str, config: &LspConfig, position: Position) -> Option<Hover> {
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
            range: Some(function_range(text, function)),
        })
    }

    fn shows_function_hint(&self, function: &FunctionComplexity, config: &LspConfig) -> bool {
        match config.lsp.inlay_hints {
            InlayHints::Never => false,
            InlayHints::Always => true,
            InlayHints::Threshold => is_over_threshold(function.complexity, config),
        }
    }
}

pub fn analyze(text: &str, version: i32, config: &LspConfig) -> Result<DocumentAnalysis, String> {
    let result = code_complexity_shared(text, false, config.no_ignore)?;
    Ok(DocumentAnalysis::new(version, result.functions))
}

pub fn is_stale(computed_version: i32, stored_version: Option<i32>) -> bool {
    stored_version != Some(computed_version)
}

pub fn is_over_threshold(complexity: u64, config: &LspConfig) -> bool {
    complexity > config.max_complexity_allowed
}

pub fn line_end_position(text: &str, line: u64) -> Position {
    let index = line.saturating_sub(1) as usize;
    let content = text.lines().nth(index).unwrap_or("");

    Position::new(index as u32, content.encode_utf16().count() as u32)
}

pub fn definition_line(text: &str, function: &FunctionComplexity) -> u64 {
    let start = function.line_start;
    let end = function.line_end.max(start);
    let lines: Vec<&str> = text.lines().collect();

    (start..=end)
        .find(|line| {
            lines
                .get(line.saturating_sub(1) as usize)
                .map(|content| {
                    let trimmed = content.trim_start();
                    trimmed.starts_with("def ") || trimmed.starts_with("async def ")
                })
                .unwrap_or(false)
        })
        .unwrap_or(start)
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

fn diagnostic(text: &str, function: &FunctionComplexity, config: &LspConfig) -> Diagnostic {
    Diagnostic {
        range: function_range(text, function),
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

fn function_range(text: &str, function: &FunctionComplexity) -> Range {
    Range::new(
        Position::new(function.line_start.saturating_sub(1) as u32, 0),
        line_end_position(text, function.line_end),
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
