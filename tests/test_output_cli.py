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
    def test_output_format_writes_explicit_file(
        self, tmp_path: Path, monkeypatch
    ):
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
        assert (
            json.loads(output_file.read_text(encoding="utf-8"))[0][
                "function_name"
            ]
            == "simple"
        )

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


_MULTI_SNIPPET = """\
def simple(x):
    return x

def medium(x):
    if x:
        if x > 1:
            return x
    return 0

def complex_fn(x):
    if x:
        for i in range(x):
            if i > 0:
                if i % 2:
                    return i
    return 0
"""


class TestTopOutput:
    def test_top_limits_to_n_functions(self, tmp_path: Path):
        import complexipy.main as main_module

        runner = CliRunner()
        source_file = tmp_path / "sample.py"
        source_file.write_text(_MULTI_SNIPPET, encoding="utf-8")

        result = runner.invoke(
            main_module.app,
            ["--top", "2", "--plain", str(source_file)],
        )

        assert result.exit_code == 0
        lines = [
            line for line in result.output.strip().splitlines() if line.strip()
        ]
        assert len(lines) == 2

    def test_top_sorts_descending(self, tmp_path: Path):
        import complexipy.main as main_module

        runner = CliRunner()
        source_file = tmp_path / "sample.py"
        source_file.write_text(_MULTI_SNIPPET, encoding="utf-8")

        result = runner.invoke(
            main_module.app,
            ["--top", "3", "--plain", str(source_file)],
        )

        lines = [
            line for line in result.output.strip().splitlines() if line.strip()
        ]
        complexities = [int(line.split()[-1]) for line in lines]
        assert complexities == sorted(complexities, reverse=True)

    def test_top_works_with_rich_output(self, tmp_path: Path):
        import complexipy.main as main_module

        runner = CliRunner()
        source_file = tmp_path / "sample.py"
        source_file.write_text(_MULTI_SNIPPET, encoding="utf-8")

        result = runner.invoke(
            main_module.app,
            ["--top", "1", str(source_file)],
        )

        assert result.exit_code == 0
        assert "complex_fn" in result.output
        assert "simple" not in result.output

    def test_top_with_failed_filters_both(self, tmp_path: Path):
        import complexipy.main as main_module

        runner = CliRunner()
        source_file = tmp_path / "sample.py"
        source_file.write_text(_MULTI_SNIPPET, encoding="utf-8")

        result = runner.invoke(
            main_module.app,
            ["--top", "1", "--plain", "--failed", "-mx", "0", str(source_file)],
        )

        assert result.exit_code == 1
        lines = [
            line for line in result.output.strip().splitlines() if line.strip()
        ]
        assert len(lines) == 1
        assert "complex_fn" in lines[0]

    def test_top_multi_file_preserves_global_descending_order(
        self, tmp_path: Path
    ):
        import complexipy.main as main_module

        runner = CliRunner()
        file_a = tmp_path / "a.py"
        file_b = tmp_path / "b.py"
        # a.py has the most complex and the least complex function;
        # b.py has a middle-complexity function. Global order must interleave.
        file_a.write_text(
            """\
def a_high(x):
    if x:
        for i in range(x):
            if i > 0:
                if i % 2:
                    if i % 3:
                        return i
    return 0

def a_low(x):
    return x
""",
            encoding="utf-8",
        )
        file_b.write_text(
            """\
def b_mid(x):
    if x:
        if x > 1:
            return x
    return 0
""",
            encoding="utf-8",
        )

        result = runner.invoke(
            main_module.app,
            ["--top", "3", "--plain", str(tmp_path)],
        )

        assert result.exit_code == 0, result.output
        lines = [
            line for line in result.output.strip().splitlines() if line.strip()
        ]
        assert len(lines) == 3
        complexities = [int(line.split()[-1]) for line in lines]
        assert complexities == sorted(complexities, reverse=True)
        # Ensure the middle entry is from b.py — proves the output is not
        # regrouped by file (which would cluster both a.py rows together).
        names = [line.split()[1] for line in lines]
        assert names[0] == "a_high"
        assert names[1] == "b_mid"
        assert names[2] == "a_low"

    def test_top_zero_errors(self, tmp_path: Path):
        import complexipy.main as main_module

        runner = CliRunner()
        source_file = tmp_path / "sample.py"
        source_file.write_text(_MULTI_SNIPPET, encoding="utf-8")

        result = runner.invoke(
            main_module.app,
            ["--top", "0", str(source_file)],
        )

        assert result.exit_code == 2


class TestPlainOutput:
    def test_plain_outputs_one_line_per_function(self, tmp_path: Path):
        import complexipy.main as main_module

        runner = CliRunner()
        source_file = tmp_path / "sample.py"
        source_file.write_text(_SNIPPET, encoding="utf-8")

        result = runner.invoke(
            main_module.app,
            ["--plain", str(source_file)],
        )

        assert result.exit_code == 0
        lines = [
            line for line in result.output.strip().splitlines() if line.strip()
        ]
        assert len(lines) == 1
        parts = lines[0].split()
        assert parts[0] == "sample.py"
        assert parts[1] == "simple"
        assert parts[2] == "1"

    def test_plain_with_failed_shows_only_failures(self, tmp_path: Path):
        import complexipy.main as main_module

        runner = CliRunner()
        source_file = tmp_path / "sample.py"
        source_file.write_text(_SNIPPET, encoding="utf-8")

        result = runner.invoke(
            main_module.app,
            ["--plain", "--failed", "-mx", "0", str(source_file)],
        )

        assert result.exit_code == 1
        lines = [
            line for line in result.output.strip().splitlines() if line.strip()
        ]
        assert len(lines) == 1
        assert "simple" in lines[0]

    def test_plain_with_failed_no_failures_is_silent(self, tmp_path: Path):
        import complexipy.main as main_module

        runner = CliRunner()
        source_file = tmp_path / "sample.py"
        source_file.write_text(_SNIPPET, encoding="utf-8")

        result = runner.invoke(
            main_module.app,
            ["--plain", "--failed", str(source_file)],
        )

        assert result.exit_code == 0
        lines = [
            line for line in result.output.strip().splitlines() if line.strip()
        ]
        assert len(lines) == 0

    def test_plain_and_quiet_errors(self, tmp_path: Path):
        import complexipy.main as main_module

        runner = CliRunner()
        source_file = tmp_path / "sample.py"
        source_file.write_text(_SNIPPET, encoding="utf-8")

        result = runner.invoke(
            main_module.app,
            ["--plain", "--quiet", str(source_file)],
        )

        assert result.exit_code == 2

    def test_plain_no_rich_decorations(self, tmp_path: Path):
        import complexipy.main as main_module

        runner = CliRunner()
        source_file = tmp_path / "sample.py"
        source_file.write_text(_SNIPPET, encoding="utf-8")

        result = runner.invoke(
            main_module.app,
            ["--plain", str(source_file)],
        )

        assert "──" not in result.output
        assert "complexipy" not in result.output.lower().replace(
            "sample.py", ""
        )
        assert "Analysis completed" not in result.output


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
