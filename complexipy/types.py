import sys
from enum import Enum
from typing import List, MutableMapping, TypeVar

if sys.version_info >= (3, 10):
    from typing import TypeAlias
else:
    from typing_extensions import TypeAlias


class ColorTypes(str, Enum):
    auto = "auto"  # Decide whether to color automatically, based on output tty
    yes = "yes"  # Use color
    no = "no"  # Do not use color


class Sort(str, Enum):
    asc = "asc"
    desc = "desc"
    file_name = "file_name"


class OutputFormat(str, Enum):
    csv = "csv"
    json = "json"
    gitlab = "gitlab"
    sarif = "sarif"


class Metric(str, Enum):
    cognitive = "cognitive"
    cyclomatic = "cyclomatic"


TOMLTypes = TypeVar(
    "TOMLTypes",
    int,
    bool,
    str,
    List[str],
    ColorTypes,
    Metric,
    OutputFormat,
    Sort,
)

TOMLType: TypeAlias = MutableMapping[str, TOMLTypes]
TOMLConfig: TypeAlias = MutableMapping[str, TOMLTypes]
TOMLBase = MutableMapping[str, TOMLConfig]
