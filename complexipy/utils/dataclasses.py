from __future__ import annotations

from dataclasses import dataclass
from typing import List

from complexipy._complexipy import RefactorPlan


@dataclass
class FunctionRow:
    name: str
    complexity: int
    passed: bool
    path: str
    file_name: str
    refactor_plans: List[RefactorPlan]


@dataclass
class FileEntry:
    path: str
    functions: List[FunctionRow]
