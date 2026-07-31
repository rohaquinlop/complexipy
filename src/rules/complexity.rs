use crate::classes::{Applicability, CodeSuggestion, RefactorPlan, RuleCategory};
use crate::refactor_plans::{ComplexityRegion, RegionKind};
use crate::rules::types::{RefactorRule, RuleMetadata};
use std::sync::OnceLock;

pub struct FlattenConditionRule;

impl RefactorRule for FlattenConditionRule {
    fn metadata(&self) -> &'static RuleMetadata {
        static META: OnceLock<RuleMetadata> = OnceLock::new();
        META.get_or_init(|| RuleMetadata {
            id: "C001".to_string(),
            name: "flatten_condition".to_string(),
            category: RuleCategory::Complexity,
            description: "Flatten nested condition blocks by using guard clauses with early returns".to_string(),
            applicability: Applicability::Informational,
            priority: 4,
            doc_url: "https://rohaquinlop.github.io/complexipy/refactoring-rules/#c001-flatten-nested-conditions".to_string(),
        })
    }

    fn check(
        &self,
        region: &ComplexityRegion,
        _source: &str,
        function_complexity: u64,
    ) -> Option<RefactorPlan> {
        if region.kind != RegionKind::If || region.nesting < 2 || region.total < 4 {
            return None;
        }

        Some(RefactorPlan {
            kind: "flatten_condition".to_string(),
            title: "Flatten nested condition block with guard clauses".to_string(),
            line_start: region.line_start,
            line_end: region.line_end,
            current_complexity: function_complexity,
            estimated_reduction: region.nesting,
            estimated_complexity_after: function_complexity.saturating_sub(region.nesting),
            rule_id: "C001".to_string(),
            category: RuleCategory::Complexity,
            applicability: Applicability::Informational,
            description:
                "Flatten nested condition blocks by using guard clauses with early returns"
                    .to_string(),
            explanation: "Deeply nested conditions are hard to follow. Using guard clauses \
                         with early returns reduces cognitive load by keeping the main path \
                         at a lower indentation level."
                .to_string(),
            references: vec![],
            suggestion: None,
            help: Some(
                "Invert the outer condition and return early. Move the main success path \
                        one indentation level left. Repeat for inner nested conditions where safe."
                    .to_string(),
            ),
        })
    }
}

pub struct LoopGuardsRule;

impl RefactorRule for LoopGuardsRule {
    fn metadata(&self) -> &'static RuleMetadata {
        static META: OnceLock<RuleMetadata> = OnceLock::new();
        META.get_or_init(|| RuleMetadata {
            id: "C002".to_string(),
            name: "loop_guards".to_string(),
            category: RuleCategory::Complexity,
            description: "Use continue guards at the top of loops to reduce nesting".to_string(),
            applicability: Applicability::MachineApplicable,
            priority: 3,
            doc_url: "https://rohaquinlop.github.io/complexipy/refactoring-rules/#c002-loop-guards"
                .to_string(),
        })
    }

    fn check(
        &self,
        region: &ComplexityRegion,
        source: &str,
        function_complexity: u64,
    ) -> Option<RefactorPlan> {
        if region.kind != RegionKind::Loop || region.total < 5 {
            return None;
        }

        let has_nested_if = region
            .children
            .iter()
            .any(|child| child.kind == RegionKind::If && child.nesting >= 1);

        if !has_nested_if {
            return None;
        }

        let reduction =
            sum_if_nesting(&region.children).min(region.total.saturating_sub(region.structural));

        if reduction == 0 {
            return None;
        }

        let suggestion = generate_loop_guard_suggestion(region, source);
        let help = if suggestion.is_none() {
            Some(
                "Add `if not <condition>: continue` guards at the top of the loop body \
                 for each nested if condition, then dedent the remaining logic by one \
                 level per guard."
                    .to_string(),
            )
        } else {
            None
        };

        Some(RefactorPlan {
            kind: "loop_guards".to_string(),
            title: "Flatten loop body with continue guards".to_string(),
            line_start: region.line_start,
            line_end: region.line_end,
            current_complexity: function_complexity,
            estimated_reduction: reduction,
            estimated_complexity_after: function_complexity.saturating_sub(reduction),
            rule_id: "C002".to_string(),
            category: RuleCategory::Complexity,
            applicability: Applicability::MachineApplicable,
            description: "Use continue guards at the top of loops to reduce nesting".to_string(),
            explanation: "Nested conditions inside loops add unnecessary indentation. \
                         Using continue guards keeps the main logic at a lower nesting level \
                         and makes the loop easier to follow."
                .to_string(),
            references: vec![],
            suggestion,
            help,
        })
    }
}

pub struct ExtractHelperRule;

impl RefactorRule for ExtractHelperRule {
    fn metadata(&self) -> &'static RuleMetadata {
        static META: OnceLock<RuleMetadata> = OnceLock::new();
        META.get_or_init(|| RuleMetadata {
            id: "C003".to_string(),
            name: "extract_helper".to_string(),
            category: RuleCategory::Complexity,
            description: "Extract complex code blocks into separate helper functions".to_string(),
            applicability: Applicability::Informational,
            priority: 3,
            doc_url: "https://rohaquinlop.github.io/complexipy/refactoring-rules/#c003-extract-helper-function".to_string(),
        })
    }

    fn check(
        &self,
        region: &ComplexityRegion,
        _source: &str,
        function_complexity: u64,
    ) -> Option<RefactorPlan> {
        let line_count = region.line_end.saturating_sub(region.line_start) + 1;

        if region.total < 6 || line_count < 5 {
            return None;
        }

        Some(RefactorPlan {
            kind: "extract_helper".to_string(),
            title: "Extract complex block into helper function".to_string(),
            line_start: region.line_start,
            line_end: region.line_end,
            current_complexity: function_complexity,
            estimated_reduction: region.total.saturating_sub(1),
            estimated_complexity_after: function_complexity
                .saturating_sub(region.total.saturating_sub(1)),
            rule_id: "C003".to_string(),
            category: RuleCategory::Complexity,
            applicability: Applicability::Informational,
            description: "Extract complex code blocks into separate helper functions".to_string(),
            explanation: "Complex code blocks should be extracted into named functions \
                         to improve readability and testability. The extracted function \
                         can be given a descriptive name that explains its purpose."
                .to_string(),
            references: vec![],
            suggestion: None,
            help: Some(format!(
                "Extract lines {}-{} into a named helper function. Pass required \
                              values as parameters and return the result needed by the caller.",
                region.line_start, region.line_end
            )),
        })
    }
}

pub struct SplitDispatcherRule;

impl RefactorRule for SplitDispatcherRule {
    fn metadata(&self) -> &'static RuleMetadata {
        static META: OnceLock<RuleMetadata> = OnceLock::new();
        META.get_or_init(|| RuleMetadata {
            id: "C004".to_string(),
            name: "split_dispatcher".to_string(),
            category: RuleCategory::Complexity,
            description: "Split long elif chains or match statements into separate handlers"
                .to_string(),
            applicability: Applicability::Informational,
            priority: 2,
            doc_url:
                "https://rohaquinlop.github.io/complexipy/refactoring-rules/#c004-split-dispatcher"
                    .to_string(),
        })
    }

    fn check(
        &self,
        region: &ComplexityRegion,
        _source: &str,
        function_complexity: u64,
    ) -> Option<RefactorPlan> {
        let is_long_elif = region.kind == RegionKind::ElifChain && region.elif_count >= 3;
        let is_long_match = region.kind == RegionKind::Match && region.case_count >= 4;

        if !is_long_elif && !is_long_match {
            return None;
        }

        let reduction = region
            .elif_count
            .max(region.case_count)
            .saturating_sub(1)
            .max(2);

        Some(RefactorPlan {
            kind: "split_dispatcher".to_string(),
            title: "Split conditional dispatcher into handlers".to_string(),
            line_start: region.line_start,
            line_end: region.line_end,
            current_complexity: function_complexity,
            estimated_reduction: reduction,
            estimated_complexity_after: function_complexity.saturating_sub(reduction),
            rule_id: "C004".to_string(),
            category: RuleCategory::Complexity,
            applicability: Applicability::Informational,
            description: "Split long elif chains or match statements into separate handlers"
                .to_string(),
            explanation: "Long conditional chains are hard to maintain and extend. \
                         Splitting them into separate handlers makes each case \
                         independently testable and the dispatch logic clearer."
                .to_string(),
            references: vec![],
            suggestion: None,
            help: Some(format!(
                "Replace the {}-branch chain with a dispatch dictionary mapping \
                              cases to handler functions. Each handler becomes independently testable.",
                region.elif_count.max(region.case_count)
            )),
        })
    }
}

pub struct ExtractPredicateRule;

impl RefactorRule for ExtractPredicateRule {
    fn metadata(&self) -> &'static RuleMetadata {
        static META: OnceLock<RuleMetadata> = OnceLock::new();
        META.get_or_init(|| RuleMetadata {
            id: "C005".to_string(),
            name: "extract_predicate".to_string(),
            category: RuleCategory::Readability,
            description: "Extract complex boolean conditions into named predicate functions"
                .to_string(),
            applicability: Applicability::MachineApplicable,
            priority: 3,
            doc_url:
                "https://rohaquinlop.github.io/complexipy/refactoring-rules/#c005-extract-predicate"
                    .to_string(),
        })
    }

    fn check(
        &self,
        region: &ComplexityRegion,
        source: &str,
        function_complexity: u64,
    ) -> Option<RefactorPlan> {
        if region.kind != RegionKind::BooleanCondition || region.boolean < 2 {
            return None;
        }

        let suggestion = generate_predicate_suggestion(region, source);
        let help = if suggestion.is_none() {
            Some(
                "Extract this boolean condition into a small named function that returns \
                 a bool, then call that function in place of the inline expression."
                    .to_string(),
            )
        } else {
            None
        };

        Some(RefactorPlan {
            kind: "extract_predicate".to_string(),
            title: "Extract complex condition into named predicate".to_string(),
            line_start: region.line_start,
            line_end: region.line_end,
            current_complexity: function_complexity,
            estimated_reduction: region.bool_op_count,
            estimated_complexity_after: function_complexity.saturating_sub(region.bool_op_count),
            rule_id: "C005".to_string(),
            category: RuleCategory::Readability,
            applicability: Applicability::MachineApplicable,
            description: "Extract complex boolean conditions into named predicate functions"
                .to_string(),
            explanation: "Complex boolean expressions are hard to understand at a glance. \
                         Extracting them into named predicates makes the code self-documenting \
                         and easier to test."
                .to_string(),
            references: vec![],
            suggestion,
            help,
        })
    }
}

pub struct ReduceNestingRule;

impl RefactorRule for ReduceNestingRule {
    fn metadata(&self) -> &'static RuleMetadata {
        static META: OnceLock<RuleMetadata> = OnceLock::new();
        META.get_or_init(|| RuleMetadata {
            id: "C006".to_string(),
            name: "reduce_nesting".to_string(),
            category: RuleCategory::Complexity,
            description: "Reduce nesting depth by using early returns and guard clauses".to_string(),
            applicability: Applicability::Informational,
            priority: 4,
            doc_url: "https://rohaquinlop.github.io/complexipy/refactoring-rules/#c006-reduce-nesting-depth".to_string(),
        })
    }

    fn check(
        &self,
        region: &ComplexityRegion,
        _source: &str,
        function_complexity: u64,
    ) -> Option<RefactorPlan> {
        if region.nesting < 3 || region.total < 5 {
            return None;
        }

        if region.kind == RegionKind::If && region.nesting >= 2 && region.total >= 4 {
            return None;
        }

        Some(RefactorPlan {
            kind: "reduce_nesting".to_string(),
            title: "Reduce nesting depth".to_string(),
            line_start: region.line_start,
            line_end: region.line_end,
            current_complexity: function_complexity,
            estimated_reduction: region.nesting.saturating_sub(1),
            estimated_complexity_after: function_complexity
                .saturating_sub(region.nesting.saturating_sub(1)),
            rule_id: "C006".to_string(),
            category: RuleCategory::Complexity,
            applicability: Applicability::Informational,
            description: "Reduce nesting depth by using early returns and guard clauses"
                .to_string(),
            explanation: "Deep nesting (3+ levels) makes code hard to follow. \
                         Consider extracting inner blocks or using guard clauses \
                         to keep indentation shallow."
                .to_string(),
            references: vec![],
            suggestion: None,
            help: Some(
                "Identify the deepest nesting level. Extract inner logic into a \
                        separate function or use early returns to reduce indentation."
                    .to_string(),
            ),
        })
    }
}

pub struct FlattenTryRule;

impl RefactorRule for FlattenTryRule {
    fn metadata(&self) -> &'static RuleMetadata {
        static META: OnceLock<RuleMetadata> = OnceLock::new();
        META.get_or_init(|| RuleMetadata {
            id: "C011".to_string(),
            name: "flatten_try".to_string(),
            category: RuleCategory::Complexity,
            description: "Flatten nested try/except blocks by combining or restructuring"
                .to_string(),
            applicability: Applicability::Informational,
            priority: 2,
            doc_url:
                "https://rohaquinlop.github.io/complexipy/refactoring-rules/#c011-flatten-tryexcept"
                    .to_string(),
        })
    }

    fn check(
        &self,
        region: &ComplexityRegion,
        _source: &str,
        function_complexity: u64,
    ) -> Option<RefactorPlan> {
        if region.kind != RegionKind::Try {
            return None;
        }

        let has_nested_try = region
            .children
            .iter()
            .any(|child| child.kind == RegionKind::Try);

        if !has_nested_try || region.total < 4 {
            return None;
        }

        Some(RefactorPlan {
            kind: "flatten_try".to_string(),
            title: "Flatten nested try/except blocks".to_string(),
            line_start: region.line_start,
            line_end: region.line_end,
            current_complexity: function_complexity,
            estimated_reduction: 2,
            estimated_complexity_after: function_complexity.saturating_sub(2),
            rule_id: "C011".to_string(),
            category: RuleCategory::Complexity,
            applicability: Applicability::Informational,
            description: "Flatten nested try/except blocks by combining or restructuring"
                .to_string(),
            explanation: "Nested try/except blocks are confusing and hard to maintain. \
                         Consider merging them or extracting the inner block into \
                         a separate function with its own error handling."
                .to_string(),
            references: vec![],
            suggestion: None,
            help: Some(
                "Review if inner try/except can be merged with outer. Consider using \
                        a single try with multiple except clauses or extract inner try block \
                        into a helper function."
                    .to_string(),
            ),
        })
    }
}

pub struct CollapsibleIfRule;

impl RefactorRule for CollapsibleIfRule {
    fn metadata(&self) -> &'static RuleMetadata {
        static META: OnceLock<RuleMetadata> = OnceLock::new();
        META.get_or_init(|| RuleMetadata {
            id: "C007".to_string(),
            name: "collapsible_if".to_string(),
            category: RuleCategory::Readability,
            description: "Merge nested if statements into a single if with combined conditions"
                .to_string(),
            applicability: Applicability::MachineApplicable,
            priority: 4,
            doc_url:
                "https://rohaquinlop.github.io/complexipy/refactoring-rules/#c007-collapsible-if"
                    .to_string(),
        })
    }

    fn check(
        &self,
        region: &ComplexityRegion,
        source: &str,
        function_complexity: u64,
    ) -> Option<RefactorPlan> {
        if region.kind != RegionKind::If {
            return None;
        }

        // Collect chain of nested ifs
        let lines: Vec<&str> = source.lines().collect();
        let chain = collect_if_chain(region, &lines);

        // Need at least 2 ifs to merge
        if chain.len() < 2 {
            return None;
        }

        // Get the innermost if (last in chain)
        let innermost = chain.last().unwrap();

        // Extract conditions from all ifs in the chain. If any condition can't be
        // extracted with confidence, we still emit the plan (the diagnosis itself —
        // "these nested ifs can be merged" — is still valid) but fall back to `help`
        // text instead of a machine-applicable `suggestion` we can't stand behind.
        let mut conditions = Vec::new();
        let mut conditions_extracted = true;
        for r in &chain {
            let line_idx = (r.line_start.saturating_sub(1)) as usize;
            if line_idx >= lines.len() {
                return None;
            }
            match extract_condition_from_line(lines[line_idx].trim_start()) {
                Some(cond) => conditions.push(cond),
                None => {
                    conditions_extracted = false;
                    break;
                }
            }
        }

        // Verify each region in the chain (except the last) has only the nested if as its child
        let outermost = chain[0];
        let outer_indent =
            get_indentation_from_str(lines[(outermost.line_start.saturating_sub(1)) as usize]);
        let indent_step = detect_indent_step(
            &lines[(outermost.line_start.saturating_sub(1)) as usize
                ..((outermost.line_end as usize).min(lines.len()))],
        );
        let body_indent = outer_indent + indent_step;

        for (i, r) in chain.iter().enumerate() {
            if i == chain.len() - 1 {
                break; // Last if doesn't need checking
            }
            let next = chain[i + 1];
            let r_end = (r.line_end as usize).min(lines.len());
            let next_end = (next.line_end as usize).min(lines.len());

            // Check that there are no non-empty statements at body indent level after the inner if
            for line_idx in next_end..r_end {
                let trimmed = lines[line_idx].trim_start();
                let indent = get_indentation_from_str(lines[line_idx]);
                if !trimmed.is_empty() && indent == body_indent {
                    return None;
                }
            }
        }

        // Generate a suggestion with all conditions merged, but only when every
        // condition in the chain was extracted with confidence. Otherwise fall back
        // to help text rather than emitting a snippet built from an incomplete
        // condition list.
        let suggestion = if conditions_extracted {
            Some(generate_collapsible_if_suggestion_chain(
                outermost,
                innermost,
                &conditions,
                &lines,
            ))
        } else {
            None
        };
        let help = if conditions_extracted {
            None
        } else {
            Some(
                "Merge the nested if statements into a single `if <outer> and <inner>:` \
                 block. The exact condition text could not be extracted automatically \
                 (it may span multiple lines or contain content the parser could not \
                 confidently isolate) — combine the conditions with `and` manually."
                    .to_string(),
            )
        };

        // Calculate reduction based on chain depth. Use `chain.len()` (not
        // `conditions.len()`) so the reduction estimate is unaffected by whether the
        // condition *text* could be extracted for the suggestion above.
        let old_complexity = region.total;
        let boolean_count = chain.len().saturating_sub(1); // one 'and' per pair
        let new_complexity = 1 + region.nesting + boolean_count as u64;
        let reduction = old_complexity.saturating_sub(new_complexity).max(2);

        Some(RefactorPlan {
            kind: "collapsible_if".to_string(),
            title: if chain.len() == 2 {
                "Merge nested if statements".to_string()
            } else {
                format!("Merge {} nested if statements", chain.len())
            },
            line_start: region.line_start,
            line_end: region.line_end,
            current_complexity: function_complexity,
            estimated_reduction: reduction,
            estimated_complexity_after: function_complexity.saturating_sub(reduction),
            rule_id: "C007".to_string(),
            category: RuleCategory::Readability,
            applicability: Applicability::MachineApplicable,
            description: "Merge nested if statements into a single if with combined conditions".to_string(),
            suggestion,
            help,
            explanation: "Nested if statements with a single body can be merged into a single if \
                         with combined conditions using 'and'. This reduces nesting and improves readability."
                .to_string(),
            references: vec![],
        })
    }
}

/// Generate a concrete suggestion for loop guards by inverting nested if
/// conditions and using continue. Uses the region tree to collect guards,
/// similar to how C007 uses `collect_if_chain`.
fn generate_loop_guard_suggestion(
    region: &ComplexityRegion,
    source: &str,
) -> Option<CodeSuggestion> {
    let lines: Vec<&str> = source.lines().collect();
    let start = (region.line_start.saturating_sub(1)) as usize;
    let end = (region.line_end as usize).min(lines.len());

    if start >= lines.len() {
        return None;
    }

    let base_indent = get_indentation_from_str(lines[start]);
    let indent_step = detect_indent_step(&lines[start..end]);
    let loop_body_indent = base_indent + indent_step;

    // Collect chain of single-child If regions inside the loop
    let mut guards = Vec::new();
    let mut current_region: Option<&ComplexityRegion> = None;

    // Find the first If child of the loop
    for child in &region.children {
        if child.kind == RegionKind::If {
            current_region = Some(child);
            break;
        }
    }

    // Walk the chain
    while let Some(r) = current_region {
        let line_idx = (r.line_start.saturating_sub(1)) as usize;
        if line_idx >= lines.len() {
            break;
        }
        let condition = match extract_condition_from_line(lines[line_idx].trim_start()) {
            Some(cond) => cond,
            None => break,
        };
        guards.push((r, condition));

        // Check for next single-child If
        if r.children.len() == 1 && r.children[0].kind == RegionKind::If {
            if !has_else_branch(r, &lines) && !has_else_branch(&r.children[0], &lines) {
                current_region = Some(&r.children[0]);
                continue;
            }
        }
        break;
    }

    if guards.is_empty() {
        return None;
    }

    // Get the innermost region (last guard)
    let innermost = guards.last().unwrap().0;
    let innermost_line_idx = (innermost.line_start.saturating_sub(1)) as usize;

    // Build replacement
    let mut result = Vec::new();
    result.push(lines[start].to_string()); // Keep the `for` line

    // Emit all guards
    for (_, guard) in &guards {
        result.push(format!("{}if not {}:", " ".repeat(loop_body_indent), guard));
        result.push(format!(
            "{}continue",
            " ".repeat(loop_body_indent + indent_step)
        ));
    }

    // Emit remaining body (from after the innermost if's condition line)
    let body_start = innermost_line_idx + 1;
    for line in &lines[body_start..end] {
        let trimmed = line.trim_start();
        if trimmed.is_empty() {
            continue;
        }
        let current_indent = get_indentation_from_str(line);
        // Shift body up by number of guard levels
        let shifted = current_indent.saturating_sub(indent_step * guards.len());
        let padding = " ".repeat(shifted);
        result.push(format!("{}{}", padding, trimmed));
    }

    Some(CodeSuggestion {
        replacement: result.join("\n"),
        applicability: Applicability::MachineApplicable,
        description: format!(
            "Convert {} nested conditions to continue guards",
            guards.len()
        ),
    })
}

/// Detect the indentation step (spaces per level) from a block of code.
fn detect_indent_step(lines: &[&str]) -> usize {
    for i in 1..lines.len() {
        let prev_indent = get_indentation_from_str(lines[i - 1]);
        let curr_indent = get_indentation_from_str(lines[i]);
        if curr_indent > prev_indent {
            return curr_indent - prev_indent;
        }
    }
    4
}

/// Generate a concrete suggestion for extracting a complex boolean condition
/// into a named predicate function.
fn generate_predicate_suggestion(
    region: &ComplexityRegion,
    source: &str,
) -> Option<CodeSuggestion> {
    let lines: Vec<&str> = source.lines().collect();
    let start = (region.line_start.saturating_sub(1)) as usize;

    if start >= lines.len() {
        return None;
    }

    let line = lines[start];
    let condition = extract_condition_from_line(line.trim_start())?;
    let base_indent = get_indentation_from_str(line);

    let predicate_indent = " ".repeat(base_indent);
    let body_indent = " ".repeat(base_indent + 4);
    let func_name = format!("_check_condition_L{}", region.line_start);

    // The rewritten `if` replaces the original one, so it sits at the original
    // statement's indentation — not nested inside the helper, which would make it
    // unreachable code after the `return`. The `...` stands in for the caller's
    // existing body, and also keeps the snippet parseable on its own.
    let replacement = format!(
        "{predicate_indent}def {func_name}() -> bool:\n\
         {body_indent}return {condition}\n\
         \n\
         {predicate_indent}if {func_name}():\n\
         {body_indent}..."
    );

    Some(CodeSuggestion {
        replacement,
        applicability: Applicability::MachineApplicable,
        description: format!("Extract condition into named predicate function `{func_name}`"),
    })
}

fn get_indentation_from_str(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

/// Extract the boolean condition text from an `if` / `elif` / `while` statement line.
///
/// Returns `None` when the condition cannot be extracted with confidence — for example
/// when brackets are left unbalanced on this line (the condition continues on a
/// following line), when the line doesn't start with a recognized keyword, or when no
/// statement-terminating colon can be found. Callers must treat `None` as "no
/// machine-applicable suggestion available" and fall back to `help` text rather than
/// guessing at a replacement.
///
/// The previous implementation located the statement colon with `str::rfind(':')` and
/// claimed (incorrectly) that this correctly handled a colon inside a string literal.
/// It did not: a trailing `# comment: like this` colon, or any colon after the real
/// one, would corrupt the extracted condition. This implementation instead walks the
/// line once, tracking bracket depth (`(`, `[`, `{`) and string-literal state (single
/// and double quotes, triple-quoted strings, backslash escapes) so that colons inside
/// strings, f-string format specs (e.g. `f"{x:>3}"`), and container literals (e.g.
/// `{1: 2}`) are never mistaken for the statement colon. The statement colon is the
/// first `:` found at bracket depth 0 outside of any string literal. A `#` reached
/// outside a string ends the scan (nothing after it can contain the statement colon).
fn extract_condition_from_line(line: &str) -> Option<String> {
    let trimmed = line.trim_start();

    let keyword_len = if trimmed.starts_with("elif ") {
        5
    } else if trimmed.starts_with("if ") {
        3
    } else if trimmed.starts_with("while ") {
        6
    } else {
        return None;
    };

    let chars: Vec<(usize, char)> = trimmed.char_indices().collect();
    let mut i = 0;
    let mut depth: i32 = 0;
    // (quote character, is_triple_quoted)
    let mut string_state: Option<(char, bool)> = None;
    let mut colon_byte: Option<usize> = None;

    while i < chars.len() {
        let (byte_idx, c) = chars[i];

        if let Some((quote, triple)) = string_state {
            if c == '\\' {
                // Skip the escaped character (works for both plain and triple-quoted
                // strings; harmless if this is technically a raw string).
                i += 2;
                continue;
            }
            if c == quote {
                let closes_triple = triple
                    && i + 2 < chars.len()
                    && chars[i + 1].1 == quote
                    && chars[i + 2].1 == quote;
                if triple {
                    if closes_triple {
                        string_state = None;
                        i += 3;
                    } else {
                        i += 1;
                    }
                } else {
                    string_state = None;
                    i += 1;
                }
                continue;
            }
            i += 1;
            continue;
        }

        match c {
            '#' => break,
            '\'' | '"' => {
                let is_triple = i + 2 < chars.len() && chars[i + 1].1 == c && chars[i + 2].1 == c;
                string_state = Some((c, is_triple));
                i += if is_triple { 3 } else { 1 };
            }
            '(' | '[' | '{' => {
                depth += 1;
                i += 1;
            }
            ')' | ']' | '}' => {
                depth -= 1;
                i += 1;
            }
            ':' if depth == 0 => {
                // `:=` is the walrus operator, not the statement colon. Treating it
                // as the terminator would silently drop the assignment (`if n :=
                // len(items):` would yield the condition `n`), producing a merged
                // condition that still parses but means something different.
                if i + 1 < chars.len() && chars[i + 1].1 == '=' {
                    i += 2;
                    continue;
                }
                colon_byte = Some(byte_idx);
                break;
            }
            _ => {
                i += 1;
            }
        }
    }

    let colon_byte = colon_byte?;
    let condition = trimmed[keyword_len..colon_byte].trim();
    if condition.is_empty() {
        return None;
    }
    Some(condition.to_string())
}

fn has_else_branch(region: &ComplexityRegion, lines: &[&str]) -> bool {
    let start = (region.line_start.saturating_sub(1)) as usize;
    let end = (region.line_end as usize).min(lines.len());

    if start >= lines.len() {
        return false;
    }

    let base_indent = get_indentation_from_str(lines[start]);

    for line in &lines[start..end] {
        let trimmed = line.trim_start();
        let current_indent = get_indentation_from_str(line);

        if current_indent == base_indent
            && (trimmed.starts_with("else:") || trimmed.starts_with("elif "))
        {
            return true;
        }
    }

    false
}

fn generate_collapsible_if_suggestion_chain(
    outermost: &ComplexityRegion,
    innermost: &ComplexityRegion,
    conditions: &[String],
    lines: &[&str],
) -> CodeSuggestion {
    let outer_line_idx = (outermost.line_start.saturating_sub(1)) as usize;
    let inner_line_idx = (innermost.line_start.saturating_sub(1)) as usize;
    let inner_end = (innermost.line_end as usize).min(lines.len());

    let outer_indent = get_indentation_from_str(lines[outer_line_idx]);
    let indent_step = detect_indent_step(&lines[outer_line_idx..inner_end]);

    // Combine all conditions with 'and', wrapping 'or' conditions in parens
    let combined = combine_conditions_chain(conditions);

    // Extract inner body (lines after the innermost if)
    let body_start = inner_line_idx + 1;
    let chain_depth = conditions.len();
    let mut body_lines = Vec::new();
    for line in &lines[body_start..inner_end] {
        let trimmed = line.trim_start();
        if trimmed.is_empty() {
            continue;
        }
        let current_indent = get_indentation_from_str(line);
        // Shift body up by (chain_depth - 1) indent levels
        let shifted = current_indent.saturating_sub(indent_step * (chain_depth - 1));
        let padding = " ".repeat(shifted);
        body_lines.push(format!("{}{}", padding, trimmed));
    }

    let indent = " ".repeat(outer_indent);
    let replacement = format!("{}if {}:\n{}", indent, combined, body_lines.join("\n"));

    CodeSuggestion {
        replacement,
        applicability: Applicability::MachineApplicable,
        description: format!("Merge nested conditions into `if {}:`", combined),
    }
}

/// Combine multiple conditions with 'and', wrapping 'or' conditions in parens.
/// Returns a copy of `condition` with the contents of string literals and of
/// bracketed groups replaced by `x`, preserving the top-level structure.
///
/// This lets callers look for top-level operators with plain substring checks
/// without being fooled by `" or "` inside a string literal or a `:=` nested
/// inside a call.
fn mask_nested(condition: &str) -> String {
    let chars: Vec<char> = condition.chars().collect();
    let mut out = String::with_capacity(chars.len());
    let mut i = 0;
    let mut depth: i32 = 0;
    let mut string_state: Option<(char, bool)> = None;

    while i < chars.len() {
        let c = chars[i];

        if let Some((quote, triple)) = string_state {
            out.push('x');
            if c == '\\' {
                if i + 1 < chars.len() {
                    out.push('x');
                }
                i += 2;
                continue;
            }
            if c == quote {
                let closes_triple =
                    triple && i + 2 < chars.len() && chars[i + 1] == quote && chars[i + 2] == quote;
                if triple {
                    if closes_triple {
                        out.push('x');
                        out.push('x');
                        string_state = None;
                        i += 3;
                    } else {
                        i += 1;
                    }
                } else {
                    string_state = None;
                    i += 1;
                }
                continue;
            }
            i += 1;
            continue;
        }

        match c {
            '\'' | '"' => {
                let is_triple = i + 2 < chars.len() && chars[i + 1] == c && chars[i + 2] == c;
                string_state = Some((c, is_triple));
                out.push('x');
                if is_triple {
                    out.push('x');
                    out.push('x');
                    i += 3;
                } else {
                    i += 1;
                }
            }
            '(' | '[' | '{' => {
                depth += 1;
                out.push('x');
                i += 1;
            }
            ')' | ']' | '}' => {
                depth -= 1;
                out.push('x');
                i += 1;
            }
            _ => {
                out.push(if depth > 0 { 'x' } else { c });
                i += 1;
            }
        }
    }

    out
}

/// True when `condition` must be parenthesized before being joined with `and`.
///
/// `or`, conditional expressions (`x if c else y`) and assignment expressions
/// (`:=`) all bind more loosely than `and`, so merging them unparenthesized
/// silently changes what the condition means while still parsing:
/// `n := len(items)` joined bare becomes `n := (len(items) and ...)`.
fn needs_parens_for_and(condition: &str) -> bool {
    let masked = mask_nested(condition);
    masked.contains(" or ") || masked.contains(" if ") || masked.contains(":=")
}

fn combine_conditions_chain(conditions: &[String]) -> String {
    conditions
        .iter()
        .map(|c| {
            if needs_parens_for_and(c) {
                format!("({})", c)
            } else {
                c.clone()
            }
        })
        .collect::<Vec<_>>()
        .join(" and ")
}

/// Walk a chain of single-child If regions, collecting all regions in the chain.
/// Stops when:
/// - Current region has != 1 child
/// - Child is not an If
/// - Current or child has else/elif
fn collect_if_chain<'a>(region: &'a ComplexityRegion, lines: &[&str]) -> Vec<&'a ComplexityRegion> {
    let mut chain = vec![region];
    let mut current = region;

    loop {
        if current.children.len() != 1 {
            break;
        }
        let child = &current.children[0];
        if child.kind != RegionKind::If {
            break;
        }
        if has_else_branch(current, lines) || has_else_branch(child, lines) {
            break;
        }
        chain.push(child);
        current = child;
    }

    chain
}

fn sum_if_nesting(regions: &[ComplexityRegion]) -> u64 {
    regions
        .iter()
        .map(|region| {
            let own = if region.kind == RegionKind::If {
                region.nesting
            } else {
                0
            };
            own + sum_if_nesting(&region.children)
        })
        .sum()
}

// Unit tests live outside the implementation file, under `src/tests/`, mirroring
// the module path. The `#[path]` attribute keeps them a child module of this one
// so they can exercise the private helpers above through `super::` without
// making any of them `pub(crate)` just for the sake of testing.
#[cfg(test)]
#[path = "../tests/rules/complexity.rs"]
mod tests;
