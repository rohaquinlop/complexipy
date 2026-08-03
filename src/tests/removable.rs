//! Unit tests for `crate::utils::filter_removable_ignores`.
//!
//! Wired in from `src/utils.rs` via `#[cfg(test)] #[path = ...] mod tests;`
//! so this stays a child module of the code it tests and can reach that
//! module's private helpers through `super::` without widening visibility.

use crate::classes::FunctionComplexity;

fn function(name: &str, complexity: u64, line_start: u64, line_end: u64) -> FunctionComplexity {
    FunctionComplexity {
        name: name.to_string(),
        complexity,
        line_start,
        line_end,
        line_complexities: vec![],
        refactor_plans: vec![],
        additional_refactor_plans: 0,
    }
}

#[test]
fn marker_below_threshold_is_removable() {
    let locations = vec![(1, "# complexipy: ignore".to_string())];
    let functions = vec![function("simple", 2, 1, 3)];
    let removable = super::filter_removable_ignores(&locations, &functions, 15);
    assert_eq!(
        removable,
        vec![(
            1,
            "# complexipy: ignore".to_string(),
            "simple".to_string(),
            2
        )]
    );
}

#[test]
fn marker_at_threshold_is_removable() {
    let locations = vec![(5, "# complexipy: ignore".to_string())];
    let functions = vec![function("edge", 15, 5, 9)];
    let removable = super::filter_removable_ignores(&locations, &functions, 15);
    assert_eq!(removable.len(), 1);
}

#[test]
fn marker_above_threshold_is_kept() {
    let locations = vec![(1, "# complexipy: ignore".to_string())];
    let functions = vec![function("complex_fn", 20, 1, 12)];
    let removable = super::filter_removable_ignores(&locations, &functions, 15);
    assert!(removable.is_empty());
}

#[test]
fn marker_matches_containing_function_range() {
    let locations = vec![(3, "# noqa: complexipy".to_string())];
    let functions = vec![function("decorated", 4, 1, 9)];
    let removable = super::filter_removable_ignores(&locations, &functions, 15);
    assert_eq!(removable.len(), 1);
    assert_eq!(removable[0].2, "decorated");
}

#[test]
fn marker_without_containing_function_is_skipped() {
    let locations = vec![(7, "# complexipy: ignore".to_string())];
    let functions = vec![function("first", 2, 1, 5), function("second", 3, 9, 11)];
    let removable = super::filter_removable_ignores(&locations, &functions, 15);
    assert!(removable.is_empty());
}

#[test]
fn marker_never_matches_ignored_analysis() {
    let locations = vec![(2, "# complexipy: ignore".to_string())];
    let functions: Vec<FunctionComplexity> = vec![];
    let removable = super::filter_removable_ignores(&locations, &functions, 15);
    assert!(removable.is_empty());
}

#[test]
fn multiple_markers_keep_source_order() {
    let locations = vec![
        (1, "# complexipy: ignore".to_string()),
        (9, "# noqa: complexipy".to_string()),
    ];
    let functions = vec![
        function("low", 3, 1, 4),
        function("high", 30, 6, 20),
        function("low_again", 5, 22, 30),
    ];
    let removable = super::filter_removable_ignores(&locations, &functions, 15);
    assert_eq!(removable.len(), 1);
    assert_eq!(
        removable[0],
        (1, "# complexipy: ignore".to_string(), "low".to_string(), 3)
    );
}
