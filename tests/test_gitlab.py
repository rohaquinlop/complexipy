from __future__ import annotations

import json
from pathlib import Path

from typer.testing import CliRunner

from complexipy._complexipy import main as _main
from complexipy.utils.gitlab import _CHECK_NAME, store_gitlab

_SNIPPET = """\
def simple(value):
    return value


def complex_func(data):
    if data:
        for item in data:
            if item:
                if item > 0:
                    for sub in item:
                        if sub:
                            return sub
    return None
"""


def _build_file_complexity(source_file: Path):
    files, _ = _main([source_file.as_posix()], False, [])
    return files


class TestGitlabOutput:
    def test_gitlab_report_has_single_final_newline(self, tmp_path: Path):
        source_file = tmp_path / "sample.py"
        source_file.write_text(_SNIPPET, encoding="utf-8")
        output_file = tmp_path / "results.gitlab.json"

        store_gitlab(
            output_file.as_posix(),
            _build_file_complexity(source_file),
            max_complexity=5,
        )

        content = output_file.read_bytes()

        assert content.endswith(b"\n")
        assert not content.endswith(b"\n\n")

    def test_gitlab_report_contains_required_fields(self, tmp_path: Path):
        source_file = tmp_path / "sample.py"
        source_file.write_text(_SNIPPET, encoding="utf-8")
        output_file = tmp_path / "results.gitlab.json"

        store_gitlab(
            output_file.as_posix(),
            _build_file_complexity(source_file),
            max_complexity=5,
        )

        report = json.loads(output_file.read_text(encoding="utf-8"))

        assert len(report) == 1
        entry = report[0]
        assert entry["check_name"] == _CHECK_NAME
        assert entry["severity"] == "minor"
        assert entry["location"]["path"] == "sample.py"
        assert entry["location"]["lines"]["begin"] >= 1
        assert entry["fingerprint"]
        assert "complex_func" in entry["description"]

    def test_gitlab_report_is_empty_when_threshold_is_high(self, tmp_path: Path):
        source_file = tmp_path / "sample.py"
        source_file.write_text(_SNIPPET, encoding="utf-8")
        output_file = tmp_path / "results.gitlab.json"

        store_gitlab(
            output_file.as_posix(),
            _build_file_complexity(source_file),
            max_complexity=9999,
        )

        report = json.loads(output_file.read_text(encoding="utf-8"))
        assert report == []

    def test_cli_output_gitlab_creates_expected_file(self, tmp_path: Path, monkeypatch):
        import complexipy.main as main_module

        runner = CliRunner()
        source_file = tmp_path / "sample.py"
        source_file.write_text(_SNIPPET, encoding="utf-8")
        monkeypatch.setattr(main_module, "INVOCATION_PATH", str(tmp_path))

        result = runner.invoke(
            main_module.app,
            ["--output-gitlab", "--max-complexity-allowed", "5", str(source_file)],
        )

        assert result.exit_code == 1, result.output

        output_file = tmp_path / "complexipy-results.gitlab.json"
        assert output_file.exists()

        report = json.loads(output_file.read_text(encoding="utf-8"))
        assert len(report) == 1
