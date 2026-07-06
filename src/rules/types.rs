use serde::{Deserialize, Serialize};

pub use crate::classes::{Applicability, CodeSnippet, RuleCategory};

use crate::refactor_plans::ComplexityRegion;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RuleMetadata {
    pub id: String,
    pub name: String,
    pub category: RuleCategory,
    pub description: String,
    pub applicability: Applicability,
    pub priority: u8,
    pub doc_url: String,
}

pub trait RefactorRule: Sync + Send {
    fn metadata(&self) -> &'static RuleMetadata;

    fn check(
        &self,
        region: &ComplexityRegion,
        source: &str,
        function_complexity: u64,
    ) -> Option<crate::classes::RefactorPlan>;
}

#[must_use]
pub fn extract_code_snippet(source: &str, line_start: u64, line_end: u64) -> CodeSnippet {
    let lines: Vec<&str> = source.lines().collect();
    let start = (line_start.saturating_sub(1)) as usize;
    let end = (line_end as usize).min(lines.len());

    if start >= lines.len() {
        return CodeSnippet {
            text: String::new(),
            line_start,
            line_end,
        };
    }

    let text = lines[start..end].join("\n");
    CodeSnippet {
        text,
        line_start,
        line_end,
    }
}
