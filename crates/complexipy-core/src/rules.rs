pub mod complexity;
pub mod registry;
pub mod types;

pub use registry::RuleRegistry;
pub use types::{AnalysisOptions, RuleSet};

use std::sync::OnceLock;

pub fn default_registry() -> &'static RuleRegistry {
    static REGISTRY: OnceLock<RuleRegistry> = OnceLock::new();
    REGISTRY.get_or_init(RuleRegistry::new)
}

pub fn registered_rule_ids() -> Vec<String> {
    default_registry().registered_ids()
}
