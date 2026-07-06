use crate::classes::{Applicability, CodeSnippet, RefactorPlan, RuleCategory};
use crate::refactor_plans::{ComplexityRegion, RegionKind};
use crate::rules::types::{RefactorRule, RuleMetadata, extract_code_snippet};

pub struct FlattenConditionRule;

impl RefactorRule for FlattenConditionRule {
    fn metadata(&self) -> RuleMetadata {
        RuleMetadata {
            id: "C001".to_string(),
            name: "flatten_condition".to_string(),
            category: RuleCategory::Complexity,
            description: "Flatten nested condition blocks by using guard clauses with early returns".to_string(),
            applicability: Applicability::MachineApplicable,
            priority: 4,
            doc_url: Some("https://rohaquinlop.github.io/complexipy/refactoring-rules/#c001-flatten-nested-conditions".to_string()),
        }
    }

    fn check(
        &self,
        region: &ComplexityRegion,
        source: &str,
        function_complexity: u64,
    ) -> Option<RefactorPlan> {
        if region.kind != RegionKind::If || region.nesting < 2 || region.total < 4 {
            return None;
        }

        let before = extract_code_snippet(source, region.line_start, region.line_end);
        let after = generate_flattened_code(region, source);

        Some(RefactorPlan {
            kind: "flatten_condition".to_string(),
            title: "Flatten nested condition block with guard clauses".to_string(),
            line_start: region.line_start,
            line_end: region.line_end,
            current_complexity: function_complexity,
            estimated_reduction: region.nesting,
            estimated_complexity_after: function_complexity.saturating_sub(region.nesting),
            steps: vec![
                "Invert the outer condition".to_string(),
                "Return early when the condition fails".to_string(),
                "Move the main success path one indentation level left".to_string(),
                "Repeat for inner nested conditions where safe".to_string(),
            ],
            rule_id: "C001".to_string(),
            category: RuleCategory::Complexity,
            applicability: Applicability::MachineApplicable,
            description:
                "Flatten nested condition blocks by using guard clauses with early returns"
                    .to_string(),
            before_code: Some(before),
            after_code: Some(after),
            explanation: "Deeply nested conditions are hard to follow. Using guard clauses \
                         with early returns reduces cognitive load by keeping the main path \
                         at a lower indentation level."
                .to_string(),
            references: vec![],
        })
    }
}

pub struct LoopGuardsRule;

impl RefactorRule for LoopGuardsRule {
    fn metadata(&self) -> RuleMetadata {
        RuleMetadata {
            id: "C002".to_string(),
            name: "loop_guards".to_string(),
            category: RuleCategory::Complexity,
            description: "Use continue guards at the top of loops to reduce nesting".to_string(),
            applicability: Applicability::MachineApplicable,
            priority: 3,
            doc_url: Some(
                "https://rohaquinlop.github.io/complexipy/refactoring-rules/#c002-loop-guards"
                    .to_string(),
            ),
        }
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

        let before = extract_code_snippet(source, region.line_start, region.line_end);
        let after = generate_loop_guard_code(region, source);

        Some(RefactorPlan {
            kind: "loop_guards".to_string(),
            title: "Flatten loop body with continue guards".to_string(),
            line_start: region.line_start,
            line_end: region.line_end,
            current_complexity: function_complexity,
            estimated_reduction: reduction,
            estimated_complexity_after: function_complexity.saturating_sub(reduction),
            steps: vec![
                "Add negative condition checks at the top of the loop".to_string(),
                "Use continue for skipped items".to_string(),
                "Keep the main processing path unindented".to_string(),
            ],
            rule_id: "C002".to_string(),
            category: RuleCategory::Complexity,
            applicability: Applicability::MachineApplicable,
            description: "Use continue guards at the top of loops to reduce nesting".to_string(),
            before_code: Some(before),
            after_code: Some(after),
            explanation: "Nested conditions inside loops add unnecessary indentation. \
                         Using continue guards keeps the main logic at a lower nesting level \
                         and makes the loop easier to follow."
                .to_string(),
            references: vec![],
        })
    }
}

pub struct ExtractHelperRule;

impl RefactorRule for ExtractHelperRule {
    fn metadata(&self) -> RuleMetadata {
        RuleMetadata {
            id: "C003".to_string(),
            name: "extract_helper".to_string(),
            category: RuleCategory::Complexity,
            description: "Extract complex code blocks into separate helper functions".to_string(),
            applicability: Applicability::MaybeIncorrect,
            priority: 3,
            doc_url: Some("https://rohaquinlop.github.io/complexipy/refactoring-rules/#c003-extract-helper-function".to_string()),
        }
    }

    fn check(
        &self,
        region: &ComplexityRegion,
        source: &str,
        function_complexity: u64,
    ) -> Option<RefactorPlan> {
        let line_count = region.line_end.saturating_sub(region.line_start) + 1;

        if region.total < 6 || line_count < 5 {
            return None;
        }

        let before = extract_code_snippet(source, region.line_start, region.line_end);

        Some(RefactorPlan {
            kind: "extract_helper".to_string(),
            title: "Extract complex block into helper function".to_string(),
            line_start: region.line_start,
            line_end: region.line_end,
            current_complexity: function_complexity,
            estimated_reduction: region.total.saturating_sub(1),
            estimated_complexity_after: function_complexity
                .saturating_sub(region.total.saturating_sub(1)),
            steps: vec![
                "Move the selected block into a private helper".to_string(),
                "Pass required values as parameters".to_string(),
                "Return the result/status needed by the caller".to_string(),
            ],
            rule_id: "C003".to_string(),
            category: RuleCategory::Complexity,
            applicability: Applicability::MaybeIncorrect,
            description: "Extract complex code blocks into separate helper functions".to_string(),
            before_code: Some(before),
            after_code: None,
            explanation: "Complex code blocks should be extracted into named functions \
                         to improve readability and testability. The extracted function \
                         can be given a descriptive name that explains its purpose."
                .to_string(),
            references: vec![],
        })
    }
}

pub struct SplitDispatcherRule;

impl RefactorRule for SplitDispatcherRule {
    fn metadata(&self) -> RuleMetadata {
        RuleMetadata {
            id: "C004".to_string(),
            name: "split_dispatcher".to_string(),
            category: RuleCategory::Complexity,
            description: "Split long elif chains or match statements into separate handlers"
                .to_string(),
            applicability: Applicability::MaybeIncorrect,
            priority: 2,
            doc_url: Some(
                "https://rohaquinlop.github.io/complexipy/refactoring-rules/#c004-split-dispatcher"
                    .to_string(),
            ),
        }
    }

    fn check(
        &self,
        region: &ComplexityRegion,
        source: &str,
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

        let before = extract_code_snippet(source, region.line_start, region.line_end);

        Some(RefactorPlan {
            kind: "split_dispatcher".to_string(),
            title: "Split conditional dispatcher into handlers".to_string(),
            line_start: region.line_start,
            line_end: region.line_end,
            current_complexity: function_complexity,
            estimated_reduction: reduction,
            estimated_complexity_after: function_complexity.saturating_sub(reduction),
            steps: vec![
                "Create one handler per case".to_string(),
                "Replace the chain with a dispatch table or small delegating match".to_string(),
                "Keep the orchestration function shallow".to_string(),
            ],
            rule_id: "C004".to_string(),
            category: RuleCategory::Complexity,
            applicability: Applicability::MaybeIncorrect,
            description: "Split long elif chains or match statements into separate handlers"
                .to_string(),
            before_code: Some(before),
            after_code: None,
            explanation: "Long conditional chains are hard to maintain and extend. \
                         Splitting them into separate handlers makes each case \
                         independently testable and the dispatch logic clearer."
                .to_string(),
            references: vec![],
        })
    }
}

pub struct ExtractPredicateRule;

impl RefactorRule for ExtractPredicateRule {
    fn metadata(&self) -> RuleMetadata {
        RuleMetadata {
            id: "C005".to_string(),
            name: "extract_predicate".to_string(),
            category: RuleCategory::Readability,
            description: "Extract complex boolean conditions into named predicate functions".to_string(),
            applicability: Applicability::MachineApplicable,
            priority: 3,
            doc_url: Some("https://rohaquinlop.github.io/complexipy/refactoring-rules/#c005-extract-predicate".to_string()),
        }
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

        let before = extract_code_snippet(source, region.line_start, region.line_end);
        let after = generate_predicate_code(region, source);

        Some(RefactorPlan {
            kind: "extract_predicate".to_string(),
            title: "Extract complex condition into named predicate".to_string(),
            line_start: region.line_start,
            line_end: region.line_end,
            current_complexity: function_complexity,
            estimated_reduction: region.bool_op_count,
            estimated_complexity_after: function_complexity.saturating_sub(region.bool_op_count),
            steps: vec![
                "Move the condition into a well-named helper function".to_string(),
                "Use the helper in the branch condition".to_string(),
                "Keep the caller focused on control flow".to_string(),
            ],
            rule_id: "C005".to_string(),
            category: RuleCategory::Readability,
            applicability: Applicability::MachineApplicable,
            description: "Extract complex boolean conditions into named predicate functions"
                .to_string(),
            before_code: Some(before),
            after_code: Some(after),
            explanation: "Complex boolean expressions are hard to understand at a glance. \
                         Extracting them into named predicates makes the code self-documenting \
                         and easier to test."
                .to_string(),
            references: vec![],
        })
    }
}

pub struct ReduceNestingRule;

impl RefactorRule for ReduceNestingRule {
    fn metadata(&self) -> RuleMetadata {
        RuleMetadata {
            id: "C006".to_string(),
            name: "reduce_nesting".to_string(),
            category: RuleCategory::Complexity,
            description: "Reduce nesting depth by using early returns and guard clauses".to_string(),
            applicability: Applicability::MachineApplicable,
            priority: 4,
            doc_url: Some("https://rohaquinlop.github.io/complexipy/refactoring-rules/#c006-reduce-nesting-depth".to_string()),
        }
    }

    fn check(
        &self,
        region: &ComplexityRegion,
        source: &str,
        function_complexity: u64,
    ) -> Option<RefactorPlan> {
        if region.nesting < 3 || region.total < 5 {
            return None;
        }

        if region.kind == RegionKind::If && region.nesting >= 2 && region.total >= 4 {
            return None;
        }

        let before = extract_code_snippet(source, region.line_start, region.line_end);

        Some(RefactorPlan {
            kind: "reduce_nesting".to_string(),
            title: "Reduce nesting depth".to_string(),
            line_start: region.line_start,
            line_end: region.line_end,
            current_complexity: function_complexity,
            estimated_reduction: region.nesting.saturating_sub(1),
            estimated_complexity_after: function_complexity
                .saturating_sub(region.nesting.saturating_sub(1)),
            steps: vec![
                "Identify the deepest nesting level".to_string(),
                "Extract inner logic into a separate function".to_string(),
                "Use early returns to reduce indentation".to_string(),
            ],
            rule_id: "C006".to_string(),
            category: RuleCategory::Complexity,
            applicability: Applicability::MachineApplicable,
            description: "Reduce nesting depth by using early returns and guard clauses"
                .to_string(),
            before_code: Some(before),
            after_code: None,
            explanation: "Deep nesting (3+ levels) makes code hard to follow. \
                         Consider extracting inner blocks or using guard clauses \
                         to keep indentation shallow."
                .to_string(),
            references: vec![],
        })
    }
}

pub struct FlattenTryRule;

impl RefactorRule for FlattenTryRule {
    fn metadata(&self) -> RuleMetadata {
        RuleMetadata {
            id: "C011".to_string(),
            name: "flatten_try".to_string(),
            category: RuleCategory::Complexity,
            description: "Flatten nested try/except blocks by combining or restructuring".to_string(),
            applicability: Applicability::MaybeIncorrect,
            priority: 2,
            doc_url: Some("https://rohaquinlop.github.io/complexipy/refactoring-rules/#c011-flatten-tryexcept".to_string()),
        }
    }

    fn check(
        &self,
        region: &ComplexityRegion,
        source: &str,
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

        let before = extract_code_snippet(source, region.line_start, region.line_end);

        Some(RefactorPlan {
            kind: "flatten_try".to_string(),
            title: "Flatten nested try/except blocks".to_string(),
            line_start: region.line_start,
            line_end: region.line_end,
            current_complexity: function_complexity,
            estimated_reduction: 2,
            estimated_complexity_after: function_complexity.saturating_sub(2),
            steps: vec![
                "Review if inner try/except can be merged with outer".to_string(),
                "Consider using a single try with multiple except clauses".to_string(),
                "Extract inner try block into a helper function".to_string(),
            ],
            rule_id: "C011".to_string(),
            category: RuleCategory::Complexity,
            applicability: Applicability::MaybeIncorrect,
            description: "Flatten nested try/except blocks by combining or restructuring"
                .to_string(),
            before_code: Some(before),
            after_code: None,
            explanation: "Nested try/except blocks are confusing and hard to maintain. \
                         Consider merging them or extracting the inner block into \
                         a separate function with its own error handling."
                .to_string(),
            references: vec![],
        })
    }
}

fn generate_flattened_code(region: &ComplexityRegion, source: &str) -> CodeSnippet {
    let lines: Vec<&str> = source.lines().collect();
    let start = (region.line_start.saturating_sub(1)) as usize;
    let end = (region.line_end as usize).min(lines.len());

    if start >= lines.len() {
        return CodeSnippet {
            text: String::new(),
            line_start: region.line_start,
            line_end: region.line_end,
        };
    }

    let mut result = Vec::new();
    let base_indent = get_indentation(lines[start]);

    for line in &lines[start..end] {
        let trimmed = line.trim_start();
        let current_indent = get_indentation(line);
        let relative_indent = current_indent.saturating_sub(base_indent);

        if trimmed.starts_with("if ") && relative_indent > 0 {
            let condition = extract_condition(trimmed);
            let new_indent = " ".repeat(base_indent + 4);
            result.push(format!("{new_indent}if not {condition}:"));
            result.push(format!("{new_indent}    return None"));
        } else if trimmed.starts_with("return ") || trimmed.starts_with("return(") {
            let new_indent = " ".repeat(base_indent + 4);
            result.push(format!("{new_indent}{trimmed}"));
        }
    }

    CodeSnippet {
        text: result.join("\n"),
        line_start: region.line_start,
        line_end: region.line_start + result.len() as u64,
    }
}

fn generate_loop_guard_code(region: &ComplexityRegion, source: &str) -> CodeSnippet {
    let lines: Vec<&str> = source.lines().collect();
    let start = (region.line_start.saturating_sub(1)) as usize;
    let end = (region.line_end as usize).min(lines.len());

    if start >= lines.len() {
        return CodeSnippet {
            text: String::new(),
            line_start: region.line_start,
            line_end: region.line_end,
        };
    }

    let mut result = Vec::new();
    let base_indent = get_indentation(lines[start]);
    let loop_body_indent = base_indent + 4;

    result.push(lines[start].to_string());

    for line in &lines[start + 1..end] {
        let trimmed = line.trim_start();
        let current_indent = get_indentation(line);
        let relative_indent = current_indent.saturating_sub(base_indent);

        if trimmed.starts_with("if ") && relative_indent == 1 {
            let condition = extract_condition(trimmed);
            let guard_indent = " ".repeat(loop_body_indent);
            result.push(format!("{guard_indent}if not {condition}:"));
            result.push(format!("{guard_indent}    continue"));
        } else if relative_indent >= 1 && !trimmed.is_empty() {
            let new_indent = " ".repeat(loop_body_indent);
            result.push(format!("{new_indent}{trimmed}"));
        }
    }

    CodeSnippet {
        text: result.join("\n"),
        line_start: region.line_start,
        line_end: region.line_start + result.len() as u64,
    }
}

fn generate_predicate_code(region: &ComplexityRegion, source: &str) -> CodeSnippet {
    let lines: Vec<&str> = source.lines().collect();
    let start = (region.line_start.saturating_sub(1)) as usize;

    if start >= lines.len() {
        return CodeSnippet {
            text: String::new(),
            line_start: region.line_start,
            line_end: region.line_end,
        };
    }

    let line = lines[start];
    let condition = extract_condition(line.trim_start());
    let base_indent = get_indentation(line);

    let predicate_indent = " ".repeat(base_indent);
    let call_indent = " ".repeat(base_indent + 4);

    let result = format!(
        "{predicate_indent}def is_valid(data) -> bool:\n\
         {predicate_indent}    # TODO: Give this a descriptive name\n\
         {predicate_indent}    return {condition}\n\
         \n\
         {call_indent}if is_valid(data):"
    );

    CodeSnippet {
        text: result,
        line_start: region.line_start,
        line_end: region.line_start + 4,
    }
}

fn get_indentation(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

fn extract_condition(line: &str) -> String {
    let trimmed = line.trim_start();
    if let Some(pos) = trimmed.find("if ") {
        let cond_start = pos + 3;
        if let Some(end) = trimmed[cond_start..].find(':') {
            return trimmed[cond_start..cond_start + end].trim().to_string();
        }
    } else if let Some(pos) = trimmed.find("elif ") {
        let cond_start = pos + 5;
        if let Some(end) = trimmed[cond_start..].find(':') {
            return trimmed[cond_start..cond_start + end].trim().to_string();
        }
    } else if let Some(pos) = trimmed.find("while ") {
        let cond_start = pos + 6;
        if let Some(end) = trimmed[cond_start..].find(':') {
            return trimmed[cond_start..cond_start + end].trim().to_string();
        }
    }
    trimmed.to_string()
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
