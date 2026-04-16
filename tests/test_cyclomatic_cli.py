from __future__ import annotations

import json
from pathlib import Path

from typer.testing import CliRunner

# Fixtures with known cyclomatic values. The cognitive numbers are also
# known from the existing fixtures but are not asserted here — this module
# is specifically about the cyclomatic metric.
_TWO_BRANCHES = """\
def two_branches(x):
    if x:
        return 1
    return 0
"""  # cyclomatic = 2 (entry + if)

_BOOL_OP_CHAIN = """\
def bool_chain(a, b, c, d, e):
    if a and b and c and d and e:
        return 1
    return 0
"""  # cyclomatic = 1 + 1 (if) + 4 (and: 5 operands => 4) = 6
# cognitive = 1 (if) + 1 (single and-run) = 2

_MATCH_FIXTURE = """\
def classify(value):
    match value:
        case 0:
            return "zero"
        case 1:
            return "one"
        case _:
            return "other"
"""  # cyclomatic = 1 + 3 cases = 4

_NO_BRANCHES = """\
def straight():
    x = 1
    y = 2
    return x + y
"""  # cyclomatic = 1


class TestCyclomaticMode:
    def test_metric_cyclomatic_default_passes(self, tmp_path: Path):
        import complexipy.main as main_module

        runner = CliRunner()
        source = tmp_path / "sample.py"
        source.write_text(_TWO_BRANCHES, encoding="utf-8")

        result = runner.invoke(
            main_module.app,
            ["--metric", "cyclomatic", "--quiet", str(source)],
        )

        assert result.exit_code == 0, result.output

    def test_metric_cyclomatic_breaches_threshold(self, tmp_path: Path):
        import complexipy.main as main_module

        runner = CliRunner()
        source = tmp_path / "sample.py"
        source.write_text(_BOOL_OP_CHAIN, encoding="utf-8")

        # bool_chain has cyclomatic=5 but cognitive<5.
        # With --metric cognitive -mx 3 the default run would pass;
        # with --metric cyclomatic -mx 3 it must fail.
        cog_result = runner.invoke(
            main_module.app,
            [
                "--metric",
                "cognitive",
                "-mx",
                "3",
                "--quiet",
                str(source),
            ],
        )
        assert cog_result.exit_code == 0, cog_result.output

        cyc_result = runner.invoke(
            main_module.app,
            [
                "--metric",
                "cyclomatic",
                "-mx",
                "3",
                "--quiet",
                str(source),
            ],
        )
        assert cyc_result.exit_code == 1, cyc_result.output

    def test_cyclomatic_csv_header_includes_column(self, tmp_path: Path):
        import complexipy.main as main_module

        runner = CliRunner()
        source = tmp_path / "sample.py"
        output = tmp_path / "out.csv"
        source.write_text(_TWO_BRANCHES, encoding="utf-8")

        result = runner.invoke(
            main_module.app,
            [
                "--metric",
                "cyclomatic",
                "--output-format",
                "csv",
                "--output",
                str(output),
                str(source),
            ],
        )

        assert result.exit_code == 0, result.output
        content = output.read_text(encoding="utf-8")
        lines = content.strip().splitlines()
        header = lines[0]
        assert "Cyclomatic Complexity" in header
        # Row has one extra column: cyclomatic value = 2
        row = lines[1].split(",")
        assert row[-1] == "2"

    def test_cognitive_csv_header_omits_cyclomatic_column(self, tmp_path: Path):
        import complexipy.main as main_module

        runner = CliRunner()
        source = tmp_path / "sample.py"
        output = tmp_path / "out.csv"
        source.write_text(_TWO_BRANCHES, encoding="utf-8")

        result = runner.invoke(
            main_module.app,
            [
                "--output-format",
                "csv",
                "--output",
                str(output),
                str(source),
            ],
        )

        assert result.exit_code == 0, result.output
        header = output.read_text(encoding="utf-8").splitlines()[0]
        assert "Cyclomatic Complexity" not in header

    def test_cyclomatic_json_emits_field(self, tmp_path: Path):
        import complexipy.main as main_module

        runner = CliRunner()
        source = tmp_path / "sample.py"
        output = tmp_path / "out.json"
        source.write_text(_MATCH_FIXTURE, encoding="utf-8")

        result = runner.invoke(
            main_module.app,
            [
                "--metric",
                "cyclomatic",
                "--output-format",
                "json",
                "--output",
                str(output),
                str(source),
            ],
        )

        assert result.exit_code == 0, result.output
        records = json.loads(output.read_text(encoding="utf-8"))
        assert len(records) == 1
        record = records[0]
        assert record["function_name"] == "classify"
        assert record["cyclomatic_complexity"] == 4

    def test_cognitive_json_omits_cyclomatic_field(self, tmp_path: Path):
        import complexipy.main as main_module

        runner = CliRunner()
        source = tmp_path / "sample.py"
        output = tmp_path / "out.json"
        source.write_text(_MATCH_FIXTURE, encoding="utf-8")

        result = runner.invoke(
            main_module.app,
            [
                "--output-format",
                "json",
                "--output",
                str(output),
                str(source),
            ],
        )

        assert result.exit_code == 0, result.output
        records = json.loads(output.read_text(encoding="utf-8"))
        assert "cyclomatic_complexity" not in records[0]

    def test_straight_line_function_is_cyclomatic_1(self, tmp_path: Path):
        import complexipy.main as main_module

        runner = CliRunner()
        source = tmp_path / "sample.py"
        output = tmp_path / "out.json"
        source.write_text(_NO_BRANCHES, encoding="utf-8")

        result = runner.invoke(
            main_module.app,
            [
                "--metric",
                "cyclomatic",
                "--output-format",
                "json",
                "--output",
                str(output),
                str(source),
            ],
        )

        assert result.exit_code == 0, result.output
        record = json.loads(output.read_text(encoding="utf-8"))[0]
        assert record["cyclomatic_complexity"] == 1

    def test_invalid_metric_rejected(self, tmp_path: Path):
        import complexipy.main as main_module

        runner = CliRunner()
        source = tmp_path / "sample.py"
        source.write_text(_TWO_BRANCHES, encoding="utf-8")

        result = runner.invoke(
            main_module.app,
            ["--metric", "halstead", "--quiet", str(source)],
        )

        assert result.exit_code != 0
