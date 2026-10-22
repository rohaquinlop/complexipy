"""Tests for --diff and --diff-only runs: which history they read, and what they warn about."""

from __future__ import annotations

import re
import subprocess
from pathlib import Path

import pytest

from complexipy._complexipy import run_cli

PLAIN = "def f(a, b):\n    return 1\n"
NESTED = (
    "def f(a, b):\n    if a:\n        if b:\n            return 1\n"
    "    return 0\n"
)

ANSI = re.compile(r"\x1b\[[0-9;]*m")


def strip_ansi(text: str) -> str:
    return ANSI.sub("", text)


def git(root: Path, *arguments: str) -> None:
    subprocess.run(
        ["git", *arguments],
        cwd=root,
        check=True,
        capture_output=True,
    )


def commit(root: Path, source: str) -> None:
    (root / "pkg" / "mod.py").write_text(source)
    git(root, "add", "-A")
    git(root, "commit", "-q", "-m", "base")
    git(root, "tag", "v1")


@pytest.fixture
def repo(tmp_path: Path) -> Path:
    root = tmp_path / "repo"
    (root / "pkg").mkdir(parents=True)
    git(root, "init", "-q")
    git(root, "config", "user.email", "test@example.com")
    git(root, "config", "user.name", "Test")
    return root


@pytest.fixture
def outside(tmp_path: Path) -> Path:
    directory = tmp_path / "outside"
    directory.mkdir()
    return directory


def test_a_run_from_outside_the_repository_reads_the_files_history(
    repo: Path, outside: Path, capfd: pytest.CaptureFixture[str]
) -> None:
    commit(repo, PLAIN)
    (repo / "pkg" / "mod.py").write_text(NESTED)

    exit_code = run_cli([str(repo / "pkg"), "--diff-only", "v1"], str(outside))

    out = strip_ansi(capfd.readouterr().out)
    assert exit_code == 0
    assert "Net: 1 regressed" in out
    assert "NEW" not in out
    assert "Warning:" not in out


def test_the_gate_from_outside_the_repository_passes_an_unchanged_function(
    repo: Path, outside: Path
) -> None:
    commit(repo, NESTED)

    exit_code = run_cli(
        [str(repo / "pkg"), "--diff", "v1", "--max-complexity-allowed", "2"],
        str(outside),
    )

    assert exit_code == 0


def test_diff_only_alone_prints_no_flag_warning(
    repo: Path, capfd: pytest.CaptureFixture[str]
) -> None:
    commit(repo, PLAIN)

    run_cli([str(repo / "pkg"), "--diff-only", "v1"], str(repo))

    assert "both set" not in capfd.readouterr().out


def test_both_diff_flags_print_the_flag_warning(
    repo: Path, capfd: pytest.CaptureFixture[str]
) -> None:
    commit(repo, PLAIN)

    run_cli([str(repo / "pkg"), "--diff", "v1", "--diff-only", "v1"], str(repo))

    out = strip_ansi(capfd.readouterr().out)
    assert "Warning: --diff and --diff-only both set." in out


def test_a_ref_missing_from_the_repository_is_named_in_a_warning(
    repo: Path, capfd: pytest.CaptureFixture[str]
) -> None:
    commit(repo, PLAIN)

    run_cli([str(repo / "pkg"), "--diff-only", "v9"], str(repo))

    out = strip_ansi(capfd.readouterr().out)
    assert 'Warning: "v9" does not name a commit in ' in out
    assert "its 1 analyzed file(s) report as NEW." in out


def test_a_file_outside_any_repository_is_named_in_a_warning(
    outside: Path, capfd: pytest.CaptureFixture[str]
) -> None:
    (outside / "loose.py").write_text(PLAIN)

    run_cli([str(outside / "loose.py"), "--diff-only", "HEAD"], str(outside))

    out = strip_ansi(capfd.readouterr().out)
    assert (
        "Warning: loose.py is not inside a git repository, so all its "
        "functions report as NEW." in out
    )
