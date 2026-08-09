use crate::classes::{Applicability, CodeSuggestion, RefactorPlan, RuleCategory};
use crate::refactor_plans::{ComplexityRegion, RegionKind};
use crate::rules::types::{RefactorRule, RuleMetadata};
use crate::utils::count_bool_ops;
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
                "Invert the outer condition and return early. Move the main success path \
                        one indentation level left. Repeat for inner nested conditions where safe."
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
                "Add `if not <condition>: continue` guards at the top of the loop body \
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
        let outer_indent =
            get_indentation_from_str(lines[(outermost.line_start.saturating_sub(1)) as usize]);
        let indent_step = detect_indent_step(
            &lines[(outermost.line_start.saturating_sub(1)) as usize
                ..((outermost.line_end as usize).min(lines.len()))],
        );
        let body_indent = outer_indent + indent_step;

        for (i, r) in chain.iter().enumerate() {
            if i == chain.len() - 1 {
                break;
            }
            let next = chain[i + 1];
            let r_end = (r.line_end as usize).min(lines.len());
            let next_end = (next.line_end as usize).min(lines.len());

            for line in &lines[next_end..r_end] {
                let trimmed = line.trim_start();
                let indent = get_indentation_from_str(line);
                if !trimmed.is_empty() && indent == body_indent {
                    return None;
                }
            }
        }

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
    if has_else_branch(region, &lines)
        || guards.is_empty()
        || has_else_branch(guards[0].0, &lines)
    {
        return None;
    }

    let innermost = guards.last().unwrap().0;
    let innermost_line_idx = (innermost.line_start.saturating_sub(1)) as usize;
    let innermost_end = (innermost.line_end as usize).min(lines.len());
    let chain_start_idx = (guards[0].0.line_start.saturating_sub(1)) as usize;

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

    for (_, guard) in &guards {
        result.push(format!("{}if not {}:", " ".repeat(loop_body_indent), guard));
        result.push(format!(
            "{}continue",
            " ".repeat(loop_body_indent + indent_step)
        ));
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
/// into a named predicate function. The rewritten `if` replaces the original
/// statement at its own indentation (nesting it inside the helper would make it
/// unreachable after the `return`); the `...` stands in for the caller's body and
/// keeps the snippet parseable on its own.
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
        spliceable: false,
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

    let combined = combine_conditions_chain(conditions);

    let body_start = inner_line_idx + 1;
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

    CodeSuggestion {
        replacement,
        applicability: Applicability::MachineApplicable,
        spliceable: true,
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
