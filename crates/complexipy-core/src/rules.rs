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

pub fn unknown_rule_warning(rule_id: &str) -> String {
    if rule_id.is_empty() {
        "Empty rule id in select or ignore; it matches no rule.".to_string()
    } else {
        format!(
            "Unknown rule id '{}'; it matches no registered rule.",
            rule_id
        )
    }
}

pub fn no_active_rule_warning() -> String {
    "No refactoring rule is active after select and ignore.".to_string()
}
