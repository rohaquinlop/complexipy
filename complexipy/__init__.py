"""complexipy - cognitive complexity analyzer for Python.

The analysis engine, the diff comparison, and the ratchet gates are
implemented in Rust and exposed through the ``complexipy._complexipy``
extension module. This package is a thin re-export layer only.
"""

from __future__ import annotations

from pathlib import Path

import complexipy._complexipy as _complexipy
from complexipy._complexipy import (
    Applicability,
    CodeComplexity,
    CodeSuggestion,
    DiffEntry,
    DiffStatus,
    FileComplexity,
    FunctionComplexity,
    IgnoredLocation,
    LineComplexity,
    RefactorPlan,
    RemovableIgnore,
    RuleCategory,
    code_complexity,
    collect_all_ignored_locations,
    collect_removable_ignored_locations,
    compute_diff,
    has_regressions,
)

__all__ = [
    "Applicability",
    "CodeComplexity",
    "CodeSuggestion",
    "DiffEntry",
    "DiffStatus",
    "FileComplexity",
    "FunctionComplexity",
    "IgnoredLocation",
    "LineComplexity",
    "RefactorPlan",
    "RemovableIgnore",
    "RuleCategory",
    "code_complexity",
    "collect_all_ignored_locations",
    "collect_removable_ignored_locations",
    "compute_diff",
    "file_complexity",
    "has_regressions",
]


def file_complexity(
    file_path: str,
    check_script: bool = False,
    no_ignore: bool = False,
) -> FileComplexity:
    """Analyze the cognitive complexity of a single Python source file.

    Args:
        file_path: Path to the Python file to analyze. Can be relative or
            absolute. The file must exist and be readable.
        check_script: If True, also report cognitive complexity of
            module-level (script) code as a '<module>' entry.
        no_ignore: If True, disregard all '# complexipy: ignore' and
            '# noqa: complexipy' comments, analyzing every function.

    Returns:
        FileComplexity object containing complete analysis results for the
        file, including all functions found and their complexity scores.

    Raises:
        FileNotFoundError: If the specified file does not exist.
        PermissionError: If the file cannot be read due to permissions.
        SyntaxError: If the Python file contains syntax errors.
    """
    path = Path(file_path).resolve()
    cwd = Path.cwd().resolve()
    try:
        path.relative_to(cwd)
    except ValueError:
        base_path = path.parent
    else:
        base_path = cwd
    return _complexipy.file_complexity(
        path.as_posix(),
        base_path.as_posix(),
        check_script,
        no_ignore,
    )
