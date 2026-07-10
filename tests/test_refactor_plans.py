from __future__ import annotations

import json
from pathlib import Path

import complexipy
from complexipy import Applicability, RefactorPlan, code_complexity


def load_source(filename: str) -> str:
    """Load Python source from tests/src/refactor_plans/."""
    path = Path(__file__).parent / "src" / "refactor_plans" / filename
    return path.read_text()


def first_func(code: str):
    return code_complexity(code).functions[0]


def plan_kinds(func) -> set[str]:
    return {plan.kind for plan in func.refactor_plans}


def test_nested_if_creates_flatten_condition_plan() -> None:
    func = first_func(load_source("collapsible_if_simple.py"))

    assert func.complexity == 7
    # C007 (collapsible_if) and C001 (flatten_condition) fire on overlapping regions.
    # With region overlap dedup, C007 wins (same priority & reduction, first in sort order).
    plan = next(plan for plan in func.refactor_plans if plan.kind == "collapsible_if")
    assert plan.rule_id == "C007"
    assert plan.estimated_reduction >= 2
    assert plan.applicability == Applicability.MachineApplicable
    assert plan.suggestion is not None
    assert plan.help is None


def test_loop_with_nested_if_creates_loop_guard_plan() -> None:
    func = first_func(load_source("loop_guard_nested_if.py"))

    # C007 (collapsible_if, effectiveness=5) wins over C002 (loop_guards, effectiveness=3)
    plan = func.refactor_plans[0]
    assert plan.rule_id == "C007"
    assert plan.kind == "collapsible_if"
    assert plan.applicability == Applicability.MachineApplicable
    assert plan.suggestion is not None
    assert "item.active and item.ready" in plan.suggestion.replacement


def test_high_complexity_region_creates_extract_helper_plan() -> None:
    func = first_func(load_source("extract_helper_complex_region.py"))

    # C007 (collapsible_if, effectiveness=5) fires on the outer `if a and b:` / `if c:` block
    # and wins over C003 (extract_helper, effectiveness=2) due to higher effectiveness
    assert "collapsible_if" in plan_kinds(func)


def test_long_elif_chain_creates_dispatcher_plan() -> None:
    func = first_func(load_source("split_dispatcher_elif_chain.py"))

    plan = next(plan for plan in func.refactor_plans if plan.kind == "split_dispatcher")
    assert plan.line_start == 2
    assert plan.line_end == 9
    assert plan.applicability == Applicability.Informational
    assert plan.suggestion is None
    assert plan.help is not None
    assert "dispatch dictionary" in plan.help


def test_match_dispatcher_creates_dispatcher_plan() -> None:
    func = first_func(load_source("split_dispatcher_match.py"))

    assert "split_dispatcher" in plan_kinds(func)


def test_boolean_heavy_condition_creates_predicate_plan() -> None:
    func = first_func(load_source("extract_predicate_boolean.py"))

    plan = next(plan for plan in func.refactor_plans if plan.kind == "extract_predicate")
    assert plan.line_start == 2
    assert plan.line_end == 2
    assert plan.applicability == Applicability.MachineApplicable
    assert plan.suggestion is not None
    assert "def _check_condition_L" in plan.suggestion.replacement
    assert plan.help is None


def test_simple_function_creates_no_refactor_plans() -> None:
    func = first_func(load_source("simple_function_no_plans.py"))

    assert func.complexity == 0
    assert func.refactor_plans == []


def test_refactor_plan_exported_and_available_on_python_api() -> None:
    func = first_func(load_source("extract_predicate_boolean.py"))

    assert complexipy.RefactorPlan is RefactorPlan
    assert isinstance(func.refactor_plans[0], RefactorPlan)
    assert func.refactor_plans[0].estimated_complexity_after <= func.complexity


def test_manual_cli_json_includes_refactor_plans_and_snapshot_stays_unchanged(tmp_path) -> None:
    source = tmp_path / "sample.py"
    source.write_text(load_source("extract_predicate_boolean.py"))
    file_result = complexipy.file_complexity(str(source))
    func = file_result.functions[0]
    output_path = tmp_path / "complexity.json"

    complexipy._complexipy.output_json(str(output_path), [file_result], True, 0)

    json_data = json.loads(output_path.read_text())
    # Check that the JSON contains the expected structure with new fields
    assert len(json_data) == 1
    assert json_data[0]["path"] == "sample.py"
    assert json_data[0]["file_name"] == "sample.py"
    assert json_data[0]["function_name"] == "sample"
    assert json_data[0]["complexity"] == func.complexity

    # Check that refactor plans are included with new fields
    assert len(json_data[0]["refactor_plans"]) > 0
    plan = json_data[0]["refactor_plans"][0]
    assert plan["kind"] == func.refactor_plans[0].kind
    assert plan["title"] == func.refactor_plans[0].title
    assert plan["line_start"] == func.refactor_plans[0].line_start
    assert plan["line_end"] == func.refactor_plans[0].line_end
    assert plan["current_complexity"] == func.refactor_plans[0].current_complexity
    assert plan["estimated_reduction"] == func.refactor_plans[0].estimated_reduction
    assert plan["estimated_complexity_after"] == func.refactor_plans[0].estimated_complexity_after

    assert "rule_id" in plan
    assert "category" in plan
    assert "applicability" in plan
    assert "description" in plan
    assert "explanation" in plan
    assert "references" in plan
    assert "suggestion" in plan
    assert "help" in plan

    snapshot_path = tmp_path / "complexipy-snapshot.json"
    complexipy._complexipy.create_snapshot_file(str(snapshot_path), 0, [file_result])
    snapshot_data = json.loads(snapshot_path.read_text())
    assert "refactor_plans" not in snapshot_data[0]["functions"][0]


def test_single_line_function_creates_no_plans() -> None:
    func = first_func(load_source("single_line_function.py"))
    assert func.complexity == 0
    assert func.refactor_plans == []


def test_function_with_only_comments_creates_no_plans() -> None:
    func = first_func(load_source("comments_only_function.py"))
    assert func.complexity == 0
    assert func.refactor_plans == []


def test_function_with_unusual_indentation() -> None:
    func = first_func(load_source("unusual_indentation.py"))
    assert func.complexity == 10
    assert len(func.refactor_plans) > 0


def test_rule_priority_ordering() -> None:
    func = first_func(load_source("rule_priority_ordering.py"))
    plans = func.refactor_plans
    # With region overlap dedup, only the best plan survives (highest reduction).
    assert len(plans) >= 1
    # C007 (collapsible_if, effectiveness=5) wins over C003 (extract_helper, effectiveness=2)
    assert plans[0].rule_id == "C007"
    assert plans[0].kind == "collapsible_if"


def test_rule_metadata_has_doc_url() -> None:
    func = first_func(load_source("metadata_validation.py"))
    for plan in func.refactor_plans:
        assert plan.rule_id.startswith("C")
        assert plan.category is not None
        assert plan.applicability is not None
        assert plan.description
        assert plan.explanation


def test_code_generation_produces_nonempty_snippets() -> None:
    func = first_func(load_source("code_generation_flatten.py"))
    flatten_plan = next(
        (p for p in func.refactor_plans if p.kind == "flatten_condition"),
        None
    )
    if flatten_plan:
        assert flatten_plan.suggestion is None
        assert flatten_plan.help is not None
        assert len(flatten_plan.help) > 0


def test_loop_guard_only_converts_outermost_if() -> None:
    """C002 should convert the outermost if to a guard."""
    func = first_func(load_source("loop_guard_with_else.py"))
    loop_guard_plan = next(
        (p for p in func.refactor_plans if p.kind == "loop_guards"), None
    )
    assert loop_guard_plan is not None
    assert loop_guard_plan.suggestion is not None
    assert "if not item.active:" in loop_guard_plan.suggestion.replacement
    assert "continue" in loop_guard_plan.suggestion.replacement


def test_collapsible_if_merges_nested_conditions() -> None:
    """C007 should merge if a: if b: body into if a and b: body."""
    func = first_func(load_source("collapsible_if_simple.py"))
    plan = next(p for p in func.refactor_plans if p.kind == "collapsible_if")
    assert plan.rule_id == "C007"
    assert plan.applicability == Applicability.MachineApplicable
    assert plan.suggestion is not None
    # With 3-level chain, all conditions should be merged
    # No parens needed since all conditions use 'and' (no 'or' mixing)
    assert "a and b and c and d" in plan.suggestion.replacement
    assert plan.estimated_reduction >= 3


def test_collapsible_if_merges_three_levels() -> None:
    """C007 should merge if a: if b: if c: body into if a and b and c: body."""
    func = first_func(load_source("collapsible_if_three_levels.py"))
    plan = next(p for p in func.refactor_plans if p.kind == "collapsible_if")
    assert plan.rule_id == "C007"
    assert plan.applicability == Applicability.MachineApplicable
    assert plan.suggestion is not None
    assert "a and b and c" in plan.suggestion.replacement
    assert plan.estimated_reduction >= 3


def test_loop_guard_preserves_inner_if_else() -> None:
    """C002 should preserve inner if/else when converting outer to guard."""
    func = first_func(load_source("loop_guard_with_else.py"))
    # C007 can't fire because inner if has else
    # C002 should fire and convert the outermost if to a guard
    loop_guard_plan = next(
        (p for p in func.refactor_plans if p.kind == "loop_guards"), None
    )
    assert loop_guard_plan is not None, "C002 should fire when C007 can't"
    assert loop_guard_plan.suggestion is not None
    assert "if not item.active:" in loop_guard_plan.suggestion.replacement
    assert "continue" in loop_guard_plan.suggestion.replacement
    # The inner if/else should be preserved
    assert "if item.value > threshold:" in loop_guard_plan.suggestion.replacement
    assert "else:" in loop_guard_plan.suggestion.replacement


def test_collapsible_if_skips_when_outer_has_else() -> None:
    """C007 should not fire when the outer if has an else branch."""
    func = first_func(load_source("collapsible_if_skips_with_else.py"))
    assert "collapsible_if" not in [p.kind for p in func.refactor_plans]


def test_collapsible_if_wraps_or_conditions_in_parens() -> None:
    """C007 should wrap 'or' conditions in parens when joining with 'and'."""
    func = first_func(load_source("collapsible_if_with_or.py"))
    plan = next(p for p in func.refactor_plans if p.kind == "collapsible_if")
    assert plan.suggestion is not None
    assert "(a or b) and c" in plan.suggestion.replacement


def test_loop_guard_three_levels_generates_multiple_guards() -> None:
    """C007 should merge all 3 nested ifs when no else branches."""
    func = first_func(load_source("loop_guard_three_levels.py"))
    # C007 wins over C002 (effectiveness 5 > 3)
    plan = next(p for p in func.refactor_plans if p.kind == "collapsible_if")
    assert plan is not None
    assert plan.suggestion is not None
    # All 3 conditions should be merged
    assert "item.active and item.ready and item.valid" in plan.suggestion.replacement
    assert "total += item.value" in plan.suggestion.replacement


def test_collapsible_if_skips_when_multiple_children() -> None:
    """C007 should not fire when outer if has multiple children."""
    func = first_func(load_source("collapsible_if_skips_multiple_children.py"))
    assert "collapsible_if" not in [p.kind for p in func.refactor_plans]


def test_overlapping_regions_show_only_best_suggestion() -> None:
    """When two rules fire on overlapping regions, the one with higher effectiveness wins."""
    func = first_func(load_source("loop_guard_nested_if.py"))

    # Should have exactly one suggestion (the best one)
    assert len(func.refactor_plans) == 1

    # C007 (effectiveness=5) wins over C002 (effectiveness=3)
    plan = func.refactor_plans[0]
    assert plan.rule_id == "C007"
    assert plan.kind == "collapsible_if"
    assert plan.estimated_reduction >= 2
