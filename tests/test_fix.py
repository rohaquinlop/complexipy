"""Tests for --fix: applying machine-applicable plans to files on disk."""

from __future__ import annotations

import ast
import inspect
import itertools
import sys
import textwrap
from pathlib import Path

import pytest

from complexipy import Applicability, code_complexity
from complexipy._complexipy import run_cli

FIXTURES = Path(__file__).parent / "fixtures" / "refactor_plans"

FIX_EVERY_FUNCTION = ["--max-complexity-allowed=0", "--ignore-complexity"]

TWO_FIXABLE = textwrap.dedent(
    """\
    def one(a, b, c, d):
        if a:
            if b:
                if c and d:
                    return 1
        return 0


    def two(a, b, c, d):
        if a:
            if b:
                if c and d:
                    return 2
        return 0
    """
)

TWO_FIXABLE_FIXED = textwrap.dedent(
    """\
    def one(a, b, c, d):
        if a and b and c and d:
            return 1
        return 0


    def two(a, b, c, d):
        if a and b and c and d:
            return 2
        return 0
    """
)

MIXED = textwrap.dedent(
    """\
    def one(a, b):
        if a:
            if b:
                return 1
        return 0


    def two(a, b, c):
        d = c
        if (a and b) or (c and d):
            return 2
        return 0
    """
)

MANY_SPOTS = (
    "def many(a, b):\n"
    + "".join(
        f"    if a:\n        if b:\n            return {index}\n"
        for index in range(1, 7)
    )
    + "    return 0\n"
)


def load_source(filename: str) -> str:
    return (FIXTURES / filename).read_text()


def strip_ansi(text: str) -> str:
    result = ""
    index = 0
    while index < len(text):
        if text[index] == "\x1b":
            index += 1
            while index < len(text) and text[index] != "m":
                index += 1
            index += 1
        else:
            result += text[index]
            index += 1
    return result


def first_func(code: str):
    return code_complexity(code).functions[0]


def fixable_plans(code: str) -> list:
    plans = [
        plan
        for func in code_complexity(code).functions
        for plan in func.refactor_plans
        if plan.applicability == Applicability.MachineApplicable
        and plan.suggestion is not None
        and plan.suggestion.spliceable
    ]
    return sorted(plans, key=lambda plan: plan.line_start, reverse=True)


def splice(source: str, plan) -> str:
    lines = source.split("\n")
    replacement = plan.suggestion.replacement.split("\n")
    return (
        "\n".join(lines[: plan.line_start - 1])
        + ("\n" if plan.line_start > 1 else "")
        + "\n".join(replacement)
        + "\n"
        + "\n".join(lines[plan.line_end :])
    )


def expected_fixed(source: str) -> str:
    expected = source
    for plan in fixable_plans(source):
        expected = splice(expected, plan)
    return expected


def test_fix_applies_c002_and_is_idempotent(tmp_path) -> None:
    source = load_source("loop_guard_leading.py")
    target = tmp_path / "loop_guard_leading.py"
    target.write_text(source)
    plan = next(
        plan
        for plan in first_func(source).refactor_plans
        if plan.rule_id == "C002"
    )
    assert plan.reduction_is_measured

    exit_code = run_cli(
        ["--fix", *FIX_EVERY_FUNCTION, str(target)], str(tmp_path)
    )

    assert exit_code == 0
    fixed = target.read_text()
    assert fixed == expected_fixed(source)
    ast.parse(fixed)
    func = first_func(fixed)
    assert all(p.rule_id != "C002" for p in func.refactor_plans)
    assert (
        first_func(source).complexity - func.complexity
        == plan.estimated_reduction
    )


def test_fix_applies_c007_to_the_file(tmp_path) -> None:
    source = load_source("collapsible_if_simple.py")
    target = tmp_path / "collapsible_if_simple.py"
    target.write_text(source)

    exit_code = run_cli(
        ["--fix", *FIX_EVERY_FUNCTION, str(target)], str(tmp_path)
    )

    assert exit_code == 0
    fixed = target.read_text()
    ast.parse(fixed)
    assert "if a and b and c and d:" in fixed
    assert "if a:\n        if b:" not in fixed
    func = first_func(fixed)
    assert all(p.rule_id != "C007" for p in func.refactor_plans)


GUARDED_LOOPS = textwrap.dedent(
    """\
    def members(parts, seen):
        found = []
        for part in parts:
            if part in seen:
                if part.ready:
                    found.append(part)
                else:
                    found.append(None)
        return found


    def identities(parts, marker):
        found = []
        for part in parts:
            if part is not marker:
                if part.ready:
                    found.append(part)
                else:
                    found.append(None)
        return found


    def calls(parts):
        found = []
        for part in parts:
            if isinstance(part, int):
                if part > 0:
                    found.append(part)
                else:
                    found.append(None)
        return found
    """
)


def test_fix_writes_loop_guards_a_linter_accepts(tmp_path) -> None:
    target = tmp_path / "guards.py"
    target.write_text(GUARDED_LOOPS)

    exit_code = run_cli(
        ["--fix", "--select=C002", *FIX_EVERY_FUNCTION, str(target)],
        str(tmp_path),
    )

    assert exit_code == 0
    fixed = target.read_text()
    ast.parse(fixed)
    assert "        if part not in seen:\n            continue\n" in fixed
    assert "        if part is marker:\n            continue\n" in fixed
    assert (
        "        if not isinstance(part, int):\n            continue\n" in fixed
    )
    assert "not (" not in fixed


def test_two_fixes_in_one_file_apply_together(tmp_path) -> None:
    target = tmp_path / "two.py"
    target.write_text(TWO_FIXABLE)
    assert len(fixable_plans(TWO_FIXABLE)) == 2

    exit_code = run_cli(
        ["--fix", *FIX_EVERY_FUNCTION, str(target)], str(tmp_path)
    )

    assert exit_code == 0
    fixed = target.read_text()
    assert fixed == TWO_FIXABLE_FIXED
    assert expected_fixed(TWO_FIXABLE) == TWO_FIXABLE_FIXED
    assert fixed.count("if a and b and c and d:") == 2


def test_fix_keeps_crlf_files_exact(tmp_path) -> None:
    target = tmp_path / "two.py"
    target.write_bytes(TWO_FIXABLE.replace("\n", "\r\n").encode())

    exit_code = run_cli(
        ["--fix", *FIX_EVERY_FUNCTION, str(target)], str(tmp_path)
    )

    assert exit_code == 0
    expected = TWO_FIXABLE_FIXED.replace("\n", "\r\n").encode()
    assert target.read_bytes() == expected


def test_fix_writes_through_a_symlink(tmp_path) -> None:
    source = load_source("collapsible_if_simple.py")
    real = tmp_path / "real.py"
    real.write_text(source)
    link = tmp_path / "link.py"
    try:
        link.symlink_to(real)
    except (OSError, NotImplementedError):
        pytest.skip("symlinks are not available")

    exit_code = run_cli(
        ["--fix", *FIX_EVERY_FUNCTION, str(link)], str(tmp_path)
    )

    assert exit_code == 0
    assert link.is_symlink()
    assert "if a and b and c and d:" in real.read_text()


def test_fix_reports_non_fixable_plans_as_skipped(tmp_path, capfd) -> None:
    target = tmp_path / "mixed.py"
    target.write_text(MIXED)

    exit_code = run_cli(
        ["--fix", *FIX_EVERY_FUNCTION, str(target)], str(tmp_path)
    )

    assert exit_code == 0
    out = strip_ansi(capfd.readouterr().out)
    assert "Fixed C007" in out
    assert "Skipped C005" in out
    assert out.count("not safe to auto-apply") == 1


def test_fix_applies_extract_predicate(tmp_path, capfd) -> None:
    target = tmp_path / "predicate.py"
    target.write_text(
        textwrap.dedent(
            """\
            def check(user, order):
                if user.active and order.total > 10 or user.admin:
                    return 1
                return 0
            """
        )
    )

    exit_code = run_cli(
        ["--fix", *FIX_EVERY_FUNCTION, str(target)], str(tmp_path)
    )

    assert exit_code == 0
    fixed = target.read_text()
    assert fixed.startswith(
        "def _check_condition_L2(user, order):\n"
        "    return user.active and order.total > 10 or user.admin\n"
    )
    assert "    if _check_condition_L2(user, order):\n" in fixed
    ast.parse(fixed)
    assert "Fixed C005" in strip_ansi(capfd.readouterr().out)
    assert not [
        plan
        for function in code_complexity(fixed).functions
        for plan in function.refactor_plans
        if plan.rule_id == "C005"
    ]


def test_later_passes_report_original_file_lines(tmp_path, capfd) -> None:
    target = tmp_path / "many.py"
    target.write_text(MANY_SPOTS)

    exit_code = run_cli(
        ["--fix", *FIX_EVERY_FUNCTION, str(target)], str(tmp_path)
    )

    assert exit_code == 0
    out = strip_ansi(capfd.readouterr().out)
    assert out.count("Fixed C007") == 6
    for span in ("2-4", "5-7", "8-10", "11-13", "14-16"):
        assert f"Fixed C007 at many.py:{span}" in out
    assert "pass 2:" in out
    assert out.index("pass 2:") < out.index("Fixed C007 at many.py:17-19")
    assert "many.py:12-14" not in out


def test_dry_run_prints_the_diff_and_writes_nothing(tmp_path, capfd) -> None:
    source = load_source("collapsible_if_simple.py")
    target = tmp_path / "collapsible_if_simple.py"
    target.write_text(source)

    exit_code = run_cli(
        ["--dry-run", *FIX_EVERY_FUNCTION, str(target)], str(tmp_path)
    )

    assert exit_code == 0
    assert target.read_text() == source
    out = strip_ansi(capfd.readouterr().out)
    assert "--- a/" in out
    assert "+++ b/" in out
    assert "@@" in out
    assert "-    if a:" in out
    assert "+    if a and b and c and d:" in out
    assert "Refactor Suggestions" not in out


def test_dry_run_says_no_fixes_when_nothing_is_fixable(tmp_path, capfd) -> None:
    target = tmp_path / "simple.py"
    target.write_text("def simple(x):\n    return x + 1\n")

    exit_code = run_cli(
        ["--dry-run", *FIX_EVERY_FUNCTION, str(target)], str(tmp_path)
    )

    assert exit_code == 0
    assert "No fixes to apply." in capfd.readouterr().out


def test_fix_says_no_fixes_when_nothing_is_fixable(tmp_path, capfd) -> None:
    target = tmp_path / "simple.py"
    target.write_text("def simple(x):\n    return x + 1\n")

    exit_code = run_cli(
        ["--fix", *FIX_EVERY_FUNCTION, str(target)], str(tmp_path)
    )

    assert exit_code == 0
    assert "No fixes to apply." in strip_ansi(capfd.readouterr().out)


def test_fix_replaces_the_file_in_place_and_leaves_no_temp_files(
    tmp_path,
) -> None:
    source = load_source("collapsible_if_simple.py")
    target = tmp_path / "collapsible_if_simple.py"
    target.write_text(source)

    exit_code = run_cli(
        ["--fix", *FIX_EVERY_FUNCTION, str(target)], str(tmp_path)
    )

    assert exit_code == 0
    assert target.read_text() != source
    entries = [path.name for path in tmp_path.iterdir()]
    assert target.name in entries
    assert all("complexipy-tmp" not in name for name in entries)


def test_fix_prints_the_diff_and_the_summary(tmp_path, capfd) -> None:
    source = load_source("collapsible_if_simple.py")
    target = tmp_path / "collapsible_if_simple.py"
    target.write_text(source)

    exit_code = run_cli(
        ["--fix", *FIX_EVERY_FUNCTION, str(target)], str(tmp_path)
    )

    assert exit_code == 0
    out = strip_ansi(capfd.readouterr().out)
    assert "--- a/" in out
    assert "+++ b/" in out
    assert "-    if a:" in out
    assert "+    if a and b and c and d:" in out
    assert f"Fixed C007 at {target.name}:2-5" in out
    assert "complexity)" in out
    assert out.index("--- a/") < out.index("Fixed C007")


def test_fix_leaves_functions_under_the_threshold_alone(
    tmp_path, capfd
) -> None:
    source = load_source("collapsible_if_simple.py")
    target = tmp_path / "collapsible_if_simple.py"
    target.write_text(source)

    exit_code = run_cli(["--fix", str(target)], str(tmp_path))

    assert exit_code == 0
    assert target.read_text() == source
    out = strip_ansi(capfd.readouterr().out)
    assert "No fixes to apply." in out
    assert "C007" in out


def test_dry_run_on_a_pipe_prints_no_color(tmp_path, capfd) -> None:
    source = load_source("collapsible_if_simple.py")
    target = tmp_path / "collapsible_if_simple.py"
    target.write_text(source)

    run_cli(["--dry-run", *FIX_EVERY_FUNCTION, str(target)], str(tmp_path))

    captured = capfd.readouterr()
    assert "+++ b/" in captured.out
    assert "\x1b" not in captured.out
    assert "\x1b" not in captured.err


def test_color_yes_styles_the_diff_on_a_pipe(tmp_path, capfd) -> None:
    source = load_source("collapsible_if_simple.py")
    target = tmp_path / "collapsible_if_simple.py"
    target.write_text(source)

    run_cli(
        ["--dry-run", "--color", "yes", *FIX_EVERY_FUNCTION, str(target)],
        str(tmp_path),
    )

    assert "\x1b[" in capfd.readouterr().out


def test_toml_select_string_splits_on_commas(tmp_path, capfd) -> None:
    source = load_source("collapsible_if_simple.py")
    target = tmp_path / "collapsible_if_simple.py"
    target.write_text(source)
    (tmp_path / "complexipy.toml").write_text('select = "C007, C002"\n')

    run_cli(["--dry-run", *FIX_EVERY_FUNCTION, str(target)], str(tmp_path))

    captured = capfd.readouterr()
    assert "Unknown rule id" not in captured.err
    assert "C007" in strip_ansi(captured.out)


def test_select_without_a_known_rule_warns_that_none_is_active(
    tmp_path, capfd
) -> None:
    target = tmp_path / "simple.py"
    target.write_text("def simple(x):\n    return x + 1\n")

    run_cli(["--select", "C999", str(target)], str(tmp_path))

    err = strip_ansi(capfd.readouterr().err)
    assert "Unknown rule id 'C999'; it matches no registered rule." in err
    assert "No refactoring rule is active after select and ignore." in err
    assert "Ignoring it" not in err


def test_select_with_one_known_rule_does_not_warn_about_none_active(
    tmp_path, capfd
) -> None:
    target = tmp_path / "simple.py"
    target.write_text("def simple(x):\n    return x + 1\n")

    run_cli(["--select", "C999,C007", str(target)], str(tmp_path))

    err = strip_ansi(capfd.readouterr().err)
    assert "Unknown rule id 'C999'" in err
    assert "No refactoring rule is active" not in err


BEHAVIOR_FIXTURES = sorted(FIXTURES.glob("fix_behavior_*.py"))

REWRITTEN = {
    "fix_behavior_c002_inline_body.py",
    "fix_behavior_c002_mixed_tabs.py",
    "fix_behavior_c002_not_or_without_space.py",
    "fix_behavior_c002_statement_after_inner_member.py",
    "fix_behavior_c007_mixed_tabs.py",
    "fix_behavior_c007_or_without_space.py",
    "fix_behavior_c007_ternary_without_space.py",
}


class StepBudgetExceeded(Exception):
    pass


def outcome(function, arguments) -> str:
    steps = 0

    def tracer(frame, event, arg):
        nonlocal steps
        steps += 1
        if steps > 10_000:
            raise StepBudgetExceeded
        return tracer

    sys.settrace(tracer)
    try:
        return repr(function(*arguments))
    except StepBudgetExceeded:
        return "does not terminate"
    except Exception as error:
        return f"raises {type(error).__name__}"
    finally:
        sys.settrace(None)


def defined_function(source: str, filename: str):
    namespace: dict = {}
    exec(compile(source, filename, "exec"), namespace)
    return namespace["f"]


def argument_sets(function):
    choices = [
        [[1, 2, 3]] if name == "xs" else [True, False]
        for name in inspect.signature(function).parameters
    ]
    return itertools.product(*choices)


@pytest.mark.parametrize(
    "fixture", BEHAVIOR_FIXTURES, ids=lambda path: path.stem
)
def test_fix_keeps_what_the_function_does(tmp_path, fixture) -> None:
    source = fixture.read_text()
    target = tmp_path / fixture.name
    target.write_text(source)

    run_cli(
        [
            "--fix",
            "--max-complexity-allowed=1",
            "--ignore-complexity",
            str(target),
        ],
        str(tmp_path),
    )

    fixed = target.read_text()
    assert (fixed != source) == (fixture.name in REWRITTEN), fixed
    before = defined_function(source, fixture.name)
    after = defined_function(fixed, fixture.name)
    for arguments in argument_sets(before):
        assert outcome(after, arguments) == outcome(before, arguments), (
            arguments,
            fixed,
        )
