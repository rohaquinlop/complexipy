//! Unit tests for `crate::rules::registry`.
//!
//! Wired in from `src/rules/registry.rs` via `#[cfg(test)] #[path = ...] mod tests;`
//! so this stays a child module of the code it tests and can reach the private
//! `RuleRegistry.rules` field through `super::` without widening its visibility.

use super::RuleRegistry;
use crate::refactor_plans::{ComplexityRegion, RegionKind};

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
        "C006" => (
            ComplexityRegion {
                kind: RegionKind::Loop,
                line_start: 1,
                line_end: 5,
                nesting: 3,
                total: 5,
                ..Default::default()
            },
            "for x in y:\n    pass\n".to_string(),
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
        checked, 8,
        "expected all 8 registered rules to be exercised by a fixture"
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
        ("C006", 4),
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
