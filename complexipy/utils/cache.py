from __future__ import annotations

import hashlib
import json
import re
from pathlib import Path
from typing import (
    List,  # It's important to use this to make it compatible with python 3.8, don't remove it
    Optional,
)

from complexipy._complexipy import FileComplexity

CACHE_DIR_NAME = ".complexipy_cache"


def remember_previous_functions(
    invocation_path: str,
    targets: List[str],
    files_complexities: List[FileComplexity],
) -> Optional[dict[tuple[str, str, str], int]]:
    """Store per-function results for the target set and return previous map.

    The cache is organized per target set (using a stable hash of the targets)
    but stores function-level complexities so future runs can compare totals
    even if the underlying functions change.
    """
    cache_key = _build_cache_key(invocation_path, targets)
    if cache_key is None:
        return None

    cache_dir = Path(invocation_path) / CACHE_DIR_NAME
    try:
        cache_dir.mkdir(exist_ok=True)
        _ensure_gitignore(cache_dir)
    except OSError:
        return None

    cache_file = cache_dir / f"{cache_key}.json"
    previous_map = _load_previous_map(cache_file)

    payload = {
        "targets": _normalize_targets(invocation_path, targets),
        "functions": _collect_functions(files_complexities),
    }
    _persist_cache(cache_file, payload)
    return previous_map


def _build_cache_key(invocation_path: str, targets: List[str]) -> Optional[str]:
    normalized_targets = _normalize_targets(invocation_path, targets)
    if not normalized_targets:
        return None

    joined = "||".join(normalized_targets)
    return hashlib.blake2b(joined.encode("utf-8"), digest_size=16).hexdigest()


def _normalize_targets(invocation_path: str, targets: List[str]) -> List[str]:
    normalized_targets: List[str] = []
    for target in targets:
        normalized = _normalize_target(invocation_path, target)
        if normalized:
            normalized_targets.append(normalized)

    normalized_targets.sort()
    return normalized_targets


def _normalize_target(invocation_path: str, target: str) -> str:
    """Normalize target paths to improve cache hits across invocations."""
    if _looks_like_remote(target):
        return target

    base_path = Path(target)
    if not base_path.is_absolute():
        base_path = Path(invocation_path) / base_path

    try:
        return base_path.resolve().as_posix()
    except OSError:
        return base_path.as_posix()


def _looks_like_remote(target: str) -> bool:
    return bool(
        re.match(r"^(https://|http://|www\.|git@)(github|gitlab)\.com", target)
    )


def _collect_functions(files_complexities: List[FileComplexity]) -> List[dict]:
    functions: List[dict] = []
    for file_complexity in files_complexities:
        for function in file_complexity.functions:
            functions.append(
                {
                    "path": file_complexity.path,
                    "file_name": file_complexity.file_name,
                    "function_name": function.name,
                    "complexity": function.complexity,
                }
            )
    return functions


def _load_previous_map(
    cache_file: Path,
) -> Optional[dict[tuple[str, str, str], int]]:
    raw = _load_cache(cache_file)
    if raw is None:
        return None

    functions = raw.get("functions", [])
    if not isinstance(functions, list):
        return None

    mapping: dict[tuple[str, str, str], int] = {}
    for entry in functions:
        if not isinstance(entry, dict):
            continue
        key = _build_function_key(
            entry.get("path", ""),
            entry.get("file_name", ""),
            entry.get("function_name", ""),
        )
        if key is None:
            continue
        complexity = entry.get("complexity")
        if isinstance(complexity, bool):
            continue
        try:
            mapping[key] = int(complexity)
        except (TypeError, ValueError):
            continue

    return mapping if mapping else None


def _load_cache(cache_file: Path) -> Optional[dict]:
    if not cache_file.exists():
        return None

    try:
        raw = json.loads(cache_file.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return None

    if not isinstance(raw, dict):
        return None

    return raw


def _build_function_key(
    path: str, file_name: str, function_name: str
) -> Optional[tuple[str, str, str]]:
    if not function_name:
        return None
    return (path, file_name, function_name)


def _persist_cache(cache_file: Path, payload: dict) -> None:
    try:
        cache_file.write_text(
            json.dumps(payload, indent=2, sort_keys=True), encoding="utf-8"
        )
    except OSError:
        # Failing to persist the cache should not break the main command.
        pass


def _ensure_gitignore(cache_dir: Path) -> None:
    """Create a .gitignore file in the cache directory to prevent accidental commits."""
    gitignore_file = cache_dir / ".gitignore"
    if not gitignore_file.exists():
        try:
            gitignore_file.write_text("*\n", encoding="utf-8")
        except OSError:
            # Failing to create .gitignore should not break the main command.
            pass
