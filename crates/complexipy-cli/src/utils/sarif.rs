use std::path::Path;

use serde_json::{Map, Value, json};

use crate::utils::fingerprint::{finding_fingerprint, function_ranks, plan_rank};
use complexipy_core::classes::{
    Applicability, FileComplexity, FunctionComplexity, RefactorPlan, RuleCategory,
};
use complexipy_core::fs_guard::write_atomic;
use complexipy_core::utils::ExportError;

const RULE_ID: &str = "CC001";
const SCHEMA: &str =
    "https://docs.oasis-open.org/sarif/sarif/v2.1.0/errata01/os/schemas/sarif-schema-2.1.0.json";
const FINGERPRINT_KEY: &str = "complexipyFindingHash/v1";
const INFO_URI: &str = "https://complexipy.com/";
const HELP_URI: &str = "https://complexipy.com/understanding-scores/";

pub fn store_sarif(
    output_path: &str,
    files: &[FileComplexity],
    max_complexity: u64,
    suggest_refactors: bool,
) -> Result<(), ExportError> {
    let (results, refactor_rule_definitions) =
        collect_results(files, max_complexity, suggest_refactors);

    let mut rules = vec![complexity_rule_definition()];
    rules.extend(refactor_rule_definitions.values().cloned());

    let sarif_doc = json!({
        "version": "2.1.0",
        "$schema": SCHEMA,
        "runs": [{
            "tool": {
                "driver": {
                    "name": "complexipy",
                    "version": env!("CARGO_PKG_VERSION"),
                    "informationUri": INFO_URI,
                    "rules": rules,
                }
            },
            "results": results,
        }],
    });

    let serialized = serde_json::to_string_pretty(&sarif_doc)
        .map_err(|e| ExportError::Serialize(format!("Failed to serialize SARIF: {}", e)))?;
    write_atomic(Path::new(output_path), serialized.as_bytes())
        .map_err(|e| ExportError::Io(format!("Failed to write SARIF to {}: {}", output_path, e)))?;
    Ok(())
}

fn collect_results(
    files: &[FileComplexity],
    max_complexity: u64,
    suggest_refactors: bool,
) -> (Vec<Value>, Map<String, Value>) {
    let mut results = Vec::new();
    let mut refactor_rule_definitions = Map::new();

    for file in files {
        let ranks = function_ranks(file);
        for function in &file.functions {
            let location = Location {
                file,
                function,
                rank: ranks
                    .get(&(function.name.as_str(), function.line_start))
                    .copied()
                    .unwrap_or(0),
            };
            if function.complexity > max_complexity {
                results.push(complexity_result(&location, max_complexity));
            }
            if suggest_refactors {
                add_refactor_plan_results(&location, &mut results, &mut refactor_rule_definitions);
            }
        }
    }

    (results, refactor_rule_definitions)
}

struct Location<'a> {
    file: &'a FileComplexity,
    function: &'a FunctionComplexity,
    rank: usize,
}

fn add_refactor_plan_results(
    location: &Location,
    results: &mut Vec<Value>,
    refactor_rule_definitions: &mut Map<String, Value>,
) {
    for plan in &location.function.refactor_plans {
        if !refactor_rule_definitions.contains_key(&plan.rule_id) {
            refactor_rule_definitions
                .insert(plan.rule_id.clone(), refactor_plan_rule_definition(plan));
        }
        results.push(refactor_plan_result(location, plan));
    }
}

/// `path` as a relative URI reference: each byte outside the unreserved
/// set and `/` is percent-encoded.
fn uri_reference(path: &str) -> String {
    let mut encoded = String::with_capacity(path.len());
    for byte in path.replace('\\', "/").bytes() {
        if byte.is_ascii_alphanumeric() || b"-._~/".contains(&byte) {
            encoded.push(char::from(byte));
        } else {
            encoded.push_str(&format!("%{:02X}", byte));
        }
    }
    encoded
}

fn complexity_result(location: &Location, max_complexity: u64) -> Value {
    let Location {
        file,
        function,
        rank,
    } = location;
    json!({
        "ruleId": RULE_ID,
        "level": "warning",
        "message": {
            "text": format!(
                "Function '{}' has a cognitive complexity of {}, which exceeds the maximum allowed complexity of {}.",
                function.name, function.complexity, max_complexity
            )
        },
        "partialFingerprints": {
            FINGERPRINT_KEY: finding_fingerprint(RULE_ID, &file.path, &function.name, *rank, 0),
        },
        "locations": [{
            "physicalLocation": {
                "artifactLocation": {
                    "uri": uri_reference(&file.path),
                    "uriBaseId": "%SRCROOT%",
                },
                "region": {
                    "startLine": function.line_start,
                    "endLine": function.line_end,
                },
            },
            "logicalLocations": [
                {"name": function.name, "kind": "function"}
            ],
        }],
    })
}

fn complexity_rule_definition() -> Value {
    json!({
        "id": RULE_ID,
        "name": "CognitiveComplexity",
        "shortDescription": {"text": "Cognitive complexity exceeds threshold"},
        "helpUri": HELP_URI,
        "properties": {"tags": ["maintainability", "readability"]},
    })
}

fn refactor_plan_result(location: &Location, plan: &RefactorPlan) -> Value {
    let Location {
        file,
        function,
        rank,
    } = location;
    json!({
        "ruleId": plan.rule_id,
        "level": refactor_plan_level(&plan.applicability),
        "message": {"text": format!("{}: {}", plan.title, plan.explanation)},
        "partialFingerprints": {
            FINGERPRINT_KEY: finding_fingerprint(
                &plan.rule_id,
                &file.path,
                &function.name,
                *rank,
                plan_rank(function, plan),
            ),
        },
        "locations": [{
            "physicalLocation": {
                "artifactLocation": {
                    "uri": uri_reference(&file.path),
                    "uriBaseId": "%SRCROOT%",
                },
                "region": {
                    "startLine": plan.line_start,
                    "startColumn": plan.column_start,
                    "endLine": plan.line_end,
                },
            },
            "logicalLocations": [
                {"name": function.name, "kind": "function"}
            ],
        }],
    })
}

fn refactor_plan_rule_definition(plan: &RefactorPlan) -> Value {
    json!({
        "id": plan.rule_id,
        "name": plan.kind,
        "shortDescription": {"text": plan.description},
        "helpUri": plan.doc_url,
        "properties": {"tags": [rule_category_tag(&plan.category)]},
    })
}

fn refactor_plan_level(applicability: &Applicability) -> &'static str {
    match applicability {
        Applicability::Informational => "note",
        _ => "warning",
    }
}

fn rule_category_tag(category: &RuleCategory) -> &'static str {
    match category {
        RuleCategory::Complexity => "complexity",
        RuleCategory::Readability => "readability",
    }
}

#[cfg(test)]
mod tests;
