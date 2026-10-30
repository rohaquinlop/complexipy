use crate::classes::{Applicability, CodeInsertion, CodeSuggestion, RefactorPlan};
use crate::syntax_tree::parse_module;
use crate::utils::LineIndex;

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
    pub insertions: Vec<CodeInsertion>,
    pub reduction: u64,
    pub reduction_is_measured: bool,
}

/// One contiguous line change of an applied fix, in the lines of the text
/// the pass started from: `removed` lines from `line_start` gave way to
/// `added` lines. An insertion removes no line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LineEdit<'a> {
    pub line_start: u64,
    pub removed: u64,
    pub added: u64,
    pub fix: &'a AppliedFix,
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

impl FixReport {
    /// The line edits of the applied fixes, ordered by line, with an
    /// insertion before a replacement that starts on the same line.
    #[must_use]
    pub fn line_edits(&self) -> Vec<LineEdit<'_>> {
        let mut edits: Vec<LineEdit<'_>> = self
            .applied
            .iter()
            .flat_map(|fix| {
                fix.insertions
                    .iter()
                    .map(move |insertion| LineEdit {
                        line_start: insertion.before_line,
                        removed: 0,
                        added: insertion.text.lines().count() as u64,
                        fix,
                    })
                    .chain(std::iter::once(LineEdit {
                        line_start: fix.line_start,
                        removed: fix.line_end - fix.line_start + 1,
                        added: fix.replacement.lines().count() as u64,
                        fix,
                    }))
            })
            .collect();
        edits.sort_by_key(|edit| (edit.line_start, edit.removed));
        edits
    }
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

    /// Registers one line edit like `register_fix`. An insertion counts as
    /// a replacement of the line it goes in front of, so its lines map to
    /// that line.
    pub fn register_edit(&mut self, edit: &LineEdit<'_>) {
        if edit.removed == 0 {
            self.register_fix(edit.line_start, edit.line_start, edit.added + 1);
        } else {
            self.register_fix(
                edit.line_start,
                edit.line_start + edit.removed - 1,
                edit.added,
            );
        }
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

/// Applies the fixable plans to the source. Every edit is placed by the byte
/// offsets of the original text, and a plan whose edits break the parse is
/// skipped, so the returned text always parses.
#[must_use]
pub fn apply_fixes(source: &str, plans: &[RefactorPlan]) -> FixReport {
    let index = LineIndex::new(source);
    let newline = line_ending_of(source);
    let mut ordered: Vec<&RefactorPlan> = plans.iter().collect();
    ordered.sort_by_key(|plan| (plan.line_start, plan.line_end));
    ordered.reverse();

    let mut report = FixReport {
        patched: source.to_string(),
        applied: Vec::new(),
        skipped: Vec::new(),
    };
    let mut applied_spans: Vec<(u64, u64)> = Vec::new();
    let mut applied_insertions: Vec<u64> = Vec::new();
    let mut accepted: Vec<Edit> = Vec::new();

    for plan in ordered {
        let Some(suggestion) = plan.suggestion.as_ref().filter(|_| fixable(plan)) else {
            report
                .skipped
                .push(SkippedFix::from_plan(plan, SkipReason::NotFixable));
            continue;
        };

        let inside = |line: u64, (start, end): (u64, u64)| start < line && line <= end;
        let overlaps_applied = applied_spans
            .iter()
            .any(|(start, end)| plan.line_start <= *end && *start <= plan.line_end)
            || suggestion.insertions.iter().any(|insertion| {
                applied_spans
                    .iter()
                    .any(|span| inside(insertion.before_line, *span))
            })
            || applied_insertions
                .iter()
                .any(|line| inside(*line, (plan.line_start, plan.line_end)));
        if overlaps_applied {
            report
                .skipped
                .push(SkippedFix::from_plan(plan, SkipReason::Overlap));
            continue;
        }

        let patched = plan_edits(plan, suggestion, source, &index, newline).and_then(|edits| {
            let text = render_edits(source, accepted.iter().chain(edits.iter()));
            parse_module(&text).is_ok().then_some((text, edits))
        });

        match patched {
            Some((text, edits)) => {
                report.patched = text;
                accepted.extend(edits);
                applied_spans.push((plan.line_start, plan.line_end));
                applied_insertions.extend(suggestion.insertions.iter().map(|i| i.before_line));
                report.applied.push(AppliedFix {
                    rule_id: plan.rule_id.clone(),
                    title: plan.title.clone(),
                    line_start: plan.line_start,
                    line_end: plan.line_end,
                    replacement: suggestion.replacement.clone(),
                    insertions: suggestion.insertions.clone(),
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

struct Edit {
    start: usize,
    end: usize,
    text: String,
}

fn plan_edits(
    plan: &RefactorPlan,
    suggestion: &CodeSuggestion,
    source: &str,
    index: &LineIndex,
    newline: &str,
) -> Option<Vec<Edit>> {
    let mut edits = vec![span_edit(plan, suggestion, source, index, newline)?];
    for insertion in &suggestion.insertions {
        let at = index.byte_of_line(insertion.before_line)?;
        let mut text = normalized(&insertion.text);
        if !text.ends_with('\n') {
            text.push('\n');
        }
        edits.push(Edit {
            start: at,
            end: at,
            text: text.replace('\n', newline),
        });
    }
    Some(edits)
}

fn render_edits<'a>(source: &str, edits: impl Iterator<Item = &'a Edit>) -> String {
    let mut ordered: Vec<&Edit> = edits.collect();
    ordered.sort_by_key(|edit| (edit.start, edit.end));
    let mut rendered = String::with_capacity(source.len());
    let mut cursor = 0;
    for edit in ordered {
        rendered.push_str(&source[cursor..edit.start]);
        rendered.push_str(&edit.text);
        cursor = edit.end;
    }
    rendered.push_str(&source[cursor..]);
    rendered
}

fn normalized(text: &str) -> String {
    text.replace("\r\n", "\n").replace('\r', "\n")
}

/// The replacement's line breaks become `newline`, and its trailing line
/// breaks collapse into the one terminator the replaced range carried.
fn span_edit(
    plan: &RefactorPlan,
    suggestion: &CodeSuggestion,
    source: &str,
    index: &LineIndex,
    newline: &str,
) -> Option<Edit> {
    let start = index.byte_of_line(plan.line_start)?;
    let end = index
        .byte_of_line(plan.line_end + 1)
        .unwrap_or(source.len());
    if start > end {
        return None;
    }
    let mut text = normalized(&suggestion.replacement)
        .trim_end_matches('\n')
        .replace('\n', newline);
    if end < source.len() || source.ends_with(['\r', '\n']) {
        text.push_str(newline);
    }
    Some(Edit { start, end, text })
}

/// Replaces the plan's line range with the replacement. Insertions take no
/// part: the function holding the plan scores the same without its helper.
pub(crate) fn splice_plan(
    plan: &RefactorPlan,
    suggestion: &CodeSuggestion,
    source: &str,
    index: &LineIndex,
    newline: &str,
) -> Option<String> {
    splice_window(plan, suggestion, source, index, newline, (0, source.len()))
}

/// Splices like `splice_plan`, but returns only the text between the two
/// byte offsets of `window`, with the replacement in place. Returns `None`
/// when the plan's range does not lie inside the window.
pub(crate) fn splice_window(
    plan: &RefactorPlan,
    suggestion: &CodeSuggestion,
    source: &str,
    index: &LineIndex,
    newline: &str,
    window: (usize, usize),
) -> Option<String> {
    let (window_start, window_end) = window;
    let edit = span_edit(plan, suggestion, source, index, newline)?;
    if edit.start < window_start || edit.end > window_end {
        return None;
    }
    let mut spliced = String::with_capacity(window_end - window_start + edit.text.len());
    spliced.push_str(&source[window_start..edit.start]);
    spliced.push_str(&edit.text);
    spliced.push_str(&source[edit.end..window_end]);
    Some(spliced)
}

pub(crate) fn line_ending_of(source: &str) -> &'static str {
    let crlf = source.matches("\r\n").count();
    let lf = source.matches('\n').count() - crlf;
    let cr = source.matches('\r').count() - crlf;
    if cr > lf && cr > crlf {
        "\r"
    } else if crlf > lf {
        "\r\n"
    } else {
        "\n"
    }
}

#[cfg(test)]
mod tests;
