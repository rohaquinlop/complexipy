"""Tests for the errors the Python API raises and for its use from forked processes."""

from __future__ import annotations

import os
import subprocess
import sys

import pytest

import complexipy


def test_a_missing_file_raises_value_error(tmp_path) -> None:
    with pytest.raises(ValueError):
        complexipy.file_complexity(str(tmp_path / "missing.py"))


def test_code_that_does_not_parse_raises_value_error() -> None:
    with pytest.raises(ValueError):
        complexipy.code_complexity("def broken(:\n")


@pytest.mark.skipif(not hasattr(os, "fork"), reason="needs os.fork")
def test_a_forked_child_runs_parallel_work_after_its_parent(tmp_path) -> None:
    for index in range(4):
        (tmp_path / f"module_{index}.py").write_text(
            "def f():  # complexipy: ignore\n    return 1\n"
        )
    script = (
        "import os, sys, time, warnings\n"
        "import complexipy\n"
        "warnings.simplefilter('ignore')\n"
        "root = sys.argv[1]\n"
        "assert len(complexipy.collect_all_ignored_locations([root], [])[0]) == 4\n"
        "child = os.fork()\n"
        "if child == 0:\n"
        "    found = complexipy.collect_all_ignored_locations([root], [])[0]\n"
        "    os._exit(0 if len(found) == 4 else 3)\n"
        "for _ in range(300):\n"
        "    pid, status = os.waitpid(child, os.WNOHANG)\n"
        "    if pid:\n"
        "        sys.exit(os.WEXITSTATUS(status) if os.WIFEXITED(status) else 1)\n"
        "    time.sleep(0.1)\n"
        "os.kill(child, 9)\n"
        "sys.exit(2)\n"
    )

    result = subprocess.run(
        [sys.executable, "-c", script, str(tmp_path)],
        capture_output=True,
        text=True,
        timeout=120,
    )

    assert result.returncode == 0, result.stderr
