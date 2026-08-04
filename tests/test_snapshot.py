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
        assert {entry.file_name for entry in snapshot_files} == {
            "tracked_a.py",
            "tracked_b.py",
        }

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
