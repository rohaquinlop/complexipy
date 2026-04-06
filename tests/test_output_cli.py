from __future__ import annotations

import json
from pathlib import Path

from typer.testing import CliRunner

from complexipy.utils.toml import get_arguments_value

_SNIPPET = """\
def simple(value):
    if value:
        return value
    return 0
"""


class TestOutputCli:
    def test_output_format_writes_explicit_file(self, tmp_path: Path, monkeypatch):
        import complexipy.main as main_module

        runner = CliRunner()
        source_file = tmp_path / "sample.py"
        output_file = tmp_path / "reports" / "report.json"
        source_file.write_text(_SNIPPET, encoding="utf-8")
        monkeypatch.setattr(main_module, "INVOCATION_PATH", str(tmp_path))

        result = runner.invoke(
            main_module.app,
            [
                "--output-format",
                "json",
                "--output",
                str(output_file),
                str(source_file),
            ],
        )

        assert result.exit_code == 0, result.output
        assert output_file.exists()
        assert json.loads(output_file.read_text(encoding="utf-8"))[0][
            "function_name"
        ] == "simple"

    def test_multiple_output_formats_require_directory(self, tmp_path: Path):
        import complexipy.main as main_module

        runner = CliRunner()
        source_file = tmp_path / "sample.py"
        source_file.write_text(_SNIPPET, encoding="utf-8")

        result = runner.invoke(
            main_module.app,
            [
                "--output-format",
                "json",
                "--output-format",
                "csv",
                "--output",
                str(tmp_path / "report.json"),
                str(source_file),
            ],
        )

        assert result.exit_code == 2
        assert not (tmp_path / "report.json").exists()

    def test_multiple_output_formats_write_default_names_in_directory(
        self,
        tmp_path: Path,
    ):
        import complexipy.main as main_module

        runner = CliRunner()
        source_file = tmp_path / "sample.py"
        output_dir = tmp_path / "reports"
        source_file.write_text(_SNIPPET, encoding="utf-8")

        result = runner.invoke(
            main_module.app,
            [
                "--output-format",
                "json",
                "--output-format",
                "csv",
                "--output",
                f"{output_dir}{Path('/').as_posix()}",
                str(source_file),
            ],
        )

        assert result.exit_code == 0, result.output
        assert (output_dir / "complexipy-results.json").exists()
        assert (output_dir / "complexipy-results.csv").exists()


class TestOutputToml:
    def test_get_arguments_value_reads_new_output_keys(self):
        values = get_arguments_value(
            {
                "paths": ["."],
                "output-format": ["json", "gitlab"],
                "output": "reports/",
            },
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )

        assert values[9] == ["json", "gitlab"]
        assert values[10] == "reports/"
