use std::collections::HashMap;

use sha2::{Digest, Sha256};

use complexipy_core::classes::{FileComplexity, FunctionComplexity, RefactorPlan};

/// Identifies a finding by its check, file, and function, and by its rank
/// among the findings that share those, so lines added above it leave the
/// fingerprint unchanged.
pub fn finding_fingerprint(
    check_id: &str,
    path: &str,
    function_name: &str,
    function_rank: usize,
    plan_rank: usize,
) -> String {
    let payload = format!(
        "{}:{}:{}:{}:{}",
        check_id, path, function_name, function_rank, plan_rank
    );
    Sha256::digest(payload.as_bytes())
        .iter()
        .map(|byte| format!("{:02x}", byte))
        .collect()
}

/// The rank of each function among the functions of `file` with its name,
/// in line order, keyed by name and first line.
pub fn function_ranks(file: &FileComplexity) -> HashMap<(&str, u64), usize> {
    let mut ordered: Vec<&FunctionComplexity> = file.functions.iter().collect();
    ordered.sort_by_key(|function| (function.name.as_str(), function.line_start));

    let mut ranks = HashMap::with_capacity(ordered.len());
    let mut previous: Option<&str> = None;
    let mut rank = 0;
    for function in ordered {
        rank = if previous == Some(function.name.as_str()) {
            rank + 1
        } else {
            0
        };
        previous = Some(function.name.as_str());
        ranks.insert((function.name.as_str(), function.line_start), rank);
    }
    ranks
}

/// The rank of `plan` among the plans of its rule in `function`, in line
/// order.
pub fn plan_rank(function: &FunctionComplexity, plan: &RefactorPlan) -> usize {
    function
        .refactor_plans
        .iter()
        .filter(|other| {
            other.rule_id == plan.rule_id
                && (other.line_start, other.line_end) < (plan.line_start, plan.line_end)
        })
        .count()
}
