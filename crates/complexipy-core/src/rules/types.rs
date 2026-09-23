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

/// The rules that may produce plans. `allowed` is `None` when every
/// registered rule is active, and `denied` always subtracts. Selection is
/// data-driven: nothing matches on a `rule_id` literal.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RuleSet {
    allowed: Option<HashSet<String>>,
    denied: HashSet<String>,
}

impl RuleSet {
    /// Builds the set from a select list and an ignore list, and returns the
    /// ids that no registered rule owns so the caller can warn about them.
    /// An empty select list means every registered rule is active; a select
    /// list with no valid id selects nothing.
    #[must_use]
    pub fn resolve(
        select: &[String],
        ignore: &[String],
        known_ids: &[String],
    ) -> (Self, Vec<String>) {
        let known: HashSet<String> = known_ids.iter().map(|id| id.to_ascii_uppercase()).collect();
        let select_ids = normalize_ids(select);
        let ignore_ids = normalize_ids(ignore);
        let mut unknown: Vec<String> = select
            .iter()
            .chain(ignore.iter())
            .map(|id| id.trim().to_ascii_uppercase())
            .filter(|id| !known.contains(id))
            .collect();
        unknown.sort();
        unknown.dedup();

        (
            Self {
                allowed: (!select.is_empty()).then(|| select_ids.into_iter().collect()),
                denied: ignore_ids.into_iter().collect(),
            },
            unknown,
        )
    }

    /// Returns the set with the given rule ids subtracted, for one function.
    #[must_use]
    pub fn without(&self, rules: &[String]) -> Self {
        let mut denied = self.denied.clone();
        denied.extend(normalize_ids(rules));
        Self {
            allowed: self.allowed.clone(),
            denied,
        }
    }

    #[must_use]
    pub fn is_active(&self, rule_id: &str) -> bool {
        if rule_id
            .bytes()
            .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit())
        {
            return self.contains(rule_id);
        }
        self.contains(&rule_id.to_ascii_uppercase())
    }

    fn contains(&self, rule_id: &str) -> bool {
        self.allowed
            .as_ref()
            .is_none_or(|allowed| allowed.contains(rule_id))
            && !self.denied.contains(rule_id)
    }
}

fn normalize_ids(ids: &[String]) -> Vec<String> {
    ids.iter()
        .map(|id| id.trim().to_ascii_uppercase())
        .filter(|id| !id.is_empty())
        .collect()
}

/// The knobs that shape one analysis run: script mode, inline suppression,
/// plan building, and the active rule set.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AnalysisOptions {
    pub check_script: bool,
    pub no_ignore: bool,
    pub with_plans: bool,
    pub rules: RuleSet,
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

#[cfg(test)]
mod tests;
