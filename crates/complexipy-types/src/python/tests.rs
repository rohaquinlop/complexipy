use super::*;

#[test]
fn python_names_cover_every_declared_member() {
    for value in [RuleCategory::Complexity, RuleCategory::Readability] {
        assert!(
            RULE_CATEGORY_MEMBERS
                .iter()
                .any(|(name, member)| *name == value.python_name() && *member == value)
        );
    }

    for value in [
        Applicability::MachineApplicable,
        Applicability::MaybeIncorrect,
        Applicability::Informational,
    ] {
        assert!(
            APPLICABILITY_MEMBERS
                .iter()
                .any(|(name, member)| *name == value.python_name() && *member == value)
        );
    }

    for value in [
        DiffStatus::Regressed,
        DiffStatus::Improved,
        DiffStatus::Unchanged,
        DiffStatus::New,
        DiffStatus::Removed,
    ] {
        assert!(
            DIFF_STATUS_MEMBERS
                .iter()
                .any(|(name, member)| *name == value.python_name() && *member == value)
        );
    }
}
