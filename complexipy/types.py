import sys
from dataclasses import dataclass
from enum import Enum
from typing import Dict, List, MutableMapping, Optional, TypeVar

if sys.version_info >= (3, 10):
    from typing import TypeAlias
else:
    from typing_extensions import TypeAlias


class ColorTypes(str, Enum):
    auto = "auto"
    yes = "yes"
    no = "no"


class Sort(str, Enum):
    asc = "asc"
    desc = "desc"
    file_name = "file_name"


class OutputFormat(str, Enum):
    csv = "csv"
    json = "json"
    gitlab = "gitlab"
    sarif = "sarif"


TOMLTypes = TypeVar(
    "TOMLTypes",
    int,
    bool,
    str,
    List[str],
    ColorTypes,
    OutputFormat,
    Sort,
)

TOMLType: TypeAlias = MutableMapping[str, TOMLTypes]
TOMLConfig: TypeAlias = MutableMapping[str, TOMLTypes]
TOMLBase = MutableMapping[str, TOMLConfig]


@dataclass
class RunConfig:
    paths: List[str]
    max_complexity_allowed: int
    snapshot_create: bool
    snapshot_ignore: bool
    quiet: bool
    ignore_complexity: bool
    failed: bool
    color: ColorTypes
    sort: Sort
    output_format: List[str]
    output: Optional[str]
    exclude: List[str]
    check_script: bool
    no_ignore: bool
    report_ignored: bool
    ratchet: bool
    plain: bool
    suggest_refactors: bool
    top: Optional[int]
    diff: Optional[str]
    diff_only: Optional[str]
    legacy_cli_output_flags: Dict[OutputFormat, Optional[bool]]


@dataclass
class ExitReport:
    display_ok: bool
    snapshot_ok: bool
    paths_ok: bool
    diff_ok: bool
    enforce_diff: bool

    @property
    def success(self) -> bool:
        if self.enforce_diff:
            return self.diff_ok and self.paths_ok and self.snapshot_ok
        return self.display_ok and self.snapshot_ok and self.paths_ok
