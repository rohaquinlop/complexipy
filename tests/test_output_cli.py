from __future__ import annotations

import json
import re
from pathlib import Path
from unittest.mock import patch

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


_REFACTOR_SNIPPET = """\
def sample(a, b, c, d):
    if a:
        if b:
            if c and d:
                return 1
    return 0
"""

_MARKUP_LIKE_SNIPPET = """\
def process(rows, a, b, flag):
    if flag:
        if rows:
            if a:
                if b:
                    msg = "[bold red]danger[/bold red]"
                    note = ["x", "y"]
                    return rows[a][b], msg, note
    return None
"""

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
    def test_top_limits_to_n_functions(self, tmp_path: Path, monkeypatch):
        import complexipy.main as main_module

        runner = CliRunner()
        source_file = tmp_path / "sample.py"
        source_file.write_text(_MULTI_SNIPPET, encoding="utf-8")
        monkeypatch.setattr(main_module, "INVOCATION_PATH", str(tmp_path))

        result = runner.invoke(
            main_module.app,
            ["--top", "2", "--plain", str(source_file)],
        )

        assert result.exit_code == 0
        lines = [
            line for line in result.output.strip().splitlines() if line.strip()
        ]
        assert len(lines) == 2

    def test_top_sorts_descending(self, tmp_path: Path, monkeypatch):
        import complexipy.main as main_module

        runner = CliRunner()
        source_file = tmp_path / "sample.py"
        source_file.write_text(_MULTI_SNIPPET, encoding="utf-8")
        monkeypatch.setattr(main_module, "INVOCATION_PATH", str(tmp_path))

        result = runner.invoke(
            main_module.app,
            ["--top", "3", "--plain", str(source_file)],
        )

        lines = [
            line for line in result.output.strip().splitlines() if line.strip()
        ]
        complexities = [int(line.split()[-1]) for line in lines]
        assert complexities == sorted(complexities, reverse=True)

    def test_top_works_with_rich_output(self, tmp_path: Path, monkeypatch):
        import complexipy.main as main_module

        runner = CliRunner()
        source_file = tmp_path / "sample.py"
        source_file.write_text(_MULTI_SNIPPET, encoding="utf-8")
        monkeypatch.setattr(main_module, "INVOCATION_PATH", str(tmp_path))

        result = runner.invoke(
            main_module.app,
            ["--top", "1", str(source_file)],
        )

        assert result.exit_code == 0
        assert "complex_fn" in result.output
        assert "simple" not in result.output

    def test_top_with_failed_filters_both(self, tmp_path: Path, monkeypatch):
        import complexipy.main as main_module

        runner = CliRunner()
        source_file = tmp_path / "sample.py"
        source_file.write_text(_MULTI_SNIPPET, encoding="utf-8")
        monkeypatch.setattr(main_module, "INVOCATION_PATH", str(tmp_path))

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
        self, tmp_path: Path, monkeypatch
    ):
        import complexipy.main as main_module

        runner = CliRunner()
        file_a = tmp_path / "a.py"
        file_b = tmp_path / "b.py"
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
        monkeypatch.setattr(main_module, "INVOCATION_PATH", str(tmp_path))

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
        names = [line.split()[1] for line in lines]
        assert names[0] == "a_high"
        assert names[1] == "b_mid"
        assert names[2] == "a_low"

    def test_top_zero_errors(self, tmp_path: Path, monkeypatch):
        import complexipy.main as main_module

        runner = CliRunner()
        source_file = tmp_path / "sample.py"
        source_file.write_text(_MULTI_SNIPPET, encoding="utf-8")
        monkeypatch.setattr(main_module, "INVOCATION_PATH", str(tmp_path))

        result = runner.invoke(
            main_module.app,
            ["--top", "0", str(source_file)],
        )

        assert result.exit_code == 2


_MANY_INDEPENDENT_PAIRS_SNIPPET = """\
def f(a, b, c, d, e, g, h, i, j, k, m, n):
    if a:
        if b:
            print(a)
    if c:
        if d:
            print(c)
    if e:
        if g:
            print(e)
    if h:
        if i:
            print(h)
    if j:
        if k:
            print(j)
    if m:
        if n:
            print(m)
"""


class TestSuggestRefactorsOutput:
    def test_suggest_refactors_prints_plan_fragments(
        self, tmp_path: Path, monkeypatch
    ):
        import complexipy.main as main_module

        runner = CliRunner()
        source_file = tmp_path / "sample.py"
        source_file.write_text(_REFACTOR_SNIPPET, encoding="utf-8")
        monkeypatch.setattr(main_module, "INVOCATION_PATH", str(tmp_path))

        result = runner.invoke(
            main_module.app,
            ["--suggest-refactors", str(source_file)],
        )

        assert result.exit_code == 0, result.output
        assert "Refactor Suggestions:" in result.output
        # With region overlap dedup, C007 (collapsible_if) wins over C001 (flatten_condition)
        # because they fire on overlapping regions with equal priority and reduction.
        assert "Merge nested if statements" in result.output
        assert "C007" in result.output
        # The exact reduction magnitude is intentionally NOT asserted here --
        # dedicated reduction-math tests in test_refactor_plans.py cover the
        # magnitude against measured ground truth; this test only guards the
        # output *format*.
        assert re.search(
            r"Estimated reduction: -\d+ complexity \(\d+ -> \d+\)",
            result.output,
        )

    def test_suggest_refactors_renders_path_line_col_anchor(
        self, tmp_path: Path, monkeypatch
    ):
        import complexipy.main as main_module

        runner = CliRunner()
        source_file = tmp_path / "sample.py"
        source_file.write_text(_REFACTOR_SNIPPET, encoding="utf-8")
        monkeypatch.setattr(main_module, "INVOCATION_PATH", str(tmp_path))

        result = runner.invoke(
            main_module.app,
            ["--suggest-refactors", str(source_file)],
        )

        assert result.exit_code == 0, result.output
        # `if a:` is line 2, indented 4 spaces, so the outer `if` keyword
        # starts at column 5.
        assert re.search(r"--> .*sample\.py:2:5", result.output)
        # `if a:` is 5 characters -- the caret underline should match exactly.
        assert "^^^^^" in result.output

    def test_suggest_refactors_renders_rule_doc_url(
        self, tmp_path: Path, monkeypatch
    ):
        """The rendered report must carry each rule's documentation link.

        This is the end-to-end half of the doc_url guard: the Rust test proves
        `plan.doc_url` matches the rule's metadata, and
        `test_rule_metadata_has_doc_url` proves the value crosses into Python,
        but only this asserts it actually reaches the terminal. C007 is the
        regression case -- it previously rendered no `References:` section
        because output.py resolved links from a hardcoded map that omitted it.
        """
        import complexipy.main as main_module

        runner = CliRunner()
        source_file = tmp_path / "sample.py"
        source_file.write_text(_REFACTOR_SNIPPET, encoding="utf-8")
        monkeypatch.setattr(main_module, "INVOCATION_PATH", str(tmp_path))

        result = runner.invoke(
            main_module.app,
            ["--suggest-refactors", str(source_file)],
        )

        assert result.exit_code == 0, result.output
        assert "References:" in result.output
        assert "#c007-collapsible-if" in result.output

    def test_suggest_refactors_renders_source_verbatim(
        self, tmp_path: Path, monkeypatch
    ):
        """Rich markup in user source must never be interpreted or stripped.

        Regression test for a bug where `console.print` fed raw source
        through Rich's markup parser, silently deleting subscripts, list
        literals, and anything resembling a `[tag]` from the suggestion the
        user is told to copy.
        """
        import complexipy.main as main_module

        runner = CliRunner()
        source_file = tmp_path / "sample.py"
        source_file.write_text(_MARKUP_LIKE_SNIPPET, encoding="utf-8")
        monkeypatch.setattr(main_module, "INVOCATION_PATH", str(tmp_path))

        result = runner.invoke(
            main_module.app,
            ["--suggest-refactors", str(source_file)],
        )

        assert result.exit_code == 0, result.output
        assert "rows[a][b]" in result.output
        assert '["x", "y"]' in result.output
        assert "[bold red]danger[/bold red]" in result.output

    def test_suggest_refactors_reports_count_of_plans_dropped_by_the_cap(
        self, tmp_path: Path, monkeypatch
    ):
        import complexipy.main as main_module

        runner = CliRunner()
        source_file = tmp_path / "sample.py"
        source_file.write_text(
            _MANY_INDEPENDENT_PAIRS_SNIPPET, encoding="utf-8"
        )
        monkeypatch.setattr(main_module, "INVOCATION_PATH", str(tmp_path))

        result = runner.invoke(
            main_module.app,
            [
                "--suggest-refactors",
                "--max-complexity-allowed",
                "100",
                str(source_file),
            ],
        )

        assert result.exit_code == 0, result.output
        assert len(re.findall(r"\[\d+\] C\d+", result.output)) == 5
        assert "... and 1 more suggestion" in result.output

    def test_failed_with_suggest_refactors_only_shows_displayed_failures(
        self, tmp_path: Path, monkeypatch
    ):
        import complexipy.main as main_module

        runner = CliRunner()
        source_file = tmp_path / "sample.py"
        source_file.write_text(
            _REFACTOR_SNIPPET + "\n\ndef simple(value):\n    return value\n",
            encoding="utf-8",
        )
        monkeypatch.setattr(main_module, "INVOCATION_PATH", str(tmp_path))

        result = runner.invoke(
            main_module.app,
            ["--failed", "-mx", "0", "--suggest-refactors", str(source_file)],
        )

        assert result.exit_code == 1, result.output
        assert "sample 7" in result.output
        assert "Refactor Suggestions:" in result.output
        assert "simple" not in result.output

    def test_plain_with_suggest_refactors_preserves_plain_output(
        self, tmp_path: Path, monkeypatch
    ):
        import complexipy.main as main_module

        runner = CliRunner()
        source_file = tmp_path / "sample.py"
        source_file.write_text(_REFACTOR_SNIPPET, encoding="utf-8")
        monkeypatch.setattr(main_module, "INVOCATION_PATH", str(tmp_path))

        without_flag = runner.invoke(
            main_module.app,
            ["--plain", str(source_file)],
        )
        with_flag = runner.invoke(
            main_module.app,
            ["--plain", "--suggest-refactors", str(source_file)],
        )

        assert without_flag.exit_code == 0, without_flag.output
        assert with_flag.exit_code == 0, with_flag.output
        assert with_flag.output == without_flag.output
        assert "Refactor plans:" not in with_flag.output


class TestPlainOutput:
    def test_plain_outputs_one_line_per_function(
        self, tmp_path: Path, monkeypatch
    ):
        import complexipy.main as main_module

        runner = CliRunner()
        source_file = tmp_path / "sample.py"
        source_file.write_text(_SNIPPET, encoding="utf-8")
        monkeypatch.setattr(main_module, "INVOCATION_PATH", str(tmp_path))

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

    def test_plain_with_failed_shows_only_failures(
        self, tmp_path: Path, monkeypatch
    ):
        import complexipy.main as main_module

        runner = CliRunner()
        source_file = tmp_path / "sample.py"
        source_file.write_text(_SNIPPET, encoding="utf-8")
        monkeypatch.setattr(main_module, "INVOCATION_PATH", str(tmp_path))

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

    def test_plain_with_failed_no_failures_is_silent(
        self, tmp_path: Path, monkeypatch
    ):
        import complexipy.main as main_module

        runner = CliRunner()
        source_file = tmp_path / "sample.py"
        source_file.write_text(_SNIPPET, encoding="utf-8")
        monkeypatch.setattr(main_module, "INVOCATION_PATH", str(tmp_path))

        result = runner.invoke(
            main_module.app,
            ["--plain", "--failed", str(source_file)],
        )

        assert result.exit_code == 0
        lines = [
            line for line in result.output.strip().splitlines() if line.strip()
        ]
        assert len(lines) == 0

    def test_plain_and_quiet_errors(self, tmp_path: Path, monkeypatch):
        import complexipy.main as main_module

        runner = CliRunner()
        source_file = tmp_path / "sample.py"
        source_file.write_text(_SNIPPET, encoding="utf-8")
        monkeypatch.setattr(main_module, "INVOCATION_PATH", str(tmp_path))

        result = runner.invoke(
            main_module.app,
            ["--plain", "--quiet", str(source_file)],
        )

        assert result.exit_code == 2

    def test_plain_no_rich_decorations(self, tmp_path: Path, monkeypatch):
        import complexipy.main as main_module

        runner = CliRunner()
        source_file = tmp_path / "sample.py"
        source_file.write_text(_SNIPPET, encoding="utf-8")
        monkeypatch.setattr(main_module, "INVOCATION_PATH", str(tmp_path))

        result = runner.invoke(
            main_module.app,
            ["--plain", str(source_file)],
        )

        assert "──" not in result.output
        assert "complexipy" not in result.output.lower().replace(
            "sample.py", ""
        )
        assert "Analysis completed" not in result.output


class TestDiffCli:
    """Tests for --diff (enforcing), --diff-only (visual), and --ratchet deprecation."""

    _SIMPLE = "def simple(x):\n    return x + 1\n"
    _COMPLEX = (
        "def simple(x):\n"
        "    if x:\n"
        "        for i in range(x):\n"
        "            if i > 0:\n"
        "                if i % 2:\n"
        "                    return i\n"
        "    return 0\n"
    )

    def test_diff_enforces_on_regression_above_threshold(
        self, tmp_path: Path, monkeypatch
    ):
        import complexipy.main as main_module

        runner = CliRunner()
        source_file = tmp_path / "sample.py"
        source_file.write_text(self._COMPLEX, encoding="utf-8")
        monkeypatch.setattr(main_module, "INVOCATION_PATH", str(tmp_path))

        with patch(
            "complexipy.utils.diff._file_content_at_ref",
            return_value=self._SIMPLE,
        ), patch(
            "complexipy.utils.diff._git_root",
            return_value=str(tmp_path),
        ):
            result = runner.invoke(
                main_module.app,
                ["--diff", "main", "-mx", "5", str(source_file)],
            )

        assert result.exit_code == 1
        assert "REGRESSED" in result.output

    def test_diff_passes_when_within_threshold(
        self, tmp_path: Path, monkeypatch
    ):
        import complexipy.main as main_module

        runner = CliRunner()
        source_file = tmp_path / "sample.py"
        source_file.write_text(self._COMPLEX, encoding="utf-8")
        monkeypatch.setattr(main_module, "INVOCATION_PATH", str(tmp_path))

        with patch(
            "complexipy.utils.diff._file_content_at_ref",
            return_value=self._SIMPLE,
        ), patch(
            "complexipy.utils.diff._git_root",
            return_value=str(tmp_path),
        ):
            result = runner.invoke(
                main_module.app,
                ["--diff", "main", "-mx", "50", str(source_file)],
            )

        assert result.exit_code == 0

    def test_diff_shows_diff_output(self, tmp_path: Path, monkeypatch):
        import complexipy.main as main_module

        runner = CliRunner()
        source_file = tmp_path / "sample.py"
        source_file.write_text(self._COMPLEX, encoding="utf-8")
        monkeypatch.setattr(main_module, "INVOCATION_PATH", str(tmp_path))

        with patch(
            "complexipy.utils.diff._file_content_at_ref",
            return_value=self._SIMPLE,
        ), patch(
            "complexipy.utils.diff._git_root",
            return_value=str(tmp_path),
        ):
            result = runner.invoke(
                main_module.app,
                ["--diff", "main", "-mx", "50", str(source_file)],
            )

        assert "Complexity diff" in result.output
        assert "main" in result.output

    def test_diff_only_does_not_enforce(self, tmp_path: Path, monkeypatch):
        import complexipy.main as main_module

        runner = CliRunner()
        source_file = tmp_path / "sample.py"
        source_file.write_text(self._COMPLEX, encoding="utf-8")
        monkeypatch.setattr(main_module, "INVOCATION_PATH", str(tmp_path))

        with patch(
            "complexipy.utils.diff._file_content_at_ref",
            return_value=self._SIMPLE,
        ), patch(
            "complexipy.utils.diff._git_root",
            return_value=str(tmp_path),
        ):
            # --diff-only: regression exists but threshold is high enough
            # for normal check to pass.  Exit code should be 0.
            result = runner.invoke(
                main_module.app,
                [
                    "--diff-only",
                    "main",
                    "-mx",
                    "50",
                    str(source_file),
                ],
            )

        assert result.exit_code == 0
        assert "Complexity diff" in result.output

    def test_diff_only_visual_no_enforcement_even_with_regression(
        self, tmp_path: Path, monkeypatch
    ):
        import complexipy.main as main_module

        runner = CliRunner()
        source_file = tmp_path / "sample.py"
        source_file.write_text(self._COMPLEX, encoding="utf-8")
        monkeypatch.setattr(main_module, "INVOCATION_PATH", str(tmp_path))

        with patch(
            "complexipy.utils.diff._file_content_at_ref",
            return_value=self._SIMPLE,
        ), patch(
            "complexipy.utils.diff._git_root",
            return_value=str(tmp_path),
        ):
            # --diff-only with low threshold: regression above threshold,
            # but --diff-only should NOT enforce.  Exit code comes from
            # normal threshold check (which WILL fail at mx=5).
            result = runner.invoke(
                main_module.app,
                [
                    "--diff-only",
                    "main",
                    "-mx",
                    "5",
                    str(source_file),
                ],
            )

        # Exit 1 from threshold check, NOT from diff enforcement.
        # Verify diff is shown.
        assert "Complexity diff" in result.output
        assert "REGRESSED" in result.output

    def test_ratchet_deprecation_warning(self, tmp_path: Path, monkeypatch):
        import complexipy.main as main_module

        runner = CliRunner()
        source_file = tmp_path / "sample.py"
        source_file.write_text(self._SIMPLE, encoding="utf-8")
        monkeypatch.setattr(main_module, "INVOCATION_PATH", str(tmp_path))

        with patch(
            "complexipy.utils.diff._file_content_at_ref",
            return_value=self._SIMPLE,
        ), patch(
            "complexipy.utils.diff._git_root",
            return_value=str(tmp_path),
        ):
            result = runner.invoke(
                main_module.app,
                [
                    "--ratchet",
                    "--diff",
                    "main",
                    "-mx",
                    "15",
                    str(source_file),
                ],
            )

        assert "Deprecated" in result.output
        assert "--ratchet" in result.output

    def test_ratchet_without_diff_errors(self, tmp_path: Path, monkeypatch):
        import complexipy.main as main_module

        runner = CliRunner()
        source_file = tmp_path / "sample.py"
        source_file.write_text(self._SIMPLE, encoding="utf-8")
        monkeypatch.setattr(main_module, "INVOCATION_PATH", str(tmp_path))

        result = runner.invoke(
            main_module.app,
            ["--ratchet", str(source_file)],
        )

        assert result.exit_code == 2
        assert "requires" in result.output.lower() or "--diff" in result.output

    def test_diff_and_diff_only_conflict(self, tmp_path: Path, monkeypatch):
        import complexipy.main as main_module

        runner = CliRunner()
        source_file = tmp_path / "sample.py"
        source_file.write_text(self._COMPLEX, encoding="utf-8")
        monkeypatch.setattr(main_module, "INVOCATION_PATH", str(tmp_path))

        with patch(
            "complexipy.utils.diff._file_content_at_ref",
            return_value=self._SIMPLE,
        ), patch(
            "complexipy.utils.diff._git_root",
            return_value=str(tmp_path),
        ):
            # Both --diff and --diff-only: warning printed, --diff-only wins
            # (no enforcement).  With high threshold, exit 0.
            result = runner.invoke(
                main_module.app,
                [
                    "--diff",
                    "main",
                    "--diff-only",
                    "main",
                    "-mx",
                    "50",
                    str(source_file),
                ],
            )

        assert "Warning" in result.output
        assert result.exit_code == 0


class TestStagedDiffCli:
    """Tests for --staged (git-index comparison)."""

    _SIMPLE = "def simple(x):\n    return x + 1\n"
    _COMPLEX = (
        "def simple(x):\n"
        "    if x:\n"
        "        for i in range(x):\n"
        "            if i > 0:\n"
        "                if i % 2:\n"
        "                    return i\n"
        "    return 0\n"
    )

    def _run(self, tmp_path: Path, monkeypatch, args):
        import complexipy.main as main_module

        runner = CliRunner()
        source_file = tmp_path / "sample.py"
        source_file.write_text(self._COMPLEX, encoding="utf-8")
        monkeypatch.setattr(main_module, "INVOCATION_PATH", str(tmp_path))

        with patch(
            "complexipy.utils.diff._git_root", return_value=str(tmp_path)
        ), patch(
            "complexipy.utils.diff._staged_python_files",
            return_value=["sample.py"],
        ), patch(
            "complexipy.utils.diff._file_content_at_ref",
            return_value=self._SIMPLE,
        ), patch(
            "complexipy.utils.diff._file_content_at_index",
            return_value=self._COMPLEX,
        ):
            return runner.invoke(main_module.app, [*args, str(source_file)])

    def test_staged_alone_defaults_to_head_and_enforces(
        self, tmp_path: Path, monkeypatch
    ):
        result = self._run(tmp_path, monkeypatch, ["--staged", "-mx", "5"])

        assert result.exit_code == 1
        assert "REGRESSED" in result.output
        assert "(staged)" in result.output

    def test_staged_alone_passes_within_threshold(
        self, tmp_path: Path, monkeypatch
    ):
        result = self._run(tmp_path, monkeypatch, ["--staged", "-mx", "50"])

        assert result.exit_code == 0
        assert "Complexity diff (vs HEAD (staged))" in result.output

    def test_staged_with_ref_compares_index_against_ref(
        self, tmp_path: Path, monkeypatch
    ):
        result = self._run(
            tmp_path, monkeypatch, ["--diff", "main", "--staged", "-mx", "50"]
        )

        assert result.exit_code == 0
        assert "Complexity diff (vs main (staged))" in result.output

    def test_staged_outside_git_repo_warns_and_passes(
        self, tmp_path: Path, monkeypatch
    ):
        import complexipy.main as main_module

        runner = CliRunner()
        source_file = tmp_path / "sample.py"
        source_file.write_text(self._COMPLEX, encoding="utf-8")
        monkeypatch.setattr(main_module, "INVOCATION_PATH", str(tmp_path))

        with patch("complexipy.utils.diff._git_root", return_value=None):
            result = runner.invoke(
                main_module.app,
                ["--staged", "-mx", "5", str(source_file)],
            )

        assert result.exit_code == 0
        assert "requires a git repository" in result.output


class TestOutputToml:
    def test_get_arguments_value_reads_new_output_keys(self):
        result = get_arguments_value(
            {
                "paths": ["."],
                "output-format": ["json", "gitlab"],
                "output": "reports/",
            },
            {},
        )

        assert result["output_format"] == ["json", "gitlab"]
        assert result["output"] == "reports/"
