use crate::classes::{Applicability, CodeSuggestion, RefactorPlan, RuleCategory};
use crate::refactor_plans::{ComplexityRegion, RegionKind};
use crate::rules::types::{RefactorRule, RuleMetadata};
use crate::utils::count_bool_ops;
use ruff_python_ast::visitor::{Visitor, walk_expr};
use ruff_python_ast::{CmpOp, Expr};
use ruff_python_parser::parse_expression;
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
            effectiveness: 4,
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
                "Invert the outer condition and exit early — `return` from the function, \
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

        let lines: Vec<&str> = source.lines().collect();
        let chain = collect_loop_if_chain(region, &lines);

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

        let suggestion = generate_loop_guard_suggestion(region, source);
        let help = if suggestion.is_none() {
            Some(
                "Add `if not (<condition>): continue` guards at the top of the loop body \
                 for each nested if condition, then dedent the remaining logic by one \
                 level per guard."
                    .to_string(),
            )
        } else {
            None
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
            doc_url:
                "https://rohaquinlop.github.io/complexipy/refactoring-rules/#c004-split-dispatcher"
                    .to_string(),
        })
    }

    fn check(
        &self,
        region: &ComplexityRegion,
        source: &str,
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

        let lines: Vec<&str> = source.lines().collect();
        let help = if let Some(subject) = single_variable_equality_subject(region, &lines) {
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

fn single_variable_equality_subject(region: &ComplexityRegion, lines: &[&str]) -> Option<String> {
    let start = (region.line_start.saturating_sub(1)) as usize;
    let end = (region.line_end as usize).min(lines.len());
    if start >= end {
        return None;
    }

    let base_indent = get_indentation_from_str(lines[start]);
    let mut subject: Option<String> = None;
    let mut clause_count = 0;

    for line in &lines[start..end] {
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
    let parsed = parse_expression(condition).ok()?;
    let expr = *parsed.into_syntax().body;
    let Expr::Compare(compare) = expr else {
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
    match expr {
        Expr::Name(n) => Some(n.id.to_string()),
        Expr::Attribute(a) => {
            let base = simple_reference_text(&a.value)?;
            Some(format!("{base}.{}", a.attr.id))
        }
        _ => None,
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

        let lines: Vec<&str> = source.lines().collect();
        let chain = collect_if_chain(region, &lines);

        if chain.len() < 2 {
            return None;
        }

        let innermost = chain.last().unwrap();

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

        let outermost = chain[0];

        for (i, r) in chain.iter().enumerate() {
            if i == chain.len() - 1 {
                break;
            }
            let next = chain[i + 1];
            let r_start = (r.line_start as usize).saturating_sub(1);
            let r_end = (r.line_end as usize).min(lines.len());
            let next_start = (next.line_start as usize).saturating_sub(1);
            let next_end = (next.line_end as usize).min(lines.len());

            let mut body_start = r_start + 1;
            if extract_condition_from_line(lines[r_start].trim_start()).is_none() {
                while body_start < next_start.min(lines.len()) {
                    if line_ends_with_statement_colon(lines[body_start]) {
                        body_start += 1;
                        break;
                    }
                    body_start += 1;
                }
            }

            for line in &lines[body_start..next_start.min(lines.len())] {
                if !line.trim_start().is_empty() {
                    return None;
                }
            }

            for line in &lines[next_end..r_end] {
                if !line.trim_start().is_empty() {
                    return None;
                }
            }
        }

        let mut suggestion = None;
        let mut help = None;
        if conditions_extracted {
            match generate_collapsible_if_suggestion_chain(
                outermost,
                innermost,
                &conditions,
                &lines,
            ) {
                Some(s) => suggestion = Some(s),
                None => {
                    help = Some(
                        "Merge the nested if statements into a single `if <outer> and <inner>:` \
                         block, adjusting the indentation of any multi-line string literals in \
                         the body by hand — an automatic replacement would change their content."
                            .to_string(),
                    );
                }
            }
        } else {
            help = Some(
                "Merge the nested if statements into a single `if <outer> and <inner>:` \
                 block. The exact condition text could not be extracted automatically \
                 (it may span multiple lines or contain content the parser could not \
                 confidently isolate) — combine the conditions with `and` manually."
                    .to_string(),
            );
        }

        let old_complexity = region.total;
        let boolean_count = if conditions_extracted {
            let combined = combine_conditions_chain(&conditions);
            match parse_expression(&combined) {
                Ok(parsed) => count_bool_ops(*parsed.into_syntax().body, region.nesting),
                Err(_) => fallback_boolean_count(&chain),
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
/// holds no guardable chain — callers fall back to `help` text then.
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

    let mut guards = Vec::new();
    let mut current_region: Option<&ComplexityRegion> = None;

    for child in &region.children {
        if child.kind == RegionKind::If {
            current_region = Some(child);
            break;
        }
    }

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

        if r.children.len() == 1
            && r.children[0].kind == RegionKind::If
            && !has_else_branch(r, &lines)
            && !has_else_branch(&r.children[0], &lines)
        {
            current_region = Some(&r.children[0]);
            continue;
        }
        break;
    }

    // A loop-level `else` (for/while-else, aligned with the loop header)
    // cannot survive the guard transformation: re-emitted as-is it becomes a
    // dangling `else` inside the body. A first chain member with its own
    // `else`/`elif` cannot become a guard either — the guard would skip the
    // `else` branch entirely.
    if has_else_branch(region, &lines) || guards.is_empty() || has_else_branch(guards[0].0, &lines)
    {
        return None;
    }

    let innermost = guards.last().unwrap().0;
    let innermost_line_idx = (innermost.line_start.saturating_sub(1)) as usize;
    let innermost_end = (innermost.line_end as usize).min(lines.len());
    let chain_start_idx = (guards[0].0.line_start.saturating_sub(1)) as usize;

    for i in 0..guards.len().saturating_sub(1) {
        let range_start = (guards[i].0.line_start as usize).min(lines.len());
        let range_end = (guards[i + 1].0.line_start.saturating_sub(1)) as usize;
        if contains_multiline_string(&lines[range_start..range_end.min(lines.len())]) {
            return None;
        }
    }
    if contains_multiline_string(&lines[(innermost_line_idx + 1)..innermost_end]) {
        return None;
    }

    // The body indent and step come from the first chain member's own line:
    // the header may span several lines, so its continuation lines cannot be
    // trusted to reveal the step. Header continuation lines fall into the
    // leading range below and pass through unchanged.
    let loop_body_indent = get_indentation_from_str(lines[chain_start_idx]);
    let indent_step = loop_body_indent.saturating_sub(base_indent);

    let mut result = Vec::new();
    result.push(lines[start].to_string());
    result.extend(
        lines[(start + 1)..chain_start_idx]
            .iter()
            .map(|line| (*line).to_string()),
    );

    for (i, (member, guard)) in guards.iter().enumerate() {
        let guard_text = match strip_top_level_not(guard) {
            Some(rest) => rest,
            None => format!("not ({guard})"),
        };
        result.push(format!(
            "{}if {}:",
            " ".repeat(loop_body_indent),
            guard_text
        ));
        result.push(format!(
            "{}continue",
            " ".repeat(loop_body_indent + indent_step)
        ));

        if i + 1 < guards.len() {
            let next_start = (guards[i + 1].0.line_start.saturating_sub(1)) as usize;
            let range_start = (member.line_start as usize).min(lines.len());
            let shift = indent_step * (i + 1);
            for line in &lines[range_start..next_start.min(lines.len())] {
                let trimmed = line.trim_start();
                if trimmed.is_empty() {
                    result.push(String::new());
                    continue;
                }
                let current_indent = get_indentation_from_str(line);
                let shifted = current_indent.saturating_sub(shift);
                let padding = " ".repeat(shifted);
                result.push(format!("{}{}", padding, trimmed));
            }
        }
    }

    for line in &lines[(innermost_line_idx + 1)..innermost_end] {
        let trimmed = line.trim_start();
        if trimmed.is_empty() {
            result.push(String::new());
            continue;
        }
        let current_indent = get_indentation_from_str(line);
        let shifted = current_indent.saturating_sub(indent_step * guards.len());
        let padding = " ".repeat(shifted);
        result.push(format!("{}{}", padding, trimmed));
    }
    result.extend(
        lines[innermost_end..end]
            .iter()
            .map(|line| (*line).to_string()),
    );

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

/// Detect the indentation step (spaces per level) from a block of code.
/// Blank and comment-only lines carry no structural indent; pairing them
/// against a body statement would report the body's full indent as the step.
fn detect_indent_step(lines: &[&str]) -> usize {
    let mut prev_indent: Option<usize> = None;
    for line in lines {
        let trimmed = line.trim_start();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let indent = get_indentation_from_str(line);
        match prev_indent {
            Some(prev) if indent > prev => return indent - prev,
            _ => {}
        }
        prev_indent = Some(indent);
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
) -> Option<CodeSuggestion> {
    let lines: Vec<&str> = source.lines().collect();
    let start = (region.line_start.saturating_sub(1)) as usize;

    if start >= lines.len() {
        return None;
    }

    let line = lines[start];
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
    let indent_step = detect_indent_step(&lines[start..]);

    let helper_body_indent = " ".repeat(indent_step);
    let mut func_name = format!("_check_condition_L{}", region.line_start);
    let mut def_pattern = format!("def {func_name}(");
    while lines.iter().any(|line| line.contains(&def_pattern)) {
        func_name.push('_');
        def_pattern = format!("def {func_name}(");
    }

    let parameters_text = parameters.join(", ");
    let call_text = format!("{keyword} {func_name}({parameters_text}):");
    let statement_context = match enclosing_header_indices(&lines, start) {
        Some(headers) => render_statement_context(&headers, &lines, start, indent_step, &call_text),
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

struct FreeNameCollector<'a> {
    builtins: &'a [&'a str],
    names: Vec<String>,
    disqualified: bool,
}

impl<'a> Visitor<'a> for FreeNameCollector<'a> {
    fn visit_expr(&mut self, expr: &'a Expr) {
        match expr {
            Expr::Named(_) | Expr::Lambda(_) => {
                self.disqualified = true;
                return;
            }
            Expr::ListComp(_) | Expr::SetComp(_) | Expr::DictComp(_) | Expr::Generator(_) => {
                self.disqualified = true;
                return;
            }
            Expr::Name(name) => {
                let is_new = !self.builtins.contains(&name.id.as_str())
                    && !self.names.iter().any(|n| n == name.id.as_str());
                if is_new {
                    self.names.push(name.id.to_string());
                }
            }
            _ => {}
        }
        walk_expr(self, expr);
    }
}

/// The condition's free variables, in first-use order, with builtins
/// excluded. Returns `None` when the condition binds a name (`:=`) or holds
/// a nested scope (lambda, comprehension) — the parameter list could not be
/// trusted, so callers fall back to help text.
fn collect_free_names(condition: &str) -> Option<Vec<String>> {
    let parsed = parse_expression(condition).ok()?;
    let expr = *parsed.into_syntax().body;
    let mut collector = FreeNameCollector {
        builtins: PYTHON_BUILTINS,
        names: Vec::new(),
        disqualified: false,
    };
    collector.visit_expr(&expr);
    if collector.disqualified {
        return None;
    }
    Some(collector.names)
}

/// Line indices (outermost first) of the block headers enclosing the
/// condition line, from module level down to the enclosing block. Returns
/// `None` when the chain cannot be traced to column 0 (module-level
/// condition, or a broken chain such as a bare statement at a lesser
/// indent) — callers render the bare call then.
fn enclosing_header_indices(lines: &[&str], start: usize) -> Option<Vec<usize>> {
    let mut headers = Vec::new();
    let mut current_indent = get_indentation_from_str(lines[start]);
    let mut expect_chain_opener = false;
    let mut i = start;
    while i > 0 {
        i -= 1;
        let line = lines[i];
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
            headers.push(i);
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
        headers.push(i);
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

fn directly_follows(lines: &[&str], item_idx: usize, prev_idx: usize) -> bool {
    (prev_idx + 1..item_idx).all(|k| {
        let trimmed = lines[k].trim_start();
        trimmed.is_empty() || trimmed.starts_with('#')
    })
}

/// Render the enclosing blocks around the call statement at their real
/// indentation, using `...` placeholders for skipped statements. The result
/// parses on its own: every placeholder is an Ellipsis expression statement.
fn render_statement_context(
    headers: &[usize],
    lines: &[&str],
    start: usize,
    indent_step: usize,
    call_text: &str,
) -> String {
    let call_indent = get_indentation_from_str(lines[start]);
    let mut out = String::new();
    let mut prev_idx: Option<usize> = None;
    for &header_idx in headers {
        let header_indent = get_indentation_from_str(lines[header_idx]);
        if let Some(prev) = prev_idx {
            let prev_indent = get_indentation_from_str(lines[prev]);
            if header_indent == prev_indent {
                out.push_str(&format!("{}...\n", " ".repeat(prev_indent + indent_step)));
            } else if !directly_follows(lines, header_idx, prev) {
                out.push_str(&format!("{}...\n", " ".repeat(header_indent)));
            }
        }
        out.push_str(lines[header_idx]);
        out.push('\n');
        prev_idx = Some(header_idx);
    }
    if let Some(prev) = prev_idx {
        let prev_indent = get_indentation_from_str(lines[prev]);
        if call_indent > prev_indent && !directly_follows(lines, start, prev) {
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
/// Returns `None` when the condition cannot be extracted with confidence — for example
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
    Some(condition.to_string())
}

fn has_else_branch(region: &ComplexityRegion, lines: &[&str]) -> bool {
    let start = (region.line_start.saturating_sub(1)) as usize;
    let end = (region.line_end as usize).min(lines.len());

    if start >= lines.len() {
        return false;
    }

    let base_indent = get_indentation_from_str(lines[start]);
    let mut in_triple_string: Option<char> = None;

    for line in &lines[start..end] {
        if in_triple_string.is_none() {
            let trimmed = line.trim_start();
            let current_indent = get_indentation_from_str(line);

            if current_indent == base_indent
                && (trimmed.starts_with("else:") || trimmed.starts_with("elif "))
            {
                return true;
            }
        }
        update_triple_string_state(line, &mut in_triple_string);
    }

    false
}

fn update_triple_string_state(line: &str, state: &mut Option<char>) {
    let chars: Vec<char> = line.chars().collect();
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
        match chars[i] {
            '#' => break,
            '\'' | '"' => {
                if i + 2 < chars.len() && chars[i + 1] == chars[i] && chars[i + 2] == chars[i] {
                    *state = Some(chars[i]);
                    i += 3;
                } else {
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

fn generate_collapsible_if_suggestion_chain(
    outermost: &ComplexityRegion,
    innermost: &ComplexityRegion,
    conditions: &[String],
    lines: &[&str],
) -> Option<CodeSuggestion> {
    let outer_line_idx = (outermost.line_start.saturating_sub(1)) as usize;
    let inner_line_idx = (innermost.line_start.saturating_sub(1)) as usize;
    let inner_end = (innermost.line_end as usize).min(lines.len());

    let body_start = inner_line_idx + 1;
    if contains_multiline_string(&lines[body_start..inner_end]) {
        return None;
    }

    let outer_indent = get_indentation_from_str(lines[outer_line_idx]);
    let indent_step = detect_indent_step(&lines[outer_line_idx..inner_end]);

    let combined = combine_conditions_chain(conditions);

    let chain_depth = conditions.len();
    let mut body_lines = Vec::new();
    for line in &lines[body_start..inner_end] {
        let trimmed = line.trim_start();
        if trimmed.is_empty() {
            body_lines.push(String::new());
            continue;
        }
        let current_indent = get_indentation_from_str(line);
        let shifted = current_indent.saturating_sub(indent_step * (chain_depth - 1));
        let padding = " ".repeat(shifted);
        body_lines.push(format!("{}{}", padding, trimmed));
    }

    let indent = " ".repeat(outer_indent);
    let replacement = format!("{}if {}:\n{}", indent, combined, body_lines.join("\n"));

    Some(CodeSuggestion {
        replacement,
        applicability: Applicability::MachineApplicable,
        spliceable: true,
        description: format!("Merge nested conditions into `if {}:`", combined),
    })
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

/// Returns the remainder of `condition` when it is a single top-level
/// `not <expr>` whose `<expr>` holds no top-level `and`/`or`/conditional/
/// walrus. The guard can then render as `if <expr>:` instead of the redundant
/// `if not (not <expr>):`. Returns `None` for anything less certain.
fn strip_top_level_not(condition: &str) -> Option<String> {
    let masked = mask_nested(condition);
    let rest_masked = masked.strip_prefix("not ")?;
    if rest_masked.contains(" and ")
        || rest_masked.contains(" or ")
        || rest_masked.contains(" if ")
        || rest_masked.contains(":=")
    {
        return None;
    }
    Some(condition[4..].to_string())
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

/// Boolean-operator estimate for when the merged condition text isn't
/// available. Sums each chain element's own `bool_op_count` plus one join per
/// merge.
fn fallback_boolean_count(chain: &[&ComplexityRegion]) -> u64 {
    chain.iter().map(|r| r.bool_op_count).sum::<u64>() + chain.len().saturating_sub(1) as u64
}

/// Walk the linear chain of single-child If regions starting from a loop's
/// first If child, stopping at the same boundaries `collect_if_chain` does
/// (a branch, an else/elif, or the end of the nesting).
fn collect_loop_if_chain<'a>(
    region: &'a ComplexityRegion,
    lines: &[&str],
) -> Vec<&'a ComplexityRegion> {
    let mut chain = Vec::new();
    let mut current = region.children.iter().find(|c| c.kind == RegionKind::If);

    while let Some(r) = current {
        chain.push(r);
        current = if r.children.len() == 1
            && r.children[0].kind == RegionKind::If
            && !has_else_branch(r, lines)
            && !has_else_branch(&r.children[0], lines)
        {
            Some(&r.children[0])
        } else {
            None
        };
    }

    chain
}

#[cfg(test)]
#[path = "../tests/rules/complexity.rs"]
mod tests;
