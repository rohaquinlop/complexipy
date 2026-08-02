use super::types::RefactorRule;
use crate::classes::RefactorPlan;
use crate::refactor_plans::ComplexityRegion;
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

    #[must_use]
    pub fn analyze(
        &self,
        regions: &[ComplexityRegion],
        source: &str,
        function_complexity: u64,
    ) -> Vec<RefactorPlan> {
        let mut plans = Vec::new();

        self.collect_plans(regions, source, function_complexity, &mut plans);

        plans.retain(|plan| plan.estimated_reduction >= 1);

        let effectiveness = self.effectiveness_by_rule_id();
        let effectiveness_of = |rule_id: &str| *effectiveness.get(rule_id).unwrap_or(&1);

        plans.sort_by(|a, b| {
            let eff_a = effectiveness_of(&a.rule_id);
            let eff_b = effectiveness_of(&b.rule_id);

            // 1. Higher effectiveness first (condition merging > nesting flattening > guard clauses > extraction)
            eff_b
                .cmp(&eff_a)
                // 2. Higher reduction within same effectiveness tier
                .then_with(|| b.estimated_reduction.cmp(&a.estimated_reduction))
                // 3. Earlier line number for same effectiveness and reduction
                .then_with(|| a.line_start.cmp(&b.line_start))
        });

        let mut selected: Vec<RefactorPlan> = Vec::new();

        for plan in plans {
            // Find any overlapping plan in selected
            let overlapping_idx = selected.iter().position(|existing| {
                plan.line_start <= existing.line_end && plan.line_end >= existing.line_start
            });

            match overlapping_idx {
                Some(idx) => {
                    let existing = &selected[idx];
                    let eff_existing = effectiveness_of(&existing.rule_id);
                    let eff_plan = effectiveness_of(&plan.rule_id);

                    // Keep the one with higher effectiveness, then higher reduction
                    if eff_plan > eff_existing
                        || (eff_plan == eff_existing
                            && plan.estimated_reduction > existing.estimated_reduction)
                    {
                        selected[idx] = plan;
                    }
                    // If equal effectiveness and reduction, keep existing (first wins due to sort order)
                }
                None => {
                    // No overlap — add to selected
                    selected.push(plan);
                }
            }

            // Cap at 5 plans per function
            if selected.len() == 5 {
                break;
            }
        }

        selected
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

#[cfg(test)]
#[path = "../tests/rules/registry.rs"]
mod tests;
