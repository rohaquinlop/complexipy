pub use crate::classes::{LineComplexity, RefactorPlan};

#[cfg(any(feature = "python", feature = "wasm"))]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum RegionKind {
    If,
    ElifChain,
    Loop,
    Try,
    Match,
    BooleanCondition,
    With,
}

#[cfg(any(feature = "python", feature = "wasm"))]
#[derive(Clone)]
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
) -> Vec<RefactorPlan> {
    let mut plans = Vec::new();
    collect_refactor_plans(function_complexity, regions, &mut plans);
    plans.retain(|plan| plan.estimated_reduction >= 2);
    plans.sort_by(|a, b| {
        b.estimated_reduction
            .cmp(&a.estimated_reduction)
            .then_with(|| b.current_complexity.cmp(&a.current_complexity))
            .then_with(|| a.line_start.cmp(&b.line_start))
    });

    let mut selected: Vec<RefactorPlan> = Vec::new();
    for plan in plans {
        if plan.kind == "extract_helper"
            && selected.iter().any(|existing| {
                existing.kind == "extract_helper"
                    && existing.line_start <= plan.line_start
                    && existing.line_end >= plan.line_end
            })
        {
            continue;
        }
        if !selected.iter().any(|existing| {
            existing.kind == plan.kind
                && existing.line_start == plan.line_start
                && existing.line_end == plan.line_end
        }) {
            selected.push(plan);
        }
        if selected.len() == 3 {
            break;
        }
    }
    selected
}

#[cfg(any(feature = "python", feature = "wasm"))]
fn collect_refactor_plans(
    function_complexity: u64,
    regions: &[ComplexityRegion],
    plans: &mut Vec<RefactorPlan>,
) {
    for region in regions {
        plan_for_region(function_complexity, region, plans);
        collect_refactor_plans(function_complexity, &region.children, plans);
    }
}

#[cfg(any(feature = "python", feature = "wasm"))]
fn plan_for_region(
    function_complexity: u64,
    region: &ComplexityRegion,
    plans: &mut Vec<RefactorPlan>,
) {
    if region.kind == RegionKind::If && region.nesting >= 2 && region.total >= 4 {
        push_plan(
            plans,
            "flatten_condition",
            "Flatten nested condition block with guard clauses",
            region,
            region.nesting,
            function_complexity,
            &[
                "invert the outer condition",
                "return early when the condition fails",
                "move the main success path one indentation level left",
                "repeat for inner nested conditions where safe",
            ],
        );
    }

    if region.kind == RegionKind::Loop
        && region.total >= 5
        && region
            .children
            .iter()
            .any(|child| child.kind == RegionKind::If && child.nesting >= 1)
    {
        let reduction =
            sum_if_nesting(&region.children).min(region.total.saturating_sub(region.structural));
        push_plan(
            plans,
            "loop_guards",
            "Flatten loop body with continue guards",
            region,
            reduction,
            function_complexity,
            &[
                "add negative condition checks at the top of the loop",
                "use continue for skipped items",
                "keep the main processing path unindented",
            ],
        );
    }

    if region.total >= 6 && region.line_end.saturating_sub(region.line_start) + 1 >= 5 {
        push_plan(
            plans,
            "extract_helper",
            "Extract complex block into helper function",
            region,
            region.total.saturating_sub(1),
            function_complexity,
            &[
                "move the selected block into a private helper",
                "pass required values as parameters",
                "return the result/status needed by the caller",
            ],
        );
    }

    if (region.kind == RegionKind::ElifChain && region.elif_count >= 3)
        || (region.kind == RegionKind::Match && region.case_count >= 4)
    {
        let reduction = region
            .elif_count
            .max(region.case_count)
            .saturating_sub(1)
            .max(2);
        push_plan(
            plans,
            "split_dispatcher",
            "Split conditional dispatcher into handlers",
            region,
            reduction,
            function_complexity,
            &[
                "create one handler per case",
                "replace the chain with a dispatch table or small delegating match",
                "keep the orchestration function shallow",
            ],
        );
    }

    if region.kind == RegionKind::BooleanCondition && region.boolean >= 2 {
        push_plan(
            plans,
            "extract_predicate",
            "Extract complex condition into named predicate",
            region,
            region.bool_op_count,
            function_complexity,
            &[
                "move the condition into a well-named helper function",
                "use the helper in the branch condition",
                "keep the caller focused on control flow",
            ],
        );
    }
}

#[cfg(any(feature = "python", feature = "wasm"))]
fn sum_if_nesting(regions: &[ComplexityRegion]) -> u64 {
    regions
        .iter()
        .map(|region| {
            let own = if region.kind == RegionKind::If {
                region.nesting
            } else {
                0
            };
            own + sum_if_nesting(&region.children)
        })
        .sum()
}

#[cfg(any(feature = "python", feature = "wasm"))]
fn push_plan(
    plans: &mut Vec<RefactorPlan>,
    kind: &str,
    title: &str,
    region: &ComplexityRegion,
    estimated_reduction: u64,
    function_complexity: u64,
    steps: &[&str],
) {
    if estimated_reduction == 0 {
        return;
    }
    plans.push(RefactorPlan {
        kind: kind.to_string(),
        title: title.to_string(),
        line_start: region.line_start,
        line_end: region.line_end,
        current_complexity: function_complexity,
        estimated_reduction,
        estimated_complexity_after: function_complexity.saturating_sub(estimated_reduction),
        steps: steps.iter().map(|step| (*step).to_string()).collect(),
    });
}
