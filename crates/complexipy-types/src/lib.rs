use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub enum RuleCategory {
    Complexity,
    Readability,
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
