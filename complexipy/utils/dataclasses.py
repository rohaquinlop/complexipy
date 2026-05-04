from __future__ import annotations

from dataclasses import dataclass
from typing import List


@dataclass
class FunctionRow:
    name: str
    complexity: int
    passed: bool
    path: str
    file_name: str


@dataclass
class FileEntry:
    path: str
    functions: List[FunctionRow]
