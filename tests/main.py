from pathlib import Path
from typing import List, Tuple

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

    def _analyze_paths(
        self, paths: List[Path]
    ) -> Tuple[List[FileComplexity], List[str]]:
        string_paths = [str(path) for path in paths]
        return _complexipy.main(string_paths, False, [])

    def test_path(self):
        path = self.local_path / "src"
        files, _ = _complexipy.main([path.resolve().as_posix()], False, [])
        total_complexity = sum([file.complexity for file in files])
        assert 47 == total_complexity

    def test(self):
        path = self.local_path / "src/test.py"
        files, _ = _complexipy.main([path.resolve().as_posix()], False, [])
        total_complexity = sum([file.complexity for file in files])
        assert 9 == total_complexity

    def test_break_continue(self):
        path = self.local_path / "src/test_break_continue.py"
        files, _ = _complexipy.main([path.resolve().as_posix()], False, [])
        total_complexity = sum([file.complexity for file in files])
        assert 3 == total_complexity

    def test_class(self):
        path = self.local_path / "src/test_class.py"
        files, _ = _complexipy.main([path.resolve().as_posix()], False, [])
        total_complexity = sum([file.complexity for file in files])
        assert 1 == total_complexity

    def test_decorator(self):
        path = self.local_path / "src/test_decorator.py"
        files, _ = _complexipy.main([path.resolve().as_posix()], False, [])
        total_complexity = sum([file.complexity for file in files])
        assert 1 == total_complexity

    def test_for(self):
        path = self.local_path / "src/test_for.py"
        files, _ = _complexipy.main([path.resolve().as_posix()], False, [])
        total_complexity = sum([file.complexity for file in files])
        assert 5 == total_complexity

    def test_for_assign(self):
        path = self.local_path / "src/test_for_assign.py"
        files, _ = _complexipy.main([path.resolve().as_posix()], False, [])
        total_complexity = sum([file.complexity for file in files])
        assert 1 == total_complexity

    def test_if(self):
        path = self.local_path / "src/test_if.py"
        files, _ = _complexipy.main([path.resolve().as_posix()], False, [])
        total_complexity = sum([file.complexity for file in files])
        assert 3 == total_complexity

    def test_main(self):
        path = self.local_path / "src/test_main.py"
        files, _ = _complexipy.main([path.resolve().as_posix()], False, [])
        total_complexity = sum([file.complexity for file in files])
        assert 0 == total_complexity

    def test_match(self):
        path = self.local_path / "src/test_match.py"
        files, _ = _complexipy.main([path.resolve().as_posix()], False, [])
        total_complexity = sum([file.complexity for file in files])
        assert 0 == total_complexity

    def test_multiple_func(self):
        path = self.local_path / "src/test_multiple_func.py"
        files, _ = _complexipy.main([path.resolve().as_posix()], False, [])
        total_complexity = sum([file.complexity for file in files])
        assert 0 == total_complexity

    def test_nested_func(self):
        path = self.local_path / "src/test_nested_func.py"
        files, _ = _complexipy.main([path.resolve().as_posix()], False, [])
        total_complexity = sum([file.complexity for file in files])
        assert 2 == total_complexity

    def test_recursive(self):
        path = self.local_path / "src/test_recursive.py"
        files, _ = _complexipy.main([path.resolve().as_posix()], False, [])
        total_complexity = sum([file.complexity for file in files])
        assert 0 == total_complexity

    def test_ternary_op(self):
        path = self.local_path / "src/test_ternary_op.py"
        files, _ = _complexipy.main([path.resolve().as_posix()], False, [])
        total_complexity = sum([file.complexity for file in files])
        assert 1 == total_complexity

    def test_try(self):
        path = self.local_path / "src/test_try.py"
        files, _ = _complexipy.main([path.resolve().as_posix()], False, [])
        total_complexity = sum([file.complexity for file in files])
        assert 3 == total_complexity

    def test_try_nested(self):
        path = self.local_path / "src/test_try_nested.py"
        files, _ = _complexipy.main([path.resolve().as_posix()], False, [])
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
        files, _ = _complexipy.main([path.resolve().as_posix()], False, [])
        total_complexity = sum([file.complexity for file in files])
        # The only function has a noqa: complexipy, so it is ignored.
        assert 0 == total_complexity

    def test_exclude(self):
        path = self.local_path / "src"
        files, _ = _complexipy.main(
            [path.resolve().as_posix()],
            False,
            ["test_exclude1.py"],
        )
        total_complexity = sum([file.complexity for file in files])
        # Excluding only by basename that does not exist at the root
        # should not exclude nested files anymore.
        assert 47 == total_complexity

    def test_exclude_full_path(self):
        path = self.local_path / "src"
        files, _ = _complexipy.main(
            [path.resolve().as_posix()],
            False,
            ["exclude_dir/test_exclude1.py"],
        )
        total_complexity = sum([file.complexity for file in files])
        assert 44 == total_complexity

    def test_exclude_whole_directory(self):
        path = self.local_path / "src"
        files, _ = _complexipy.main(
            [path.resolve().as_posix()],
            False,
            ["exclude_dir"],
        )
        total_complexity = sum([file.complexity for file in files])
        assert 41 == total_complexity

    def test_snapshot_watermark_passes_and_updates_snapshot(
        self, tmp_path: Path
    ):
        source_file = tmp_path / "tracked.py"
        source_file.write_text(
            "def tracked(value):\n"
            "    if value:\n"
            "        return 1\n"
            "    return 0\n",
            encoding="utf-8",
        )

        files, _ = self._analyze_paths([source_file])
        snapshot_path = tmp_path / "complexipy-snapshot.json"
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
        tracked_file = tmp_path / "tracked.py"
        tracked_file.write_text(
            "def tracked(value):\n"
            "    if value:\n"
            "        return value\n"
            "    return 0\n",
            encoding="utf-8",
        )

        files, _ = self._analyze_paths([tracked_file])
        snapshot_path = tmp_path / "complexipy-snapshot.json"
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
            "def tracked(value):\n"
            "    if value:\n"
            "        return value\n"
            "    return 0\n",
            encoding="utf-8",
        )
        new_file = tmp_path / "new_file.py"
        new_file.write_text(
            "def newcomer(value):\n"
            "    if value:\n"
            "        return value\n"
            "    return 0\n",
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
        source_file = tmp_path / "tracked.py"
        source_file.write_text(
            "def tracked(value):\n"
            "    if value:\n"
            "        return 1\n"
            "    return 0\n",
            encoding="utf-8",
        )
        files, _ = self._analyze_paths([source_file])
        snapshot_path = tmp_path / "complexipy-snapshot.json"

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
        source_file = tmp_path / "tracked.py"
        source_file.write_text(
            "def tracked(value):\n"
            "    if value:\n"
            "        return 1\n"
            "    return 0\n",
            encoding="utf-8",
        )
        files, _ = self._analyze_paths([source_file])
        snapshot_path = tmp_path / "complexipy-snapshot.json"
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
