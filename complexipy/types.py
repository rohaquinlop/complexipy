from enum import Enum
from typing import TypeVar, Dict, List


class DetailTypes(str, Enum):
    low = "low"  # Show only files with complexity above the max_complexity
    normal = "normal"  # Show all files with their complexity


class ColorTypes(str, Enum):
    auto = "auto"  # Decide whether to color automatically, based on output tty
    yes = "yes"  # Use color
    no = "no"  # Do not use color


class Sort(str, Enum):
    asc = "asc"
    desc = "desc"
    name = "name"


TOMLType = TypeVar(
    "TOMLType", int, bool, str, List[str], DetailTypes, Sort, Dict[str, "TOMLType"]
)
