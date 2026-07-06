pub use crate::classes::{LineComplexity, RefactorPlan};

#[cfg(any(feature = "python", feature = "wasm"))]
use crate::rules::RuleRegistry;

#[cfg(any(feature = "python", feature = "wasm"))]
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum RegionKind {
    #[default]
    If,
    ElifChain,
    Loop,
    Try,
    Match,
    BooleanCondition,
    With,
}

#[cfg(any(feature = "python", feature = "wasm"))]
#[derive(Clone, Default)]
pub struct ComplexityRegion {
    pub kind: RegionKind,
    pub line_start: u64,
    pub line_end: u64,
    pub structural: u64,
    pub nesting: u64,
    pub boolean: u64,
    pub total: u64,
    pub elif_count: u64,
    pub case_count: u64,
    pub bool_op_count: u64,
    pub children: Vec<ComplexityRegion>,
}

#[cfg(any(feature = "python", feature = "wasm"))]
pub struct ComplexityResult {
    pub complexity: u64,
    pub line_complexities: Vec<LineComplexity>,
    pub regions: Vec<ComplexityRegion>,
}

#[cfg(any(feature = "python", feature = "wasm"))]
pub fn build_refactor_plans(
    function_complexity: u64,
    regions: &[ComplexityRegion],
    source: &str,
) -> Vec<RefactorPlan> {
    let registry = RuleRegistry::new();
    registry.analyze(regions, source, function_complexity)
}
