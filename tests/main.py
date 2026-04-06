from pathlib import Path
from typing import List, Tuple

import pytest
from typer.testing import CliRunner

from complexipy import (
    _complexipy,
    code_complexity,
    file_complexity,
)
from complexipy._complexipy import FileComplexity
from complexipy.utils.snapshot import (
    handle_snapshot_file_creation,
    handle_snapshot_functions_load,
    handle_snapshot_watermark,
)


class TestFiles:
    local_path = Path(__file__).resolve().parent
    tracked_path = "tracked.py"
    tracked_function_body = [
        "def tracked(value):\n    if value:\n        return 1\n    return 0\n",
    ]
    complexipy_snapshot_file = "complexipy-snapshot.json"

    def _analyze_paths(
        self, paths: List[Path]
    ) -> Tuple[List[FileComplexity], List[str]]:
        string_paths = [str(path) for path in paths]
        return _complexipy.main(string_paths, False, [], False)

    def test_missing_path_is_reported(self):
        missing = self.local_path / "this_file_does_not_exist.py"

        files, failed = _complexipy.main([missing.as_posix()], False, [], False)

        assert files == []
        assert failed == [missing.as_posix()]

    def test_path(self):
        path = self.local_path / "src"
        files, _ = _complexipy.main(
            [path.resolve().as_posix()], False, [], False
        )
        total_complexity = sum([file.complexity for file in files])
        assert 75 == total_complexity

    def test(self):
        path = self.local_path / "src/test.py"
        files, _ = _complexipy.main(
            [path.resolve().as_posix()], False, [], False
        )
        total_complexity = sum([file.complexity for file in files])
        assert 9 == total_complexity

    def test_break_continue(self):
        path = self.local_path / "src/test_break_continue.py"
        files, _ = _complexipy.main(
            [path.resolve().as_posix()], False, [], False
        )
        total_complexity = sum([file.complexity for file in files])
        assert 3 == total_complexity

    def test_class(self):
        path = self.local_path / "src/test_class.py"
        files, _ = _complexipy.main(
            [path.resolve().as_posix()], False, [], False
        )
        total_complexity = sum([file.complexity for file in files])
        assert 1 == total_complexity

    def test_decorator(self):
        path = self.local_path / "src/test_decorator.py"
        files, _ = _complexipy.main(
            [path.resolve().as_posix()], False, [], False
        )
        total_complexity = sum([file.complexity for file in files])
        assert 1 == total_complexity

    def test_for(self):
        path = self.local_path / "src/test_for.py"
        files, _ = _complexipy.main(
            [path.resolve().as_posix()], False, [], False
        )
        total_complexity = sum([file.complexity for file in files])
        assert 5 == total_complexity

    def test_for_assign(self):
        path = self.local_path / "src/test_for_assign.py"
        files, _ = _complexipy.main(
            [path.resolve().as_posix()], False, [], False
        )
        total_complexity = sum([file.complexity for file in files])
        assert 2 == total_complexity

    def test_if(self):
        path = self.local_path / "src/test_if.py"
        files, _ = _complexipy.main(
            [path.resolve().as_posix()], False, [], False
        )
        total_complexity = sum([file.complexity for file in files])
        assert 3 == total_complexity

    def test_main(self):
        path = self.local_path / "src/test_main.py"
        files, _ = _complexipy.main(
            [path.resolve().as_posix()], False, [], False
        )
        total_complexity = sum([file.complexity for file in files])
        assert 0 == total_complexity

    def test_match(self):
        path = self.local_path / "src/test_match.py"
        files, _ = _complexipy.main(
            [path.resolve().as_posix()], False, [], False
        )
        total_complexity = sum([file.complexity for file in files])
        assert 0 == total_complexity

    def test_multiple_func(self):
        path = self.local_path / "src/test_multiple_func.py"
        files, _ = _complexipy.main(
            [path.resolve().as_posix()], False, [], False
        )
        total_complexity = sum([file.complexity for file in files])
        assert 0 == total_complexity

    def test_nested_func(self):
        path = self.local_path / "src/test_nested_func.py"
        files, _ = _complexipy.main(
            [path.resolve().as_posix()], False, [], False
        )
        total_complexity = sum([file.complexity for file in files])
        assert 2 == total_complexity

    def test_recursive(self):
        path = self.local_path / "src/test_recursive.py"
        files, _ = _complexipy.main(
            [path.resolve().as_posix()], False, [], False
        )
        total_complexity = sum([file.complexity for file in files])
        assert 0 == total_complexity

    def test_ternary_op(self):
        path = self.local_path / "src/test_ternary_op.py"
        files, _ = _complexipy.main(
            [path.resolve().as_posix()], False, [], False
        )
        total_complexity = sum([file.complexity for file in files])
        assert 1 == total_complexity

    def test_comprehension(self):
        path = self.local_path / "src/test_comprehension.py"
        files, _ = _complexipy.main(
            [path.resolve().as_posix()], False, [], False
        )
        total_complexity = sum([file.complexity for file in files])
        assert 16 == total_complexity

    def test_try(self):
        path = self.local_path / "src/test_try.py"
        files, _ = _complexipy.main(
            [path.resolve().as_posix()], False, [], False
        )
        total_complexity = sum([file.complexity for file in files])
        assert 3 == total_complexity

    def test_try_nested(self):
        path = self.local_path / "src/test_try_nested.py"
        files, _ = _complexipy.main(
            [path.resolve().as_posix()], False, [], False
        )
        total_complexity = sum([file.complexity for file in files])
        assert 12 == total_complexity

    def test_file_complexity(self):
        path = self.local_path / "src/test_try_nested.py"
        result = file_complexity(str(path))
        assert 12 == result.complexity

    def test_code_complexity(self):
        snippet = """\
def hello_world(s: str) -> str:
    ans = ""

    def nested_func(s: str) -> str:
        if s == "complexipy":
            return "complexipy is awesome!"
        return f"I don't know what to say, hello {s}(?)"

    ans = nested_func(s)

    return ans
"""
        result = code_complexity(snippet)
        assert 2 == result.complexity

    def test_noqa_complexipy_ignore(self):
        path = self.local_path / "src/test_noqa_complex.py"
        files, _ = _complexipy.main(
            [path.resolve().as_posix()], False, [], False
        )
        total_complexity = sum([file.complexity for file in files])
        # The only function has a noqa: complexipy, so it is ignored.
        assert 0 == total_complexity

    def test_noqa_complexipy_ignore_with_decorator(self):
        path = self.local_path / "src/test_noqa_decorator.py"
        files, _ = _complexipy.main(
            [path.resolve().as_posix()], False, [], False
        )
        total_complexity = sum([file.complexity for file in files])
        assert 0 == total_complexity

    def test_complexipy_ignore(self):
        path = self.local_path / "src/test_complexipy_ignore.py"
        files, _ = _complexipy.main(
            [path.resolve().as_posix()], False, [], False
        )
        total_complexity = sum([file.complexity for file in files])
        # The only complex function has a complexipy: ignore, so it is ignored.
        assert 0 == total_complexity

    def test_complexipy_ignore_with_decorator(self):
        path = self.local_path / "src/test_complexipy_ignore_decorator.py"
        files, _ = _complexipy.main(
            [path.resolve().as_posix()], False, [], False
        )
        total_complexity = sum([file.complexity for file in files])
        assert 0 == total_complexity

    def test_exclude(self):
        path = self.local_path / "src"
        files, _ = _complexipy.main(
            [path.resolve().as_posix()],
            False,
            ["test_exclude1.py"],
            False,
        )
        total_complexity = sum([file.complexity for file in files])
        # Excluding only by basename that does not exist at the root
        # should not exclude nested files anymore.
        assert 75 == total_complexity

    def test_exclude_full_path(self):
        path = self.local_path / "src"
        files, _ = _complexipy.main(
            [path.resolve().as_posix()],
            False,
            ["exclude_dir/test_exclude1.py"],
            False,
        )
        total_complexity = sum([file.complexity for file in files])
        assert 72 == total_complexity

    def test_exclude_whole_directory(self):
        path = self.local_path / "src"
        files, _ = _complexipy.main(
            [path.resolve().as_posix()],
            False,
            ["exclude_dir"],
            False,
        )
        total_complexity = sum([file.complexity for file in files])
        assert 69 == total_complexity

    def test_exclude_glob_single_file(self):
        path = self.local_path / "src"
        files, _ = _complexipy.main(
            [path.resolve().as_posix()],
            False,
            ["**/test_exclude1.py"],
            False,
        )
        total_complexity = sum([file.complexity for file in files])
        # **/test_exclude1.py should match exclude_dir/test_exclude1.py (complexity 3)
        assert 72 == total_complexity

    def test_exclude_glob_directory(self):
        path = self.local_path / "src"
        files, _ = _complexipy.main(
            [path.resolve().as_posix()],
            False,
            ["**/exclude_dir/**"],
            False,
        )
        total_complexity = sum([file.complexity for file in files])
        # **/exclude_dir/** should match all files under exclude_dir (complexity 6 total)
        assert 69 == total_complexity

    def test_exclude_glob_wildcard(self):
        path = self.local_path / "src"
        files, _ = _complexipy.main(
            [path.resolve().as_posix()],
            False,
            ["**/test_exclude*.py"],
            False,
        )
        total_complexity = sum([file.complexity for file in files])
        # **/test_exclude*.py matches both test_exclude1.py and test_exclude2.py (complexity 3+3=6)
        assert 69 == total_complexity

    def test_snapshot_watermark_passes_and_updates_snapshot(
        self, tmp_path: Path
    ):
        source_file = tmp_path / self.tracked_path
        source_file.write_text(
            *self.tracked_function_body,
            encoding="utf-8",
        )

        files, _ = self._analyze_paths([source_file])
        snapshot_path = tmp_path / self.complexipy_snapshot_file
        handle_snapshot_file_creation(True, str(snapshot_path), 0, files)
        assert snapshot_path.exists()

        snapshot_files = handle_snapshot_functions_load(str(snapshot_path))
        ok, messages = handle_snapshot_watermark(
            True,
            snapshot_path.exists(),
            str(snapshot_path),
            files,
            snapshot_files,
            0,
        )

        assert ok
        assert messages == []

        source_file.write_text(
            "def tracked(value):\n    return value\n",
            encoding="utf-8",
        )
        files_reduced, _ = self._analyze_paths([source_file])
        snapshot_files = handle_snapshot_functions_load(str(snapshot_path))
        ok, messages = handle_snapshot_watermark(
            True,
            snapshot_path.exists(),
            str(snapshot_path),
            files_reduced,
            snapshot_files,
            0,
        )

        assert ok
        assert messages == []
        assert len(handle_snapshot_functions_load(str(snapshot_path))) == 0, (
            "Snapshot should be empty after the regression is fixed"
        )

    def test_snapshot_watermark_detects_regressions(self, tmp_path: Path):
        tracked_file = tmp_path / self.tracked_path
        tracked_file.write_text(
            "def tracked(value):\n    if value:\n        return value\n    return 0\n",
            encoding="utf-8",
        )

        files, _ = self._analyze_paths([tracked_file])
        snapshot_path = tmp_path / self.complexipy_snapshot_file
        handle_snapshot_file_creation(True, str(snapshot_path), 0, files)

        tracked_file.write_text(
            "def tracked(value):\n"
            "    if value:\n"
            "        if value > 1:\n"
            "            return value\n"
            "    return 0\n",
            encoding="utf-8",
        )
        files_regressed, _ = self._analyze_paths([tracked_file])
        snapshot_files = handle_snapshot_functions_load(str(snapshot_path))
        ok, messages = handle_snapshot_watermark(
            True,
            snapshot_path.exists(),
            str(snapshot_path),
            files_regressed,
            snapshot_files,
            0,
        )

        assert not ok
        assert any("tracked.py:tracked" in message for message in messages)

        tracked_file.write_text(
            "def tracked(value):\n    if value:\n        return value\n    return 0\n",
            encoding="utf-8",
        )
        new_file = tmp_path / "new_file.py"
        new_file.write_text(
            "def newcomer(value):\n    if value:\n        return value\n    return 0\n",
            encoding="utf-8",
        )
        files_with_new, _ = self._analyze_paths([tracked_file, new_file])
        snapshot_files = handle_snapshot_functions_load(str(snapshot_path))
        ok, messages = handle_snapshot_watermark(
            True,
            snapshot_path.exists(),
            str(snapshot_path),
            files_with_new,
            snapshot_files,
            0,
        )

        assert not ok
        assert any("new_file.py:newcomer" in message for message in messages)

    def test_snapshot_watermark_requires_existing_file(self, tmp_path: Path):
        source_file = tmp_path / self.tracked_path
        source_file.write_text(
            *self.tracked_function_body,
            encoding="utf-8",
        )
        files, _ = self._analyze_paths([source_file])
        snapshot_path = tmp_path / self.complexipy_snapshot_file

        ok, messages = handle_snapshot_watermark(
            True,
            snapshot_path.exists(),
            str(snapshot_path),
            files,
            [],
            0,
        )

        assert not ok
        assert "Snapshot watermark requested" in messages[0]

    def test_snapshot_watermark_can_be_ignored(self, tmp_path: Path):
        source_file = tmp_path / self.tracked_path
        source_file.write_text(
            *self.tracked_function_body,
            encoding="utf-8",
        )
        files, _ = self._analyze_paths([source_file])
        snapshot_path = tmp_path / self.complexipy_snapshot_file
        handle_snapshot_file_creation(True, str(snapshot_path), 0, files)
        snapshot_files = handle_snapshot_functions_load(str(snapshot_path))

        ok, messages = handle_snapshot_watermark(
            False,
            snapshot_path.exists(),
            str(snapshot_path),
            files,
            snapshot_files,
            0,
        )

        assert ok
        assert messages == []

    def test_snapshot_watermark_passes_exits_with_code_0(
        self, tmp_path: Path, monkeypatch: pytest.MonkeyPatch
    ):
        """Regression test for issue #136.

        When a snapshot exists and the watermark passes (functions exceed the
        threshold but haven't gotten worse since the snapshot was taken), the
        CLI should exit with code 0 rather than 1.
        """
        import complexipy.main as main_module

        runner = CliRunner()

        # Create a file with a function whose complexity exceeds the default
        # threshold of 15.  Six nested if-statements produces a cognitive
        # complexity of 21 (1+2+3+4+5+6).
        source_file = tmp_path / "complex.py"
        source_file.write_text(
            "def high_complexity(a, b, c, d, e, f):\n"
            "    if a:\n"
            "        if b:\n"
            "            if c:\n"
            "                if d:\n"
            "                    if e:\n"
            "                        if f:\n"
            "                            return True\n"
            "    return False\n",
            encoding="utf-8",
        )

        # Point the module's snapshot output to tmp_path so we don't pollute
        # the project directory and tests remain isolated.
        monkeypatch.setattr(main_module, "INVOCATION_PATH", str(tmp_path))

        # Step 1: create the snapshot – should exit 0 (watermark is satisfied
        # because we just created the baseline).
        result_create = runner.invoke(
            main_module.app,
            ["--snapshot-create", str(source_file)],
        )
        assert (tmp_path / "complexipy-snapshot.json").exists(), (
            "Snapshot file was not created"
        )
        assert result_create.exit_code == 0, (
            f"--snapshot-create exited with {result_create.exit_code}.\n"
            f"Output:\n{result_create.output}"
        )

        # Step 2: re-run without --snapshot-create.  The function still
        # exceeds the threshold, but it hasn't regressed since the snapshot,
        # so the watermark passes and the exit code must be 0.
        result_check = runner.invoke(
            main_module.app,
            [str(source_file)],
        )
        assert result_check.exit_code == 0, (
            f"Expected exit code 0 when snapshot watermark passes, "
            f"got {result_check.exit_code}.\n"
            f"Output:\n{result_check.output}"
        )
        # The function should be displayed as passed (not failed) since it is
        # grandfathered by the snapshot.
        assert "PASSED" in result_check.output, (
            f"Expected 'PASSED' in output.\nOutput:\n{result_check.output}"
        )
        assert "FAILED" not in result_check.output, (
            f"Expected no 'FAILED' in output.\nOutput:\n{result_check.output}"
        )


class TestComprehension:
    """Unit tests for comprehension cognitive complexity scoring."""

    def _complexity(self, snippet: str) -> int:
        return code_complexity(snippet).complexity

    def test_simple_listcomp(self):
        # [x for x in items] at depth 0: +1 (comprehension base)
        snippet = "def f(items):\n    return [x for x in items]\n"
        assert 1 == self._complexity(snippet)

    def test_listcomp_with_if_filter(self):
        # +1 (comprehension) +1 (if clause) = 2
        snippet = "def f(items):\n    return [x for x in items if x > 0]\n"
        assert 2 == self._complexity(snippet)

    def test_listcomp_two_for_clauses(self):
        # +1 (comprehension) +1 (second for) = 2
        snippet = (
            "def f(matrix):\n    return [x for row in matrix for x in row]\n"
        )
        assert 2 == self._complexity(snippet)

    def test_listcomp_two_for_clauses_with_if(self):
        # +1 (comprehension) +1 (second for) +1 (if) = 3
        snippet = "def f(matrix):\n    return [x for row in matrix for x in row if x > 0]\n"
        assert 3 == self._complexity(snippet)

    def test_nested_listcomp(self):
        # outer at depth 0: +1; inner at depth 1: +1+1=2; total = 3
        snippet = (
            "def f(matrix):\n    return [[x for x in row] for row in matrix]\n"
        )
        assert 3 == self._complexity(snippet)

    def test_generator_in_call(self):
        # generator expression inside sum(): +1
        snippet = "def f(items):\n    return sum(x * 2 for x in items)\n"
        assert 1 == self._complexity(snippet)

    def test_setcomp(self):
        # +1 = 1
        snippet = "def f(items):\n    return {x for x in items}\n"
        assert 1 == self._complexity(snippet)

    def test_dictcomp_simple(self):
        # +1 = 1
        snippet = "def f(keys):\n    return {k: k * 2 for k in keys}\n"
        assert 1 == self._complexity(snippet)

    def test_dictcomp_with_if_filter(self):
        # +1 (comprehension) +1 (if) = 2
        snippet = "def f(keys):\n    return {k: k * 2 for k in keys if k > 0}\n"
        assert 2 == self._complexity(snippet)

    def test_comprehension_inside_if_block(self):
        # if (nesting 0→1): +1; listcomp inside if body at nesting_level=1: +1+1=2
        # total = 3
        snippet = "def f(data, items):\n    if data:\n        return [x for x in items]\n"
        assert 3 == self._complexity(snippet)


class TestScriptComplexity:
    """Tests for module-level (script) complexity analysis."""

    local_path = Path(__file__).resolve().parent

    def test_script_simple_default(self):
        """Default behavior: module-level code not reported as <module>."""
        path = self.local_path / "src/test_script_simple.py"
        files, _ = _complexipy.main(
            [path.resolve().as_posix()], False, [], False
        )
        assert len(files) == 1
        assert len(files[0].functions) == 0

    def test_script_simple_check_script(self):
        """Simple assignments have 0 complexity, no <module> emitted."""
        path = self.local_path / "src/test_script_simple.py"
        files, _ = _complexipy.main(
            [path.resolve().as_posix()], False, [], True
        )
        assert len(files) == 1
        module_funcs = [f for f in files[0].functions if f.name == "<module>"]
        assert len(module_funcs) == 0

    def test_script_complex_default(self):
        """Default: complex script still reports 0 functions."""
        path = self.local_path / "src/test_script_complex.py"
        files, _ = _complexipy.main(
            [path.resolve().as_posix()], False, [], False
        )
        assert len(files) == 1
        assert len(files[0].functions) == 0

    def test_script_complex_check_script(self):
        """With check_script: complex script reports <module>."""
        path = self.local_path / "src/test_script_complex.py"
        files, _ = _complexipy.main(
            [path.resolve().as_posix()], False, [], True
        )
        assert len(files) == 1
        module_funcs = [f for f in files[0].functions if f.name == "<module>"]
        assert len(module_funcs) == 1
        assert module_funcs[0].complexity == 7

    def test_script_mixed_check_script(self):
        """Mixed file: both function and <module> reported."""
        path = self.local_path / "src/test_script_mixed.py"
        files, _ = _complexipy.main(
            [path.resolve().as_posix()], False, [], True
        )
        assert len(files) == 1
        func_names = {f.name for f in files[0].functions}
        assert "simple_func" in func_names
        assert "<module>" in func_names
        simple = next(f for f in files[0].functions if f.name == "simple_func")
        module = next(f for f in files[0].functions if f.name == "<module>")
        assert simple.complexity == 1
        assert module.complexity == 3

    def test_code_complexity_check_script(self):
        """Python API: code_complexity with check_script."""
        code = "for i in range(10):\n    if i > 5:\n        print(i)\n"
        result = code_complexity(code, check_script=True)
        module_funcs = [f for f in result.functions if f.name == "<module>"]
        assert len(module_funcs) == 1
        assert module_funcs[0].complexity == 3

    def test_code_complexity_default_no_module(self):
        """Python API: code_complexity default does not emit <module>."""
        code = "for i in range(10):\n    if i > 5:\n        print(i)\n"
        result = code_complexity(code)
        module_funcs = [f for f in result.functions if f.name == "<module>"]
        assert len(module_funcs) == 0

    def test_cli_check_script(self):
        """CLI: --check-script flag works end-to-end."""
        from complexipy.main import app

        runner = CliRunner()
        path = str(self.local_path / "src/test_script_complex.py")
        result = runner.invoke(app, [path, "--check-script"])
        assert result.exit_code == 0
        assert "<module>" in result.output

    def test_cli_no_check_script(self):
        """CLI: without --check-script, no <module> in output."""
        from complexipy.main import app

        runner = CliRunner()
        path = str(self.local_path / "src/test_script_complex.py")
        result = runner.invoke(app, [path])
        assert "<module>" not in result.output

    def test_cli_script_strict_fails(self):
        """CLI: --script-strict exits 1 when module complexity exceeds threshold."""
        from complexipy.main import app

        runner = CliRunner()
        path = str(self.local_path / "src/test_script_complex.py")
        result = runner.invoke(
            app, [path, "--check-script", "--script-strict", "-mx", "1"]
        )
        assert result.exit_code == 1

    def test_cli_script_strict_passes(self):
        """CLI: --script-strict exits 0 when module complexity is within threshold."""
        from complexipy.main import app

        runner = CliRunner()
        path = str(self.local_path / "src/test_script_simple.py")
        result = runner.invoke(
            app, [path, "--check-script", "--script-strict", "-mx", "15"]
        )
        assert result.exit_code == 0
