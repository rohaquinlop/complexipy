import os
from typing import Any, Dict, List, Tuple

from complexipy._complexipy import (
    FileComplexity,
    create_snapshot_file,
    load_snapshot_file,
)


def handle_snapshot_file_creation(
    create_snapshot: bool,
    snapshot_file_path: str,
    max_complexity_allowed: int,
    files_complexities: List[FileComplexity],
) -> None:
    if create_snapshot:
        create_snapshot_file(
            snapshot_file_path, max_complexity_allowed, files_complexities
        )


def handle_snapshot_functions_load(
    snapshot_file_path: str,
) -> List[FileComplexity]:
    if os.path.exists(snapshot_file_path):
        return load_snapshot_file(snapshot_file_path)
    else:
        return []


def handle_snapshot_watermark(
    snapshot_watermark: bool,
    snapshot_exists: bool,
    snapshot_file_path: str,
    files_complexities: List[FileComplexity],
    snapshot_files: List[FileComplexity],
    max_complexity_allowed: int,
) -> Tuple[bool, List[str]]:
    if not snapshot_watermark:
        return True, []

    if not snapshot_exists:
        return (
            False,
            [
                "Snapshot watermark requested but no snapshot file was found. Run complexipy with --snapshot-create first."
            ],
        )

    if snapshot_files and _is_legacy_snapshot(snapshot_files):
        return (
            False,
            [
                "Snapshot file was created with an older version of complexipy. Please recreate it with --snapshot-create."
            ],
        )

    snapshot_map = build_snapshot_map(snapshot_files)
    violations: List[str] = []

    for file_complexity in files_complexities:
        high_complexity_functions = filter(
            lambda function: function.complexity > max_complexity_allowed,
            file_complexity.functions,
        )
        for function in high_complexity_functions:
            key = _build_function_key(
                file_complexity.path, file_complexity.file_name, function.name
            )
            previous_complexity = snapshot_map.get(key)

            if previous_complexity is None:
                violations.append(
                    f"{_format_function_location(file_complexity.path, file_complexity.file_name, function.name)} exceeds {max_complexity_allowed} but was not part of the snapshot."
                )
            elif function.complexity > previous_complexity:
                violations.append(
                    f"{_format_function_location(file_complexity.path, file_complexity.file_name, function.name)} increased from {previous_complexity} to {function.complexity}."
                )

    if violations:
        return False, violations

    create_snapshot_file(
        snapshot_file_path, max_complexity_allowed, files_complexities
    )

    return True, []


def build_snapshot_map(
    snapshot_files: List[FileComplexity],
) -> Dict[Tuple[str, str, str], int]:
    snapshot_map: Dict[Tuple[str, str, str], int] = {}
    for file_complexity in snapshot_files:
        for function in file_complexity.functions:
            key = _build_function_key(
                file_complexity.path, file_complexity.file_name, function.name
            )
            snapshot_map[key] = function.complexity
    return snapshot_map


def _build_function_key(
    path: str, file_name: str, function_name: str
) -> Tuple[str, str, str]:
    return (path, file_name, function_name)


def _format_function_location(
    path: str, file_name: str, function_name: str
) -> str:
    if path:
        location = os.path.join(path, file_name)
    else:
        location = file_name
    return f"{location}:{function_name}"


def _is_legacy_snapshot(snapshot_files: List[Any]) -> bool:
    return any(
        not hasattr(file_complexity, "functions")
        for file_complexity in snapshot_files
    )
