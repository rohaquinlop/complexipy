from enum import Enum
from typing import TypeVar, Dict, List


class DetailTypes(str, Enum):
    low = "low"  # Show only files with complexity above the max_complexity
    normal = "normal"  # Show all files with their complexity


class Sort(str, Enum):
    asc = "asc"
    desc = "desc"
    name = "name"


TOMLType = TypeVar(
    "TOMLType", int, bool, str, List[str], DetailTypes, Sort, Dict[str, "TOMLType"]
)
