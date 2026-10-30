use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub enum RuleCategory {
    Complexity,
    Readability,
}

impl RuleCategory {
    pub const ALL: [(&str, RuleCategory); 2] = [
        ("Complexity", RuleCategory::Complexity),
        ("Readability", RuleCategory::Readability),
    ];

    #[must_use]
    pub fn name(&self) -> &'static str {
        match self {
            Self::Complexity => "Complexity",
            Self::Readability => "Readability",
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub enum Applicability {
    MachineApplicable,
    MaybeIncorrect,
    Informational,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DiffStatus {
    Regressed,
    Improved,
    Unchanged,
    New,
    Removed,
}

#[cfg(feature = "python")]
mod python;

#[cfg(feature = "python")]
pub use python::{applicability_class, diff_status_class, rule_category_class};
