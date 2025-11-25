from enum import Enum
from typing import List, Mapping, TypeVar

try:
    from typing import TypeAlias
except ImportError:
    from typing_extensions import TypeAlias


class ColorTypes(str, Enum):
    auto = "auto"  # Decide whether to color automatically, based on output tty
    yes = "yes"  # Use color
    no = "no"  # Do not use color


class Sort(str, Enum):
    asc = "asc"
    desc = "desc"
    file_name = "file_name"


TOMLTypes = TypeVar(
    "TOMLTypes",
    int,
    bool,
    List[str],
    ColorTypes,
    Sort,
)

TOMLType: TypeAlias = Mapping[str, TOMLTypes]
TOMLConfig: TypeAlias = Mapping[str, TOMLTypes]
TOMLBase = Mapping[str, TOMLConfig]
