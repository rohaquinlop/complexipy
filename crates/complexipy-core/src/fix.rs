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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Segment {
    current_start: u64,
    current_len: u64,
    original_start: u64,
    original_end: u64,
    coarse: bool,
}

impl Segment {
    fn current_end(&self) -> u64 {
        self.current_start + self.current_len - 1
    }

    fn shift(&self, delta: i64) -> Self {
        Self {
            current_start: (self.current_start as i64 + delta) as u64,
            ..*self
        }
    }

    fn mapped(&self, from: u64, to: u64) -> (u64, u64) {
        if self.coarse {
            return (self.original_start, self.original_end);
        }
        (
            self.original_start + (from - self.current_start),
            self.original_start + (to - self.current_start),
        )
    }
}

/// Tracks how the fix passes rewrite a file, so a line of any later text
/// maps back to the lines the file had before the run. Lines inside a
/// replaced span map to the original span of that replaced region.
pub struct LineMap {
    segments: Vec<Segment>,
}

impl LineMap {
    /// Builds the identity map for a file with `line_count` lines.
    #[must_use]
    pub fn new(line_count: u64) -> Self {
        let mut segments = Vec::new();
        if line_count > 0 {
            segments.push(Segment {
                current_start: 1,
                current_len: line_count,
                original_start: 1,
                original_end: line_count,
                coarse: false,
            });
        }
        Self { segments }
    }

    /// Registers one applied fix, whose span lies in the current text and
    /// whose replacement holds `new_line_count` lines. Register the fixes
    /// of one pass from the highest span down, so every span stays valid.
    pub fn register_fix(&mut self, line_start: u64, line_end: u64, new_line_count: u64) {
        let delta = new_line_count as i64 - (line_end - line_start + 1) as i64;
        let mut rebuilt: Vec<Segment> = Vec::with_capacity(self.segments.len() + 1);
        let mut covered: Option<(u64, u64)> = None;
        let mut insert_at = rebuilt.len();
        for segment in self.segments.iter().copied() {
            if segment.current_len == 0 {
                if segment.current_start < line_start {
                    rebuilt.push(segment);
                } else {
                    rebuilt.push(segment.shift(delta));
                }
                continue;
            }
            let end = segment.current_end();
            if end < line_start {
                rebuilt.push(segment);
                continue;
            }
            if segment.current_start > line_end {
                rebuilt.push(segment.shift(delta));
                continue;
            }
            if covered.is_none() {
                insert_at = rebuilt.len() + usize::from(segment.current_start < line_start);
            }
            let from = line_start.max(segment.current_start);
            let to = line_end.min(end);
            let (mapped_start, mapped_end) = segment.mapped(from, to);
            covered = Some(match covered {
                Some((start, finish)) => (start.min(mapped_start), finish.max(mapped_end)),
                None => (mapped_start, mapped_end),
            });
            if segment.current_start < line_start {
                rebuilt.push(Segment {
                    current_len: line_start - segment.current_start,
                    ..segment
                });
            }
            if end > line_end {
                let offset = line_end + 1 - segment.current_start;
                rebuilt.push(
                    Segment {
                        current_start: line_end + 1,
                        current_len: end - line_end,
                        original_start: if segment.coarse {
                            segment.original_start
                        } else {
                            segment.original_start + offset
                        },
                        ..segment
                    }
                    .shift(delta),
                );
            }
        }
        if let Some((original_start, original_end)) = covered {
            rebuilt.insert(
                insert_at,
                Segment {
                    current_start: line_start,
                    current_len: new_line_count,
                    original_start,
                    original_end,
                    coarse: true,
                },
            );
        }
        self.segments = rebuilt;
    }

    /// Returns the original span that covers the given current span.
    /// Lines in untouched text map exactly; lines inside a replaced span
    /// map to the original span of that replaced region.
    #[must_use]
    pub fn original_span(&self, line_start: u64, line_end: u64) -> (u64, u64) {
        let mut result: Option<(u64, u64)> = None;
        for segment in &self.segments {
            if segment.current_len == 0 {
                continue;
            }
            let end = segment.current_end();
            if end < line_start || segment.current_start > line_end {
                continue;
            }
            let from = line_start.max(segment.current_start);
            let to = line_end.min(end);
            let (mapped_start, mapped_end) = segment.mapped(from, to);
            result = Some(match result {
                Some((start, finish)) => (start.min(mapped_start), finish.max(mapped_end)),
                None => (mapped_start, mapped_end),
            });
        }
        result.unwrap_or((line_start, line_end))
    }
}

/// Returns whether the plan's suggestion is a faithful source splice that a
/// fix may write. The tier must be `MachineApplicable` on the plan and on
/// the suggestion, and the suggestion must be spliceable; the rule identity
/// takes no part in the decision.
#[must_use]
pub fn fixable(plan: &RefactorPlan) -> bool {
    plan.applicability == Applicability::MachineApplicable
        && plan.suggestion.as_ref().is_some_and(|suggestion| {
            suggestion.spliceable && suggestion.applicability == Applicability::MachineApplicable
        })
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
/// progressively patched text. The replacement's line breaks become the
/// file's line breaks, and its trailing line breaks collapse into the one
/// terminator the replaced range carried.
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
    let normalized = suggestion
        .replacement
        .replace("\r\n", "\n")
        .replace('\r', "\n");
    spliced.push_str(&normalized.trim_end_matches('\n').replace('\n', newline));
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
