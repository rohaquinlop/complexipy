from __future__ import annotations

import os
import subprocess
from dataclasses import dataclass
from typing import Dict, List, Optional

from complexipy import code_complexity as _code_complexity
from complexipy._complexipy import FileComplexity

# Status labels with fixed width for aligned output.
_STATUS_REGRESSED = "REGRESSED"
_STATUS_IMPROVED = "IMPROVED"
_STATUS_UNCHANGED = "UNCHANGED"
_STATUS_NEW = "NEW"
_STATUS_REMOVED = "REMOVED"


@dataclass
class DiffEntry:
    file_path: str
    func_name: str
    old_complexity: Optional[int]  # None when the function is new
    new_complexity: Optional[int]  # None when the function was removed

    @property
    def status(self) -> str:
        if self.old_complexity is None:
            return _STATUS_NEW
        if self.new_complexity is None:
            return _STATUS_REMOVED
        if self.new_complexity > self.old_complexity:
            return _STATUS_REGRESSED
        if self.new_complexity < self.old_complexity:
            return _STATUS_IMPROVED
        return _STATUS_UNCHANGED

    @property
    def delta(self) -> Optional[int]:
        if self.old_complexity is None or self.new_complexity is None:
            return None
        return self.new_complexity - self.old_complexity


def _git_root(cwd: str) -> Optional[str]:
    """Return the absolute path of the git repository root, or None."""
    try:
        result = subprocess.run(
            ["git", "rev-parse", "--show-toplevel"],
            cwd=cwd,
            capture_output=True,
            text=True,
            timeout=10,
        )
        if result.returncode == 0:
            return result.stdout.strip()
    except (subprocess.TimeoutExpired, OSError):
        pass
    return None


def _file_content_at_ref(git_ref: str, path_from_root: str, cwd: str) -> Optional[str]:
    """Return the file content at *git_ref*, or None if unavailable."""
    try:
        result = subprocess.run(
            ["git", "show", f"{git_ref}:{path_from_root}"],
            cwd=cwd,
            capture_output=True,
            text=True,
            timeout=15,
        )
        if result.returncode == 0:
            return result.stdout
    except (subprocess.TimeoutExpired, OSError):
        pass
    return None


def _build_func_map(file: FileComplexity) -> Dict[str, int]:
    return {f.name: f.complexity for f in file.functions}


def compute_diff(
    current_files: List[FileComplexity],
    git_ref: str,
    invocation_path: str,
) -> List[DiffEntry]:
    """Compare the current complexity results against *git_ref*.

    For each file in *current_files*, the function retrieves the file's
    content at *git_ref* using ``git show`` and re-analyses it with
    :func:`code_complexity`.  Functions that appear only in the current or
    only in the historical version are marked NEW / REMOVED respectively.

    The *invocation_path* is used as the ``cwd`` for git commands and to
    resolve file paths relative to the repository root.

    Returns a list of :class:`DiffEntry` objects, one per function that
    either changed or is new/removed.  Unchanged functions are included so
    callers can choose how to filter.
    """
    root = _git_root(invocation_path) or invocation_path
    entries: List[DiffEntry] = []

    for file in current_files:
        # Build the path relative to the git root so ``git show`` can find it.
        abs_file = os.path.normpath(os.path.join(invocation_path, file.path))
        try:
            path_from_root = os.path.relpath(abs_file, root)
        except ValueError:
            # relpath fails across drives on Windows – fall back to file.path
            path_from_root = file.path

        old_content = _file_content_at_ref(git_ref, path_from_root, invocation_path)
        current_map = _build_func_map(file)

        if old_content is None:
            # File did not exist at the reference – every function is new.
            for name, new_c in sorted(current_map.items()):
                entries.append(DiffEntry(file.path, name, None, new_c))
            continue

        try:
            old_result = _code_complexity(old_content)
        except Exception:
            continue

        old_map: Dict[str, int] = {f.name: f.complexity for f in old_result.functions}
        all_names = sorted(set(old_map) | set(current_map))

        for name in all_names:
            old_c = old_map.get(name)
            new_c = current_map.get(name)
            entries.append(DiffEntry(file.path, name, old_c, new_c))

    return entries


def format_diff(entries: List[DiffEntry], git_ref: str) -> str:
    """Return a human-readable diff table as a string."""
    if not entries:
        return f"No functions changed relative to {git_ref}.\n"

    # Filter to only entries with changes (or new/removed).
    changed = [
        e for e in entries
        if e.status in (_STATUS_REGRESSED, _STATUS_IMPROVED, _STATUS_NEW, _STATUS_REMOVED)
    ]

    lines: List[str] = []
    sep = "─" * 72
    lines.append(f"Complexity diff  (vs {git_ref})")
    lines.append(sep)

    for e in changed:
        label = e.status.ljust(10)
        location = f"{e.file_path}::{e.func_name}"

        if e.status == _STATUS_NEW:
            score = f"{e.new_complexity}"
            note = "(new)"
        elif e.status == _STATUS_REMOVED:
            score = f"{e.old_complexity}"
            note = "(removed)"
        else:
            delta = e.delta
            sign = "+" if delta and delta > 0 else ""
            score = f"{e.old_complexity} → {e.new_complexity}  ({sign}{delta})"
            note = ""

        lines.append(f"  {label}  {location:<55}  {score}  {note}")

    lines.append(sep)

    regressed = sum(1 for e in changed if e.status == _STATUS_REGRESSED)
    improved = sum(1 for e in changed if e.status == _STATUS_IMPROVED)
    new = sum(1 for e in changed if e.status == _STATUS_NEW)
    removed = sum(1 for e in changed if e.status == _STATUS_REMOVED)

    parts = []
    if regressed:
        parts.append(f"{regressed} regressed")
    if improved:
        parts.append(f"{improved} improved")
    if new:
        parts.append(f"{new} new")
    if removed:
        parts.append(f"{removed} removed")

    summary = ", ".join(parts) if parts else "no changes"
    lines.append(f"Net: {summary}")

    return "\n".join(lines) + "\n"
