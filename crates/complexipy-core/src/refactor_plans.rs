pub use crate::classes::{LineComplexity, RefactorPlan};

use crate::rules::RuleRegistry;
use crate::utils::LineIndex;
use std::sync::OnceLock;

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
) -> (Vec<RefactorPlan>, u64) {
    static REGISTRY: OnceLock<RuleRegistry> = OnceLock::new();
    let registry = REGISTRY.get_or_init(RuleRegistry::new);
    registry.analyze(
        regions,
        source,
        index,
        def_names,
        function_complexity,
        is_module,
    )
}
