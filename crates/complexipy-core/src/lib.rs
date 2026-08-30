#[cfg(feature = "runner")]
pub mod api;
pub mod classes;
pub mod cognitive_complexity;
pub mod diff;
#[cfg(feature = "runner")]
pub(crate) mod helpers;
mod refactor_plans;
mod rules;
#[cfg(feature = "runner")]
pub mod runner;
pub mod utils;

#[cfg(feature = "runner")]
pub use api::{code_complexity, file_complexity};
/// Stable public API - mirrors `complexipy/__init__.py`'s `__all__`.
/// Names here are a compatibility promise; do not move or rename them
/// without a major release.
pub use classes::{
    Applicability, CodeComplexity, CodeSuggestion, FileComplexity, FunctionComplexity,
    IgnoredLocation, LineComplexity, RefactorPlan, RemovableIgnore, RuleCategory,
};
pub use diff::{DiffEntry, DiffStatus, compute_diff, compute_staged_diff, has_regressions};
#[cfg(feature = "runner")]
pub use runner::{
    collect_all_ignored_locations_shared as collect_all_ignored_locations,
    collect_removable_ignored_locations_shared as collect_removable_ignored_locations,
    run_analysis_shared,
};
