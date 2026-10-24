use super::types::{AnalysisOptions, RefactorRule, RuleSet};
use crate::classes::{CodeSuggestion, RefactorPlan};
use crate::cognitive_complexity::function_level_cognitive_complexity_shared;
use crate::fix::{splice_plan, splice_window};
use crate::refactor_plans::ComplexityRegion;
use crate::syntax_tree::parse_module;
use crate::utils::LineIndex;
use ruff_python_ast::Stmt;
use std::collections::{BTreeMap, HashMap};

/// The lines of the function being planned, and whether it is a method,
/// whose indented text only parses inside a class body.
#[derive(Clone, Copy)]
pub struct FunctionSpan {
    pub line_start: u64,
    pub line_end: u64,
    pub is_method: bool,
}

/// Everything a rule needs about the function or module being planned.
/// `function` is `None` for the module-level plans of `--check-script`, and
/// `newline` is the file's line break.
pub struct PlanContext<'a> {
    pub source: &'a str,
    pub index: &'a LineIndex,
    pub def_names: &'a std::collections::HashSet<String>,
    pub function_complexity: u64,
    pub is_module: bool,
    pub active: &'a RuleSet,
    pub function: Option<FunctionSpan>,
    pub newline: &'a str,
}

pub struct RuleRegistry {
    rules: Vec<Box<dyn RefactorRule>>,
}

impl RuleRegistry {
    pub fn new() -> Self {
        let mut registry = Self { rules: Vec::new() };
        registry.register_defaults();
        registry
    }

    fn register_defaults(&mut self) {
        use super::complexity::*;

        self.register(Box::new(FlattenConditionRule));
        self.register(Box::new(LoopGuardsRule));
        self.register(Box::new(ExtractHelperRule));
        self.register(Box::new(SplitDispatcherRule));
        self.register(Box::new(ExtractPredicateRule));
        self.register(Box::new(FlattenTryRule));
        self.register(Box::new(CollapsibleIfRule));
    }

    pub fn register(&mut self, rule: Box<dyn RefactorRule>) {
        self.rules.push(rule);
    }

    pub fn registered_ids(&self) -> Vec<String> {
        self.rules
            .iter()
            .map(|rule| rule.metadata().id.clone())
            .collect()
    }

    /// Build a `rule_id -> effectiveness` lookup from the currently registered
    /// rules. `RefactorPlan`s are plain data decoupled from the `RefactorRule`
    /// objects that produced them (see `collect_plans`), so ordering can't read
    /// effectiveness off the rule directly once we're holding a `Vec<RefactorPlan>` --
    /// this map is what lets us look it up by `rule_id` without hardcoding a
    /// `match rule_id` here. Adding a 9th rule only requires registering it in
    /// `register_defaults` and giving it an `effectiveness` in its own metadata;
    /// nothing in this file needs to change.
    fn effectiveness_by_rule_id(&self) -> HashMap<&str, u8> {
        self.rules
            .iter()
            .map(|rule| {
                let meta = rule.metadata();
                (meta.id.as_str(), meta.effectiveness)
            })
            .collect()
    }

    /// Returns the selected plans (capped at 5) plus a count of additional
    /// plans that survived dedup but were not emitted, so callers can render
    /// "... and N more suggestions" instead of silently truncating. The count
    /// covers both cap drops and selected plans whose measured reduction
    /// fell below 1 after measurement.
    #[must_use]
    pub fn analyze(
        &self,
        regions: &[ComplexityRegion],
        ctx: &PlanContext<'_>,
    ) -> (Vec<RefactorPlan>, u64) {
        let mut plans = Vec::new();

        self.collect_plans(regions, ctx, &mut plans);
        plans.retain(|plan| plan.estimated_reduction >= 1);

        let effectiveness = self.effectiveness_by_rule_id();
        let (mut selected, cap_dropped) = select_non_overlapping(plans, &effectiveness);

        let measured_before = selected.len();
        self.measure_plans(&mut selected, ctx);
        selected.retain(|plan| plan.estimated_reduction >= 1);
        let measurement_dropped = measured_before.saturating_sub(selected.len());

        (selected, cap_dropped + measurement_dropped as u64)
    }

    /// Plans whose measurement fails keep their formula estimate with
    /// `reduction_is_measured = false` - never a panic, never a fabricated
    /// measured number.
    fn measure_plans(&self, plans: &mut [RefactorPlan], ctx: &PlanContext<'_>) {
        for plan in plans.iter_mut() {
            let Some(suggestion) = &plan.suggestion else {
                continue;
            };
            if !suggestion.spliceable {
                continue;
            }
            let Some(measured) = measure_reduction(plan, suggestion, ctx) else {
                continue;
            };
            plan.estimated_reduction = measured;
            plan.estimated_complexity_after = plan.current_complexity.saturating_sub(measured);
            plan.reduction_is_measured = true;
        }
    }

    fn collect_plans(
        &self,
        regions: &[ComplexityRegion],
        ctx: &PlanContext<'_>,
        plans: &mut Vec<RefactorPlan>,
    ) {
        for region in regions {
            for rule in &self.rules {
                if !ctx.active.is_active(&rule.metadata().id) {
                    continue;
                }
                if let Some(plan) = rule.check(
                    region,
                    ctx.source,
                    ctx.index,
                    ctx.def_names,
                    ctx.function_complexity,
                ) {
                    plans.push(plan);
                }
            }

            self.collect_plans(&region.children, ctx, plans);
        }
    }
}

impl Default for RuleRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Measures a plan by splicing its suggestion into the source and scoring
/// the result. A function's plan is measured on that function's text alone,
/// so the cost follows the size of the function rather than of the file.
/// When the spliced text no longer parses as exactly that one function, the
/// whole file is measured instead, so both paths always report the number a
/// whole-file measurement would.
///
/// Returns `None` when the spliced source cannot be parsed or the target
/// cannot be located; the caller keeps the formula estimate then.
fn measure_reduction(
    plan: &RefactorPlan,
    suggestion: &CodeSuggestion,
    ctx: &PlanContext<'_>,
) -> Option<u64> {
    let in_function = match ctx.function {
        Some(function) if !ctx.is_module => measure_in_function(plan, suggestion, ctx, function),
        _ => None,
    };
    in_function.or_else(|| measure_in_file(plan, suggestion, ctx))
}

fn measure_in_function(
    plan: &RefactorPlan,
    suggestion: &CodeSuggestion,
    ctx: &PlanContext<'_>,
    function: FunctionSpan,
) -> Option<u64> {
    let start = ctx.index.byte_of_line(function.line_start)?;
    let end = ctx
        .index
        .byte_of_line(function.line_end + 1)
        .unwrap_or(ctx.source.len());
    let spliced = splice_window(
        plan,
        suggestion,
        ctx.source,
        ctx.index,
        ctx.newline,
        (start, end),
    )?;
    let snippet = if function.is_method {
        format!("class _:{}{}", ctx.newline, spliced)
    } else {
        spliced
    };
    let parsed = parse_module(&snippet).ok()?;
    let suite = parsed.suite();
    let only_the_function = match suite.as_slice() {
        [Stmt::FunctionDef(_)] => !function.is_method,
        [Stmt::ClassDef(class)] => {
            function.is_method && matches!(class.body.as_slice(), [Stmt::FunctionDef(_)])
        }
        _ => false,
    };
    if !only_the_function {
        return None;
    }
    let (functions, _) = function_level_cognitive_complexity_shared(
        suite,
        &snippet,
        &AnalysisOptions {
            no_ignore: true,
            ..Default::default()
        },
    );
    let [measured] = functions.as_slice() else {
        return None;
    };
    Some(plan.current_complexity.saturating_sub(measured.complexity))
}

/// The target is `<module>` for script-mode plans (module-level regions
/// only reach the registry through the `is_module` call); otherwise it is
/// the function with the greatest `line_start` still before the plan's
/// region - the splice only shifts lines after the region, so the
/// containing function's start is unchanged between the two parses.
fn measure_in_file(
    plan: &RefactorPlan,
    suggestion: &CodeSuggestion,
    ctx: &PlanContext<'_>,
) -> Option<u64> {
    let spliced = splice_plan(plan, suggestion, ctx.source, ctx.index, ctx.newline)?;
    let parsed = parse_module(&spliced).ok()?;
    let (functions, _) = function_level_cognitive_complexity_shared(
        parsed.suite(),
        &spliced,
        &AnalysisOptions {
            check_script: true,
            no_ignore: true,
            ..Default::default()
        },
    );

    let new_complexity = if ctx.is_module {
        functions.iter().find(|f| f.name == "<module>")?.complexity
    } else {
        let containing = functions
            .iter()
            .filter(|f| f.name != "<module>" && f.line_start <= plan.line_start)
            .max_by_key(|f| f.line_start)?;
        containing.complexity
    };

    Some(plan.current_complexity.saturating_sub(new_complexity))
}

/// Sorts by spliceable/effectiveness/reduction/line, then keeps each plan
/// that overlaps no plan kept before it, so the highest
/// spliceable/effectiveness/reduction wins any overlap, capped at 5. A
/// spliceable plan beats a help-only plan of higher
/// effectiveness: the tool stands behind a faithful, measured replacement
/// more than behind prose. Split out from `RuleRegistry::analyze` so the
/// selection logic can be unit tested directly against hand-built plans,
/// without needing real rules to organically produce a given overlap shape.
fn select_non_overlapping(
    mut plans: Vec<RefactorPlan>,
    effectiveness: &HashMap<&str, u8>,
) -> (Vec<RefactorPlan>, u64) {
    let effectiveness_of = |rule_id: &str| *effectiveness.get(rule_id).unwrap_or(&1);
    let spliceable_of =
        |plan: &RefactorPlan| plan.suggestion.as_ref().is_some_and(|s| s.spliceable);

    plans.sort_by(|a, b| {
        let spliceable_a = spliceable_of(a);
        let spliceable_b = spliceable_of(b);
        let eff_a = effectiveness_of(&a.rule_id);
        let eff_b = effectiveness_of(&b.rule_id);

        spliceable_b
            .cmp(&spliceable_a)
            .then_with(|| eff_b.cmp(&eff_a))
            .then_with(|| b.estimated_reduction.cmp(&a.estimated_reduction))
            .then_with(|| a.line_start.cmp(&b.line_start))
    });

    let mut taken: BTreeMap<u64, u64> = BTreeMap::new();
    let mut selected: Vec<RefactorPlan> = Vec::new();

    for plan in plans {
        let overlaps = taken
            .range(..=plan.line_end)
            .next_back()
            .is_some_and(|(_, &line_end)| line_end >= plan.line_start);
        if !overlaps {
            taken.insert(plan.line_start, plan.line_end);
            selected.push(plan);
        }
    }

    let additional = selected.len().saturating_sub(5) as u64;
    selected.truncate(5);

    (selected, additional)
}

#[cfg(test)]
mod tests;
