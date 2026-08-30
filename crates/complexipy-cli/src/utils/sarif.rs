use std::fs;

use serde_json::{Map, Value, json};

use complexipy_core::classes::{
    Applicability, FileComplexity, FunctionComplexity, RefactorPlan, RuleCategory,
};
use complexipy_core::utils::ExportError;

const RULE_ID: &str = "CC001";
const SCHEMA: &str = "https://raw.githubusercontent.com/oasis-tcs/sarif-spec/master/Schemata/sarif-schema-2.1.0.json";
const INFO_URI: &str = "https://rohaquinlop.github.io/complexipy/";
const HELP_URI: &str = "https://rohaquinlop.github.io/complexipy/understanding-scores/";

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
    fs::write(output_path, serialized)
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
        for function in &file.functions {
            if function.complexity > max_complexity {
                results.push(complexity_result(file, function, max_complexity));
            }
            if suggest_refactors {
                add_refactor_plan_results(
                    file,
                    function,
                    &mut results,
                    &mut refactor_rule_definitions,
                );
            }
        }
    }

    (results, refactor_rule_definitions)
}

fn add_refactor_plan_results(
    file: &FileComplexity,
    function: &FunctionComplexity,
    results: &mut Vec<Value>,
    refactor_rule_definitions: &mut Map<String, Value>,
) {
    for plan in &function.refactor_plans {
        if !refactor_rule_definitions.contains_key(&plan.rule_id) {
            refactor_rule_definitions
                .insert(plan.rule_id.clone(), refactor_plan_rule_definition(plan));
        }
        results.push(refactor_plan_result(file, function, plan));
    }
}

fn complexity_result(
    file: &FileComplexity,
    function: &FunctionComplexity,
    max_complexity: u64,
) -> Value {
    json!({
        "ruleId": RULE_ID,
        "level": "warning",
        "message": {
            "text": format!(
                "Function '{}' has a cognitive complexity of {}, which exceeds the maximum allowed complexity of {}.",
                function.name, function.complexity, max_complexity
            )
        },
        "locations": [{
            "physicalLocation": {
                "artifactLocation": {
                    "uri": file.path,
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

fn refactor_plan_result(
    file: &FileComplexity,
    function: &FunctionComplexity,
    plan: &RefactorPlan,
) -> Value {
    json!({
        "ruleId": plan.rule_id,
        "level": refactor_plan_level(&plan.applicability),
        "message": {"text": format!("{}: {}", plan.title, plan.explanation)},
        "locations": [{
            "physicalLocation": {
                "artifactLocation": {
                    "uri": file.path,
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
