use super::types::RefactorRule;
use crate::classes::{CodeSuggestion, RefactorPlan};
use crate::cognitive_complexity::function_level_cognitive_complexity_shared;
use crate::refactor_plans::ComplexityRegion;
use ruff_python_parser::parse_module;
use std::collections::HashMap;

pub struct RuleRegistry {
    rules: Vec<Box<dyn RefactorRule>>,
}

impl RuleRegistry {
    pub fn new() -> Self {
        let mut registry = Self { rules: Vec::new() };
        registry.register_defaults();
        registry
    }

    fn register_defaults(&mut self) {
        use super::complexity::*;

        self.register(Box::new(FlattenConditionRule));
        self.register(Box::new(LoopGuardsRule));
        self.register(Box::new(ExtractHelperRule));
        self.register(Box::new(SplitDispatcherRule));
        self.register(Box::new(ExtractPredicateRule));
        self.register(Box::new(FlattenTryRule));
        self.register(Box::new(CollapsibleIfRule));
    }

    pub fn register(&mut self, rule: Box<dyn RefactorRule>) {
        self.rules.push(rule);
    }

    /// Build a `rule_id -> effectiveness` lookup from the currently registered
    /// rules. `RefactorPlan`s are plain data decoupled from the `RefactorRule`
    /// objects that produced them (see `collect_plans`), so ordering can't read
    /// effectiveness off the rule directly once we're holding a `Vec<RefactorPlan>` --
    /// this map is what lets us look it up by `rule_id` without hardcoding a
    /// `match rule_id` here. Adding a 9th rule only requires registering it in
    /// `register_defaults` and giving it an `effectiveness` in its own metadata;
    /// nothing in this file needs to change.
    fn effectiveness_by_rule_id(&self) -> HashMap<&str, u8> {
        self.rules
            .iter()
            .map(|rule| {
                let meta = rule.metadata();
                (meta.id.as_str(), meta.effectiveness)
            })
            .collect()
    }

    /// Returns the selected plans (capped at 5) plus a count of additional
    /// plans that survived dedup but were dropped purely by the cap, so
    /// callers can render "... and N more suggestions" instead of silently
    /// truncating.
    #[must_use]
    pub fn analyze(
        &self,
        regions: &[ComplexityRegion],
        source: &str,
        function_complexity: u64,
        is_module: bool,
    ) -> (Vec<RefactorPlan>, u64) {
        let mut plans = Vec::new();

        self.collect_plans(regions, source, function_complexity, &mut plans);
        self.measure_plans(&mut plans, source, is_module);

        plans.retain(|plan| plan.estimated_reduction >= 1);

        let effectiveness = self.effectiveness_by_rule_id();
        select_non_overlapping(plans, &effectiveness)
    }

    /// Plans whose measurement fails keep their formula estimate with
    /// `reduction_is_measured = false` — never a panic, never a fabricated
    /// measured number.
    fn measure_plans(&self, plans: &mut [RefactorPlan], source: &str, is_module: bool) {
        for plan in plans.iter_mut() {
            let Some(suggestion) = &plan.suggestion else {
                continue;
            };
            if !suggestion.spliceable {
                continue;
            }
            let Some(measured) = measure_reduction(plan, suggestion, source, is_module) else {
                continue;
            };
            plan.estimated_reduction = measured;
            plan.estimated_complexity_after = plan
                .current_complexity
                .saturating_sub(measured);
            plan.reduction_is_measured = true;
        }
    }

    fn collect_plans(
        &self,
        regions: &[ComplexityRegion],
        source: &str,
        function_complexity: u64,
        plans: &mut Vec<RefactorPlan>,
    ) {
        for region in regions {
            for rule in &self.rules {
                if let Some(plan) = rule.check(region, source, function_complexity) {
                    plans.push(plan);
                }
            }

            self.collect_plans(&region.children, source, function_complexity, plans);
        }
    }
}

impl Default for RuleRegistry {
    fn default() -> Self {
        Self::new()
    }
}

fn splice_plan(plan: &RefactorPlan, suggestion: &CodeSuggestion, source: &str) -> Option<String> {
    let start: usize = plan.line_start.saturating_sub(1) as usize;
    let end: usize = plan.line_end as usize;
    let lines: Vec<&str> = source.lines().collect();
    if start > end || end > lines.len() {
        return None;
    }
    let mut spliced = Vec::with_capacity(lines.len() + 1);
    spliced.extend(lines[..start].iter().copied());
    spliced.push(suggestion.replacement.as_str());
    spliced.extend(lines[end..].iter().copied());
    Some(spliced.join("\n"))
}

/// The target is `<module>` for script-mode plans (module-level regions
/// only reach the registry through the `is_module` call); otherwise it is
/// the function with the greatest `line_start` still before the plan's
/// region — the splice only shifts lines after the region, so the
/// containing function's start is unchanged between the two parses.
///
/// Returns `None` when the spliced source cannot be parsed or the target
/// cannot be located; the caller keeps the formula estimate then.
fn measure_reduction(
    plan: &RefactorPlan,
    suggestion: &CodeSuggestion,
    source: &str,
    is_module: bool,
) -> Option<u64> {
    let spliced = splice_plan(plan, suggestion, source)?;
    let parsed = parse_module(&spliced).ok()?;
    let (functions, _) = function_level_cognitive_complexity_shared(
        &parsed.into_suite(),
        &spliced,
        true,
        true,
        false,
    );

    let new_complexity = if is_module {
        functions.iter().find(|f| f.name == "<module>")?.complexity
    } else {
        let containing = functions
            .iter()
            .filter(|f| f.name != "<module>" && f.line_start <= plan.line_start)
            .max_by_key(|f| f.line_start)?;
        containing.complexity
    };

    Some(plan.current_complexity.saturating_sub(new_complexity))
}

/// Sorts by effectiveness/reduction/line, then keeps only non-overlapping
/// plans (highest effectiveness/reduction wins any overlap), capped at 5.
/// Split out from `RuleRegistry::analyze` so the selection logic can be unit
/// tested directly against hand-built plans, without needing real rules to
/// organically produce a given overlap shape.
fn select_non_overlapping(
    mut plans: Vec<RefactorPlan>,
    effectiveness: &HashMap<&str, u8>,
) -> (Vec<RefactorPlan>, u64) {
    let effectiveness_of = |rule_id: &str| *effectiveness.get(rule_id).unwrap_or(&1);

    plans.sort_by(|a, b| {
        let eff_a = effectiveness_of(&a.rule_id);
        let eff_b = effectiveness_of(&b.rule_id);

        eff_b
            .cmp(&eff_a)
            .then_with(|| b.estimated_reduction.cmp(&a.estimated_reduction))
            .then_with(|| a.line_start.cmp(&b.line_start))
    });

    let mut selected: Vec<RefactorPlan> = Vec::new();

    for plan in plans {
        let overlapping: Vec<usize> = selected
            .iter()
            .enumerate()
            .filter(|(_, existing)| {
                plan.line_start <= existing.line_end && plan.line_end >= existing.line_start
            })
            .map(|(idx, _)| idx)
            .collect();

        if overlapping.is_empty() {
            selected.push(plan);
            continue;
        }

        let eff_plan = effectiveness_of(&plan.rule_id);
        let beats_all_overlaps = overlapping.iter().all(|&idx| {
            let existing = &selected[idx];
            let eff_existing = effectiveness_of(&existing.rule_id);
            eff_plan > eff_existing
                || (eff_plan == eff_existing
                    && plan.estimated_reduction > existing.estimated_reduction)
        });

        if beats_all_overlaps {
            for &idx in overlapping.iter().rev() {
                selected.remove(idx);
            }
            selected.push(plan);
        }
    }

    let additional = selected.len().saturating_sub(5) as u64;
    selected.truncate(5);

    (selected, additional)
}

#[cfg(test)]
#[path = "../tests/rules/registry.rs"]
mod tests;
