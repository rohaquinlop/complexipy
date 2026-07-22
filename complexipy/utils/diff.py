from __future__ import annotations

import os
import subprocess
from dataclasses import dataclass
from typing import Dict, List, Optional, Tuple

import typer
from rich.console import Console

from complexipy._complexipy import (
    FileComplexity,
)
from complexipy._complexipy import (
    code_complexity as _code_complexity,
)

_STATUS_REGRESSED = "REGRESSED"
_STATUS_IMPROVED = "IMPROVED"
_STATUS_UNCHANGED = "UNCHANGED"
_STATUS_NEW = "NEW"
_STATUS_REMOVED = "REMOVED"


@dataclass
class DiffEntry:
    file_path: str
    func_name: str
    old_complexity: Optional[int]
    new_complexity: Optional[int]

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


def _file_content_at_ref(
    git_ref: str, path_from_root: str, cwd: str
) -> Optional[str]:
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


def _resolve_git_path(
    file_path: str, git_ref: str, invocation_path: str
) -> str:
    """Resolve a runner-relative file path to a git-root-relative path.

    The Rust runner computes ``file.path`` relative to the *parent* of the
    target directory (its ``base_dir``).  When the invocation path equals
    the git root (e.g. ``complexipy .``), this produces paths like
    ``complexipy/complexipy/main.py`` instead of the git-correct
    ``complexipy/main.py``.

    We fix this by trying the path as-is first, then progressively stripping
    leading components until ``git show`` can locate the file.
    """
    normalized = file_path.replace(os.sep, "/").replace("\\", "/")
    parts = normalized.split("/")

    for i in range(len(parts)):
        candidate = "/".join(parts[i:])
        if (
            _file_content_at_ref(git_ref, candidate, invocation_path)
            is not None
        ):
            return candidate

    return normalized


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
    entries: List[DiffEntry] = []

    for file in current_files:
        # file.path is relative to the parent of invocation_path (the Rust
        # runner's base_dir).  Resolve it to a git-root-relative path so
        # ``git show`` can find the file at the reference.
        path_from_root = _resolve_git_path(file.path, git_ref, invocation_path)

        old_content = _file_content_at_ref(
            git_ref, path_from_root, invocation_path
        )
        current_map = _build_func_map(file)

        if old_content is None:
            for name, new_c in sorted(current_map.items()):
                entries.append(DiffEntry(file.path, name, None, new_c))
            continue

        try:
            old_result = _code_complexity(old_content)
        except Exception:
            continue

        old_map: Dict[str, int] = {
            f.name: f.complexity for f in old_result.functions
        }
        all_names = sorted(set(old_map) | set(current_map))

        for name in all_names:
            old_c = old_map.get(name)
            new_c = current_map.get(name)
            entries.append(DiffEntry(file.path, name, old_c, new_c))

    return entries


def _status_style(status: str) -> str:
    """Return Rich markup for a diff status label."""
    if status == _STATUS_REGRESSED:
        return "[bold red]REGRESSED[/bold red]"
    if status == _STATUS_IMPROVED:
        return "[bold green]IMPROVED[/bold green]"
    if status == _STATUS_NEW:
        return "[bold yellow]NEW[/bold yellow]"
    if status == _STATUS_REMOVED:
        return "[dim]REMOVED[/dim]"
    return status


def _format_change(e: DiffEntry) -> str:
    """Return the change column value for a diff entry."""
    if e.status == _STATUS_NEW:
        return f"[bold yellow]{e.new_complexity}[/bold yellow]  (new)"
    if e.status == _STATUS_REMOVED:
        return f"[dim]{e.old_complexity}[/dim]  (removed)"
    delta = e.delta
    sign = "+" if delta and delta > 0 else ""
    if e.status == _STATUS_REGRESSED:
        return f"[red]{e.old_complexity} → {e.new_complexity}  ({sign}{delta})[/red]"
    if e.status == _STATUS_IMPROVED:
        return f"[green]{e.old_complexity} → {e.new_complexity}  ({sign}{delta})[/green]"
    return f"{e.old_complexity} → {e.new_complexity}  ({sign}{delta})"


def _build_diff_summary(changed: List[DiffEntry]) -> str:
    counts = {
        _STATUS_REGRESSED: 0,
        _STATUS_IMPROVED: 0,
        _STATUS_NEW: 0,
        _STATUS_REMOVED: 0,
    }
    for e in changed:
        if e.status in counts:
            counts[e.status] += 1

    labels = [
        (_STATUS_REGRESSED, "regressed", "red"),
        (_STATUS_IMPROVED, "improved", "green"),
        (_STATUS_NEW, "new", "yellow"),
        (_STATUS_REMOVED, "removed", "dim"),
    ]
    parts = [
        f"[{style}]{counts[s]} {label}[/{style}]"
        for s, label, style in labels
        if counts[s]
    ]
    return ", ".join(parts) if parts else "no changes"


def has_regressions(entries: List[DiffEntry], max_complexity: int) -> bool:
    """Return True if any entry represents a ratchet failure.

    A ratchet failure is:
    - A REGRESSED function whose new complexity exceeds *max_complexity*
      (the function both increased in complexity AND breaches the threshold)
    - A NEW function whose complexity exceeds *max_complexity*

    Modified functions that increase in complexity but stay at or below the
    threshold are not considered failures: the threshold remains the main
    contract, and ratchet only blocks changes that push a function over it
    (or make an already-over function worse).
    """
    for e in entries:
        if (
            e.status == _STATUS_REGRESSED
            and e.new_complexity is not None
            and e.new_complexity > max_complexity
        ):
            return True
        if (
            e.status == _STATUS_NEW
            and e.new_complexity is not None
            and e.new_complexity > max_complexity
        ):
            return True
    return False


def format_diff(
    console: Console, entries: List[DiffEntry], git_ref: str
) -> None:
    """Render a complexity diff table to the console."""
    if not entries:
        console.print(f"No functions changed relative to {git_ref}.")
        return

    changed = [
        e
        for e in entries
        if e.status
        in (_STATUS_REGRESSED, _STATUS_IMPROVED, _STATUS_NEW, _STATUS_REMOVED)
    ]

    if not changed:
        console.print(f"No functions changed relative to {git_ref}.")
        return

    from rich.table import Table

    table = Table(
        show_header=True,
        header_style="bold",
        box=None,
        pad_edge=False,
        padding=(0, 2),
    )
    table.add_column("Status")
    table.add_column("Location")
    table.add_column("Change", justify="right")

    for e in changed:
        table.add_row(
            _status_style(e.status),
            f"{e.file_path}::{e.func_name}",
            _format_change(e),
        )

    console.print()
    console.rule(f"Complexity diff (vs {git_ref})")
    console.print(table)
    console.print(f"\nNet: {_build_diff_summary(changed)}")


def handle_diff_output(
    console: Console,
    diff: Optional[str],
    files_complexities: List[FileComplexity],
    quiet: bool,
    invocation_path: str,
) -> Optional[List[DiffEntry]]:
    if diff:
        if files_complexities:
            entries = compute_diff(files_complexities, diff, invocation_path)
            if not quiet:
                format_diff(console, entries, diff)
            return entries
        return []
    return None


def resolve_diff_flags(
    console: Console,
    diff: Optional[str],
    diff_only: Optional[str],
    ratchet: bool,
) -> Tuple[Optional[str], Optional[str]]:
    if ratchet and diff:
        console.print(
            "[yellow]Deprecated:[/yellow] --ratchet is deprecated. "
            "--diff now enforces by default. Remove --ratchet."
        )
    elif ratchet and not diff:
        console.print("[bold red]Error:[/bold red] --ratchet requires --diff")
        raise typer.Exit(code=2)

    if diff_only and diff:
        console.print(
            "[yellow]Warning:[/yellow] --diff and --diff-only both set. "
            "Using --diff-only (visual only, no enforcement)."
        )
        diff = None

    return diff, diff_only
