from __future__ import annotations

import json

import complexipy
from complexipy import RefactorPlan, code_complexity


def first_func(code: str):
    return code_complexity(code).functions[0]


def plan_kinds(func) -> set[str]:
    return {plan.kind for plan in func.refactor_plans}


def test_nested_if_creates_flatten_condition_plan() -> None:
    func = first_func(
        """
def sample(a, b, c, d):
    if a:
        if b:
            if c and d:
                return 1
    return 0
"""
    )

    assert func.complexity == 7
    plan = next(
        plan for plan in func.refactor_plans if plan.kind == "flatten_condition"
    )
    assert plan.title == "Flatten nested condition block with guard clauses"
    assert plan.line_start == 5
    assert plan.line_end == 6
    assert plan.estimated_reduction == 2


def test_loop_with_nested_if_creates_loop_guard_plan() -> None:
    func = first_func(
        """
def sample(items):
    total = 0
    for item in items:
        if item.active:
            if item.ready:
                total += item.value
    return total
"""
    )

    assert "loop_guards" in plan_kinds(func)


def test_high_complexity_region_creates_extract_helper_plan() -> None:
    func = first_func(
        """
def sample(a, b, c, d):
    if a and b:
        if c:
            for value in d:
                if value.ready:
                    return value
    return None
"""
    )

    assert "extract_helper" in plan_kinds(func)


def test_long_elif_chain_creates_dispatcher_plan() -> None:
    func = first_func(
        """
def sample(kind):
    if kind == "a":
        return 1
    elif kind == "b":
        return 2
    elif kind == "c":
        return 3
    elif kind == "d":
        return 4
    return 0
"""
    )

    plan = next(
        plan for plan in func.refactor_plans if plan.kind == "split_dispatcher"
    )
    assert plan.line_start == 3
    assert plan.line_end == 10


def test_match_dispatcher_creates_dispatcher_plan() -> None:
    func = first_func(
        """
def sample(kind):
    match kind:
        case "a":
            return 1
        case "b":
            return 2
        case "c":
            return 3
        case "d":
            return 4
    return 0
"""
    )

    assert "split_dispatcher" in plan_kinds(func)


def test_boolean_heavy_condition_creates_predicate_plan() -> None:
    func = first_func(
        """
def sample(a, b, c, d):
    if (a and b) or (c and d):
        return 1
    return 0
"""
    )

    plan = next(
        plan for plan in func.refactor_plans if plan.kind == "extract_predicate"
    )
    assert plan.line_start == 3
    assert plan.line_end == 3


def test_simple_function_creates_no_refactor_plans() -> None:
    func = first_func(
        """
def sample(a, b):
    return a + b
"""
    )

    assert func.complexity == 0
    assert func.refactor_plans == []


def test_refactor_plan_exported_and_available_on_python_api() -> None:
    func = first_func(
        """
def sample(a, b, c, d):
    if (a and b) or (c and d):
        return 1
    return 0
"""
    )

    assert complexipy.RefactorPlan is RefactorPlan
    assert isinstance(func.refactor_plans[0], RefactorPlan)
    assert func.refactor_plans[0].estimated_complexity_after <= func.complexity


def test_manual_cli_json_includes_refactor_plans_and_snapshot_stays_unchanged(
    tmp_path,
) -> None:
    source = tmp_path / "sample.py"
    source.write_text(
        """
def sample(a, b, c, d):
    if (a and b) or (c and d):
        return 1
    return 0
"""
    )
    file_result = complexipy.file_complexity(str(source))
    func = file_result.functions[0]
    output_path = tmp_path / "complexity.json"

    complexipy._complexipy.output_json(str(output_path), [file_result], True, 0)

    json_data = json.loads(output_path.read_text())
    assert json_data == [
        {
            "path": "sample.py",
            "file_name": "sample.py",
            "function_name": "sample",
            "complexity": func.complexity,
            "refactor_plans": [
                {
                    "kind": func.refactor_plans[0].kind,
                    "title": func.refactor_plans[0].title,
                    "line_start": func.refactor_plans[0].line_start,
                    "line_end": func.refactor_plans[0].line_end,
                    "current_complexity": func.refactor_plans[
                        0
                    ].current_complexity,
                    "estimated_reduction": func.refactor_plans[
                        0
                    ].estimated_reduction,
                    "estimated_complexity_after": func.refactor_plans[
                        0
                    ].estimated_complexity_after,
                    "steps": list(func.refactor_plans[0].steps),
                }
            ],
        }
    ]

    snapshot_path = tmp_path / "complexipy-snapshot.json"
    complexipy._complexipy.create_snapshot_file(
        str(snapshot_path), 0, [file_result]
    )
    snapshot_data = json.loads(snapshot_path.read_text())
    assert "refactor_plans" not in snapshot_data[0]["functions"][0]
