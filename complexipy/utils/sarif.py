from __future__ import annotations

import json
from importlib.metadata import PackageNotFoundError
from importlib.metadata import version as pkg_version
from typing import List

from complexipy._complexipy import FileComplexity

_RULE_ID = "CC001"
_SCHEMA = (
    "https://raw.githubusercontent.com/oasis-tcs/sarif-spec"
    "/master/Schemata/sarif-schema-2.1.0.json"
)
_INFO_URI = "https://rohaquinlop.github.io/complexipy/"
_HELP_URI = "https://rohaquinlop.github.io/complexipy/understanding-scores/"


def _get_version() -> str:
    try:
        return pkg_version("complexipy")
    except PackageNotFoundError:
        return "unknown"


def _refactor_plan_level(applicability: object) -> str:
    return "note" if "Informational" in str(applicability) else "warning"


def _refactor_plan_result(file: FileComplexity, function, plan) -> dict:
    return {
        "ruleId": plan.rule_id,
        "level": _refactor_plan_level(plan.applicability),
        "message": {"text": f"{plan.title}: {plan.explanation}"},
        "locations": [
            {
                "physicalLocation": {
                    "artifactLocation": {
                        "uri": file.path,
                        "uriBaseId": "%SRCROOT%",
                    },
                    "region": {
                        "startLine": int(plan.line_start),
                        "startColumn": int(plan.column_start),
                        "endLine": int(plan.line_end),
                    },
                },
                "logicalLocations": [
                    {"name": function.name, "kind": "function"}
                ],
            }
        ],
    }


def _refactor_plan_rule_definition(plan) -> dict:
    return {
        "id": plan.rule_id,
        "name": plan.kind,
        "shortDescription": {"text": plan.description},
        "helpUri": plan.doc_url,
        "properties": {"tags": [str(plan.category).lower()]},
    }


def _complexity_result(
    file: FileComplexity, function, max_complexity: int
) -> dict:
    return {
        "ruleId": _RULE_ID,
        "level": "warning",
        "message": {
            "text": (
                f"Function '{function.name}' has a cognitive complexity"
                f" of {function.complexity}, which exceeds the maximum"
                f" allowed complexity of {max_complexity}."
            )
        },
        "locations": [
            {
                "physicalLocation": {
                    "artifactLocation": {
                        "uri": file.path,
                        "uriBaseId": "%SRCROOT%",
                    },
                    "region": {
                        "startLine": int(function.line_start),
                        "endLine": int(function.line_end),
                    },
                },
                "logicalLocations": [
                    {"name": function.name, "kind": "function"}
                ],
            }
        ],
    }


def _complexity_rule_definition() -> dict:
    return {
        "id": _RULE_ID,
        "name": "CognitiveComplexity",
        "shortDescription": {"text": "Cognitive complexity exceeds threshold"},
        "helpUri": _HELP_URI,
        "properties": {"tags": ["maintainability", "readability"]},
    }


def _add_refactor_plan_results(
    file: FileComplexity,
    function,
    results: list,
    refactor_rule_definitions: dict,
) -> None:
    for plan in function.refactor_plans:
        if plan.rule_id not in refactor_rule_definitions:
            refactor_rule_definitions[plan.rule_id] = (
                _refactor_plan_rule_definition(plan)
            )
        results.append(_refactor_plan_result(file, function, plan))


def _collect_results(
    files: List[FileComplexity], max_complexity: int, suggest_refactors: bool
) -> tuple:
    results: list = []
    refactor_rule_definitions: dict = {}

    for file in files:
        for function in file.functions:
            if function.complexity > max_complexity:
                results.append(
                    _complexity_result(file, function, max_complexity)
                )
            if suggest_refactors:
                _add_refactor_plan_results(
                    file, function, results, refactor_rule_definitions
                )

    return results, refactor_rule_definitions


def store_sarif(
    output_path: str,
    files: List[FileComplexity],
    max_complexity: int,
    suggest_refactors: bool = False,
) -> None:
    """Write complexity violations as a SARIF 2.1.0 file.

    Functions whose complexity exceeds *max_complexity* are always emitted as
    SARIF results under the built-in cognitive-complexity rule. When
    *suggest_refactors* is True, each function's ranked refactor plans are
    also emitted as results under their own rule ID (C001, C007, ...), with
    rule definitions built dynamically from the plans actually encountered so
    the catalog can never drift from `RuleMetadata`. The file is written to
    *output_path*.
    """
    results, refactor_rule_definitions = _collect_results(
        files, max_complexity, suggest_refactors
    )

    sarif_doc = {
        "version": "2.1.0",
        "$schema": _SCHEMA,
        "runs": [
            {
                "tool": {
                    "driver": {
                        "name": "complexipy",
                        "version": _get_version(),
                        "informationUri": _INFO_URI,
                        "rules": [
                            _complexity_rule_definition(),
                            *refactor_rule_definitions.values(),
                        ],
                    }
                },
                "results": results,
            }
        ],
    }

    with open(output_path, "w", encoding="utf-8") as f:
        json.dump(sarif_doc, f, indent=2)
