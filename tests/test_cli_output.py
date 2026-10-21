"""Tests for how a CLI run writes its report: pipes, exit codes, color, and Ctrl-C."""

from __future__ import annotations

import os
import subprocess
import sys
from pathlib import Path

from complexipy._complexipy import run_cli

NESTED = (
    "def nested(a, b, c):\n"
    "    if a:\n"
    "        if b:\n"
    "            if c:\n"
    "                return 1\n"
    "    return 0\n"
)

RUN_CLI = (
    "import sys\n"
    "from complexipy._complexipy import run_cli\n"
    "sys.exit(run_cli(sys.argv[1:]))\n"
)


def project(tmp_path: Path) -> Path:
    root = tmp_path / "project"
    root.mkdir()
    for index in range(40):
        (root / f"module_{index}.py").write_text(NESTED)
    return root


def run_console(
    arguments: list[str], cwd: Path, **options
) -> subprocess.CompletedProcess:
    return subprocess.run(
        [sys.executable, "-c", RUN_CLI, *arguments],
        cwd=cwd,
        stderr=subprocess.PIPE,
        text=True,
        timeout=120,
        **options,
    )


def test_a_closed_stdout_ends_the_report_without_a_crash(tmp_path) -> None:
    root = project(tmp_path)
    reader, writer = os.pipe()
    os.close(reader)
    try:
        result = run_console(
            [".", "--suggest-refactors", "--max-complexity-allowed=0"],
            root,
            stdout=writer,
        )
    finally:
        os.close(writer)

    assert "panicked" not in result.stderr
    assert "Traceback" not in result.stderr
    assert result.returncode == 1


def test_quiet_keeps_ignore_complexity(tmp_path) -> None:
    root = project(tmp_path)
    arguments = [".", "--max-complexity-allowed=0", "--ignore-complexity"]

    assert run_cli(arguments, str(root)) == 0
    assert run_cli([*arguments, "--quiet"], str(root)) == 0
    assert (
        run_cli([".", "--max-complexity-allowed=0", "--quiet"], str(root)) == 1
    )


def git(root: Path, *arguments: str) -> None:
    subprocess.run(
        ["git", *arguments], cwd=root, check=True, capture_output=True
    )


def test_a_diff_ref_wider_than_the_terminal_prints(tmp_path) -> None:
    root = tmp_path / "repo"
    root.mkdir()
    git(root, "init", "-q")
    git(root, "config", "user.email", "test@example.com")
    git(root, "config", "user.name", "Test")
    (root / "sample.py").write_text("def nested(a):\n    return a\n")
    git(root, "add", "-A")
    git(root, "commit", "-q", "-m", "base")
    branch = "b" * 120
    git(root, "branch", branch)
    (root / "sample.py").write_text(
        NESTED.replace("nested(a, b, c)", "nested(a)")
    )

    result = run_console([".", "--diff", branch], root, stdout=subprocess.PIPE)

    assert "panicked" not in result.stderr
    assert "Complexity diff (vs bbbb" in result.stdout
    assert "Net: 1 regressed" in result.stdout
