use crate::classes::{Applicability, CodeSuggestion, RefactorPlan};
use crate::utils::LineIndex;
use ruff_python_parser::parse_module;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SkipReason {
    NotFixable,
    Overlap,
    ParseFailure,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppliedFix {
    pub rule_id: String,
    pub title: String,
    pub line_start: u64,
    pub line_end: u64,
    pub replacement: String,
    pub reduction: u64,
    pub reduction_is_measured: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SkippedFix {
    pub rule_id: String,
    pub line_start: u64,
    pub line_end: u64,
    pub reason: SkipReason,
}

impl SkippedFix {
    fn from_plan(plan: &RefactorPlan, reason: SkipReason) -> Self {
        Self {
            rule_id: plan.rule_id.clone(),
            line_start: plan.line_start,
            line_end: plan.line_end,
            reason,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FixReport {
    pub patched: String,
    pub applied: Vec<AppliedFix>,
    pub skipped: Vec<SkippedFix>,
}

/// Returns whether the plan's suggestion is a faithful source splice that a
/// fix may write. The tier must be `MachineApplicable` and the suggestion
/// must be spliceable; the rule identity takes no part in the decision.
#[must_use]
pub fn fixable(plan: &RefactorPlan) -> bool {
    plan.applicability == Applicability::MachineApplicable
        && plan
            .suggestion
            .as_ref()
            .is_some_and(|suggestion| suggestion.spliceable)
}

/// Returns whether the text parses as a Python module.
#[must_use]
pub fn parses(source: &str) -> bool {
    parse_module(source).is_ok()
}

/// Applies the fixable plans to the source, bottom-to-top so the pending
/// spans stay valid, parse-checking after each splice. A splice that breaks
/// the parse is reverted and recorded as skipped. The returned text always
/// parses.
#[must_use]
pub fn apply_fixes(source: &str, plans: &[RefactorPlan]) -> FixReport {
    let index = LineIndex::new(source);
    let mut ordered: Vec<&RefactorPlan> = plans.iter().collect();
    ordered.sort_by_key(|plan| (plan.line_start, plan.line_end));
    ordered.reverse();

    let mut report = FixReport {
        patched: source.to_string(),
        applied: Vec::new(),
        skipped: Vec::new(),
    };
    let mut applied_spans: Vec<(u64, u64)> = Vec::new();

    for plan in ordered {
        let Some(suggestion) = plan.suggestion.as_ref().filter(|_| fixable(plan)) else {
            report
                .skipped
                .push(SkippedFix::from_plan(plan, SkipReason::NotFixable));
            continue;
        };

        let overlaps_applied = applied_spans
            .iter()
            .any(|(start, end)| plan.line_start <= *end && *start <= plan.line_end);
        if overlaps_applied {
            report
                .skipped
                .push(SkippedFix::from_plan(plan, SkipReason::Overlap));
            continue;
        }

        let spliced = splice_plan(plan, suggestion, &report.patched, &index)
            .filter(|text| parse_module(text).is_ok());

        match spliced {
            Some(patched) => {
                report.patched = patched;
                applied_spans.push((plan.line_start, plan.line_end));
                report.applied.push(AppliedFix {
                    rule_id: plan.rule_id.clone(),
                    title: plan.title.clone(),
                    line_start: plan.line_start,
                    line_end: plan.line_end,
                    replacement: suggestion.replacement.clone(),
                    reduction: plan.estimated_reduction,
                    reduction_is_measured: plan.reduction_is_measured,
                });
            }
            None => report
                .skipped
                .push(SkippedFix::from_plan(plan, SkipReason::ParseFailure)),
        }
    }

    report
        .applied
        .sort_by_key(|applied| (applied.line_start, applied.line_end));
    report
        .skipped
        .sort_by_key(|skipped| (skipped.line_start, skipped.line_end));
    report
}

/// Replaces the plan's line range in the source with the replacement. The
/// byte offsets come from the index of the original text; callers splice
/// bottom-to-top so the offsets of a pending span stay valid in the
/// progressively patched text.
pub(crate) fn splice_plan(
    plan: &RefactorPlan,
    suggestion: &CodeSuggestion,
    source: &str,
    index: &LineIndex,
) -> Option<String> {
    let byte_start = index.byte_of_line(plan.line_start)?;
    let byte_end = index
        .byte_of_line(plan.line_end + 1)
        .unwrap_or(source.len());
    if byte_start > byte_end {
        return None;
    }
    let mut spliced = String::with_capacity(source.len() + suggestion.replacement.len());
    spliced.push_str(&source[..byte_start]);
    let newline = line_ending_of(source);
    spliced.push_str(
        &suggestion
            .replacement
            .replace("\r\n", "\n")
            .replace('\n', newline),
    );
    if byte_end < source.len() || source.ends_with('\n') {
        spliced.push_str(newline);
    }
    spliced.push_str(&source[byte_end..]);
    Some(spliced)
}

fn line_ending_of(source: &str) -> &'static str {
    let crlf = source.matches("\r\n").count();
    let lf = source.matches('\n').count() - crlf;
    if crlf > lf { "\r\n" } else { "\n" }
}

#[cfg(test)]
mod tests;
