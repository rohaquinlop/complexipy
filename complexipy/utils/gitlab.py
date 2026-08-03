from __future__ import annotations

import hashlib
import json
from typing import List

from complexipy._complexipy import FileComplexity

from .output import normalize_path

_CHECK_NAME = "complexipy/cognitive-complexity"
_DEFAULT_SEVERITY = "minor"


def _build_description(
    function_name: str, complexity: int, max_complexity: int
) -> str:
    return (
        f"Function '{function_name}' has cognitive complexity {complexity} "
        f"(max allowed: {max_complexity})."
    )


def _build_fingerprint(
    check_id: str, path: str, function_name: str, line_start: int
) -> str:
    payload = f"{check_id}:{path}:{function_name}:{line_start}"
    return hashlib.sha256(payload.encode("utf-8")).hexdigest()


def _refactor_plan_severity(applicability: object) -> str:
    applicability_str = str(applicability)
    if "MachineApplicable" in applicability_str:
        return "minor"
    if "MaybeIncorrect" in applicability_str:
        return "major"
    return "info"


def _refactor_plan_issue(relative_path: str, function_name: str, plan) -> dict:
    return {
        "description": f"[{plan.rule_id}] {plan.title}: {plan.explanation}",
        "check_name": f"complexipy/{plan.rule_id.lower()}",
        "fingerprint": _build_fingerprint(
            plan.rule_id, relative_path, function_name, int(plan.line_start)
        ),
        "severity": _refactor_plan_severity(plan.applicability),
        "location": {
            "path": relative_path,
            "lines": {"begin": int(plan.line_start), "end": int(plan.line_end)},
        },
    }


def store_gitlab(
    output_path: str,
    files: List[FileComplexity],
    max_complexity: int,
    suggest_refactors: bool = False,
) -> None:
    """Write complexity violations as a GitLab Code Quality report.

    Functions whose complexity exceeds *max_complexity* are always reported
    under the built-in cognitive-complexity check. When *suggest_refactors*
    is True, each function's ranked refactor plans are also reported, one
    issue per plan, under a `complexipy/<rule_id>` check name.
    """
    report = []

    for file in files:
        normalized_path = normalize_path(file.path, file.file_name)
        relative_path = (
            normalized_path[2:]
            if normalized_path.startswith("./")
            else normalized_path
        )

        for function in file.functions:
            if function.complexity > max_complexity:
                report.append(
                    {
                        "description": _build_description(
                            function.name,
                            int(function.complexity),
                            max_complexity,
                        ),
                        "check_name": _CHECK_NAME,
                        "fingerprint": _build_fingerprint(
                            "CC001",
                            relative_path,
                            function.name,
                            int(function.line_start),
                        ),
                        "severity": _DEFAULT_SEVERITY,
                        "location": {
                            "path": relative_path,
                            "lines": {"begin": int(function.line_start)},
                        },
                    }
                )

            if suggest_refactors:
                for plan in function.refactor_plans:
                    report.append(
                        _refactor_plan_issue(relative_path, function.name, plan)
                    )

    with open(output_path, "w", encoding="utf-8") as f:
        json.dump(report, f, indent=2)
        f.write("\n")
