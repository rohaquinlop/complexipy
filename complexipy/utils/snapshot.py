from __future__ import annotations

import os
from dataclasses import dataclass
from typing import Dict, List, Optional, Tuple

from rich.console import Console

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


@dataclass
class SnapshotEvaluation:
    should_run: bool
    active_snapshot_map: Optional[Dict[Tuple[str, str, str], int]]
    watermark_success: bool
    watermark_messages: List[str]
    snapshot_result: bool


def evaluate_snapshot(
    snapshot_create: bool,
    snapshot_ignore: bool,
    output_snapshot_path: str,
    max_complexity_allowed: int,
    files_complexities: List[FileComplexity],
) -> SnapshotEvaluation:
    handle_snapshot_file_creation(
        snapshot_create,
        output_snapshot_path,
        max_complexity_allowed,
        files_complexities,
    )

    snapshot_file_exists = os.path.exists(output_snapshot_path)
    snapshot_files = handle_snapshot_functions_load(output_snapshot_path)
    should_run = snapshot_file_exists and not snapshot_ignore

    active_snapshot_map = (
        build_snapshot_map(snapshot_files) if should_run else None
    )

    watermark_success, watermark_messages = handle_snapshot_watermark(
        should_run,
        snapshot_file_exists,
        output_snapshot_path,
        files_complexities,
        snapshot_files,
        max_complexity_allowed,
    )

    if should_run:
        snapshot_result = watermark_success
    else:
        snapshot_result = True

    return SnapshotEvaluation(
        should_run=should_run,
        active_snapshot_map=active_snapshot_map,
        watermark_success=watermark_success,
        watermark_messages=watermark_messages,
        snapshot_result=snapshot_result,
    )


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


def _is_legacy_snapshot(snapshot_files: List[FileComplexity]) -> bool:
    return any(
        not hasattr(file_complexity, "functions")
        for file_complexity in snapshot_files
    )


def handle_snapshot(
    console: Console,
    snap: SnapshotEvaluation,
    output_snapshot_path: str,
) -> bool:
    if not snap.should_run:
        return True

    if not snap.watermark_messages:
        console.print(
            f"Snapshot watermark passed. Baseline stored at {output_snapshot_path}"
        )
        return snap.watermark_success

    for message in snap.watermark_messages:
        console.print(f"[bold red]Snapshot watermark[/bold red]: {message}")

    return snap.watermark_success
