from __future__ import annotations

import ast
import textwrap
from pathlib import Path

import pytest

from complexipy import Applicability, code_complexity
from complexipy._complexipy import run_cli

FIXTURES = Path(__file__).parent / "fixtures" / "refactor_plans"

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


    def two(a, b, c, d):
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

    exit_code = run_cli(["--fix", str(target)], str(tmp_path))

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

    exit_code = run_cli(["--fix", str(target)], str(tmp_path))

    assert exit_code == 0
    fixed = target.read_text()
    ast.parse(fixed)
    assert "if a and b and c and d:" in fixed
    assert "if a:\n        if b:" not in fixed
    func = first_func(fixed)
    assert all(p.rule_id != "C007" for p in func.refactor_plans)


def test_two_fixes_in_one_file_apply_together(tmp_path) -> None:
    target = tmp_path / "two.py"
    target.write_text(TWO_FIXABLE)
    assert len(fixable_plans(TWO_FIXABLE)) == 2

    exit_code = run_cli(["--fix", str(target)], str(tmp_path))

    assert exit_code == 0
    fixed = target.read_text()
    assert fixed == TWO_FIXABLE_FIXED
    assert expected_fixed(TWO_FIXABLE) == TWO_FIXABLE_FIXED
    assert fixed.count("if a and b and c and d:") == 2


def test_fix_keeps_crlf_files_exact(tmp_path) -> None:
    target = tmp_path / "two.py"
    target.write_bytes(TWO_FIXABLE.replace("\n", "\r\n").encode())

    exit_code = run_cli(["--fix", str(target)], str(tmp_path))

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

    exit_code = run_cli(["--fix", str(link)], str(tmp_path))

    assert exit_code == 0
    assert link.is_symlink()
    assert "if a and b and c and d:" in real.read_text()


def test_fix_reports_non_fixable_plans_as_skipped(tmp_path, capfd) -> None:
    target = tmp_path / "mixed.py"
    target.write_text(MIXED)

    exit_code = run_cli(["--fix", str(target)], str(tmp_path))

    assert exit_code == 0
    out = strip_ansi(capfd.readouterr().out)
    assert "Fixed C007" in out
    assert "Skipped C005" in out
    assert out.count("not safe to auto-apply") == 1


def test_later_passes_report_original_file_lines(tmp_path, capfd) -> None:
    target = tmp_path / "many.py"
    target.write_text(MANY_SPOTS)

    exit_code = run_cli(["--fix", str(target)], str(tmp_path))

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

    exit_code = run_cli(["--dry-run", str(target)], str(tmp_path))

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

    exit_code = run_cli(["--dry-run", str(target)], str(tmp_path))

    assert exit_code == 0
    assert "No fixes to apply." in capfd.readouterr().out


def test_fix_says_no_fixes_when_nothing_is_fixable(tmp_path, capfd) -> None:
    target = tmp_path / "simple.py"
    target.write_text("def simple(x):\n    return x + 1\n")

    exit_code = run_cli(["--fix", str(target)], str(tmp_path))

    assert exit_code == 0
    assert "No fixes to apply." in strip_ansi(capfd.readouterr().out)


def test_fix_replaces_the_file_in_place_and_leaves_no_temp_files(
    tmp_path,
) -> None:
    source = load_source("collapsible_if_simple.py")
    target = tmp_path / "collapsible_if_simple.py"
    target.write_text(source)

    exit_code = run_cli(["--fix", str(target)], str(tmp_path))

    assert exit_code == 0
    assert target.read_text() != source
    entries = [path.name for path in tmp_path.iterdir()]
    assert target.name in entries
    assert all("complexipy-tmp" not in name for name in entries)


def test_fix_prints_the_diff_and_the_summary(tmp_path, capfd) -> None:
    source = load_source("collapsible_if_simple.py")
    target = tmp_path / "collapsible_if_simple.py"
    target.write_text(source)

    exit_code = run_cli(["--fix", str(target)], str(tmp_path))

    assert exit_code == 0
    out = strip_ansi(capfd.readouterr().out)
    assert "--- a/" in out
    assert "+++ b/" in out
    assert "-    if a:" in out
    assert "+    if a and b and c and d:" in out
    assert f"Fixed C007 at {target.name}:2-5" in out
    assert "complexity)" in out
    assert out.index("--- a/") < out.index("Fixed C007")
