from __future__ import annotations

import ast
import textwrap
from pathlib import Path

import complexipy
from complexipy import (
    Applicability,
    CodeSuggestion,
    RefactorPlan,
    code_complexity,
)


def load_source(filename: str) -> str:
    """Load Python source from tests/fixtures/refactor_plans/.

    These fixtures live outside tests/src on purpose: tests/src is the
    corpus that tests/main.py aggregates complexity totals over, and
    these deliberately-complex fixtures would otherwise inflate those
    hardcoded totals.
    """
    path = Path(__file__).parent / "fixtures" / "refactor_plans" / filename
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
    plan = next(
        plan for plan in func.refactor_plans if plan.kind == "collapsible_if"
    )
    assert plan.rule_id == "C007"
    assert plan.estimated_reduction >= 2
    assert plan.applicability == Applicability.MachineApplicable
    assert plan.suggestion is not None
    assert plan.help is None


def test_collapsible_if_skips_when_nested_if_has_preceding_sibling() -> None:
    func = first_func(load_source("collapsible_if_skips_preceding_sibling.py"))

    assert all(plan.kind != "collapsible_if" for plan in func.refactor_plans)


def test_issue_228_does_not_merge_recursive_call_with_nested_if() -> None:
    func = first_func(load_source("issue_228_update_parent_rows.py"))

    assert all(plan.kind != "collapsible_if" for plan in func.refactor_plans)


def test_collapsible_if_skips_sibling_after_innermost_if() -> None:
    func = first_func(load_source("collapsible_if_skips_nested_tail.py"))

    assert all(plan.kind != "collapsible_if" for plan in func.refactor_plans)


def test_collapsible_if_skips_preceding_sibling_after_trailing_comment() -> None:
    func = first_func(
        load_source("collapsible_if_skips_trailing_comment_sibling.py")
    )

    assert all(plan.kind != "collapsible_if" for plan in func.refactor_plans)


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


def test_long_elif_chain_on_single_variable_recommends_match() -> None:
    """Every branch compares `kind` against a literal, so a `match` statement
    is the honest recommendation here: unlike a dispatch dict, it requires no
    extra indirection, and complexipy's own complexity model charges `match` a
    flat cost regardless of case count -- converting genuinely reduces the
    measured complexity, which a dispatch dict wouldn't."""
    func = first_func(load_source("split_dispatcher_elif_chain.py"))

    plan = next(
        plan for plan in func.refactor_plans if plan.kind == "split_dispatcher"
    )
    assert plan.line_start == 2
    assert plan.line_end == 9
    assert plan.applicability == Applicability.Informational
    assert plan.suggestion is None
    assert plan.help is not None
    assert "match kind:" in plan.help


def test_long_elif_chain_on_ranges_recommends_dispatch_dict() -> None:
    """The branches test ranges on the same variable, not equality against
    literals, so this shape can't become a `match` with plain `case <value>:`
    arms -- the dispatch-dict suggestion still applies here."""
    func = first_func(load_source("split_dispatcher_range_chain.py"))

    plan = next(
        plan for plan in func.refactor_plans if plan.kind == "split_dispatcher"
    )
    assert plan.applicability == Applicability.Informational
    assert plan.suggestion is None
    assert plan.help is not None
    assert "dispatch dictionary" in plan.help
    assert "match" not in plan.help


def test_match_statement_never_creates_dispatcher_plan() -> None:
    """`match` costs a flat `1 + nesting` regardless of case count, so
    "split this dispatcher" is never an honest suggestion for a `match` --
    unlike an `elif` chain, splitting it wouldn't reduce the measured
    complexity. C004 fires on `elif` chains only; a `match` with many
    cases and no other nested complexity gets no dispatcher suggestion,
    even though it has the same shape an `elif` chain would trigger on."""
    func = first_func(load_source("match_statement_no_dispatcher_plan.py"))

    assert "split_dispatcher" not in plan_kinds(func)


def test_boolean_heavy_condition_creates_predicate_plan() -> None:
    func = first_func(load_source("extract_predicate_boolean.py"))

    plan = next(
        plan for plan in func.refactor_plans if plan.kind == "extract_predicate"
    )
    assert plan.line_start == 2
    assert plan.line_end == 2
    assert plan.applicability == Applicability.MachineApplicable
    assert plan.suggestion is not None
    assert "def _check_condition_L" in plan.suggestion.replacement
    assert plan.help is None


def test_try_nested_through_with_creates_flatten_try_plan() -> None:
    func = first_func(load_source("flatten_try_with_nested.py"))

    plan = next(
        plan for plan in func.refactor_plans if plan.kind == "flatten_try"
    )
    assert plan.rule_id == "C011"
    assert plan.applicability == Applicability.Informational
    assert plan.suggestion is None
    assert plan.help is not None
    assert plan.estimated_reduction >= 1


def test_simple_function_creates_no_refactor_plans() -> None:
    func = first_func(load_source("simple_function_no_plans.py"))

    assert func.complexity == 0
    assert func.refactor_plans == []


def test_refactor_plan_exported_and_available_on_python_api() -> None:
    func = first_func(load_source("extract_predicate_boolean.py"))

    assert complexipy.RefactorPlan is RefactorPlan
    assert isinstance(func.refactor_plans[0], RefactorPlan)
    assert func.refactor_plans[0].estimated_complexity_after <= func.complexity


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
    """Every plan must carry the metadata a clippy-style report needs, `doc_url`
    included.

    The `doc_url` assertions are the point of this test and were missing until
    the metadata layer was wired in: the field existed on `RuleMetadata` but
    never reached `RefactorPlan`, so Python rendered links from a hardcoded
    duplicate map that had no entry for C007 -- C007 plans showed no
    `References:` section at all. This fixture yields exactly one plan, C007,
    so it guards that regression directly.
    """
    func = first_func(load_source("metadata_validation.py"))
    # Without this the loop below is vacuous: a fixture that stops producing
    # plans would let every assertion pass by never running.
    assert func.refactor_plans, "fixture produced no plans to validate"
    for plan in func.refactor_plans:
        assert plan.rule_id.startswith("C")
        assert plan.category is not None
        assert plan.applicability is not None
        assert plan.description
        assert plan.explanation
        assert plan.doc_url, f"{plan.rule_id} carries no doc_url"
        assert plan.doc_url.startswith("https://")
        # Ties the URL to its own rule, so a copy-pasted link from a
        # neighbouring rule fails rather than passing a generic prefix check.
        assert plan.rule_id.lower() in plan.doc_url


def test_code_generation_produces_nonempty_snippets() -> None:
    func = first_func(load_source("code_generation_flatten.py"))
    flatten_plan = next(
        (p for p in func.refactor_plans if p.kind == "flatten_condition"), None
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
    assert (
        "if item.value > threshold:" in loop_guard_plan.suggestion.replacement
    )
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
    assert (
        "item.active and item.ready and item.valid"
        in plan.suggestion.replacement
    )
    assert "total += item.value" in plan.suggestion.replacement


def test_collapsible_if_skips_when_multiple_children() -> None:
    """C007 should not fire when outer if has multiple children."""
    func = first_func(load_source("collapsible_if_skips_multiple_children.py"))
    assert "collapsible_if" not in [p.kind for p in func.refactor_plans]


def test_code_suggestion_is_importable_and_used_by_refactor_plan() -> None:
    """CodeSuggestion must be importable and be the actual type of plan.suggestion."""
    func = first_func(load_source("collapsible_if_simple.py"))
    plan = next(p for p in func.refactor_plans if p.kind == "collapsible_if")

    assert plan.suggestion is not None
    assert isinstance(plan.suggestion, CodeSuggestion)
    assert isinstance(plan.suggestion.replacement, str)
    assert plan.suggestion.applicability == Applicability.MachineApplicable
    assert isinstance(plan.suggestion.description, str)


def test_trailing_comment_with_colon_does_not_corrupt_suggestion() -> None:
    """Regression test for the extract_condition_from_line rfind(':') bug.

    `if a:  # gate: primary` used to have its condition extracted via
    `rfind(':')`, which matched the colon inside the trailing comment instead
    of the statement colon, corrupting the merged condition into invalid
    Python (`if a:  # gate and b:`). The suggestion must either be a correct,
    parseable merge, or absent with help text -- never invalid.
    """
    func = first_func(load_source("collapsible_if_comment_colon.py"))
    plan = next(p for p in func.refactor_plans if p.kind == "collapsible_if")
    assert plan.rule_id == "C007"

    if plan.suggestion is not None:
        assert "gate" not in plan.suggestion.replacement
        assert "#" not in plan.suggestion.replacement
        assert "a and b" in plan.suggestion.replacement
        ast.parse(textwrap.dedent(plan.suggestion.replacement))
    else:
        assert plan.help is not None


def test_multiline_condition_produces_no_suggestion_but_keeps_help() -> None:
    """A condition spanning multiple lines via an unclosed '(' can't be safely
    read from a single line, so no machine-applicable suggestion should be
    emitted -- the plan must still carry help text and complexity numbers.
    """
    func = first_func(load_source("collapsible_if_multiline_condition.py"))
    plan = next(p for p in func.refactor_plans if p.kind == "collapsible_if")
    assert plan.rule_id == "C007"
    assert plan.suggestion is None
    assert plan.help is not None
    assert plan.current_complexity == func.complexity
    assert plan.estimated_complexity_after <= plan.current_complexity


def test_walrus_condition_keeps_its_assignment_in_the_merge() -> None:
    """An unparenthesized walrus must survive condition extraction.

    The `:` in `:=` sits at bracket depth 0, so a scanner that takes the first
    depth-0 colon as the statement colon extracted just `n` from
    `if n := len(items):` and emitted `if n and n < 9:` -- valid Python that
    silently drops the assignment, which no parse check can detect.
    """
    func = first_func(load_source("collapsible_if_walrus.py"))
    plan = next(p for p in func.refactor_plans if p.kind == "collapsible_if")
    assert plan.rule_id == "C007"

    if plan.suggestion is not None:
        replacement = plan.suggestion.replacement
        assert ":=" in replacement
        assert "len(items)" in replacement
        assert "n < 9" in replacement
        ast.parse(textwrap.dedent(replacement))

        # `:=` binds looser than `and`, so the walrus must be parenthesized:
        # bare `n := len(items) and n < 9` parses but assigns the *conjunction*.
        merged = ast.parse(
            replacement.strip().splitlines()[0] + "\n    pass"
        ).body[0]
        assert isinstance(merged, ast.If)
        assert isinstance(merged.test, ast.BoolOp), (
            f"merged condition must be a boolean AND, got "
            f"{type(merged.test).__name__}: {ast.unparse(merged.test)}"
        )
        assert isinstance(merged.test.op, ast.And)
        assert isinstance(merged.test.values[0], ast.NamedExpr)
    else:
        assert plan.help is not None


def test_all_refactor_plan_suggestions_are_parseable_python() -> None:
    """Global safety net over every fixture: a `suggestion.replacement` must be
    valid Python, and must never carry a `#` comment.

    What this catches: replacements that don't parse (it found a real dangling
    `if _check_condition_L2():` with no body in the predicate rule), and any
    comment text leaking into a generated condition.

    What it deliberately does NOT claim to catch: semantic corruption that still
    parses. The original rfind(':') bug emitted `if a:  # gate and b:`, which is
    *valid* Python -- the dropped `b` condition hides inside a comment -- so an
    ast.parse check alone accepts it. The `#` assertion below is what closes that
    specific hole; corruption that neither breaks parsing nor leaves a comment
    behind is covered only by the per-rule tests above, not here.
    """
    fixtures_dir = Path(__file__).parent / "fixtures" / "refactor_plans"
    checked = 0
    for fixture_path in sorted(fixtures_dir.glob("*.py")):
        source = fixture_path.read_text()
        result = code_complexity(source)
        for func in result.functions:
            for plan in func.refactor_plans:
                if plan.suggestion is None:
                    # Design constraint: a missing suggestion is fine as long as
                    # help text explains what to do instead.
                    assert plan.help is not None, (
                        f"{fixture_path.name}: plan {plan.rule_id} ({plan.kind}) "
                        "has neither a suggestion nor help text"
                    )
                    continue

                replacement = plan.suggestion.replacement
                checked += 1
                try:
                    ast.parse(textwrap.dedent(replacement))
                except SyntaxError as exc:
                    raise AssertionError(
                        f"{fixture_path.name}: plan {plan.rule_id} ({plan.kind}) "
                        f"produced a suggestion.replacement that is not valid "
                        f"Python after dedenting: {exc}\n--- replacement ---\n"
                        f"{replacement}\n--------------------"
                    ) from exc

                assert "#" not in replacement, (
                    f"{fixture_path.name}: plan {plan.rule_id} ({plan.kind}) "
                    f"leaked a comment into its replacement -- a generated "
                    f"condition must never contain '#'\n--- replacement ---\n"
                    f"{replacement}\n--------------------"
                )

    assert checked > 0, "expected at least one fixture to produce a suggestion"


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


# --- Reduction-math ground truth ------------------------------------------
#
# Each case below measures a fixture's real complexity, measures the real
# complexity of the actual refactored code the rule's suggestion describes,
# and asserts the rule's `estimated_reduction` against that measured delta --
# the only acceptable ground truth (see AGENTS.md / rules/registry.rs). A
# rule may UNDERstate (an honest, conservative estimate) but must never
# OVERstate: the tool must never claim more reduction than the refactor
# actually delivers.


def measured_reduction(after_filename: str, before_complexity: int) -> int:
    after_source = load_source(after_filename)
    after_complexity = code_complexity(after_source).functions[0].complexity
    return before_complexity - after_complexity


def test_collapsible_if_reduction_is_not_floored_to_two() -> None:
    """Regression test for the `.max(2)` floor: a 2-level merge whose honest
    reduction is 1 must report 1, not a fabricated 2."""
    func = first_func(load_source("reduction_math_two_level.py"))
    plan = next(p for p in func.refactor_plans if p.kind == "collapsible_if")

    real_reduction = measured_reduction(
        "reduction_math_two_level_after.py", func.complexity
    )
    assert real_reduction == 1
    assert plan.estimated_reduction == real_reduction


def test_collapsible_if_or_mixing_does_not_overstate_reduction() -> None:
    """Regression test for undercounted booleans: merging an `or`-condition
    with another `if` forces parens, which is real complexity the naive
    "one operator per pair" estimate ignored, overstating the reduction."""
    func = first_func(load_source("collapsible_if_with_or.py"))
    plan = next(p for p in func.refactor_plans if p.kind == "collapsible_if")

    real_reduction = measured_reduction(
        "reduction_math_or_mixing_after.py", func.complexity
    )
    assert real_reduction == 1
    assert plan.estimated_reduction == real_reduction


def test_split_dispatcher_elif_reduction_does_not_overstate() -> None:
    """C004 (Informational, help-only) estimates a lower bound. A dispatch
    dict is one valid interpreted (not machine-generated) refactor of this
    chain, achieving at least as much reduction as the estimate -- this holds
    regardless of which concrete refactor the help text ends up recommending
    for a given chain's shape."""
    func = first_func(load_source("split_dispatcher_elif_chain.py"))
    plan = next(p for p in func.refactor_plans if p.kind == "split_dispatcher")

    real_reduction = measured_reduction(
        "reduction_math_dispatcher_elif_after.py", func.complexity
    )
    assert plan.estimated_reduction <= real_reduction


def test_collapsible_if_with_unabsorbed_tail_does_not_overstate_reduction() -> (
    None
):
    """Regression test: when the if-chain doesn't reach all the way down to a
    leaf statement (here, a `for` loop the chain can't absorb), that tail
    survives the merge -- just dedented -- so it must still be charged
    against `new_complexity`. Before this fix, `old_complexity` (which rolls
    up the whole subtree) was compared against only the merged if's own cost,
    silently dropping the tail and overstating the reduction 2x."""
    func = first_func(
        load_source("reduction_math_collapsible_if_remaining_subtree.py")
    )
    plan = next(p for p in func.refactor_plans if p.kind == "collapsible_if")

    real_reduction = measured_reduction(
        "reduction_math_collapsible_if_remaining_subtree_after.py",
        func.complexity,
    )
    assert real_reduction == 2
    assert 1 <= plan.estimated_reduction <= real_reduction


def test_loop_guard_with_else_reduction_matches_measured_delta() -> None:
    """C002's reduction now comes from the actual nesting delta of hoisting a
    guard chain (plus the tail that gets dedented along with it), not from
    summing every nested if's raw nesting value regardless of chain shape."""
    func = first_func(load_source("loop_guard_with_else.py"))
    plan = next(p for p in func.refactor_plans if p.kind == "loop_guards")

    real_reduction = measured_reduction(
        "reduction_math_loop_guard_with_else_after.py", func.complexity
    )
    assert real_reduction == 1
    assert plan.estimated_reduction == real_reduction


def splice_and_measure(source: str, plan: RefactorPlan) -> int:
    """Apply a plan's suggestion by hand and re-score the result.

    Independent Python-side ground truth for the Rust measurement pass: the
    plan's numbers must equal what splicing and re-scoring actually produce.
    """
    lines = source.split("\n")
    spliced = "\n".join(
        lines[: plan.line_start - 1]
        + [plan.suggestion.replacement]  # type: ignore[union-attr]
        + lines[plan.line_end :]
    )
    return code_complexity(spliced).functions[0].complexity


def test_loop_guard_leading_suggestion_is_faithful_and_measured() -> None:
    func = first_func(load_source("loop_guard_leading.py"))
    plan = next(p for p in func.refactor_plans if p.kind == "loop_guards")

    assert plan.suggestion is not None
    assert plan.suggestion.spliceable
    assert plan.reduction_is_measured
    assert plan.estimated_reduction == func.complexity - splice_and_measure(
        load_source("loop_guard_leading.py"), plan
    )


def test_loop_guard_trailing_suggestion_is_faithful_and_measured() -> None:
    func = first_func(load_source("loop_guard_trailing.py"))
    plan = next(p for p in func.refactor_plans if p.kind == "loop_guards")

    assert plan.suggestion is not None
    assert plan.suggestion.spliceable
    assert plan.reduction_is_measured
    assert plan.estimated_complexity_after == splice_and_measure(
        load_source("loop_guard_trailing.py"), plan
    )


def test_loop_guard_multiline_header_suggestion_is_faithful_and_measured() -> (
    None
):
    func = first_func(load_source("loop_guard_multiline_header.py"))
    plan = next(p for p in func.refactor_plans if p.kind == "loop_guards")

    assert plan.suggestion is not None
    assert plan.suggestion.spliceable
    assert plan.reduction_is_measured
    assert plan.estimated_complexity_after == splice_and_measure(
        load_source("loop_guard_multiline_header.py"), plan
    )


def test_reduction_is_measured_flag_marks_spliced_and_estimated_plans() -> None:
    # Machine-applicable spliceable rules measure their reduction.
    func = first_func(load_source("collapsible_if_simple.py"))
    collapsible = next(
        p for p in func.refactor_plans if p.kind == "collapsible_if"
    )
    assert collapsible.reduction_is_measured

    # Help-only rules keep a formula estimate.
    func = first_func(load_source("flatten_try_with_nested.py"))
    flatten = next(p for p in func.refactor_plans if p.kind == "flatten_try")
    assert not flatten.reduction_is_measured
    assert flatten.suggestion is None

    # C005 carries a suggestion but it is a snippet with a placeholder body,
    # not a faithful splice -- it stays estimated.
    func = first_func(load_source("extract_predicate_boolean.py"))
    predicate = next(
        p for p in func.refactor_plans if p.kind == "extract_predicate"
    )
    assert predicate.suggestion is not None
    assert not predicate.suggestion.spliceable
    assert not predicate.reduction_is_measured
