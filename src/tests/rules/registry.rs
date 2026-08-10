//! Unit tests for `crate::rules::registry`.
//!
//! Wired in from `src/rules/registry.rs` via `#[cfg(test)] #[path = ...] mod tests;`
//! so this stays a child module of the code it tests and can reach the private
//! `RuleRegistry.rules` field through `super::` without widening its visibility.

use super::{RuleRegistry, measure_reduction, select_non_overlapping, splice_plan};
use crate::classes::{Applicability, CodeSuggestion, RefactorPlan, RuleCategory};
use crate::cognitive_complexity::function_level_cognitive_complexity_shared;
use crate::refactor_plans::{ComplexityRegion, RegionKind};
use ruff_python_parser::parse_module;
use std::collections::HashMap;

fn plan(rule_id: &str, line_start: u64, line_end: u64, estimated_reduction: u64) -> RefactorPlan {
    RefactorPlan {
        kind: rule_id.to_string(),
        title: String::new(),
        line_start,
        line_end,
        column_start: 1,
        current_complexity: 10,
        estimated_reduction,
        estimated_complexity_after: 10u64.saturating_sub(estimated_reduction),
        reduction_is_measured: false,
        rule_id: rule_id.to_string(),
        category: RuleCategory::Complexity,
        applicability: Applicability::Informational,
        description: String::new(),
        explanation: String::new(),
        references: vec![],
        suggestion: None,
        help: None,
        doc_url: String::new(),
    }
}

fn plans_overlap(a: &RefactorPlan, b: &RefactorPlan) -> bool {
    a.line_start <= b.line_end && a.line_end >= b.line_start
}

/// The core soundness property of overlap dedup: whatever survives, no two
/// selected plans may overlap each other. This is a regression test for a
/// prior version that only checked the *first* overlapping already-selected
/// plan (via `Vec::position`) instead of every one a candidate might touch.
#[test]
fn select_non_overlapping_never_returns_overlapping_plans() {
    let candidates = vec![
        plan("A", 1, 10, 3),
        plan("B", 5, 15, 3),
        plan("C", 20, 30, 3),
        plan("D", 25, 35, 2),
        plan("E", 1, 100, 1),
    ];
    let effectiveness: HashMap<&str, u8> = [("A", 2), ("B", 2), ("C", 2), ("D", 2), ("E", 5)]
        .into_iter()
        .collect();

    let (selected, _) = select_non_overlapping(candidates, &effectiveness);

    for i in 0..selected.len() {
        for j in (i + 1)..selected.len() {
            assert!(
                !plans_overlap(&selected[i], &selected[j]),
                "{} and {} overlap in the final selection",
                selected[i].rule_id,
                selected[j].rule_id
            );
        }
    }
}

#[test]
fn select_non_overlapping_caps_at_five_and_reports_the_rest() {
    let ids = ["R0", "R1", "R2", "R3", "R4", "R5", "R6"];
    let candidates: Vec<RefactorPlan> = ids
        .iter()
        .enumerate()
        .map(|(i, id)| plan(id, (i * 10 + 1) as u64, (i * 10 + 5) as u64, 1))
        .collect();
    let effectiveness: HashMap<&str, u8> = ids.iter().map(|&id| (id, 2)).collect();

    let (selected, additional) = select_non_overlapping(candidates, &effectiveness);

    assert_eq!(selected.len(), 5);
    assert_eq!(additional, 2);
}

/// Build the minimal `(region, source)` pair that makes a given rule's
/// `check()` return `Some(..)`. Each fixture is deliberately as small as
/// possible -- just enough to clear that rule's own guard conditions -- so a
/// failure here points straight at either the fixture or the rule, not at
/// some unrelated interaction.
fn fixture_for(rule_id: &str) -> (ComplexityRegion, String) {
    match rule_id {
        "C001" => (
            ComplexityRegion {
                kind: RegionKind::If,
                line_start: 1,
                line_end: 4,
                nesting: 2,
                total: 4,
                ..Default::default()
            },
            "if a:\n    if b:\n        pass\n".to_string(),
        ),
        "C002" => (
            ComplexityRegion {
                kind: RegionKind::Loop,
                line_start: 1,
                line_end: 3,
                total: 5,
                children: vec![ComplexityRegion {
                    kind: RegionKind::If,
                    line_start: 2,
                    line_end: 3,
                    nesting: 1,
                    children: vec![ComplexityRegion {
                        kind: RegionKind::If,
                        line_start: 3,
                        line_end: 3,
                        nesting: 2,
                        ..Default::default()
                    }],
                    ..Default::default()
                }],
                ..Default::default()
            },
            "for x in y:\n    if a:\n        if b:\n            pass\n".to_string(),
        ),
        "C003" => (
            ComplexityRegion {
                kind: RegionKind::If,
                line_start: 1,
                line_end: 5,
                total: 6,
                ..Default::default()
            },
            "if a:\n    pass\n".to_string(),
        ),
        "C004" => (
            ComplexityRegion {
                kind: RegionKind::ElifChain,
                line_start: 1,
                line_end: 3,
                elif_count: 3,
                ..Default::default()
            },
            "if a:\n    pass\n".to_string(),
        ),
        "C005" => (
            ComplexityRegion {
                kind: RegionKind::BooleanCondition,
                line_start: 1,
                line_end: 1,
                boolean: 2,
                bool_op_count: 1,
                ..Default::default()
            },
            "if a and b:\n    pass\n".to_string(),
        ),
        "C011" => (
            ComplexityRegion {
                kind: RegionKind::Try,
                line_start: 1,
                line_end: 4,
                total: 4,
                children: vec![ComplexityRegion {
                    kind: RegionKind::Try,
                    ..Default::default()
                }],
                ..Default::default()
            },
            "try:\n    try:\n        pass\n    except Exception:\n        pass\nexcept Exception:\n    pass\n".to_string(),
        ),
        "C007" => (
            ComplexityRegion {
                kind: RegionKind::If,
                line_start: 1,
                line_end: 3,
                total: 5,
                nesting: 0,
                children: vec![ComplexityRegion {
                    kind: RegionKind::If,
                    line_start: 2,
                    line_end: 3,
                    ..Default::default()
                }],
                ..Default::default()
            },
            "if a:\n    if b:\n        pass\n".to_string(),
        ),
        other => panic!("no fixture registered for rule {other} -- add one in fixture_for()"),
    }
}

/// For every rule registered in `RuleRegistry::register_defaults`, build a
/// fixture that triggers it, call `check()` directly, and assert every field
/// the plan takes from `metadata()` actually matches `metadata()` -- proving
/// there is exactly one source of truth for id/name/category/description/
/// applicability/doc_url instead of a second, hand-copied literal per rule.
#[test]
fn every_registered_rule_produces_a_plan_consistent_with_its_own_metadata() {
    let registry = RuleRegistry::new();
    let mut checked = 0;

    for rule in &registry.rules {
        let meta = rule.metadata();
        let (region, source) = fixture_for(&meta.id);

        let plan = rule.check(&region, &source, 10).unwrap_or_else(|| {
            panic!(
                "rule {} did not produce a plan for its own fixture",
                meta.id
            )
        });

        assert_eq!(plan.rule_id, meta.id, "rule_id mismatch for {}", meta.id);
        assert_eq!(plan.kind, meta.name, "kind/name mismatch for {}", meta.id);
        assert_eq!(
            plan.category, meta.category,
            "category mismatch for {}",
            meta.id
        );
        assert_eq!(
            plan.applicability, meta.applicability,
            "applicability mismatch for {}",
            meta.id
        );
        assert_eq!(
            plan.description, meta.description,
            "description mismatch for {}",
            meta.id
        );
        assert_eq!(
            plan.doc_url, meta.doc_url,
            "doc_url mismatch for {}",
            meta.id
        );
        // Equality above passes when *both* sides are empty, which is the bug
        // this whole layer exists to prevent: a rule that omits `doc_url`
        // renders no `References:` link at all, silently. Require a real URL.
        assert!(
            plan.doc_url.starts_with("https://"),
            "rule {} has no usable doc_url (got {:?})",
            meta.id,
            plan.doc_url
        );

        checked += 1;
    }

    // Every rule in `register_defaults` must be exercised above -- if a 9th
    // rule is added without a fixture, `fixture_for` panics before this line
    // is ever reached, so this count is a floor, not a ceiling.
    assert_eq!(
        checked, 7,
        "expected all 7 registered rules to be exercised by a fixture"
    );
}

/// The ranking used for ordering/overlap-resolution is read live off each
/// rule's `metadata()` rather than a hardcoded `match rule_id`. Prove that by
/// checking the values registry.rs is documented to preserve (the old,
/// previously-dead-code `refactor_effectiveness()` values).
#[test]
fn effectiveness_matches_documented_tiers() {
    let registry = RuleRegistry::new();
    let expected: &[(&str, u8)] = &[
        ("C001", 4),
        ("C002", 3),
        ("C003", 2),
        ("C004", 2),
        ("C005", 2),
        ("C007", 5),
        ("C011", 2),
    ];

    for (rule_id, effectiveness) in expected {
        let found = registry
            .rules
            .iter()
            .map(|r| r.metadata())
            .find(|meta| meta.id == *rule_id)
            .unwrap_or_else(|| panic!("rule {rule_id} is not registered"));
        assert_eq!(
            found.effectiveness, *effectiveness,
            "effectiveness mismatch for {rule_id}"
        );
    }
}

fn module_complexity(source: &str) -> u64 {
    let parsed = parse_module(source).unwrap();
    let (functions, _) =
        function_level_cognitive_complexity_shared(&parsed.into_suite(), source, true, true, false);
    functions
        .iter()
        .find(|f| f.name == "<module>")
        .unwrap()
        .complexity
}

/// A loop whose first if has two children (one plain statement, one nested
/// if): C007 cannot collapse it (multi-child chain), so C002 is the only
/// rule that fires.
fn loop_guard_regions() -> (Vec<ComplexityRegion>, String) {
    let source =
        "for x in y:\n    if a:\n        total += x\n        if b:\n            pass\n".to_string();
    let regions = vec![ComplexityRegion {
        kind: RegionKind::Loop,
        line_start: 1,
        line_end: 5,
        total: 6,
        children: vec![ComplexityRegion {
            kind: RegionKind::If,
            line_start: 2,
            line_end: 5,
            nesting: 1,
            total: 5,
            children: vec![
                ComplexityRegion {
                    kind: RegionKind::If,
                    line_start: 3,
                    line_end: 3,
                    ..Default::default()
                },
                ComplexityRegion {
                    kind: RegionKind::If,
                    line_start: 4,
                    line_end: 5,
                    nesting: 2,
                    total: 1,
                    ..Default::default()
                },
            ],
            ..Default::default()
        }],
        ..Default::default()
    }];
    (regions, source)
}

/// The whole point of this change: a spliceable plan's `estimated_reduction`
/// is the literal measured delta of applying its suggestion, and the
/// independent ground truth (splice by hand, re-score) must match exactly.
#[test]
fn spliceable_plan_reports_the_measured_reduction() {
    let (regions, source) = loop_guard_regions();
    let complexity = module_complexity(&source);
    let registry = RuleRegistry::new();

    let (plans, _) = registry.analyze(&regions, &source, complexity, true);

    assert_eq!(plans.len(), 1);
    let plan = &plans[0];
    assert_eq!(plan.rule_id, "C002");
    assert!(plan.reduction_is_measured);

    let spliced = splice_plan(plan, plan.suggestion.as_ref().unwrap(), &source).unwrap();
    let measured_after = module_complexity(&spliced);
    assert_eq!(
        plan.estimated_reduction,
        complexity.saturating_sub(measured_after)
    );
    assert_eq!(plan.estimated_complexity_after, measured_after);
}

/// A suggestion whose splice cannot be parsed must yield `None` from
/// `measure_reduction` — the caller keeps the formula estimate and never
/// panics and never prints a fabricated measured number.
#[test]
fn unparseable_splice_measures_to_none() {
    let source = "for x in y:\n    pass\n";
    let mut plan = plan("C002", 1, 2, 3);
    plan.current_complexity = 5;
    plan.suggestion = Some(CodeSuggestion {
        replacement: "for x in y: ((".to_string(),
        applicability: Applicability::MachineApplicable,
        description: String::new(),
        spliceable: true,
    });

    let measured = measure_reduction(&plan, plan.suggestion.as_ref().unwrap(), source, true);
    assert!(measured.is_none());
}

/// A suggestion whose splice changes nothing measures 0 — the value the
/// noise filter drops. Real rules rarely produce this (their formulas
/// understate, so measured >= formula >= 1), which is exactly why the
/// behavior is proven at the unit level rather than through a contrived
/// rule shape.
#[test]
fn no_op_splice_measures_zero() {
    let source = "def f():\n    pass\n";
    let mut plan = plan("C002", 1, 2, 3);
    plan.current_complexity = 0;
    plan.suggestion = Some(CodeSuggestion {
        replacement: "def f():\n    pass".to_string(),
        applicability: Applicability::MachineApplicable,
        description: String::new(),
        spliceable: true,
    });

    let measured = measure_reduction(&plan, plan.suggestion.as_ref().unwrap(), source, false);
    assert_eq!(measured, Some(0));
}
