use super::types::RefactorRule;
use crate::classes::RefactorPlan;
use crate::refactor_plans::ComplexityRegion;

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

        plans.retain(|plan| plan.estimated_reduction >= 2);

        plans.sort_by(|a, b| {
            let priority_a = self.find_rule_priority(&a.rule_id).unwrap_or(0);
            let priority_b = self.find_rule_priority(&b.rule_id).unwrap_or(0);
            priority_b
                .cmp(&priority_a)
                .then_with(|| b.estimated_reduction.cmp(&a.estimated_reduction))
                .then_with(|| a.line_start.cmp(&b.line_start))
        });

        let mut selected: Vec<RefactorPlan> = Vec::new();
        for plan in plans {
            if plan.kind == "extract_helper"
                && selected.iter().any(|existing| {
                    existing.kind == "extract_helper"
                        && existing.line_start <= plan.line_start
                        && existing.line_end >= plan.line_end
                })
            {
                continue;
            }

            if !selected.iter().any(|existing| {
                existing.rule_id == plan.rule_id
                    && existing.line_start == plan.line_start
                    && existing.line_end == plan.line_end
            }) {
                selected.push(plan);
            }

            if selected.len() == 5 {
                break;
            }
        }

        selected
    }

    fn find_rule_priority(&self, rule_id: &str) -> Option<u8> {
        self.rules
            .iter()
            .find(|r| r.metadata().id == rule_id)
            .map(|r| r.metadata().priority)
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
