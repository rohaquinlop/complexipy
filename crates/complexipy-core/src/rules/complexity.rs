use crate::classes::{Applicability, CodeSuggestion, RefactorPlan, RuleCategory};
use crate::refactor_plans::{ComplexityRegion, RegionKind};
use crate::rules::types::{RefactorRule, RuleMetadata};
use crate::syntax_tree::{Node, PreOrder, parse_expression};
use crate::utils::{LineIndex, count_bool_ops, split_lines};
use ruff_python_ast::{BoolOp, CmpOp, Expr, UnaryOp};
use std::sync::OnceLock;

pub struct FlattenConditionRule;

impl RefactorRule for FlattenConditionRule {
    fn metadata(&self) -> &'static RuleMetadata {
        static META: OnceLock<RuleMetadata> = OnceLock::new();
        META.get_or_init(|| RuleMetadata {
            id: "C001".to_string(),
            name: "flatten_condition".to_string(),
            category: RuleCategory::Complexity,
            description:
                "Flatten nested condition blocks by using guard clauses with early returns"
                    .to_string(),
            applicability: Applicability::Informational,
            effectiveness: 4,
            doc_url: "https://complexipy.com/refactoring-rules/#c001-flatten-nested-conditions"
                .to_string(),
        })
    }

    fn check(
        &self,
        region: &ComplexityRegion,
        _source: &str,
        _index: &LineIndex,
        _def_names: &std::collections::HashSet<String>,
        function_complexity: u64,
    ) -> Option<RefactorPlan> {
        if region.kind != RegionKind::If || region.nesting < 2 || region.total < 4 {
            return None;
        }

        Some(RefactorPlan {
            title: "Flatten nested condition block with guard clauses".to_string(),
            line_start: region.line_start,
            line_end: region.line_end,
            column_start: region.column_start,
            current_complexity: function_complexity,
            estimated_reduction: region.nesting,
            estimated_complexity_after: function_complexity.saturating_sub(region.nesting),
            explanation: "Deeply nested conditions are hard to follow. Using guard clauses \
                         with early returns reduces cognitive load by keeping the main path \
                         at a lower indentation level."
                .to_string(),
            help: Some(
                "Invert the outer condition and exit early - `return` from the function, \
                 or `continue`/`break` when the block sits inside a loop. Move the main \
                 success path one indentation level left. Repeat for inner nested conditions where safe."
                    .to_string(),
            ),
            ..self.metadata().new_plan()
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
            effectiveness: 3,
            doc_url: "https://complexipy.com/refactoring-rules/#c002-loop-guards".to_string(),
        })
    }

    fn check(
        &self,
        region: &ComplexityRegion,
        source: &str,
        index: &LineIndex,
        _def_names: &std::collections::HashSet<String>,
        function_complexity: u64,
    ) -> Option<RefactorPlan> {
        if region.kind != RegionKind::Loop || region.total < 5 {
            return None;
        }

        let chain = collect_loop_if_chain(region);

        if chain.is_empty() {
            return None;
        }

        let base_nesting = chain[0].nesting;
        let guard_savings: u64 = chain
            .iter()
            .map(|r| r.nesting.saturating_sub(base_nesting))
            .sum();
        let remaining_bonus = chain.len() as u64 * chain.last().unwrap().children.len() as u64;
        let reduction = guard_savings + remaining_bonus;

        if reduction == 0 {
            return None;
        }

        let suggestion = generate_loop_guard_suggestion(region, source, index);
        let help = match suggestion {
            Some(_) => None,
            None if chain[0].line_end != region.line_end => Some(
                "A `continue` guard would also skip the statements that follow the nested \
                 block in the loop body. Extract the nested block into a helper function, \
                 or move those statements above it when their order allows."
                    .to_string(),
            ),
            None => Some(
                "Add `if not (<condition>): continue` guards at the top of the loop body \
                 for each nested if condition, then dedent the remaining logic by one \
                 level per guard."
                    .to_string(),
            ),
        };

        Some(RefactorPlan {
            title: "Flatten loop body with continue guards".to_string(),
            line_start: region.line_start,
            line_end: region.line_end,
            column_start: region.column_start,
            current_complexity: function_complexity,
            estimated_reduction: reduction,
            estimated_complexity_after: function_complexity.saturating_sub(reduction),
            explanation: "Nested conditions inside loops add unnecessary indentation. \
                         Using continue guards keeps the main logic at a lower nesting level \
                         and makes the loop easier to follow."
                .to_string(),
            suggestion,
            help,
            ..self.metadata().new_plan()
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
            effectiveness: 2,
            doc_url: "https://complexipy.com/refactoring-rules/#c003-extract-helper-function"
                .to_string(),
        })
    }

    fn check(
        &self,
        region: &ComplexityRegion,
        _source: &str,
        _index: &LineIndex,
        _def_names: &std::collections::HashSet<String>,
        function_complexity: u64,
    ) -> Option<RefactorPlan> {
        let line_count = region.line_end.saturating_sub(region.line_start) + 1;

        if region.total < 6 || line_count < 5 {
            return None;
        }

        let region_own_cost = region.structural + region.nesting + region.boolean;
        let reduction = region.total.saturating_sub(region_own_cost);

        Some(RefactorPlan {
            title: "Extract complex block into helper function".to_string(),
            line_start: region.line_start,
            line_end: region.line_end,
            column_start: region.column_start,
            current_complexity: function_complexity,
            estimated_reduction: reduction,
            estimated_complexity_after: function_complexity.saturating_sub(reduction),
            explanation: "Complex code blocks should be extracted into named functions \
                         to improve readability and testability. The extracted function \
                         can be given a descriptive name that explains its purpose."
                .to_string(),
            help: Some(format!(
                "Extract lines {}-{} into a named helper function. Pass required \
                              values as parameters and return the result needed by the caller.",
                region.line_start, region.line_end
            )),
            ..self.metadata().new_plan()
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
            description: "Split long elif chains into separate handlers".to_string(),
            applicability: Applicability::Informational,
            effectiveness: 2,
            doc_url: "https://complexipy.com/refactoring-rules/#c004-split-dispatcher".to_string(),
        })
    }

    fn check(
        &self,
        region: &ComplexityRegion,
        source: &str,
        index: &LineIndex,
        _def_names: &std::collections::HashSet<String>,
        function_complexity: u64,
    ) -> Option<RefactorPlan> {
        if region.kind != RegionKind::ElifChain || region.elif_count < 3 {
            return None;
        }

        let region_true_total = region.total + region.elif_count;
        let branch_estimate = region.elif_count.saturating_sub(1);
        let reduction = branch_estimate.min(region_true_total.saturating_sub(1));

        if reduction == 0 {
            return None;
        }

        let help = if let Some(subject) = single_variable_equality_subject(region, source, index) {
            format!(
                "This chain only compares `{subject}` against literal values, so it can \
                 become `match {subject}:` with one `case <value>:` per branch. Unlike an \
                 elif chain, a match statement's cost doesn't grow with the number of \
                 cases, so this actually reduces the measured complexity -- a dispatch \
                 dictionary would not."
            )
        } else {
            format!(
                "Replace the {}-branch chain with a dispatch dictionary mapping \
                 cases to handler functions. Each handler becomes independently testable.",
                region.elif_count
            )
        };

        Some(RefactorPlan {
            title: "Split conditional dispatcher into handlers".to_string(),
            line_start: region.line_start,
            line_end: region.line_end,
            column_start: region.column_start,
            current_complexity: function_complexity,
            estimated_reduction: reduction,
            estimated_complexity_after: function_complexity.saturating_sub(reduction),
            explanation: "Long conditional chains are hard to maintain and extend. \
                         Splitting them into separate handlers makes each case \
                         independently testable and the dispatch logic clearer."
                .to_string(),
            help: Some(help),
            ..self.metadata().new_plan()
        })
    }
}

fn single_variable_equality_subject(
    region: &ComplexityRegion,
    source: &str,
    index: &LineIndex,
) -> Option<String> {
    let lines = span_lines(source, index, region.line_start, region.line_end)?;
    if lines.is_empty() {
        return None;
    }

    let base_indent = get_indentation_from_str(lines[0]);
    let mut subject: Option<String> = None;
    let mut clause_count = 0;

    for line in &lines {
        if get_indentation_from_str(line) != base_indent {
            continue;
        }
        let trimmed = line.trim_start();
        if !trimmed.starts_with("if ") && !trimmed.starts_with("elif ") {
            continue;
        }

        let condition = extract_condition_from_line(trimmed)?;
        let clause_subject = equality_dispatch_subject(&condition)?;
        clause_count += 1;

        match &subject {
            Some(existing) if *existing != clause_subject => return None,
            Some(_) => {}
            None => subject = Some(clause_subject),
        }
    }

    if clause_count >= 3 { subject } else { None }
}

fn equality_dispatch_subject(condition: &str) -> Option<String> {
    let parsed = parse_expression(condition)?;
    let Expr::Compare(compare) = parsed.expr() else {
        return None;
    };
    if compare.ops.as_ref() != [CmpOp::Eq] {
        return None;
    }
    let [comparator] = compare.comparators.as_ref() else {
        return None;
    };
    if !is_literal_expr(comparator) {
        return None;
    }
    simple_reference_text(&compare.left)
}

fn is_literal_expr(expr: &Expr) -> bool {
    matches!(
        expr,
        Expr::StringLiteral(_)
            | Expr::NumberLiteral(_)
            | Expr::BooleanLiteral(_)
            | Expr::NoneLiteral(_)
    )
}

fn simple_reference_text(expr: &Expr) -> Option<String> {
    let mut attributes: Vec<&str> = Vec::new();
    let mut current = expr;
    loop {
        match current {
            Expr::Name(name) => {
                let mut text = name.id.to_string();
                for attribute in attributes.iter().rev() {
                    text.push('.');
                    text.push_str(attribute);
                }
                return Some(text);
            }
            Expr::Attribute(attribute) => {
                attributes.push(attribute.attr.id.as_str());
                current = &attribute.value;
            }
            _ => return None,
        }
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
            effectiveness: 2,
            doc_url: "https://complexipy.com/refactoring-rules/#c005-extract-predicate".to_string(),
        })
    }

    fn check(
        &self,
        region: &ComplexityRegion,
        source: &str,
        index: &LineIndex,
        def_names: &std::collections::HashSet<String>,
        function_complexity: u64,
    ) -> Option<RefactorPlan> {
        if region.kind != RegionKind::BooleanCondition || region.boolean < 2 {
            return None;
        }

        let suggestion = generate_predicate_suggestion(region, source, index, def_names);
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
            title: "Extract complex condition into named predicate".to_string(),
            line_start: region.line_start,
            line_end: region.line_end,
            column_start: region.column_start,
            current_complexity: function_complexity,
            estimated_reduction: region.bool_op_count,
            estimated_complexity_after: function_complexity.saturating_sub(region.bool_op_count),
            explanation: "Complex boolean expressions are hard to understand at a glance. \
                         Extracting them into named predicates makes the code self-documenting \
                         and easier to test."
                .to_string(),
            suggestion,
            help,
            ..self.metadata().new_plan()
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
            effectiveness: 2,
            doc_url: "https://complexipy.com/refactoring-rules/#c011-flatten-tryexcept".to_string(),
        })
    }

    fn check(
        &self,
        region: &ComplexityRegion,
        _source: &str,
        _index: &LineIndex,
        _def_names: &std::collections::HashSet<String>,
        function_complexity: u64,
    ) -> Option<RefactorPlan> {
        if region.kind != RegionKind::Try {
            return None;
        }

        let nested_try = find_nested_try(region)?;
        let estimated_reduction = nested_try.structural.max(1);

        Some(RefactorPlan {
            title: "Flatten nested try/except blocks".to_string(),
            line_start: region.line_start,
            line_end: region.line_end,
            column_start: region.column_start,
            current_complexity: function_complexity,
            estimated_reduction,
            estimated_complexity_after: function_complexity.saturating_sub(estimated_reduction),
            explanation: "Nested try/except blocks are confusing and hard to maintain. \
                         Consider merging them or extracting the inner block into \
                         a separate function with its own error handling."
                .to_string(),
            suggestion: None,
            help: Some(
                "Review if inner try/except can be merged with outer. Consider using \
                        a single try with multiple except clauses or extract inner try block \
                        into a helper function."
                    .to_string(),
            ),
            ..self.metadata().new_plan()
        })
    }
}

fn find_nested_try(region: &ComplexityRegion) -> Option<&ComplexityRegion> {
    for child in &region.children {
        if child.kind == RegionKind::Try {
            return Some(child);
        }
        if let Some(found) = find_nested_try(child) {
            return Some(found);
        }
    }
    None
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
            effectiveness: 5,
            doc_url: "https://complexipy.com/refactoring-rules/#c007-collapsible-if".to_string(),
        })
    }

    fn check(
        &self,
        region: &ComplexityRegion,
        source: &str,
        index: &LineIndex,
        _def_names: &std::collections::HashSet<String>,
        function_complexity: u64,
    ) -> Option<RefactorPlan> {
        if region.kind != RegionKind::If {
            return None;
        }

        let chain = collect_if_chain(region);

        if chain.len() < 2 {
            return None;
        }

        let innermost = chain.last().unwrap();

        let mut conditions = Vec::new();
        let mut comments = Vec::new();
        let mut conditions_extracted = true;
        for r in &chain {
            let line = line_at(source, index, r.line_start)?;
            match extract_condition_from_line(line.trim_start()) {
                Some(cond) => {
                    conditions.push(cond);
                    comments.push(header_comment(line));
                }
                None => {
                    conditions_extracted = false;
                    break;
                }
            }
        }

        let outermost = chain[0];

        for (i, r) in chain.iter().enumerate() {
            if i == chain.len() - 1 {
                break;
            }
            let next = chain[i + 1];

            let mut body_start = r.line_start + 1;
            let r_start_line = line_at(source, index, r.line_start);
            if r_start_line
                .is_none_or(|line| extract_condition_from_line(line.trim_start()).is_none())
            {
                while body_start < next.line_start {
                    let Some(body_line) = line_at(source, index, body_start) else {
                        break;
                    };
                    if line_ends_with_statement_colon(body_line) {
                        body_start += 1;
                        break;
                    }
                    body_start += 1;
                }
            }

            let mut gap_line = body_start;
            while gap_line < next.line_start {
                let Some(gap_text) = line_at(source, index, gap_line) else {
                    break;
                };
                if !gap_text.trim_start().is_empty() {
                    return None;
                }
                gap_line += 1;
            }

            let mut tail_line = next.line_end + 1;
            while tail_line <= r.line_end {
                let Some(tail_text) = line_at(source, index, tail_line) else {
                    break;
                };
                if !tail_text.trim_start().is_empty() {
                    return None;
                }
                tail_line += 1;
            }
        }

        let mut suggestion = None;
        let mut help = None;
        if conditions_extracted {
            let comments: Option<Vec<&str>> = comments.into_iter().collect();
            let written: Option<Vec<&str>> = comments.map(|comments| {
                comments
                    .into_iter()
                    .filter(|comment| !comment.is_empty())
                    .collect()
            });
            match written.as_deref() {
                None => {
                    help = Some(
                        "Merge the nested if statements into a single `if <outer> and <inner>:` \
                         block, and move the inner body onto its own line below it."
                            .to_string(),
                    );
                }
                Some([_, _, ..]) => {
                    help = Some(
                        "Merge the nested if statements into a single `if <outer> and <inner>:` \
                         block, and decide by hand where the comments of the merged header \
                         lines belong."
                            .to_string(),
                    );
                }
                Some(comment) => match generate_collapsible_if_suggestion_chain(
                    outermost,
                    innermost,
                    &conditions,
                    comment.first().copied().unwrap_or(""),
                    source,
                    index,
                ) {
                    Some(s) => suggestion = Some(s),
                    None => {
                        help = Some(
                            "Merge the nested if statements into a single \
                             `if <outer> and <inner>:` block and dedent its body by hand. An \
                             automatic replacement would change a multi-line string literal or \
                             could not keep the indentation of the body consistent."
                                .to_string(),
                        );
                    }
                },
            }
        } else {
            help = Some(
                "Merge the nested if statements into a single `if <outer> and <inner>:` \
                 block. The exact condition text could not be extracted automatically \
                 (it may span multiple lines or contain content the parser could not \
                 confidently isolate) - combine the conditions with `and` manually."
                    .to_string(),
            );
        }

        let old_complexity = region.total;
        let boolean_count = if conditions_extracted {
            let combined = combine_conditions_chain(&conditions);
            match parse_expression(&combined) {
                Some(parsed) => count_bool_ops(parsed.expr(), region.nesting),
                None => fallback_boolean_count(&chain),
            }
        } else {
            fallback_boolean_count(&chain)
        };
        let innermost_own = innermost.structural + innermost.nesting + innermost.boolean;
        let remaining_cost = innermost.total.saturating_sub(innermost_own);
        let new_complexity = 1 + region.nesting + boolean_count + remaining_cost;
        let reduction = old_complexity.saturating_sub(new_complexity);

        Some(RefactorPlan {
            title: if chain.len() == 2 {
                "Merge nested if statements".to_string()
            } else {
                format!("Merge {} nested if statements", chain.len())
            },
            line_start: region.line_start,
            line_end: region.line_end,
            column_start: region.column_start,
            current_complexity: function_complexity,
            estimated_reduction: reduction,
            estimated_complexity_after: function_complexity.saturating_sub(reduction),
            suggestion,
            help,
            explanation: "Nested if statements with a single body can be merged into a single if \
                         with combined conditions using 'and'. This reduces nesting and improves readability."
                .to_string(),
            ..self.metadata().new_plan()
        })
    }
}

/// Generate a concrete suggestion for loop guards by inverting nested if
/// conditions and using continue. Uses the region tree to collect guards,
/// similar to how C007 uses `collect_if_chain`. Returns `None` when the body
/// holds no guardable chain - callers fall back to `help` text then.
fn generate_loop_guard_suggestion(
    region: &ComplexityRegion,
    source: &str,
    index: &LineIndex,
) -> Option<CodeSuggestion> {
    let chain = collect_loop_if_chain(region);
    let head = *chain.first()?;
    if region.has_else || head.has_else || head.line_end != region.line_end {
        return None;
    }

    let lines = span_lines(source, index, region.line_start, region.line_end)?;
    let base = region.line_start;
    let span_idx = |line: u64| -> Option<usize> {
        let idx = line.checked_sub(base)? as usize;
        (idx < lines.len()).then_some(idx)
    };

    let mut headers = Vec::with_capacity(chain.len());
    for member in &chain {
        headers.push(span_idx(member.line_start)?);
    }

    let mut guards = Vec::new();
    for &header in &headers {
        let line = lines[header].trim_start();
        let (Some(condition), Some(comment)) =
            (extract_condition_from_line(line), header_comment(line))
        else {
            break;
        };
        guards.push((header, condition, comment));
    }
    if guards.is_empty() {
        return None;
    }

    let head_idx = headers[0];
    let head_prefix = indentation_of(lines[head_idx]);
    if loop_body_prefix(&lines)? != head_prefix {
        return None;
    }
    if contains_multiline_string(lines.get(head_idx + 1..)?) {
        return None;
    }

    let body_prefixes: Vec<&str> = (0..guards.len())
        .map(|i| match headers.get(i + 1) {
            Some(&next) => Some(indentation_of(lines[next])),
            None => first_code_line(lines.get(headers[i] + 1..)?).map(indentation_of),
        })
        .collect::<Option<_>>()?;
    let header_prefixes = headers.iter().map(|&header| indentation_of(lines[header]));
    if !header_prefixes
        .zip(&body_prefixes)
        .all(|(header, body)| body.len() > header.len() && body.starts_with(header))
    {
        return None;
    }
    let continue_prefix = format!("{head_prefix}{}", &body_prefixes[0][head_prefix.len()..]);

    let mut result: Vec<String> = lines
        .get(..head_idx)?
        .iter()
        .map(|line| (*line).to_string())
        .collect();
    for (i, (header, guard, comment)) in guards.iter().enumerate() {
        let guard_text = match strip_top_level_not(guard) {
            Some(rest) => rest,
            None => format!("not ({guard})"),
        };
        result.push(format!(
            "{head_prefix}if {guard_text}:{}",
            comment_suffix(comment)
        ));
        result.push(format!("{continue_prefix}continue"));

        let body_end = match guards.get(i + 1) {
            Some((next, _, _)) => *next,
            None => lines.len(),
        };
        let body = lines.get(header + 1..body_end)?;
        result.extend(reindented(body, body_prefixes[i], head_prefix)?);
    }

    Some(CodeSuggestion {
        replacement: result.join("\n"),
        applicability: Applicability::MachineApplicable,
        spliceable: true,
        description: format!(
            "Convert {} nested conditions to continue guards",
            guards.len()
        ),
    })
}

fn indentation_of(line: &str) -> &str {
    &line[..line.len() - line.trim_start().len()]
}

fn is_code_line(line: &str) -> bool {
    let trimmed = line.trim_start();
    !trimmed.is_empty() && !trimmed.starts_with('#')
}

fn first_code_line<'a>(lines: &[&'a str]) -> Option<&'a str> {
    lines.iter().copied().find(|line| is_code_line(line))
}

fn loop_body_prefix<'a>(lines: &[&'a str]) -> Option<&'a str> {
    let header_end = lines
        .iter()
        .position(|line| line_ends_with_statement_colon(line))?;
    first_code_line(lines.get(header_end + 1..)?).map(indentation_of)
}

/// Moves `lines` from the block indentation `from` to `to`. Blank lines
/// become empty and comment lines outside the block keep their text.
/// Returns `None` when a code line does not start with `from`.
fn reindented(lines: &[&str], from: &str, to: &str) -> Option<Vec<String>> {
    lines
        .iter()
        .map(|line| match line.strip_prefix(from) {
            _ if line.trim_start().is_empty() => Some(String::new()),
            Some(rest) => Some(format!("{to}{rest}")),
            None if is_code_line(line) => None,
            None => Some((*line).to_string()),
        })
        .collect()
}

fn comment_suffix(comment: &str) -> String {
    if comment.is_empty() {
        String::new()
    } else {
        format!("  {comment}")
    }
}

/// Detect the indentation step (spaces per level) forward from a header line.
/// The scan stops at the first blank-or-comment-free line that does not open
/// a deeper block: the enclosing block's body always follows a header, so the
/// step is the indent delta of that first deeper line.
fn detect_indent_step_from(source: &str, index: &LineIndex, start_line: u64) -> usize {
    let Some(first) = line_at(source, index, start_line) else {
        return 4;
    };
    let base_indent = get_indentation_from_str(first);
    let mut line_no = start_line + 1;
    while let Some(line) = line_at(source, index, line_no) {
        let trimmed = line.trim_start();
        if !trimmed.is_empty() && !trimmed.starts_with('#') {
            let indent = get_indentation_from_str(line);
            if indent > base_indent {
                return indent - base_indent;
            }
            return 4;
        }
        line_no += 1;
    }
    4
}

/// Generate a concrete suggestion for extracting a complex boolean condition
/// into a named predicate function. The rewritten statement keeps the
/// original keyword (`if` or `while`) and the `...` stands in for the
/// caller's body, keeping the snippet parseable on its own. The helper is
/// emitted at module level with the condition's free variables as
/// parameters, so it is importable and unit-testable; the call site passes
/// the same names. `elif` conditions return `None`: a `def` cannot sit
/// inside an if-chain, so no faithful replacement exists. Conditions that
/// bind a name (`:=`) or hold a nested scope (lambda, comprehension) also
/// return `None`: the parameter list could not be trusted.
fn generate_predicate_suggestion(
    region: &ComplexityRegion,
    source: &str,
    index: &LineIndex,
    def_names: &std::collections::HashSet<String>,
) -> Option<CodeSuggestion> {
    let line = line_at(source, index, region.line_start)?;
    let trimmed = line.trim_start();
    let keyword = if trimmed.starts_with("if ") {
        "if"
    } else if trimmed.starts_with("while ") {
        "while"
    } else {
        return None;
    };
    let condition = extract_condition_from_line(trimmed)?;
    let parameters = collect_free_names(&condition)?;
    let indent_step = detect_indent_step_from(source, index, region.line_start);

    let helper_body_indent = " ".repeat(indent_step);
    let mut func_name = format!("_check_condition_L{}", region.line_start);
    while def_names.contains(&func_name) {
        func_name.push('_');
    }

    let parameters_text = parameters.join(", ");
    let call_text = format!("{keyword} {func_name}({parameters_text}):");
    let statement_context = match enclosing_header_indices(source, index, region.line_start) {
        Some(headers) => render_statement_context(
            &headers,
            source,
            index,
            region.line_start,
            indent_step,
            &call_text,
        ),
        None => format!(
            "{call_text}\n\
             {helper_body_indent}..."
        ),
    };
    let replacement = format!(
        "def {func_name}({parameters_text}) -> bool:\n\
         {helper_body_indent}return {condition}\n\
         \n\
         {statement_context}"
    );

    Some(CodeSuggestion {
        replacement,
        applicability: Applicability::MachineApplicable,
        spliceable: false,
        description: format!("Extract condition into named predicate function `{func_name}`"),
    })
}

fn line_at<'a>(source: &'a str, index: &LineIndex, line: u64) -> Option<&'a str> {
    index.line_text(source, line)
}

fn span_lines<'a>(
    source: &'a str,
    index: &LineIndex,
    start_line: u64,
    end_line: u64,
) -> Option<Vec<&'a str>> {
    let start_byte = index.byte_of_line(start_line)?;
    let end_byte = index.byte_of_line(end_line + 1).unwrap_or(source.len());
    if end_byte < start_byte {
        return Some(Vec::new());
    }
    Some(split_lines(&source[start_byte..end_byte]).collect())
}

fn get_indentation_from_str(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

const PYTHON_BUILTINS: &[&str] = &[
    "__import__",
    "abs",
    "aiter",
    "all",
    "anext",
    "any",
    "ascii",
    "bin",
    "bool",
    "breakpoint",
    "bytearray",
    "bytes",
    "callable",
    "chr",
    "classmethod",
    "compile",
    "complex",
    "delattr",
    "dict",
    "dir",
    "divmod",
    "enumerate",
    "eval",
    "exec",
    "filter",
    "float",
    "format",
    "frozenset",
    "getattr",
    "globals",
    "hasattr",
    "hash",
    "help",
    "hex",
    "id",
    "input",
    "int",
    "isinstance",
    "issubclass",
    "iter",
    "len",
    "list",
    "locals",
    "map",
    "max",
    "memoryview",
    "min",
    "next",
    "object",
    "oct",
    "open",
    "ord",
    "pow",
    "print",
    "property",
    "range",
    "repr",
    "reversed",
    "round",
    "set",
    "setattr",
    "slice",
    "sorted",
    "staticmethod",
    "str",
    "sum",
    "super",
    "tuple",
    "type",
    "vars",
    "zip",
];

/// The condition's free variables, in first-use order, with builtins
/// excluded. Returns `None` when the condition binds a name (`:=`) or holds
/// a nested scope (lambda, comprehension) - the parameter list could not be
/// trusted, so callers fall back to help text.
fn collect_free_names(condition: &str) -> Option<Vec<String>> {
    let parsed = parse_expression(condition)?;
    let mut names: Vec<String> = Vec::new();
    let mut walk = PreOrder::new(std::iter::once(Node::Expr(parsed.expr())));
    while let Some(node) = walk.next() {
        match node {
            Node::Expr(
                Expr::Named(_)
                | Expr::Lambda(_)
                | Expr::ListComp(_)
                | Expr::SetComp(_)
                | Expr::DictComp(_)
                | Expr::Generator(_),
            ) => return None,
            Node::Expr(Expr::Name(name))
                if !PYTHON_BUILTINS.contains(&name.id.as_str())
                    && !names.iter().any(|known| known == name.id.as_str()) =>
            {
                names.push(name.id.to_string());
            }
            _ => {}
        }
        walk.descend(node);
    }
    Some(names)
}

/// Line indices (outermost first) of the block headers enclosing the
/// condition line, from module level down to the enclosing block. Returns
/// `None` when the chain cannot be traced to column 0 (module-level
/// condition, or a broken chain such as a bare statement at a lesser
/// indent) - callers render the bare call then.
fn enclosing_header_indices(source: &str, index: &LineIndex, start_line: u64) -> Option<Vec<u64>> {
    let mut headers = Vec::new();
    let mut current_indent = get_indentation_from_str(line_at(source, index, start_line)?);
    let mut expect_chain_opener = false;
    let mut line_no = start_line;
    while line_no > 1 {
        line_no -= 1;
        let line = line_at(source, index, line_no)?;
        let trimmed = line.trim_start();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let indent = get_indentation_from_str(line);
        if indent > current_indent {
            continue;
        }
        if trimmed.starts_with('@') {
            continue;
        }
        if !line_opens_block(trimmed) {
            return None;
        }
        if expect_chain_opener {
            if indent != current_indent {
                return None;
            }
            headers.push(line_no);
            if !is_continuation_header(trimmed) {
                expect_chain_opener = false;
                if indent == 0 {
                    headers.reverse();
                    return Some(headers);
                }
            }
            continue;
        }
        if indent == current_indent {
            return None;
        }
        headers.push(line_no);
        current_indent = indent;
        if is_continuation_header(trimmed) {
            expect_chain_opener = true;
        } else if indent == 0 {
            headers.reverse();
            return Some(headers);
        }
    }
    None
}

fn is_continuation_header(trimmed: &str) -> bool {
    trimmed.starts_with("elif ")
        || trimmed.starts_with("else:")
        || trimmed.starts_with("except")
        || trimmed.starts_with("finally:")
        || trimmed.starts_with("case ")
}

fn line_opens_block(trimmed: &str) -> bool {
    if !line_ends_with_statement_colon(trimmed) {
        return false;
    }
    [
        "def ",
        "class ",
        "async def ",
        "if ",
        "elif ",
        "else:",
        "for ",
        "async for ",
        "while ",
        "with ",
        "async with ",
        "try:",
        "except",
        "finally:",
        "match ",
        "case ",
    ]
    .iter()
    .any(|head| trimmed.starts_with(head))
}

fn directly_follows(source: &str, index: &LineIndex, item_line: u64, prev_line: u64) -> bool {
    (prev_line + 1..item_line).all(|line_no| {
        let trimmed = line_at(source, index, line_no)
            .unwrap_or("")
            .trim_start()
            .to_string();
        trimmed.is_empty() || trimmed.starts_with('#')
    })
}

/// Render the enclosing blocks around the call statement at their real
/// indentation, using `...` placeholders for skipped statements. The result
/// parses on its own: every placeholder is an Ellipsis expression statement.
fn render_statement_context(
    headers: &[u64],
    source: &str,
    index: &LineIndex,
    start_line: u64,
    indent_step: usize,
    call_text: &str,
) -> String {
    let call_indent = get_indentation_from_str(line_at(source, index, start_line).unwrap_or(""));
    let mut out = String::new();
    let mut prev_line: Option<u64> = None;
    for &header_line in headers {
        let header_text = line_at(source, index, header_line).unwrap_or("");
        let header_indent = get_indentation_from_str(header_text);
        if let Some(prev) = prev_line {
            let prev_indent = get_indentation_from_str(line_at(source, index, prev).unwrap_or(""));
            if header_indent == prev_indent {
                out.push_str(&format!("{}...\n", " ".repeat(prev_indent + indent_step)));
            } else if !directly_follows(source, index, header_line, prev) {
                out.push_str(&format!("{}...\n", " ".repeat(header_indent)));
            }
        }
        out.push_str(header_text);
        out.push('\n');
        prev_line = Some(header_line);
    }
    if let Some(prev) = prev_line {
        let prev_indent = get_indentation_from_str(line_at(source, index, prev).unwrap_or(""));
        if call_indent > prev_indent && !directly_follows(source, index, start_line, prev) {
            out.push_str(&format!("{}...\n", " ".repeat(call_indent)));
        }
    }
    out.push_str(&format!("{}{}\n", " ".repeat(call_indent), call_text));
    out.push_str(&format!("{}...\n", " ".repeat(call_indent + indent_step)));
    out.push_str(&format!("{}...\n", " ".repeat(call_indent)));
    out
}

fn line_ends_with_statement_colon(line: &str) -> bool {
    let mut string_state: Option<(char, bool)> = None;
    let chars: Vec<char> = line.chars().collect();
    let mut i = 0;
    let mut last_code_char = None;

    while i < chars.len() {
        let c = chars[i];
        if let Some((quote, triple)) = string_state {
            if c == '\\' {
                i += 2;
                continue;
            }
            if triple {
                if c == quote
                    && chars.get(i + 1) == Some(&quote)
                    && chars.get(i + 2) == Some(&quote)
                {
                    string_state = None;
                    i += 3;
                    continue;
                }
            } else if c == quote {
                string_state = None;
            }
        } else if c == '\'' || c == '"' {
            let triple = chars.get(i + 1) == Some(&c) && chars.get(i + 2) == Some(&c);
            string_state = Some((c, triple));
            i += if triple { 3 } else { 1 };
            continue;
        } else if c == '#' {
            break;
        } else if !c.is_whitespace() {
            last_code_char = Some(c);
        }
        i += 1;
    }

    last_code_char == Some(':')
}

/// Extract the boolean condition text from an `if` / `elif` / `while` statement line.
///
/// Returns `None` when the condition cannot be extracted with confidence - for example
/// when brackets are left unbalanced on this line (the condition continues on a
/// following line), when the line doesn't start with a recognized keyword, or when no
/// statement-terminating colon can be found. Callers must treat `None` as "no
/// machine-applicable suggestion available" and fall back to `help` text rather than
/// guessing at a replacement.
///
/// It walks the line once, tracking bracket depth (`(`, `[`, `{`) and string-literal
/// state (single and double quotes, triple-quoted strings, backslash escapes) so that
/// colons inside strings, f-string format specs (e.g. `f"{x:>3}"`), and container
/// literals (e.g. `{1: 2}`) are never mistaken for the statement colon, and `:=`
/// walrus operators are not treated as terminators either. The statement colon is the
/// first `:` found at bracket depth 0 outside of any string literal. A `#` reached
/// outside a string ends the scan (nothing after it can contain the statement colon).
fn extract_condition_from_line(line: &str) -> Option<String> {
    split_header(line).map(|(condition, _)| condition.to_string())
}

/// The trailing comment of an `if` / `elif` / `while` header line: `Some("")`
/// when nothing follows the statement colon, the comment text when one does,
/// and `None` when code follows the colon or no colon is found.
fn header_comment(line: &str) -> Option<&str> {
    let (_, rest) = split_header(line)?;
    let rest = rest.trim();
    (rest.is_empty() || rest.starts_with('#')).then_some(rest)
}

fn split_header(line: &str) -> Option<(&str, &str)> {
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
    let mut string_state: Option<(char, bool)> = None;
    let mut colon_byte: Option<usize> = None;

    while i < chars.len() {
        let (byte_idx, c) = chars[i];

        if let Some((quote, triple)) = string_state {
            if c == '\\' {
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
    Some((condition, &trimmed[colon_byte + 1..]))
}

fn update_triple_string_state(line: &str, state: &mut Option<char>) {
    let chars: Vec<char> = line.chars().collect();
    let mut single_quote: Option<char> = None;
    let mut i = 0;
    while i < chars.len() {
        if let Some(quote) = *state {
            if chars[i] == '\\' {
                i += 2;
                continue;
            }
            if chars[i] == quote
                && i + 2 < chars.len()
                && chars[i + 1] == quote
                && chars[i + 2] == quote
            {
                *state = None;
                i += 3;
                continue;
            }
            i += 1;
            continue;
        }
        if let Some(quote) = single_quote {
            if chars[i] == '\\' {
                i += 2;
                continue;
            }
            if chars[i] == quote {
                single_quote = None;
            }
            i += 1;
            continue;
        }
        match chars[i] {
            '#' => break,
            '\'' | '"' => {
                if i + 2 < chars.len() && chars[i + 1] == chars[i] && chars[i + 2] == chars[i] {
                    *state = Some(chars[i]);
                    i += 3;
                } else {
                    single_quote = Some(chars[i]);
                    i += 1;
                }
            }
            _ => i += 1,
        }
    }
}

/// True when any line in the slice lies inside a multi-line (triple-quoted)
/// string. Dedenting such lines changes the string's value, so suggestions
/// that shift a range must refuse when this returns true.
fn contains_multiline_string(lines: &[&str]) -> bool {
    let mut in_triple_string: Option<char> = None;
    for line in lines {
        if in_triple_string.is_some() {
            return true;
        }
        update_triple_string_state(line, &mut in_triple_string);
    }
    false
}

/// True when any line in the range lies inside a multi-line (triple-quoted)
/// string. Dedenting such lines changes the string's value, so suggestions
/// that shift a range must refuse when this returns true.
fn contains_multiline_string_lines(
    source: &str,
    index: &LineIndex,
    from_line: u64,
    to_line_excl: u64,
) -> bool {
    let mut in_triple_string: Option<char> = None;
    let mut line_no = from_line;
    while line_no < to_line_excl {
        let Some(line) = line_at(source, index, line_no) else {
            break;
        };
        if in_triple_string.is_some() {
            return true;
        }
        update_triple_string_state(line, &mut in_triple_string);
        line_no += 1;
    }
    false
}

fn generate_collapsible_if_suggestion_chain(
    outermost: &ComplexityRegion,
    innermost: &ComplexityRegion,
    conditions: &[String],
    comment: &str,
    source: &str,
    index: &LineIndex,
) -> Option<CodeSuggestion> {
    let outer_line = line_at(source, index, outermost.line_start)?;
    let inner_line = line_at(source, index, innermost.line_start)?;
    let inner_start = innermost.line_start + 1;
    let inner_end = innermost.line_end + 1;
    if contains_multiline_string_lines(source, index, inner_start, inner_end) {
        return None;
    }

    let body = span_lines(source, index, inner_start, innermost.line_end)?;
    let body_lines = reindented(
        &body,
        indentation_of(inner_line),
        indentation_of(outer_line),
    )?;
    let combined = combine_conditions_chain(conditions);
    let replacement = format!(
        "{}if {}:{}\n{}",
        indentation_of(outer_line),
        combined,
        comment_suffix(comment),
        body_lines.join("\n")
    );

    Some(CodeSuggestion {
        replacement,
        applicability: Applicability::MachineApplicable,
        spliceable: true,
        description: format!("Merge nested conditions into `if {}:`", combined),
    })
}

/// True when `condition` must be parenthesized before being joined with `and`.
///
/// `or`, conditional expressions (`x if c else y`), assignment expressions
/// (`:=`) and lambdas bind more loosely than `and`, so merging them
/// unparenthesized silently changes what the condition means while still
/// parsing. A condition that does not parse on its own is parenthesized too.
fn needs_parens_for_and(condition: &str) -> bool {
    let Some(parsed) = parse_expression(condition) else {
        return true;
    };
    let loose_start = match parsed.expr() {
        Expr::BoolOp(bool_op) if bool_op.op == BoolOp::Or => bool_op.range.start(),
        Expr::If(conditional) => conditional.range.start(),
        Expr::Named(named) => named.range.start(),
        Expr::Lambda(lambda) => lambda.range.start(),
        _ => return false,
    };
    usize::from(loose_start) == 0
}

/// Returns the operand text of `condition` when the whole condition is one
/// `not <expr>`, so the guard can render as `if <expr>:` instead of the
/// redundant `if not (not <expr>):`. Returns `None` otherwise.
fn strip_top_level_not(condition: &str) -> Option<String> {
    let parsed = parse_expression(condition)?;
    let Expr::UnaryOp(unary) = parsed.expr() else {
        return None;
    };
    if unary.op != UnaryOp::Not || usize::from(unary.range.start()) != 0 {
        return None;
    }
    Some(condition.strip_prefix("not")?.trim_start().to_string())
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
fn collect_if_chain(region: &ComplexityRegion) -> Vec<&ComplexityRegion> {
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
        if current.has_else || child.has_else {
            break;
        }
        chain.push(child);
        current = child;
    }

    chain
}

/// Boolean-operator estimate for when the merged condition text isn't
/// available. Sums each chain element's own `bool_op_count` plus one join per
/// merge.
fn fallback_boolean_count(chain: &[&ComplexityRegion]) -> u64 {
    chain.iter().map(|r| r.bool_op_count).sum::<u64>() + chain.len().saturating_sub(1) as u64
}

/// Walk the linear chain of single-child If regions starting from a loop's
/// first If child. Each next member must be the last statement of the one
/// before it, and the walk stops at a branch, an else/elif, or the end of
/// the nesting.
fn collect_loop_if_chain(region: &ComplexityRegion) -> Vec<&ComplexityRegion> {
    let mut chain = Vec::new();
    let mut current = region.children.iter().find(|c| c.kind == RegionKind::If);

    while let Some(r) = current {
        chain.push(r);
        current = if r.children.len() == 1
            && r.children[0].kind == RegionKind::If
            && !r.has_else
            && !r.children[0].has_else
            && r.children[0].line_end == r.line_end
        {
            Some(&r.children[0])
        } else {
            None
        };
    }

    chain
}

#[cfg(test)]
mod tests;
