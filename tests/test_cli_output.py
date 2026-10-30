"""Tests for how a CLI run writes its report: pipes, exit codes, color, and Ctrl-C."""

from __future__ import annotations

import os
import signal
import subprocess
import sys
import threading
from pathlib import Path

import pytest

from complexipy import cli
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


def project(tmp_path: Path, files: int = 40) -> Path:
    root = tmp_path / "project"
    root.mkdir()
    for index in range(files):
        (root / f"module_{index}.py").write_text(NESTED)
    return root


def environment_without_color_overrides() -> dict[str, str]:
    overrides = (
        "NO_COLOR",
        "FORCE_COLOR",
        "CLICOLOR_FORCE",
        "IGNORE_IS_TERMINAL",
    )
    return {
        key: value for key, value in os.environ.items() if key not in overrides
    }


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


@pytest.mark.parametrize(
    "arguments, environment",
    [
        (["--color", "no"], {}),
        ([], {"NO_COLOR": "1"}),
        ([], {}),
    ],
    ids=["color-no", "no-color-env", "pipe"],
)
def test_suggestions_carry_no_color_when_color_is_off(
    tmp_path, arguments, environment
) -> None:
    root = project(tmp_path)
    result = run_console(
        [".", "--suggest-refactors", *arguments],
        root,
        stdout=subprocess.PIPE,
        env={**environment_without_color_overrides(), **environment},
    )

    assert "Replacement:" in result.stdout
    assert "\x1b[" not in result.stdout


@pytest.mark.skipif(sys.platform == "win32", reason="needs a pseudo-terminal")
def test_color_is_decided_again_on_each_run(tmp_path) -> None:
    root = project(tmp_path, files=1)
    script = (
        "import os, pty, sys\n"
        "from complexipy._complexipy import run_cli\n"
        "leader, follower = pty.openpty()\n"
        "saved = os.dup(1)\n"
        "os.dup2(follower, 1)\n"
        "run_cli(['.', '-C', 'auto'])\n"
        "reader, writer = os.pipe()\n"
        "os.dup2(writer, 1)\n"
        "os.close(writer)\n"
        "run_cli(['.', '-C', 'auto'])\n"
        "os.dup2(saved, 1)\n"
        "chunks = []\n"
        "while True:\n"
        "    chunk = os.read(reader, 65536)\n"
        "    if not chunk:\n"
        "        break\n"
        "    chunks.append(chunk)\n"
        "sys.stderr.write(repr(b''.join(chunks)))\n"
    )
    environment = environment_without_color_overrides()
    environment["TERM"] = "xterm-256color"
    result = subprocess.run(
        [sys.executable, "-c", script],
        cwd=root,
        capture_output=True,
        text=True,
        timeout=120,
        env=environment,
    )

    assert "complexipy" in result.stderr
    assert "\\x1b[" not in result.stderr


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


def test_the_console_script_lets_ctrl_c_end_the_run(monkeypatch) -> None:
    seen = {}

    def record_handler(arguments: list[str]) -> int:
        seen["handler"] = signal.getsignal(signal.SIGINT)
        return 0

    monkeypatch.setattr(cli, "run_cli", record_handler)
    monkeypatch.setattr(sys, "argv", ["complexipy", "."])
    previous = signal.getsignal(signal.SIGINT)
    try:
        with pytest.raises(SystemExit):
            cli.main()
    finally:
        signal.signal(signal.SIGINT, previous)

    assert seen["handler"] == signal.SIG_DFL


def test_the_console_script_runs_from_a_thread(monkeypatch) -> None:
    outcome = {}

    def record_arguments(arguments: list[str]) -> int:
        outcome["arguments"] = arguments
        return 0

    def run_main() -> None:
        try:
            cli.main()
        except SystemExit as stop:
            outcome["exit"] = stop.code
        except BaseException as error:
            outcome["error"] = error

    monkeypatch.setattr(cli, "run_cli", record_arguments)
    monkeypatch.setattr(sys, "argv", ["complexipy", "."])
    previous = signal.getsignal(signal.SIGINT)
    thread = threading.Thread(target=run_main)
    thread.start()
    thread.join()

    assert outcome == {"arguments": ["."], "exit": 0}
    assert signal.getsignal(signal.SIGINT) == previous


def test_explain_prints_the_rule_without_reading_config_or_paths(
    tmp_path,
) -> None:
    (tmp_path / "complexipy.toml").write_text("not = [valid toml\n")
    result = run_console(
        ["--explain", "c007", "missing.py"],
        tmp_path,
        stdout=subprocess.PIPE,
    )

    assert result.returncode == 0
    assert result.stderr == ""
    assert result.stdout.startswith("C007: collapsible_if\n")
    assert (
        "https://complexipy.com/refactoring-rules/#c007-collapsible-if"
        in result.stdout
    )


def test_explain_rejects_an_unknown_rule_and_lists_the_valid_ids(
    tmp_path,
) -> None:
    result = run_console(
        ["--explain", "C999"], tmp_path, stdout=subprocess.PIPE
    )

    assert result.returncode == 1
    assert result.stdout == ""
    assert "'C999'" in result.stderr
    assert "C001, C002, C003, C004, C005, C007, C011" in result.stderr
