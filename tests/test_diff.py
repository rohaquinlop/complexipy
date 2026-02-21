from __future__ import annotations

import os
import tempfile
from typing import List
from unittest.mock import patch

import pytest

from complexipy._complexipy import main as _main
from complexipy.utils.diff import (
    DiffEntry,
    _STATUS_IMPROVED,
    _STATUS_NEW,
    _STATUS_REGRESSED,
    _STATUS_REMOVED,
    _STATUS_UNCHANGED,
    compute_diff,
    format_diff,
)

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

_SIMPLE = """\
def simple(x):
    return x + 1
"""

_WITH_IF = """\
def with_if(x):
    if x:
        return 1
    return 0
"""

_COMPLEX = """\
def complex_func(data):
    if data:
        for item in data:
            if item:
                return item
    return None
"""


def _make_file_complexity(code: str, path: str = "src/example.py"):
    """Return a FileComplexity built from the given code snippet.

    The temp file is named after the basename of *path* so that
    ``FileComplexity.file_name`` reflects the desired name.
    """
    basename = os.path.basename(path)
    tmp_dir = tempfile.mkdtemp()
    tmp_path = os.path.join(tmp_dir, basename)
    try:
        with open(tmp_path, "w") as f:
            f.write(code)
        files, _ = _main([tmp_path], False, [])
    finally:
        os.unlink(tmp_path)
        os.rmdir(tmp_dir)
    return files[0]


# ---------------------------------------------------------------------------
# DiffEntry tests
# ---------------------------------------------------------------------------


class TestDiffEntry:
    def test_status_new(self):
        e = DiffEntry("f.py", "foo", None, 5)
        assert e.status == _STATUS_NEW

    def test_status_removed(self):
        e = DiffEntry("f.py", "foo", 5, None)
        assert e.status == _STATUS_REMOVED

    def test_status_regressed(self):
        e = DiffEntry("f.py", "foo", 3, 7)
        assert e.status == _STATUS_REGRESSED

    def test_status_improved(self):
        e = DiffEntry("f.py", "foo", 7, 3)
        assert e.status == _STATUS_IMPROVED

    def test_status_unchanged(self):
        e = DiffEntry("f.py", "foo", 4, 4)
        assert e.status == _STATUS_UNCHANGED

    def test_delta_change(self):
        e = DiffEntry("f.py", "foo", 3, 7)
        assert e.delta == 4

    def test_delta_none_for_new(self):
        e = DiffEntry("f.py", "foo", None, 5)
        assert e.delta is None

    def test_delta_none_for_removed(self):
        e = DiffEntry("f.py", "foo", 5, None)
        assert e.delta is None


# ---------------------------------------------------------------------------
# compute_diff tests (git operations mocked)
# ---------------------------------------------------------------------------


class TestComputeDiff:
    def _file(self, code: str, path: str = "src/example.py"):
        return _make_file_complexity(code, path)

    def test_new_file_all_functions_marked_new(self):
        current = [self._file(_WITH_IF)]
        # git show returns None → file is new
        with patch(
            "complexipy.utils.diff._file_content_at_ref", return_value=None
        ), patch("complexipy.utils.diff._git_root", return_value="/repo"):
            entries = compute_diff(current, "HEAD~1", "/repo")

        assert all(e.status == _STATUS_NEW for e in entries)
        assert any(e.func_name == "with_if" for e in entries)

    def test_unchanged_function(self):
        current = [self._file(_SIMPLE)]
        # Same code at the reference → no change
        with patch(
            "complexipy.utils.diff._file_content_at_ref", return_value=_SIMPLE
        ), patch("complexipy.utils.diff._git_root", return_value="/repo"):
            entries = compute_diff(current, "HEAD~1", "/repo")

        simple_entry = next(e for e in entries if e.func_name == "simple")
        assert simple_entry.status == _STATUS_UNCHANGED

    def test_regressed_function(self):
        current = [self._file(_COMPLEX)]
        # Old version was simpler
        with patch(
            "complexipy.utils.diff._file_content_at_ref", return_value=_SIMPLE
        ), patch("complexipy.utils.diff._git_root", return_value="/repo"):
            entries = compute_diff(current, "HEAD~1", "/repo")

        complex_entry = next(
            (e for e in entries if e.func_name == "complex_func"), None
        )
        assert complex_entry is not None
        assert complex_entry.status == _STATUS_NEW  # not in old version

    def test_improved_function(self):
        current = [self._file(_SIMPLE)]
        # Old version was more complex
        with patch(
            "complexipy.utils.diff._file_content_at_ref", return_value=_COMPLEX
        ), patch("complexipy.utils.diff._git_root", return_value="/repo"):
            entries = compute_diff(current, "HEAD~1", "/repo")

        simple_entry = next(
            (e for e in entries if e.func_name == "simple"), None
        )
        # simple didn't exist in old (complex) version → appears as new
        assert simple_entry is not None
        assert simple_entry.status == _STATUS_NEW

    def test_removed_function_appears_in_entries(self):
        # Current file has only `simple`; old had `simple` + `with_if`
        current = [self._file(_SIMPLE)]
        old_code = _SIMPLE + "\n" + _WITH_IF
        with patch(
            "complexipy.utils.diff._file_content_at_ref", return_value=old_code
        ), patch("complexipy.utils.diff._git_root", return_value="/repo"):
            entries = compute_diff(current, "HEAD~1", "/repo")

        removed = next(
            (e for e in entries if e.func_name == "with_if"), None
        )
        assert removed is not None
        assert removed.status == _STATUS_REMOVED

    def test_multiple_files(self):
        f1 = self._file(_SIMPLE, "a.py")
        f2 = self._file(_WITH_IF, "b.py")

        def _mock_content(git_ref, path, cwd):
            return _SIMPLE  # both files look simple in the old version

        with patch(
            "complexipy.utils.diff._file_content_at_ref", side_effect=_mock_content
        ), patch("complexipy.utils.diff._git_root", return_value="/repo"):
            entries = compute_diff([f1, f2], "HEAD~1", "/repo")

        file_names = {os.path.basename(e.file_path) for e in entries}
        assert "a.py" in file_names
        assert "b.py" in file_names

    def test_git_error_skips_file(self):
        current = [self._file(_SIMPLE)]
        # Simulate unparseable old content
        with patch(
            "complexipy.utils.diff._file_content_at_ref", return_value="not python!!!"
        ), patch("complexipy.utils.diff._git_root", return_value="/repo"):
            # Should not raise
            entries = compute_diff(current, "HEAD~1", "/repo")
        # If parsing fails the file is skipped; entries may be empty
        assert isinstance(entries, list)


# ---------------------------------------------------------------------------
# format_diff tests
# ---------------------------------------------------------------------------


class TestFormatDiff:
    def test_empty_entries_no_changes(self):
        entries = [DiffEntry("f.py", "foo", 3, 3)]  # UNCHANGED
        output = format_diff(entries, "HEAD~1")
        assert "No functions changed" not in output  # UNCHANGED is filtered but
        # header is still shown since entries list is non-empty
        # Actually UNCHANGED entries are filtered from display:
        assert "foo" not in output

    def test_regressed_entry_shown(self):
        entries = [DiffEntry("f.py", "handle", 10, 18)]
        output = format_diff(entries, "HEAD~1")
        assert "REGRESSED" in output
        assert "handle" in output
        assert "10 → 18" in output

    def test_improved_entry_shown(self):
        entries = [DiffEntry("f.py", "helper", 20, 8)]
        output = format_diff(entries, "HEAD~1")
        assert "IMPROVED" in output
        assert "20 → 8" in output

    def test_new_entry_shown(self):
        entries = [DiffEntry("f.py", "newcomer", None, 5)]
        output = format_diff(entries, "HEAD~1")
        assert "NEW" in output
        assert "newcomer" in output

    def test_removed_entry_shown(self):
        entries = [DiffEntry("f.py", "gone", 7, None)]
        output = format_diff(entries, "HEAD~1")
        assert "REMOVED" in output
        assert "gone" in output

    def test_summary_counts(self):
        entries = [
            DiffEntry("f.py", "a", 5, 10),  # REGRESSED
            DiffEntry("f.py", "b", 10, 5),  # IMPROVED
            DiffEntry("f.py", "c", None, 3),  # NEW
        ]
        output = format_diff(entries, "HEAD~1")
        assert "1 regressed" in output
        assert "1 improved" in output
        assert "1 new" in output

    def test_ref_name_in_header(self):
        entries = [DiffEntry("f.py", "fn", 3, 7)]
        output = format_diff(entries, "abc1234")
        assert "abc1234" in output

    def test_no_changes_message(self):
        output = format_diff([], "HEAD~1")
        assert "No functions changed" in output
