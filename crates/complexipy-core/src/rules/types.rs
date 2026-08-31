use serde::{Deserialize, Serialize};

pub use crate::classes::{Applicability, RuleCategory};

use crate::classes::RefactorPlan;
use crate::refactor_plans::ComplexityRegion;
use crate::utils::LineIndex;
use std::collections::HashSet;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RuleMetadata {
    pub id: String,
    pub name: String,
    pub category: RuleCategory,
    pub description: String,
    pub applicability: Applicability,
    /// Ranking used to pick which refactor to surface when two rules fire on
    /// overlapping regions, and to order the plans that survive. Higher wins.
    ///
    /// Effectiveness tiers:
    /// - 5: Condition merging (C007) -- reduces number of conditions, best readability
    /// - 4: Nesting flattening (C001) -- reduces indentation depth
    /// - 3: Guard clauses (C002) -- reduces nesting but adds negation/continue
    /// - 2: Extraction (C003, C004, C005, C011) -- moves complexity elsewhere
    /// - 1: Default fallback
    pub effectiveness: u8,
    pub doc_url: String,
}

impl RuleMetadata {
    /// A `RefactorPlan` prefilled from this metadata, with the per-plan
    /// dynamic fields (title, line range, complexity numbers, explanation,
    /// suggestion, help) left at their defaults. Callers build the real plan
    /// with `..self.metadata().new_plan()` so the id/name/category/
    /// description/applicability/doc_url can only ever come from one place.
    pub fn new_plan(&self) -> RefactorPlan {
        RefactorPlan {
            kind: self.name.clone(),
            title: String::new(),
            line_start: 0,
            line_end: 0,
            column_start: 0,
            current_complexity: 0,
            estimated_reduction: 0,
            estimated_complexity_after: 0,
            reduction_is_measured: false,
            rule_id: self.id.clone(),
            category: self.category.clone(),
            applicability: self.applicability.clone(),
            description: self.description.clone(),
            explanation: String::new(),
            references: vec![],
            suggestion: None,
            help: None,
            doc_url: self.doc_url.clone(),
        }
    }
}

pub trait RefactorRule: Sync + Send {
    fn metadata(&self) -> &'static RuleMetadata;

    fn check(
        &self,
        region: &ComplexityRegion,
        source: &str,
        index: &LineIndex,
        def_names: &HashSet<String>,
        function_complexity: u64,
    ) -> Option<crate::classes::RefactorPlan>;
}
