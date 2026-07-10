use serde::{Deserialize, Serialize};

pub use crate::classes::{Applicability, RuleCategory};

use crate::refactor_plans::ComplexityRegion;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RuleMetadata {
    pub id: String,
    pub name: String,
    pub category: RuleCategory,
    pub description: String,
    pub applicability: Applicability,
    pub priority: u8,
    pub doc_url: String,
}

pub trait RefactorRule: Sync + Send {
    fn metadata(&self) -> &'static RuleMetadata;

    fn check(
        &self,
        region: &ComplexityRegion,
        source: &str,
        function_complexity: u64,
    ) -> Option<crate::classes::RefactorPlan>;
}
