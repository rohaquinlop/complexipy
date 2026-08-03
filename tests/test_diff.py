from __future__ import annotations

import os
import subprocess
import tempfile
from unittest.mock import patch

from complexipy import DiffStatus, file_complexity
from complexipy._complexipy import main as _main
from complexipy.types import ExitReport
from complexipy.utils.diff import (
    DiffEntry,
    _resolve_git_path,
    compute_diff,
    format_diff,
    has_regressions,
)

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
        files, _ = _main(
            [tmp_path],
            False,
            [],
            invocation_path=tmp_dir,
        )
    finally:
        os.unlink(tmp_path)
        os.rmdir(tmp_dir)
    return files[0]


class TestDiffEntry:
    def test_status_new(self):
        e = DiffEntry("f.py", "foo", None, 5)
        assert e.status == DiffStatus.NEW

    def test_status_removed(self):
        e = DiffEntry("f.py", "foo", 5, None)
        assert e.status == DiffStatus.REMOVED

    def test_status_regressed(self):
        e = DiffEntry("f.py", "foo", 3, 7)
        assert e.status == DiffStatus.REGRESSED

    def test_status_improved(self):
        e = DiffEntry("f.py", "foo", 7, 3)
        assert e.status == DiffStatus.IMPROVED

    def test_status_unchanged(self):
        e = DiffEntry("f.py", "foo", 4, 4)
        assert e.status == DiffStatus.UNCHANGED

    def test_delta_change(self):
        e = DiffEntry("f.py", "foo", 3, 7)
        assert e.delta == 4

    def test_delta_none_for_new(self):
        e = DiffEntry("f.py", "foo", None, 5)
        assert e.delta is None

    def test_delta_none_for_removed(self):
        e = DiffEntry("f.py", "foo", 5, None)
        assert e.delta is None


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

        assert all(e.status == DiffStatus.NEW for e in entries)
        assert any(e.func_name == "with_if" for e in entries)

    def test_unchanged_function(self):
        current = [self._file(_SIMPLE)]
        # Same code at the reference → no change
        with patch(
            "complexipy.utils.diff._file_content_at_ref", return_value=_SIMPLE
        ), patch("complexipy.utils.diff._git_root", return_value="/repo"):
            entries = compute_diff(current, "HEAD~1", "/repo")

        simple_entry = next(e for e in entries if e.func_name == "simple")
        assert simple_entry.status == DiffStatus.UNCHANGED

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
        assert complex_entry.status == DiffStatus.NEW  # not in old version

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
        assert simple_entry.status == DiffStatus.NEW

    def test_removed_function_appears_in_entries(self):
        # Current file has only `simple`; old had `simple` + `with_if`
        current = [self._file(_SIMPLE)]
        old_code = _SIMPLE + "\n" + _WITH_IF
        with patch(
            "complexipy.utils.diff._file_content_at_ref", return_value=old_code
        ), patch("complexipy.utils.diff._git_root", return_value="/repo"):
            entries = compute_diff(current, "HEAD~1", "/repo")

        removed = next((e for e in entries if e.func_name == "with_if"), None)
        assert removed is not None
        assert removed.status == DiffStatus.REMOVED

    def test_multiple_files(self):
        f1 = self._file(_SIMPLE, "a.py")
        f2 = self._file(_WITH_IF, "b.py")

        def _mock_content(git_ref, path, cwd):
            return _SIMPLE  # both files look simple in the old version

        with patch(
            "complexipy.utils.diff._file_content_at_ref",
            side_effect=_mock_content,
        ), patch("complexipy.utils.diff._git_root", return_value="/repo"):
            entries = compute_diff([f1, f2], "HEAD~1", "/repo")

        file_names = {os.path.basename(e.file_path) for e in entries}
        assert "a.py" in file_names
        assert "b.py" in file_names

    def test_git_error_skips_file(self):
        current = [self._file(_SIMPLE)]
        # Simulate unparseable old content
        with patch(
            "complexipy.utils.diff._file_content_at_ref",
            return_value="not python!!!",
        ), patch("complexipy.utils.diff._git_root", return_value="/repo"):
            # Should not raise
            entries = compute_diff(current, "HEAD~1", "/repo")
        # If parsing fails the file is skipped; entries may be empty
        assert isinstance(entries, list)


class TestFormatDiff:
    @staticmethod
    def _render(entries, git_ref="HEAD~1"):
        """Render diff to a string via a capturing Console."""
        import io

        from rich.console import Console

        buf = io.StringIO()
        console = Console(file=buf, color_system=None, highlight=False)
        format_diff(console, entries, git_ref)
        return buf.getvalue()

    def test_empty_entries_no_changes(self):
        entries = [DiffEntry("f.py", "foo", 3, 3)]  # UNCHANGED
        output = self._render(entries)
        # All entries are UNCHANGED — filtered out, shows no-changes message
        assert "No functions changed" in output
        assert "foo" not in output

    def test_regressed_entry_shown(self):
        entries = [DiffEntry("f.py", "handle", 10, 18)]
        output = self._render(entries)
        assert "REGRESSED" in output
        assert "handle" in output
        assert "10 → 18" in output

    def test_improved_entry_shown(self):
        entries = [DiffEntry("f.py", "helper", 20, 8)]
        output = self._render(entries)
        assert "IMPROVED" in output
        assert "20 → 8" in output

    def test_new_entry_shown(self):
        entries = [DiffEntry("f.py", "newcomer", None, 5)]
        output = self._render(entries)
        assert "NEW" in output
        assert "newcomer" in output

    def test_removed_entry_shown(self):
        entries = [DiffEntry("f.py", "gone", 7, None)]
        output = self._render(entries)
        assert "REMOVED" in output
        assert "gone" in output

    def test_summary_counts(self):
        entries = [
            DiffEntry("f.py", "a", 5, 10),  # REGRESSED
            DiffEntry("f.py", "b", 10, 5),  # IMPROVED
            DiffEntry("f.py", "c", None, 3),  # NEW
        ]
        output = self._render(entries)
        assert "1 regressed" in output
        assert "1 improved" in output
        assert "1 new" in output

    def test_ref_name_in_header(self):
        entries = [DiffEntry("f.py", "fn", 3, 7)]
        output = self._render(entries, "abc1234")
        assert "abc1234" in output

    def test_no_changes_message(self):
        output = self._render([], "HEAD~1")
        assert "No functions changed" in output


class TestHasRegressions:
    def test_regressed_above_threshold_triggers(self):
        # Regressed AND breaches the threshold.
        entries = [DiffEntry("f.py", "foo", 10, 20)]
        assert has_regressions(entries, max_complexity=15) is True

    def test_regressed_within_threshold_passes(self):
        # Regressed but still under the threshold — should not fail.
        entries = [DiffEntry("f.py", "foo", 3, 4)]
        assert has_regressions(entries, max_complexity=15) is False

    def test_regressed_at_threshold_passes(self):
        # Regressed to exactly the threshold is allowed.
        entries = [DiffEntry("f.py", "foo", 10, 15)]
        assert has_regressions(entries, max_complexity=15) is False

    def test_regressed_already_above_threshold_gets_worse_triggers(self):
        # Already above the threshold and getting worse must fail.
        entries = [DiffEntry("f.py", "foo", 16, 18)]
        assert has_regressions(entries, max_complexity=15) is True

    def test_new_function_above_threshold_triggers(self):
        entries = [DiffEntry("f.py", "bar", None, 20)]
        assert has_regressions(entries, max_complexity=15) is True

    def test_new_function_within_threshold_passes(self):
        entries = [DiffEntry("f.py", "bar", None, 10)]
        assert has_regressions(entries, max_complexity=15) is False

    def test_new_function_at_threshold_passes(self):
        entries = [DiffEntry("f.py", "bar", None, 15)]
        assert has_regressions(entries, max_complexity=15) is False

    def test_improved_function_passes(self):
        entries = [DiffEntry("f.py", "foo", 20, 10)]
        assert has_regressions(entries, max_complexity=15) is False

    def test_unchanged_function_passes(self):
        entries = [DiffEntry("f.py", "foo", 5, 5)]
        assert has_regressions(entries, max_complexity=15) is False

    def test_removed_function_passes(self):
        entries = [DiffEntry("f.py", "foo", 10, None)]
        assert has_regressions(entries, max_complexity=15) is False

    def test_empty_entries_passes(self):
        assert has_regressions([], max_complexity=15) is False

    def test_mixed_entries_regressed_above_threshold_triggers(self):
        entries = [
            DiffEntry("f.py", "good", 10, 5),  # IMPROVED
            DiffEntry("f.py", "bad", 14, 20),  # REGRESSED and over threshold
            DiffEntry("f.py", "same", 3, 3),  # UNCHANGED
        ]
        assert has_regressions(entries, max_complexity=15) is True

    def test_mixed_entries_no_regression_passes(self):
        entries = [
            DiffEntry("f.py", "good", 10, 5),  # IMPROVED
            DiffEntry("f.py", "new_ok", None, 8),  # NEW, under threshold
            DiffEntry("f.py", "bumped", 5, 12),  # REGRESSED but under threshold
            DiffEntry("f.py", "same", 3, 3),  # UNCHANGED
        ]
        assert has_regressions(entries, max_complexity=15) is False


class TestResolveGitPath:
    """Tests for the git path resolution helper.

    The Rust runner computes ``file.path`` relative to the *parent* of the
    target directory.  When the invocation path equals the git root (e.g.
    ``complexipy .``), this produces paths with an extra leading component.
    ``_resolve_git_path`` strips components until ``git show`` finds the file.
    """

    # Paths that exist at the mocked git ref.
    _KNOWN = {"complexipy/main.py", "complexipy/utils/output.py", "src/app.py"}

    def _mock_content(self, git_ref, path, cwd):
        return "content" if path in self._KNOWN else None

    def test_path_exists_as_is(self):
        """Path found directly — no stripping needed."""
        with patch(
            "complexipy.utils.diff._file_content_at_ref",
            side_effect=self._mock_content,
        ):
            result = _resolve_git_path("complexipy/main.py", "main", "/repo")
        assert result == "complexipy/main.py"

    def test_extra_prefix_stripped(self):
        """Runner produced a double-nested path — strip the extra prefix."""
        with patch(
            "complexipy.utils.diff._file_content_at_ref",
            side_effect=self._mock_content,
        ):
            result = _resolve_git_path(
                "complexipy/complexipy/main.py", "main", "/repo"
            )
        assert result == "complexipy/main.py"

    def test_extra_prefix_stripped_nested_path(self):
        """Extra prefix with a nested subdirectory path."""
        with patch(
            "complexipy.utils.diff._file_content_at_ref",
            side_effect=self._mock_content,
        ):
            result = _resolve_git_path(
                "complexipy/complexipy/utils/output.py", "main", "/repo"
            )
        assert result == "complexipy/utils/output.py"

    def test_no_match_returns_original(self):
        """File doesn't exist at the ref — return original path."""
        with patch(
            "complexipy.utils.diff._file_content_at_ref",
            return_value=None,
        ):
            result = _resolve_git_path("nonexistent/file.py", "main", "/repo")
        assert result == "nonexistent/file.py"

    def test_single_segment_path(self):
        """Path with no slashes — nothing to strip."""
        with patch(
            "complexipy.utils.diff._file_content_at_ref",
            side_effect=self._mock_content,
        ):
            result = _resolve_git_path("app.py", "main", "/repo")
        assert result == "app.py"

    def test_backslash_normalization(self):
        """Windows-style backslashes are converted to forward slashes."""
        with patch(
            "complexipy.utils.diff._file_content_at_ref",
            side_effect=self._mock_content,
        ):
            result = _resolve_git_path(
                "complexipy\\complexipy\\main.py", "main", "/repo"
            )
        assert result == "complexipy/main.py"

    def test_triple_nested_prefix(self):
        """Three extra components — strip until a match is found."""
        with patch(
            "complexipy.utils.diff._file_content_at_ref",
            side_effect=self._mock_content,
        ):
            result = _resolve_git_path(
                "a/b/complexipy/main.py", "main", "/repo"
            )
        assert result == "complexipy/main.py"

    def test_basename_fallback_unique_match(self):
        """Bare basename resolves to the unique tracked file with that name."""
        with patch(
            "complexipy.utils.diff._file_content_at_ref",
            return_value=None,
        ), patch(
            "complexipy.utils.diff._git_tracked_paths",
            return_value=["src/app.py", "complexipy/main.py"],
        ):
            result = _resolve_git_path("app.py", "main", "/repo")
        assert result == "src/app.py"

    def test_basename_fallback_exact_match(self):
        """A tracked path equal to the bare basename is a match."""
        with patch(
            "complexipy.utils.diff._file_content_at_ref",
            return_value=None,
        ), patch(
            "complexipy.utils.diff._git_tracked_paths",
            return_value=["app.py"],
        ):
            result = _resolve_git_path("app.py", "main", "/repo")
        assert result == "app.py"

    def test_basename_fallback_ambiguous_returns_original(self):
        """Several tracked files share the basename — keep the original."""
        with patch(
            "complexipy.utils.diff._file_content_at_ref",
            return_value=None,
        ), patch(
            "complexipy.utils.diff._git_tracked_paths",
            return_value=["src/app.py", "lib/app.py"],
        ):
            result = _resolve_git_path("app.py", "main", "/repo")
        assert result == "app.py"

    def test_basename_fallback_no_match_returns_original(self):
        """No tracked file matches the basename — keep the original."""
        with patch(
            "complexipy.utils.diff._file_content_at_ref",
            return_value=None,
        ), patch(
            "complexipy.utils.diff._git_tracked_paths",
            return_value=["src/app.py"],
        ):
            result = _resolve_git_path("other.py", "main", "/repo")
        assert result == "other.py"


class TestExitReport:
    def test_enforce_true_regression_above_threshold_fails(self):
        report = ExitReport(
            display_ok=True,
            snapshot_ok=True,
            paths_ok=True,
            diff_ok=False,
            enforce_diff=True,
        )
        assert report.success is False

    def test_enforce_true_regression_within_threshold_passes(self):
        report = ExitReport(
            display_ok=True,
            snapshot_ok=True,
            paths_ok=True,
            diff_ok=True,
            enforce_diff=True,
        )
        assert report.success is True

    def test_enforce_true_new_above_threshold_fails(self):
        report = ExitReport(
            display_ok=True,
            snapshot_ok=True,
            paths_ok=True,
            diff_ok=False,
            enforce_diff=True,
        )
        assert report.success is False

    def test_enforce_true_new_within_threshold_passes(self):
        report = ExitReport(
            display_ok=True,
            snapshot_ok=True,
            paths_ok=True,
            diff_ok=True,
            enforce_diff=True,
        )
        assert report.success is True

    def test_enforce_true_no_regressions_passes(self):
        report = ExitReport(
            display_ok=True,
            snapshot_ok=True,
            paths_ok=True,
            diff_ok=True,
            enforce_diff=True,
        )
        assert report.success is True

    def test_enforce_false_ignores_diff(self):
        report = ExitReport(
            display_ok=True,
            snapshot_ok=True,
            paths_ok=True,
            diff_ok=False,
            enforce_diff=False,
        )
        assert report.success is True

    def test_enforce_false_uses_display_ok(self):
        report = ExitReport(
            display_ok=False,
            snapshot_ok=True,
            paths_ok=True,
            diff_ok=True,
            enforce_diff=False,
        )
        assert report.success is False

    def test_enforce_true_invalid_paths_fails(self):
        report = ExitReport(
            display_ok=True,
            snapshot_ok=True,
            paths_ok=False,
            diff_ok=True,
            enforce_diff=True,
        )
        assert report.success is False

    def test_enforce_true_snapshot_failure_fails(self):
        report = ExitReport(
            display_ok=True,
            snapshot_ok=False,
            paths_ok=True,
            diff_ok=True,
            enforce_diff=True,
        )
        assert report.success is False

    def test_snapshot_counted_once_in_normal_mode(self):
        report = ExitReport(
            display_ok=True,
            snapshot_ok=False,
            paths_ok=True,
            diff_ok=True,
            enforce_diff=False,
        )
        assert report.success is False

    def test_display_ok_ignored_in_diff_mode(self):
        report = ExitReport(
            display_ok=False,
            snapshot_ok=True,
            paths_ok=True,
            diff_ok=True,
            enforce_diff=True,
        )
        assert report.success is True

    def test_paths_ok_required_in_both_modes(self):
        for enforce in (True, False):
            report = ExitReport(
                display_ok=True,
                snapshot_ok=True,
                paths_ok=False,
                diff_ok=True,
                enforce_diff=enforce,
            )
            assert report.success is False


class TestPublicApi:
    def _file(self, code: str, path: str = "src/example.py"):
        return _make_file_complexity(code, path)

    def test_exported_names_are_public(self):
        import complexipy

        for name in (
            "compute_diff",
            "has_regressions",
            "DiffEntry",
            "DiffStatus",
        ):
            assert name in complexipy.__all__
            assert getattr(complexipy, name) is not None

    def test_reexports_are_the_same_objects(self):
        import complexipy

        from complexipy.utils.diff import DiffEntry as _DiffEntry
        from complexipy.utils.diff import DiffStatus as _DiffStatus
        from complexipy.utils.diff import compute_diff as _compute_diff
        from complexipy.utils.diff import has_regressions as _has_regressions

        assert complexipy.DiffEntry is _DiffEntry
        assert complexipy.DiffStatus is _DiffStatus
        assert complexipy.compute_diff is _compute_diff
        assert complexipy.has_regressions is _has_regressions

    def test_status_enum_is_str_comparable(self):
        assert DiffStatus.REGRESSED == "REGRESSED"
        assert DiffStatus.IMPROVED == "IMPROVED"
        assert DiffStatus.UNCHANGED == "UNCHANGED"
        assert DiffStatus.NEW == "NEW"
        assert DiffStatus.REMOVED == "REMOVED"

    def test_status_is_enum_member(self):
        e = DiffEntry("f.py", "foo", 3, 7)
        assert e.status is DiffStatus.REGRESSED

    def test_issue_example_works_through_public_api(self):
        current = [self._file(_WITH_IF)]
        with patch(
            "complexipy.utils.diff._file_content_at_ref", return_value=_SIMPLE
        ), patch("complexipy.utils.diff._git_root", return_value="/repo"):
            entries = compute_diff(current, "origin/main", ".")

        regressions = [
            e
            for e in entries
            if e.status == DiffStatus.REGRESSED
            and e.new_complexity is not None
            and e.new_complexity > 15
        ]
        assert isinstance(entries, list)
        assert all(isinstance(e, DiffEntry) for e in entries)
        assert isinstance(regressions, list)
        assert has_regressions(entries, 15) is False

    def test_compute_diff_defaults_to_cwd(self, monkeypatch):
        monkeypatch.setattr("complexipy.utils.diff.os.getcwd", lambda: "/cwd")
        current = [self._file(_SIMPLE)]
        with patch(
            "complexipy.utils.diff._file_content_at_ref", return_value=_SIMPLE
        ), patch("complexipy.utils.diff._git_root", return_value="/cwd"):
            entries = compute_diff(current, "HEAD~1")
        assert entries[0].status == DiffStatus.UNCHANGED

    def test_file_complexity_path_is_cwd_relative(self, tmp_path, monkeypatch):
        repo = tmp_path / "repo"
        pkg = repo / "pkg"
        pkg.mkdir(parents=True)
        target = pkg / "calc.py"
        target.write_text("def calc(x):\n    return x + 1\n")
        monkeypatch.chdir(repo)

        result = file_complexity(str(target))
        assert result.path == "pkg/calc.py"

    def test_issue_flow_real_git_repo(self, tmp_path):
        """The issue's CI flow end to end against a real git repository.

        ``file_complexity`` on a changed file, ``compute_diff`` against a
        git ref, then status filtering and the ratchet gate — the exact
        scenario from issue #202, without mocks.
        """
        repo = tmp_path / "repo"
        pkg = repo / "pkg"
        pkg.mkdir(parents=True)
        target = pkg / "calc.py"
        target.write_text("def calc(x):\n    return x + 1\n")

        def _git(*args):
            return subprocess.run(
                ["git", "-c", "user.name=t", "-c", "user.email=t@t", *args],
                cwd=repo,
                capture_output=True,
                text=True,
                check=False,
            )

        assert _git("init", "-q").returncode == 0
        assert _git("add", ".").returncode == 0
        assert _git("commit", "-q", "-m", "v1").returncode == 0

        target.write_text(
            "def calc(x):\n"
            "    if x:\n"
            "        for i in range(x):\n"
            "            if i:\n"
            "                return i\n"
            "    return x + 1\n"
        )

        current = [file_complexity(str(target))]
        entries = compute_diff(current, "HEAD", str(repo))

        entry = next(e for e in entries if e.func_name == "calc")
        assert entry.status == DiffStatus.REGRESSED
        assert entry.old_complexity == 0
        assert entry.new_complexity > 1
        assert has_regressions(entries, 15) is False
        assert has_regressions(entries, 5) is True
