"""Tests for the diff API round trip.

`compute_diff` is the only public function that accepts analysis objects
back from Python, so it is the only place the reverse conversion of the
enum fields runs. Before this test the round trip had no coverage at all.
"""

from __future__ import annotations

import subprocess
from pathlib import Path

import pytest

import complexipy

NESTED = (
    "def f(a, b):\n    if a:\n        if b:\n            return 1\n"
    "    return 0\n"
)
PLAIN = "def f(a, b):\n    return 1\n"


def git(root: Path, *arguments: str) -> None:
    subprocess.run(
        ["git", *arguments],
        cwd=root,
        check=True,
        capture_output=True,
    )


def analyze(root: Path) -> complexipy.FileComplexity:
    return complexipy.file_complexity(str(root / "sample.py"))


def commit(root: Path, source: str) -> None:
    (root / "sample.py").write_text(source)
    git(root, "add", "-A")
    git(root, "commit", "-q", "-m", "base")


@pytest.fixture
def repo(tmp_path: Path) -> Path:
    root = tmp_path / "repo"
    root.mkdir()
    git(root, "init", "-q")
    git(root, "config", "user.email", "test@example.com")
    git(root, "config", "user.name", "Test")

    return root


def test_the_round_trip_compares_against_a_git_ref(repo: Path) -> None:
    commit(repo, NESTED)
    (repo / "sample.py").write_text(PLAIN)

    entries = complexipy.compute_diff([analyze(repo)], "HEAD", str(repo))

    assert len(entries) == 1
    entry = entries[0]

    assert entry.func_name == "f"
    assert entry.old_complexity == 3
    assert entry.new_complexity == 0
    assert entry.status is complexipy.DiffStatus.IMPROVED


def test_the_objects_passed_back_carry_enum_fields(repo: Path) -> None:
    commit(repo, NESTED)
    current = [analyze(repo)]
    plan = current[0].functions[0].refactor_plans[0]

    assert type(plan.category) is complexipy.RuleCategory
    assert plan.suggestion is not None
    assert type(plan.suggestion.applicability) is complexipy.Applicability
    assert len(complexipy.compute_diff(current, "HEAD", str(repo))) == 1


def test_a_ratchet_failure_is_reported(repo: Path) -> None:
    commit(repo, PLAIN)
    (repo / "sample.py").write_text(NESTED)

    entries = complexipy.compute_diff([analyze(repo)], "HEAD", str(repo))

    assert entries[0].status is complexipy.DiffStatus.REGRESSED
    assert entries[0].new_complexity == 3
    assert complexipy.has_regressions(entries, 2)
    assert not complexipy.has_regressions(entries, 3)
