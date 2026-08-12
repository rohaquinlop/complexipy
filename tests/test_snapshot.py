from pathlib import Path

from complexipy import _complexipy
from complexipy.utils.snapshot import (
    evaluate_snapshot,
)


class TestEvaluateSnapshot:
    local_path = Path(__file__).resolve().parent
    tracked_path = "tracked.py"
    tracked_function_body = [
        "def tracked(value):\n    if value:\n        return 1\n    return 0\n",
    ]
    complexipy_snapshot_file = "complexipy-snapshot.json"

    def _analyze_paths(self, paths):
        string_paths = [str(path) for path in paths]
        return _complexipy.main(string_paths, False, [], False)

    def test_no_snapshot_file_exists(self, tmp_path: Path):
        source_file = tmp_path / self.tracked_path
        source_file.write_text(*self.tracked_function_body, encoding="utf-8")
        files, _ = self._analyze_paths([source_file])
        snapshot_path = tmp_path / self.complexipy_snapshot_file

        result = evaluate_snapshot(False, False, str(snapshot_path), 0, files)

        assert result.should_run is False
        assert result.active_snapshot_map is None
        assert result.snapshot_result is True

    def test_snapshot_file_exists_not_ignored(self, tmp_path: Path):
        source_file = tmp_path / self.tracked_path
        source_file.write_text(*self.tracked_function_body, encoding="utf-8")
        files, _ = self._analyze_paths([source_file])
        snapshot_path = tmp_path / self.complexipy_snapshot_file

        evaluate_snapshot(True, False, str(snapshot_path), 0, files)

        result = evaluate_snapshot(False, False, str(snapshot_path), 0, files)

        assert result.should_run is True
        assert result.active_snapshot_map is not None
        assert len(result.active_snapshot_map) > 0
        assert result.watermark_success is True
        assert result.watermark_messages == []
        assert result.snapshot_result is True

    def test_snapshot_file_exists_ignored(self, tmp_path: Path):
        source_file = tmp_path / self.tracked_path
        source_file.write_text(*self.tracked_function_body, encoding="utf-8")
        files, _ = self._analyze_paths([source_file])
        snapshot_path = tmp_path / self.complexipy_snapshot_file

        evaluate_snapshot(True, False, str(snapshot_path), 0, files)

        result = evaluate_snapshot(False, True, str(snapshot_path), 0, files)

        assert result.should_run is False
        assert result.active_snapshot_map is None
        assert result.snapshot_result is True

    def test_snapshot_create_generates_file(self, tmp_path: Path):
        source_file = tmp_path / self.tracked_path
        source_file.write_text(*self.tracked_function_body, encoding="utf-8")
        files, _ = self._analyze_paths([source_file])
        snapshot_path = tmp_path / self.complexipy_snapshot_file

        assert not snapshot_path.exists()

        evaluate_snapshot(True, False, str(snapshot_path), 0, files)

        assert snapshot_path.exists()

    def test_watermark_passes_when_no_regressions(self, tmp_path: Path):
        source_file = tmp_path / self.tracked_path
        source_file.write_text(*self.tracked_function_body, encoding="utf-8")
        files, _ = self._analyze_paths([source_file])
        snapshot_path = tmp_path / self.complexipy_snapshot_file

        evaluate_snapshot(True, False, str(snapshot_path), 0, files)

        result = evaluate_snapshot(False, False, str(snapshot_path), 0, files)

        assert result.watermark_success is True
        assert result.watermark_messages == []
        assert result.snapshot_result is True

    def test_snapshot_result_neutral_when_not_running(self, tmp_path: Path):
        source_file = tmp_path / self.tracked_path
        source_file.write_text(*self.tracked_function_body, encoding="utf-8")
        files, _ = self._analyze_paths([source_file])
        snapshot_path = tmp_path / self.complexipy_snapshot_file

        result = evaluate_snapshot(False, True, str(snapshot_path), 0, files)

        assert result.should_run is False
        assert result.snapshot_result is True

    def test_partial_run_preserves_unanalyzed_files(self, tmp_path: Path):
        tracked_a = tmp_path / "tracked_a.py"
        tracked_a.write_text(*self.tracked_function_body, encoding="utf-8")
        tracked_b = tmp_path / "tracked_b.py"
        tracked_b.write_text(*self.tracked_function_body, encoding="utf-8")
        files, _ = self._analyze_paths([tracked_a, tracked_b])
        snapshot_path = tmp_path / self.complexipy_snapshot_file

        evaluate_snapshot(True, False, str(snapshot_path), 0, files)

        files_a, _ = self._analyze_paths([tracked_a])
        result = evaluate_snapshot(False, False, str(snapshot_path), 0, files_a)

        assert result.watermark_success is True
        snapshot_files = _complexipy.load_snapshot_file(str(snapshot_path))
        assert [entry.file_name for entry in snapshot_files] == [
            "tracked_a.py",
            "tracked_b.py",
        ]

    def test_partial_run_keeps_analyzed_file_position(self, tmp_path: Path):
        tracked_a = tmp_path / "tracked_a.py"
        tracked_a.write_text(*self.tracked_function_body, encoding="utf-8")
        tracked_b = tmp_path / "tracked_b.py"
        tracked_b.write_text(*self.tracked_function_body, encoding="utf-8")
        tracked_c = tmp_path / "tracked_c.py"
        tracked_c.write_text(*self.tracked_function_body, encoding="utf-8")
        files, _ = self._analyze_paths([tracked_a, tracked_b, tracked_c])
        snapshot_path = tmp_path / self.complexipy_snapshot_file

        evaluate_snapshot(True, False, str(snapshot_path), 0, files)

        tracked_b.write_text(
            "def tracked(value):\n"
            "    if value:\n"
            "        if value > 1:\n"
            "            return 1\n"
            "    return 0\n",
            encoding="utf-8",
        )
        files_b, _ = self._analyze_paths([tracked_b])
        evaluate_snapshot(True, False, str(snapshot_path), 0, files_b)

        snapshot_files = _complexipy.load_snapshot_file(str(snapshot_path))
        assert [entry.file_name for entry in snapshot_files] == [
            "tracked_a.py",
            "tracked_b.py",
            "tracked_c.py",
        ]
        assert (
            snapshot_files[1].functions[0].complexity
            == files_b[0].functions[0].complexity
        )

    def test_repeated_partial_runs_keep_snapshot_byte_identical(
        self, tmp_path: Path
    ):
        tracked_a = tmp_path / "tracked_a.py"
        tracked_a.write_text(*self.tracked_function_body, encoding="utf-8")
        tracked_b = tmp_path / "tracked_b.py"
        tracked_b.write_text(*self.tracked_function_body, encoding="utf-8")
        files, _ = self._analyze_paths([tracked_a, tracked_b])
        snapshot_path = tmp_path / self.complexipy_snapshot_file

        evaluate_snapshot(True, False, str(snapshot_path), 0, files)

        files_b, _ = self._analyze_paths([tracked_b])
        evaluate_snapshot(False, False, str(snapshot_path), 0, files_b)
        first_content = snapshot_path.read_text(encoding="utf-8")

        evaluate_snapshot(False, False, str(snapshot_path), 0, files_b)

        assert snapshot_path.read_text(encoding="utf-8") == first_content

    def test_new_file_appends_at_end(self, tmp_path: Path):
        tracked_a = tmp_path / "tracked_a.py"
        tracked_a.write_text(*self.tracked_function_body, encoding="utf-8")
        tracked_b = tmp_path / "tracked_b.py"
        tracked_b.write_text(*self.tracked_function_body, encoding="utf-8")
        files, _ = self._analyze_paths([tracked_a, tracked_b])
        snapshot_path = tmp_path / self.complexipy_snapshot_file

        evaluate_snapshot(True, False, str(snapshot_path), 0, files)

        tracked_c = tmp_path / "tracked_c.py"
        tracked_c.write_text(*self.tracked_function_body, encoding="utf-8")
        files_ac, _ = self._analyze_paths([tracked_a, tracked_c])
        evaluate_snapshot(True, False, str(snapshot_path), 0, files_ac)

        snapshot_files = _complexipy.load_snapshot_file(str(snapshot_path))
        assert [entry.file_name for entry in snapshot_files] == [
            "tracked_a.py",
            "tracked_b.py",
            "tracked_c.py",
        ]

    def test_duplicate_snapshot_entries_collapse_in_place(self, tmp_path: Path):
        tracked_a = tmp_path / "tracked_a.py"
        tracked_a.write_text(*self.tracked_function_body, encoding="utf-8")
        tracked_b = tmp_path / "tracked_b.py"
        tracked_b.write_text(*self.tracked_function_body, encoding="utf-8")
        tracked_c = tmp_path / "tracked_c.py"
        tracked_c.write_text(*self.tracked_function_body, encoding="utf-8")
        files, _ = self._analyze_paths([tracked_a, tracked_b, tracked_c])
        snapshot_path = tmp_path / self.complexipy_snapshot_file

        evaluate_snapshot(True, False, str(snapshot_path), 0, files)

        snapshot_files = _complexipy.load_snapshot_file(str(snapshot_path))
        duplicated = [
            snapshot_files[0],
            snapshot_files[1],
            snapshot_files[1],
            snapshot_files[2],
        ]
        _complexipy.create_snapshot_file(str(snapshot_path), 0, duplicated)

        files_b, _ = self._analyze_paths([tracked_b])
        evaluate_snapshot(False, False, str(snapshot_path), 0, files_b)

        snapshot_files = _complexipy.load_snapshot_file(str(snapshot_path))
        assert [entry.file_name for entry in snapshot_files] == [
            "tracked_a.py",
            "tracked_b.py",
            "tracked_c.py",
        ]
        assert (
            snapshot_files[1].functions[0].complexity
            == files_b[0].functions[0].complexity
        )

    def test_snapshot_create_partial_run_preserves_positions(
        self, tmp_path: Path
    ):
        tracked_a = tmp_path / "tracked_a.py"
        tracked_a.write_text(*self.tracked_function_body, encoding="utf-8")
        tracked_b = tmp_path / "tracked_b.py"
        tracked_b.write_text(*self.tracked_function_body, encoding="utf-8")
        files, _ = self._analyze_paths([tracked_a, tracked_b])
        snapshot_path = tmp_path / self.complexipy_snapshot_file

        evaluate_snapshot(True, False, str(snapshot_path), 0, files)

        files_b, _ = self._analyze_paths([tracked_b])
        evaluate_snapshot(True, False, str(snapshot_path), 0, files_b)

        snapshot_files = _complexipy.load_snapshot_file(str(snapshot_path))
        assert [entry.file_name for entry in snapshot_files] == [
            "tracked_a.py",
            "tracked_b.py",
        ]

    def test_partial_run_removes_improved_function(self, tmp_path: Path):
        tracked_a = tmp_path / "tracked_a.py"
        tracked_a.write_text(*self.tracked_function_body, encoding="utf-8")
        tracked_b = tmp_path / "tracked_b.py"
        tracked_b.write_text(*self.tracked_function_body, encoding="utf-8")
        files, _ = self._analyze_paths([tracked_a, tracked_b])
        snapshot_path = tmp_path / self.complexipy_snapshot_file

        evaluate_snapshot(True, False, str(snapshot_path), 0, files)

        tracked_a.write_text("def simple():\n    return 1\n", encoding="utf-8")
        files_a, _ = self._analyze_paths([tracked_a])
        result = evaluate_snapshot(False, False, str(snapshot_path), 0, files_a)

        assert result.watermark_success is True
        snapshot_files = _complexipy.load_snapshot_file(str(snapshot_path))
        assert [entry.file_name for entry in snapshot_files] == ["tracked_b.py"]

    def test_snapshot_create_partial_run_preserves_baseline(
        self, tmp_path: Path
    ):
        tracked_a = tmp_path / "tracked_a.py"
        tracked_a.write_text(*self.tracked_function_body, encoding="utf-8")
        tracked_b = tmp_path / "tracked_b.py"
        tracked_b.write_text(*self.tracked_function_body, encoding="utf-8")
        files, _ = self._analyze_paths([tracked_a, tracked_b])
        snapshot_path = tmp_path / self.complexipy_snapshot_file

        evaluate_snapshot(True, False, str(snapshot_path), 0, files)

        files_a, _ = self._analyze_paths([tracked_a])
        evaluate_snapshot(True, False, str(snapshot_path), 0, files_a)

        snapshot_files = _complexipy.load_snapshot_file(str(snapshot_path))
        assert {entry.file_name for entry in snapshot_files} == {
            "tracked_a.py",
            "tracked_b.py",
        }
