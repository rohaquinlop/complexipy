pub use crate::classes::{LineComplexity, RefactorPlan};

use crate::rules::{RuleSet, default_registry, registry::PlanContext};
use crate::utils::LineIndex;

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

#[derive(Clone, Default)]
pub struct ComplexityRegion {
    pub kind: RegionKind,
    pub line_start: u64,
    pub line_end: u64,
    pub column_start: u64,
    pub structural: u64,
    pub nesting: u64,
    pub boolean: u64,
    pub total: u64,
    pub elif_count: u64,
    pub bool_op_count: u64,
    pub children: Vec<ComplexityRegion>,
}

pub struct ComplexityResult {
    pub complexity: u64,
    pub line_complexities: Vec<LineComplexity>,
    pub regions: Vec<ComplexityRegion>,
}

pub fn build_refactor_plans(
    function_complexity: u64,
    regions: &[ComplexityRegion],
    source: &str,
    index: &LineIndex,
    def_names: &std::collections::HashSet<String>,
    is_module: bool,
    active: &RuleSet,
) -> (Vec<RefactorPlan>, u64) {
    default_registry().analyze(
        regions,
        &PlanContext {
            source,
            index,
            def_names,
            function_complexity,
            is_module,
            active,
        },
    )
}
