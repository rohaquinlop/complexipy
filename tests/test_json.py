from __future__ import annotations

import json
from pathlib import Path

from typer.testing import CliRunner

from complexipy._complexipy import main as _main
from complexipy.utils.json import store_json

_SNIPPET = """\
def simple(value):
    if value:
        return value
    return 0
"""


def _build_file_complexity(source_file: Path):
    files, _ = _main([source_file.as_posix()], False, [])
    return files


class TestJsonOutput:
    def test_json_has_single_final_newline(self, tmp_path: Path):
        source_file = tmp_path / "sample.py"
        source_file.write_text(_SNIPPET, encoding="utf-8")
        output_file = tmp_path / "results.json"

        store_json(
            output_file.as_posix(),
            _build_file_complexity(source_file),
            show_details=True,
            max_complexity=0,
        )

        content = output_file.read_bytes()

        assert content.endswith(b"\n")
        assert not content.endswith(b"\n\n")
        assert json.loads(content.decode("utf-8"))[0]["function_name"] == "simple"

    def test_cli_json_output_has_final_newline(self, tmp_path: Path, monkeypatch):
        import complexipy.main as main_module

        runner = CliRunner()
        source_file = tmp_path / "sample.py"
        source_file.write_text(_SNIPPET, encoding="utf-8")
        monkeypatch.setattr(main_module, "INVOCATION_PATH", str(tmp_path))

        result = runner.invoke(
            main_module.app,
            ["--output-json", str(source_file)],
        )

        assert result.exit_code == 0, result.output

        output_file = tmp_path / "complexipy-results.json"
        assert output_file.exists()
        assert output_file.read_bytes().endswith(b"\n")
