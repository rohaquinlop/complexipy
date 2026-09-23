use super::RuleSet;

fn ids(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| value.to_string()).collect()
}

#[test]
fn no_selection_activates_every_rule() {
    let (rules, unknown) = RuleSet::resolve(&[], &[], &ids(&["C001", "C007"]));

    assert!(rules.is_active("C001"));
    assert!(rules.is_active("C007"));
    assert!(unknown.is_empty());
}

#[test]
fn select_activates_only_listed_rules() {
    let (rules, _) = RuleSet::resolve(&ids(&["c001"]), &[], &ids(&["C001", "C007"]));

    assert!(rules.is_active("C001"));
    assert!(!rules.is_active("C007"));
}

#[test]
fn ignore_deactivates_listed_rules() {
    let (rules, _) = RuleSet::resolve(&[], &ids(&["C007"]), &ids(&["C001", "C007"]));

    assert!(rules.is_active("C001"));
    assert!(!rules.is_active("C007"));
}

#[test]
fn ignore_wins_over_select() {
    let (rules, _) = RuleSet::resolve(
        &ids(&["C001", "C007"]),
        &ids(&["C007"]),
        &ids(&["C001", "C007"]),
    );

    assert!(rules.is_active("C001"));
    assert!(!rules.is_active("C007"));
}

#[test]
fn unknown_ids_are_reported_sorted_and_deduplicated() {
    let (_, unknown) = RuleSet::resolve(
        &ids(&["c999", "C999"]),
        &ids(&["C001", "c888"]),
        &ids(&["C001", "C007"]),
    );

    assert_eq!(unknown, vec!["C888".to_string(), "C999".to_string()]);
}

#[test]
fn select_with_only_blank_entries_selects_nothing() {
    let (rules, unknown) = RuleSet::resolve(&ids(&[" ", ""]), &[], &ids(&["C001", "C007"]));

    assert!(!rules.is_active("C001"));
    assert!(!rules.is_active("C007"));
    assert_eq!(unknown, vec![String::new()]);
}

#[test]
fn blank_entries_are_reported_and_padded_ids_are_normalized() {
    let (rules, unknown) = RuleSet::resolve(&ids(&[" C001 ", ""]), &[], &ids(&["C001", "C007"]));

    assert!(rules.is_active("C001"));
    assert!(!rules.is_active("C007"));
    assert_eq!(unknown, vec![String::new()]);
}

#[test]
fn without_subtracts_rules_and_keeps_the_original() {
    let (rules, _) = RuleSet::resolve(&[], &[], &ids(&["C001", "C007"]));
    let narrowed = rules.without(&ids(&["c007"]));

    assert!(narrowed.is_active("C001"));
    assert!(!narrowed.is_active("C007"));
    assert!(rules.is_active("C007"));
}

#[test]
fn empty_and_padded_entries_are_normalized() {
    let (rules, _) = RuleSet::resolve(&ids(&[" C001 "]), &[], &ids(&["C001", "C007"]));

    assert!(rules.is_active("C001"));
    assert!(!rules.is_active("C007"));
}

#[test]
fn is_active_matches_canonical_ids_without_allocating_input() {
    let (rules, _) = RuleSet::resolve(&[], &ids(&["C007"]), &ids(&["C001", "C007"]));

    assert!(rules.is_active("C001"));
    assert!(rules.is_active("c001"));
    assert!(!rules.is_active("C007"));
    assert!(!rules.is_active("c007"));
}
