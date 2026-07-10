use super::types::RefactorRule;
use crate::classes::RefactorPlan;
use crate::refactor_plans::ComplexityRegion;

/// Returns a numeric effectiveness score for different refactor types.
/// Higher scores indicate refactors that produce better code quality.
///
/// Effectiveness tiers:
/// - 5: Condition merging (C007) — reduces number of conditions, best readability
/// - 4: Nesting flattening (C001, C006) — reduces indentation depth
/// - 3: Guard clauses (C002) — reduces nesting but adds negation/continue
/// - 2: Extraction (C003, C004, C005, C011) — moves complexity elsewhere
/// - 1: Default fallback
fn refactor_effectiveness(rule_id: &str) -> u8 {
    match rule_id {
        "C007" => 5, // Condition merging — highest effectiveness
        "C001" => 4, // Flatten condition — nesting reduction
        "C006" => 4, // Reduce nesting — nesting reduction
        "C002" => 3, // Loop guards — nesting reduction with negation
        "C003" => 2, // Extract helper — complexity reduction
        "C004" => 2, // Split dispatcher — complexity reduction
        "C005" => 2, // Extract predicate — readability
        "C011" => 2, // Flatten try — complexity reduction
        _ => 1,
    }
}

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
        self.register(Box::new(ReduceNestingRule));
        self.register(Box::new(FlattenTryRule));
        self.register(Box::new(CollapsibleIfRule));
    }

    pub fn register(&mut self, rule: Box<dyn RefactorRule>) {
        self.rules.push(rule);
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

        // Filter out low-impact plans: a reduction of 1 is noise — it doesn't meaningfully
        // improve readability or maintainability, so we only surface plans with reduction >= 2.
        plans.retain(|plan| plan.estimated_reduction >= 2);

        plans.sort_by(|a, b| {
            let eff_a = refactor_effectiveness(&a.rule_id);
            let eff_b = refactor_effectiveness(&b.rule_id);

            // 1. Higher effectiveness first (condition merging > nesting flattening > guard clauses > extraction)
            eff_b.cmp(&eff_a)
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
                    let eff_existing = refactor_effectiveness(&existing.rule_id);
                    let eff_plan = refactor_effectiveness(&plan.rule_id);

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
