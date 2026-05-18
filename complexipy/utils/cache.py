from __future__ import annotations

import hashlib
import json
import re
import time
from pathlib import Path
from typing import (
    Dict,
    List,  # It's important to use this to make it compatible with python 3.8, don't remove it
    Optional,
    cast,
)

from complexipy._complexipy import FileComplexity

CACHE_DIR_NAME = ".complexipy_cache"
CACHE_VALUES_DIR = "v/cache"
FUNCTIONS_CACHE_KEY = "functions"
MAX_CACHE_ENTRIES = 64
CACHEDIR_TAG_CONTENT = """Signature: 8a477f597d28d172789f06886806bc55
# This file is a cache directory tag created by complexipy.
# For information about cache directory tags, see:
#	https://bford.info/cachedir/spec.html
"""
README_CONTENT = """# complexipy cache directory #

This directory contains data from complexipy's cache, which stores previous
complexity results so future runs can compare per-function complexity changes.

**Do not** commit this to version control.
"""


def remember_previous_functions(
    invocation_path: str,
    targets: List[str],
    files_complexities: List[FileComplexity],
) -> Optional[dict[tuple[str, str, str], int]]:
    """Store per-function results for the target set and return previous map.

    The cache is stored in a pytest-like key/value layout to keep the number of
    files bounded. Different target sets are entries inside one cache value, and
    old target-set entries are pruned automatically.
    """
    cache_key = _build_cache_key(invocation_path, targets)
    if cache_key is None:
        return None

    cache_dir = Path(invocation_path) / CACHE_DIR_NAME
    cache_file = _cache_value_path(cache_dir, FUNCTIONS_CACHE_KEY)
    try:
        _ensure_cache_dir_and_supporting_files(cache_dir)
    except OSError:
        return None

    cache_store = _load_cache_store(cache_file)
    _migrate_legacy_cache_entry(cache_dir, cache_store, cache_key)
    previous_map = _load_previous_map_from_store(cache_store, cache_key)

    cache_store.setdefault("entries", {})[cache_key] = {
        "targets": _normalize_targets(invocation_path, targets),
        "functions": _collect_functions(files_complexities),
        "updated_at": time.time(),
    }
    _prune_cache_store(cache_store)
    if _persist_cache(cache_file, cache_store):
        _remove_legacy_cache_files(cache_dir)
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


def _cache_value_path(cache_dir: Path, key: str) -> Path:
    return cache_dir.joinpath(CACHE_VALUES_DIR, key)


def _load_cache_store(cache_file: Path) -> dict:
    raw = _load_cache(cache_file)
    if raw is None:
        return {"entries": {}}

    entries = raw.get("entries")
    if not isinstance(entries, dict):
        return {"entries": {}}

    return raw


def _migrate_legacy_cache_entry(
    cache_dir: Path,
    cache_store: dict,
    cache_key: str,
) -> None:
    entries = cache_store.get("entries")
    if not isinstance(entries, dict) or cache_key in entries:
        return

    legacy_cache_file = cache_dir / f"{cache_key}.json"
    legacy_payload = _load_cache(legacy_cache_file)
    if legacy_payload is None:
        return

    try:
        updated_at = legacy_cache_file.stat().st_mtime
    except OSError:
        updated_at = 0.0

    entries[cache_key] = {
        "targets": legacy_payload.get("targets", []),
        "functions": legacy_payload.get("functions", []),
        "updated_at": updated_at,
    }


def _load_previous_map_from_store(
    cache_store: dict,
    cache_key: str,
) -> Optional[dict[tuple[str, str, str], int]]:
    entries = cache_store.get("entries")
    if not isinstance(entries, dict):
        return None

    entry = entries.get(cache_key)
    if not isinstance(entry, dict):
        return None

    return _load_previous_map(entry)


def _load_previous_map(
    raw: dict,
) -> Optional[dict[tuple[str, str, str], int]]:
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


def _prune_cache_store(cache_store: dict) -> None:
    entries = cache_store.get("entries")
    if not isinstance(entries, dict) or len(entries) <= MAX_CACHE_ENTRIES:
        return

    sorted_entries = sorted(
        entries.items(),
        key=lambda item: _entry_updated_at(item[1]),
        reverse=True,
    )
    cache_store["entries"] = dict(sorted_entries[:MAX_CACHE_ENTRIES])


def _entry_updated_at(entry: object) -> float:
    if not isinstance(entry, dict):
        return 0.0

    entry_dict = cast(Dict[str, object], entry)
    updated_at = entry_dict.get("updated_at")
    if isinstance(updated_at, bool):
        return 0.0

    if isinstance(updated_at, (int, float, str)):
        try:
            return float(updated_at)
        except ValueError:
            return 0.0

    return 0.0


def _persist_cache(cache_file: Path, payload: dict) -> bool:
    try:
        cache_file.parent.mkdir(parents=True, exist_ok=True)
        cache_file.write_text(
            json.dumps(payload, indent=2, sort_keys=True), encoding="utf-8"
        )
    except OSError:
        # Failing to persist the cache should not break the main command.
        return False

    return True


def _remove_legacy_cache_files(cache_dir: Path) -> None:
    for legacy_cache_file in cache_dir.glob("*.json"):
        try:
            legacy_cache_file.unlink()
        except OSError:
            # Failing to remove legacy cache files should not break the main command.
            pass


def _ensure_cache_dir_and_supporting_files(cache_dir: Path) -> None:
    """Create the cache directory and support files used by cache tools."""
    cache_dir.mkdir(exist_ok=True)
    _write_support_file(cache_dir / ".gitignore", "*\n")
    _write_support_file(cache_dir / "CACHEDIR.TAG", CACHEDIR_TAG_CONTENT)
    _write_support_file(cache_dir / "README.md", README_CONTENT)


def _write_support_file(path: Path, content: str) -> None:
    if path.exists():
        return

    try:
        path.write_text(content, encoding="utf-8")
    except OSError:
        # Failing to create support files should not break the main command.
        pass
