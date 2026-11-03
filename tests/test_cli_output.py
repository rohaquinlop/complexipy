"""Tests for how a CLI run writes its report: pipes, exit codes, color, and Ctrl-C."""

from __future__ import annotations

import json
import os
import signal
import subprocess
import sys
import threading
from pathlib import Path

import pytest

from complexipy import cli, file_complexity
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
        **{"stderr": subprocess.PIPE, "text": True, "timeout": 120, **options},
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


def commit_all(root: Path) -> None:
    git(root, "init", "-q")
    git(root, "config", "user.email", "test@example.com")
    git(root, "config", "user.name", "Test")
    git(root, "add", "-A")
    git(root, "commit", "-q", "-m", "base")


def test_a_diff_ref_wider_than_the_terminal_prints(tmp_path) -> None:
    root = tmp_path / "repo"
    root.mkdir()
    (root / "sample.py").write_text("def nested(a):\n    return a\n")
    commit_all(root)
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


def run_binary(arguments: list[str], cwd: Path) -> subprocess.CompletedProcess:
    return run_console(
        arguments,
        cwd,
        stdout=subprocess.PIPE,
        text=False,
        env={**environment_without_color_overrides(), "NO_COLOR": "1"},
    )


def assert_stdout_matches_file_report(
    arguments: list[str], root: Path
) -> subprocess.CompletedProcess:
    report = root.parent / "report.out"
    to_file = run_binary([*arguments, "--output", str(report)], root)
    to_stdout = run_binary([*arguments, "--output", "-"], root)

    assert to_stdout.stdout == report.read_bytes()
    assert to_stdout.returncode == to_file.returncode
    return to_stdout


@pytest.mark.parametrize("threshold", ["15", "0"])
@pytest.mark.parametrize(
    "output_format, refactors",
    [
        ("json", []),
        ("json", ["--suggest-refactors"]),
        ("csv", []),
        ("gitlab", ["--suggest-refactors"]),
        ("sarif", ["--suggest-refactors"]),
    ],
)
def test_output_dash_writes_only_the_report_to_stdout(
    tmp_path, output_format, refactors, threshold
) -> None:
    root = project(tmp_path, files=3)
    result = assert_stdout_matches_file_report(
        [
            ".",
            f"--output-format={output_format}",
            f"--max-complexity-allowed={threshold}",
            *refactors,
        ],
        root,
    )

    if output_format != "csv":
        json.loads(result.stdout)


@pytest.mark.parametrize("diff_flag", ["--diff", "--diff-only"])
def test_output_dash_keeps_the_diff_table_off_stdout(
    tmp_path, diff_flag
) -> None:
    root = project(tmp_path, files=3)
    commit_all(root)
    (root / "module_0.py").write_text(
        NESTED + "\n\n" + NESTED.replace("nested", "added")
    )

    result = assert_stdout_matches_file_report(
        [
            ".",
            "--output-format=sarif",
            f"{diff_flag}=HEAD",
            "--max-complexity-allowed=0",
        ],
        root,
    )

    json.loads(result.stdout)


def test_output_dash_from_config_writes_the_report_to_stdout(tmp_path) -> None:
    root = project(tmp_path, files=3)
    (root / "complexipy.toml").write_text(
        'output = "-"\noutput-format = ["json"]\n'
    )
    report = tmp_path / "report.out"

    to_stdout = run_binary(["."], root)
    run_binary([".", "--output", str(report)], root)

    assert to_stdout.stdout == report.read_bytes()


def test_output_dash_with_two_formats_fails_before_writing(tmp_path) -> None:
    root = project(tmp_path, files=3)
    result = run_binary(
        [".", "--output-format=json,csv", "--output", "-", "--snapshot-create"],
        root,
    )

    assert result.returncode == 1
    assert result.stdout == b""
    assert b"select exactly one --output-format" in result.stderr
    assert not (root / "complexipy-snapshot.json").exists()


def test_output_dash_without_a_format_keeps_the_console_report(
    tmp_path,
) -> None:
    root = project(tmp_path, files=3)
    plain = run_binary([".", "--max-complexity-allowed=0"], root)
    dashed = run_binary(
        [".", "--max-complexity-allowed=0", "--output", "-"], root
    )

    assert dashed.returncode == plain.returncode == 1
    assert b"nested" in dashed.stdout
    assert dashed.stderr == plain.stderr


def test_output_dash_sends_invalid_paths_to_stderr(tmp_path) -> None:
    root = project(tmp_path, files=3)
    result = run_binary(
        [".", "missing.py", "--output-format=json", "--output", "-"], root
    )

    assert result.returncode == 1
    assert len(json.loads(result.stdout)) == 3
    assert b"Failed to process missing.py" in result.stderr


def test_output_dash_with_report_ignored_writes_the_ignored_file(
    tmp_path,
) -> None:
    root = project(tmp_path, files=1)
    (root / "skipped.py").write_text(
        "def skipped():  # complexipy: ignore\n    return 1\n"
    )
    result = assert_stdout_matches_file_report(
        [".", "--output-format=json", "--report-ignored"], root
    )

    json.loads(result.stdout)
    assert (root / "complexipy-ignored.json").exists()
    assert b"complexipy-ignored.json" in result.stderr


def test_output_dash_survives_a_closed_stdout(tmp_path) -> None:
    root = project(tmp_path)
    reader, writer = os.pipe()
    os.close(reader)
    try:
        result = run_console(
            [".", "--output-format=json", "--output", "-"], root, stdout=writer
        )
    finally:
        os.close(writer)

    assert "panicked" not in result.stderr
    assert "Traceback" not in result.stderr
    assert result.returncode == 0


def test_json_entries_carry_function_lines_and_scoring_lines(tmp_path) -> None:
    root = tmp_path / "project"
    root.mkdir()
    method = "    def run(self, a):\n        if a:\n            return f(a)\n"
    source = (
        "def f(n):\n    x = f(n)\n    if n and f(n):\n        return 1\n\n\n"
        f"class First:\n{method}\n\nclass Second:\n{method}"
    )
    (root / "sample.py").write_text(source)

    result = run_binary(
        ["sample.py", "--output-format=json", "--output", "-"], root
    )
    entries = json.loads(result.stdout)
    expected = {
        (function.name, function.line_start): function
        for function in file_complexity(str(root / "sample.py")).functions
    }

    assert sorted(expected) == sorted(
        (entry["function_name"], entry["line_start"]) for entry in entries
    )
    for entry in entries:
        function = expected[(entry["function_name"], entry["line_start"])]
        lines = [line["line"] for line in entry["line_complexities"]]
        assert entry["line_end"] == function.line_end
        assert lines == sorted(set(lines))
        assert all(
            line["complexity"] > 0 for line in entry["line_complexities"]
        )
        assert (
            sum(line["complexity"] for line in entry["line_complexities"])
            == entry["complexity"]
        )


@pytest.mark.parametrize("color", ["yes", "no"])
def test_plain_lines_carry_the_line_and_one_line_per_plan(
    tmp_path, monkeypatch, color
) -> None:
    root = project(tmp_path, files=1)
    monkeypatch.chdir(root)
    function = file_complexity("module_0.py").functions[0]
    result = run_console(
        [
            "module_0.py",
            "--plain",
            "--suggest-refactors",
            "--max-complexity-allowed=0",
            f"--color={color}",
        ],
        root,
        stdout=subprocess.PIPE,
    )

    assert result.stdout.splitlines() == [
        f"module_0.py:{function.line_start} {function.name} {function.complexity}",
        "  C007 Merge 3 nested if statements (-4)",
    ]
