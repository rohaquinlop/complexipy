from __future__ import annotations

import hashlib
import json
from typing import List

from complexipy._complexipy import FileComplexity

from .output import normalize_path

_CHECK_NAME = "complexipy/cognitive-complexity"
_DEFAULT_SEVERITY = "minor"


def _build_description(
    function_name: str, complexity: int, max_complexity: int
) -> str:
    return (
        f"Function '{function_name}' has cognitive complexity {complexity} "
        f"(max allowed: {max_complexity})."
    )


def _build_fingerprint(path: str, function_name: str, line_start: int) -> str:
    payload = f"CC001:{path}:{function_name}:{line_start}"
    return hashlib.sha256(payload.encode("utf-8")).hexdigest()


def store_gitlab(
    output_path: str,
    files: List[FileComplexity],
    max_complexity: int,
) -> None:
    """Write complexity violations as a GitLab Code Quality report."""
    report = []

    for file in files:
        normalized_path = normalize_path(file.path, file.file_name)
        relative_path = (
            normalized_path[2:]
            if normalized_path.startswith("./")
            else normalized_path
        )

        for function in file.functions:
            if function.complexity <= max_complexity:
                continue

            report.append(
                {
                    "description": _build_description(
                        function.name,
                        int(function.complexity),
                        max_complexity,
                    ),
                    "check_name": _CHECK_NAME,
                    "fingerprint": _build_fingerprint(
                        relative_path,
                        function.name,
                        int(function.line_start),
                    ),
                    "severity": _DEFAULT_SEVERITY,
                    "location": {
                        "path": relative_path,
                        "lines": {"begin": int(function.line_start)},
                    },
                }
            )

    with open(output_path, "w", encoding="utf-8") as f:
        json.dump(report, f, indent=2)
        f.write("\n")
