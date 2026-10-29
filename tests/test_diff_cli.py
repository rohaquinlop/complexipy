"""Tests for --diff and --diff-only runs: which history they read, and what they warn about."""

from __future__ import annotations

import json
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
DEEPER = (
    "def g(a, b):\n    if a:\n        if b:\n            if a:\n"
    "                return 1\n    return 0\n"
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


def init(root: Path) -> None:
    git(root, "init", "-q")
    git(root, "config", "user.email", "test@example.com")
    git(root, "config", "user.name", "Test")


@pytest.fixture
def repo(tmp_path: Path) -> Path:
    root = tmp_path / "repo"
    (root / "pkg").mkdir(parents=True)
    init(root)
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

    captured = capfd.readouterr()
    out = strip_ansi(captured.out)
    assert exit_code == 0
    assert "Net: 1 regressed" in out
    assert "NEW" not in out
    assert "Warning:" not in strip_ansi(captured.err)


def test_the_gate_from_outside_the_repository_passes_an_unchanged_function(
    repo: Path, outside: Path
) -> None:
    commit(repo, NESTED)

    exit_code = run_cli(
        [str(repo / "pkg"), "--diff", "v1", "--max-complexity-allowed", "2"],
        str(outside),
    )

    assert exit_code == 0


def test_the_gate_sees_a_nested_repository_the_change_adds_as_new(
    repo: Path, capfd: pytest.CaptureFixture[str]
) -> None:
    commit(repo, PLAIN)
    nested = repo / "vendored"
    nested.mkdir()
    init(nested)
    (nested / "lib.py").write_text(NESTED)
    git(nested, "add", "-A")
    git(nested, "commit", "-q", "-m", "base")
    git(nested, "tag", "v1")
    git(repo, "add", "-A")
    git(repo, "commit", "-q", "-m", "add vendored")

    exit_code = run_cli(
        [str(repo), "--diff", "v1", "--max-complexity-allowed", "2"],
        str(repo),
    )

    err = strip_ansi(capfd.readouterr().err)
    assert exit_code == 1
    assert (
        'vendored sits inside another repository but is not pinned there at "v1"'
        in err
    )


def test_diff_only_alone_prints_no_flag_warning(
    repo: Path, capfd: pytest.CaptureFixture[str]
) -> None:
    commit(repo, PLAIN)

    run_cli([str(repo / "pkg"), "--diff-only", "v1"], str(repo))

    assert "both set" not in capfd.readouterr().err


def test_both_diff_flags_print_the_flag_warning(
    repo: Path, capfd: pytest.CaptureFixture[str]
) -> None:
    commit(repo, PLAIN)

    run_cli([str(repo / "pkg"), "--diff", "v1", "--diff-only", "v1"], str(repo))

    err = strip_ansi(capfd.readouterr().err)
    assert "Warning: --diff and --diff-only both set." in err


def test_a_ref_missing_from_the_repository_is_named_in_a_warning(
    repo: Path, capfd: pytest.CaptureFixture[str]
) -> None:
    commit(repo, PLAIN)

    run_cli([str(repo / "pkg"), "--diff-only", "v9"], str(repo))

    err = strip_ansi(capfd.readouterr().err)
    assert 'Warning: "v9" does not name a commit in ' in err
    assert "its 1 analyzed file(s) report as NEW." in err


def test_a_file_outside_any_repository_is_named_in_a_warning(
    outside: Path, capfd: pytest.CaptureFixture[str]
) -> None:
    (outside / "loose.py").write_text(PLAIN)

    run_cli([str(outside / "loose.py"), "--diff-only", "HEAD"], str(outside))

    err = strip_ansi(capfd.readouterr().err)
    assert (
        "Warning: loose.py is not inside a git repository, so all its "
        "functions report as NEW." in err
    )


def test_a_staged_run_compares_only_the_files_it_selects(
    repo: Path, capfd: pytest.CaptureFixture[str]
) -> None:
    commit(repo, PLAIN)
    (repo / "one.py").write_text(PLAIN)
    (repo / "two.py").write_text(NESTED)
    git(repo, "add", "one.py", "two.py")

    exit_code = run_cli(
        [str(repo / "one.py"), "--staged", "--max-complexity-allowed", "1"],
        str(repo),
    )

    out = strip_ansi(capfd.readouterr().out)
    assert exit_code == 0
    assert "one.py::f" in out
    assert "two.py" not in out


def test_a_moved_file_passes_the_diff_gate(repo: Path) -> None:
    commit(repo, NESTED)
    git(repo, "mv", "pkg/mod.py", "pkg/moved.py")

    for flag in (["--diff", "v1"], ["--staged"]):
        exit_code = run_cli(
            [str(repo / "pkg"), *flag, "--max-complexity-allowed", "2"],
            str(repo),
        )

        assert exit_code == 0, flag


def test_a_renamed_function_passes_the_gate_until_it_grows(
    repo: Path, capfd: pytest.CaptureFixture[str]
) -> None:
    commit(repo, NESTED)
    arguments = [
        str(repo / "pkg"),
        "--diff",
        "v1",
        "--max-complexity-allowed",
        "2",
    ]

    (repo / "pkg" / "mod.py").write_text(NESTED.replace("def f", "def g"))
    renamed = run_cli(arguments, str(repo))
    renamed_out = strip_ansi(capfd.readouterr().out)
    (repo / "pkg" / "mod.py").write_text(DEEPER)
    grown = run_cli(arguments, str(repo))

    assert renamed == 0
    assert "replaces" in renamed_out
    assert "mod.py::f" in renamed_out
    assert grown == 1


def test_a_moved_file_fails_the_snapshot_gate_until_it_is_recorded(
    repo: Path,
) -> None:
    (repo / "pkg" / "mod.py").write_text(NESTED)
    arguments = [str(repo / "pkg"), "--max-complexity-allowed", "2"]
    run_cli([*arguments, "--snapshot-create"], str(repo))
    (repo / "pkg" / "mod.py").rename(repo / "pkg" / "moved.py")

    moved = run_cli(arguments, str(repo))
    run_cli([*arguments, "--snapshot-create"], str(repo))
    recorded = run_cli(arguments, str(repo))

    snapshot = json.loads((repo / "complexipy-snapshot.json").read_text())
    paths = [entry["path"].replace("\\", "/") for entry in snapshot]
    assert moved == 1
    assert recorded == 0
    assert len(paths) == 1
    assert paths[0].endswith("pkg/moved.py")


def test_a_diff_gated_run_shows_which_functions_fail_it(
    repo: Path, capfd: pytest.CaptureFixture[str]
) -> None:
    old = NESTED.replace("def f", "def old")
    commit(repo, old + NESTED.replace("def f", "def worse"))
    (repo / "pkg" / "mod.py").write_text(
        old + DEEPER.replace("def g", "def worse")
    )
    arguments = [
        str(repo / "pkg"),
        "--diff",
        "v1",
        "--max-complexity-allowed",
        "2",
    ]

    failed_code = run_cli([*arguments, "--failed"], str(repo))
    failed_out = strip_ansi(capfd.readouterr().out)
    full_code = run_cli(arguments, str(repo))
    full_out = strip_ansi(capfd.readouterr().out)

    def row(out: str, name: str) -> str:
        rows = [
            line for line in out.splitlines() if line.startswith(f"    {name} ")
        ]
        return rows[0] if rows else ""

    assert failed_code == 1
    assert full_code == 1
    assert "FAILED" in row(failed_out, "worse")
    assert row(failed_out, "old") == ""
    assert "Gate: 1 function fails" in failed_out
    assert "FAILS" in failed_out
    assert "ALLOWED" in row(full_out, "old")
    assert "FAILED" in row(full_out, "worse")


def test_the_reports_mark_what_the_diff_gate_fails(repo: Path) -> None:
    old = NESTED.replace("def f", "def old")
    commit(repo, old + NESTED.replace("def f", "def worse"))
    (repo / "pkg" / "mod.py").write_text(
        old + DEEPER.replace("def g", "def worse")
    )

    def reports(diff_flag: str) -> tuple[dict, dict]:
        run_cli(
            [
                str(repo / "pkg"),
                diff_flag,
                "v1",
                "--max-complexity-allowed",
                "2",
                "--output-format",
                "sarif",
                "--output-format",
                "gitlab",
            ],
            str(repo),
        )
        sarif = json.loads((repo / "complexipy-results.sarif").read_text())
        gitlab = json.loads(
            (repo / "complexipy-results.gitlab.json").read_text()
        )
        results = {
            result["locations"][0]["logicalLocations"][0]["name"]: (
                result["level"],
                result.get("baselineState"),
            )
            for result in sarif["runs"][0]["results"]
        }
        severities = {
            issue["description"].split("'")[1]: issue["severity"]
            for issue in gitlab
        }
        return results, severities

    gated, gated_severities = reports("--diff")
    visual, visual_severities = reports("--diff-only")

    assert gated == {
        "old": ("note", "unchanged"),
        "worse": ("error", "updated"),
    }
    assert gated_severities == {"old": "info", "worse": "major"}
    assert visual == {
        "old": ("warning", "unchanged"),
        "worse": ("warning", "updated"),
    }
    assert visual_severities == {"old": "minor", "worse": "minor"}


def test_a_file_moved_in_from_an_excluded_directory_is_new_to_the_gate(
    repo: Path,
) -> None:
    (repo / "pkg" / "vendor").mkdir()
    (repo / "pkg" / "vendor" / "tool.py").write_text(NESTED)
    commit(repo, PLAIN)
    git(repo, "mv", "pkg/vendor/tool.py", "pkg/tool.py")

    exit_code = run_cli(
        [
            str(repo / "pkg"),
            "--exclude",
            "vendor/**",
            "--diff",
            "v1",
            "--max-complexity-allowed",
            "2",
        ],
        str(repo),
    )

    assert exit_code == 1


def test_a_function_an_ignore_marker_hides_pays_for_no_new_function(
    repo: Path,
) -> None:
    commit(repo, NESTED)
    marked = NESTED.replace(
        "def f(a, b):", "def f(a, b):  # complexipy: ignore"
    )
    (repo / "pkg" / "mod.py").write_text(
        marked + "\n\n" + NESTED.replace("def f", "def fresh")
    )

    exit_code = run_cli(
        [str(repo / "pkg"), "--diff", "v1", "--max-complexity-allowed", "2"],
        str(repo),
    )

    assert exit_code == 1


def test_the_reports_mark_a_function_the_snapshot_fails_under_the_diff_gate(
    repo: Path,
) -> None:
    commit(repo, NESTED)
    run_cli(
        [
            str(repo / "pkg"),
            "--snapshot-create",
            "--max-complexity-allowed",
            "5",
        ],
        str(repo),
    )

    exit_code = run_cli(
        [
            str(repo / "pkg"),
            "--diff",
            "v1",
            "--max-complexity-allowed",
            "2",
            "--output-format",
            "sarif",
            "--output-format",
            "gitlab",
        ],
        str(repo),
    )

    sarif = json.loads((repo / "complexipy-results.sarif").read_text())
    gitlab = json.loads((repo / "complexipy-results.gitlab.json").read_text())
    [result] = sarif["runs"][0]["results"]
    [issue] = gitlab
    assert exit_code == 1
    assert (result["level"], result["baselineState"]) == ("error", "unchanged")
    assert issue["severity"] == "major"


def test_a_staged_run_outside_a_repository_claims_no_baseline(
    outside: Path, capfd: pytest.CaptureFixture[str]
) -> None:
    (outside / "loose.py").write_text(NESTED)

    exit_code = run_cli(
        [
            str(outside / "loose.py"),
            "--staged",
            "--max-complexity-allowed",
            "1",
        ],
        str(outside),
    )

    captured = capfd.readouterr()
    out = strip_ansi(captured.out)
    assert exit_code == 0
    assert "--staged requires a git repository" in strip_ansi(captured.err)
    assert "FAILED" in out
    assert "ALLOWED" not in out
    assert "baseline" not in out


@pytest.mark.parametrize(
    ("arguments", "warning"),
    [
        (["--diff-only", "v9"], 'Warning: "v9" does not name a commit in '),
        (
            ["--diff", "v1", "--diff-only", "v1"],
            "Warning: --diff and --diff-only both set.",
        ),
        (["--select", "C999"], "Warning: Unknown rule id 'C999'"),
        (["--select", "C999"], "Warning: No refactoring rule is active"),
    ],
)
def test_quiet_prints_warnings_on_stderr_only(
    repo: Path,
    capfd: pytest.CaptureFixture[str],
    arguments: list[str],
    warning: str,
) -> None:
    commit(repo, PLAIN)

    run_cli([str(repo / "pkg"), "--quiet", *arguments], str(repo))

    captured = capfd.readouterr()
    assert warning in strip_ansi(captured.err)
    assert "Warning" not in captured.out
