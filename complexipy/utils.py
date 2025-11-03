from __future__ import annotations

import os
import platform
from typing import (
    List,  # It's important to use this to make it compatible with python 3.8, don't remove it
    Literal,
    Tuple,
    overload,
)

from rich.align import Align
from rich.console import Console
from rich.table import Table
from rich.text import Text
from typer import Exit

from complexipy._complexipy import (
    FileComplexity,
    FunctionComplexity,
)
from complexipy.types import (
    ColorTypes,
    DetailTypes,
    Sort,
    TOMLBase,
    TOMLConfig,
    TOMLType,
    TOMLTypes,
)

try:
    import tomli as toml_library
except ImportError:
    import tomllib as toml_library


def get_brain_icon():
    """Get platform-appropriate brain icon"""
    if platform.system() == "Windows":
        return "Brain"
    else:
        return ":brain:"


@overload
def load_values_from_toml_key(
    key: Literal["details"], value: str | DetailTypes
) -> DetailTypes: ...
@overload
def load_values_from_toml_key(
    key: Literal["color"], value: str | ColorTypes
) -> ColorTypes: ...
@overload
def load_values_from_toml_key(
    key: Literal["sort"], value: str | Sort
) -> Sort: ...
@overload
def load_values_from_toml_key(
    key: Literal["paths", "exclude"], value: str | List[str]
) -> List[str]: ...
@overload
def load_values_from_toml_key(
    key: str,
    value: int
    | bool
    | str
    | List[str]
    | DetailTypes
    | Sort
    | ColorTypes
    | TOMLType,
) -> int | bool | List[str] | DetailTypes | ColorTypes | Sort | TOMLType: ...


def load_values_from_toml_key(
    key: str,
    value: int
    | bool
    | str
    | List[str]
    | DetailTypes
    | Sort
    | ColorTypes
    | TOMLType,
):
    """Normalize TOML values to expected runtime types.

    - Convert `details` and `sort` string values to their Enum variants.
    - Ensure `paths` and `exclude` are lists when provided as strings.
    """
    if key == "details":
        if isinstance(value, DetailTypes):
            return value
        elif isinstance(value, str):
            return DetailTypes(value)
        return value
    elif key == "color":
        if isinstance(value, ColorTypes):
            return value
        elif isinstance(value, str):
            return ColorTypes(value)
        return value
    elif key == "sort":
        if isinstance(value, Sort):
            return value
        elif isinstance(value, str):
            return Sort(value)
        return value
    elif key in ("paths", "exclude"):
        if isinstance(value, str):
            return [value]
        return value

    return value


def get_dict_from_pyproject(
    data: TOMLBase,
) -> TOMLConfig | None:
    tools_info = data.get("tool", None)

    if tools_info is None:
        return None

    return tools_info.get("complexipy", None)


def load_toml_config(
    invocation_path: str, config_file_name: str
) -> TOMLConfig | None:
    config_file_path = os.path.join(invocation_path, config_file_name)

    if not os.path.exists(config_file_path):
        return None

    with open(config_file_path, "rb") as config_file:
        if config_file_name == "pyproject.toml":
            toml_base: TOMLBase = toml_library.load(config_file)
            loaded = get_dict_from_pyproject(toml_base)

            if loaded is None:
                return None
            else:
                data = loaded

        else:
            data: TOMLConfig = toml_library.load(config_file)

    for key, value in data.items():
        data[key] = load_values_from_toml_key(key, value)

    return data


def get_complexipy_toml_config(
    invocation_path: str,
) -> TOMLConfig | None:
    toml = load_toml_config(invocation_path, "complexipy.toml")

    if toml is not None:
        return toml

    toml = load_toml_config(invocation_path, ".complexipy.toml")

    if toml is not None:
        return toml

    return load_toml_config(invocation_path, "pyproject.toml")


def template_getter(
    toml_config: TOMLConfig | None,
    arg_name: str,
    default_value: TOMLTypes,
) -> TOMLTypes:
    if toml_config is None:
        print(
            f"You need to define {arg_name} in the CLI call arguments or in complexipy.toml file"
        )
        raise Exit(code=1)

    arg_value = toml_config.get(arg_name, default_value)

    if arg_value is None:
        print(
            f"You need to define {arg_name} in the CLI call arguments or in complexipy.toml file"
        )
        raise Exit(code=1)

    return arg_value


def get_argument_value(
    toml_config: TOMLConfig | None,
    arg_name: str,
    arg_value: TOMLTypes | None,
    default_value: TOMLTypes,
) -> TOMLTypes:
    if arg_value is not None:
        return arg_value

    if toml_config is None and arg_name != "paths":
        return default_value
    elif toml_config is None and arg_name == "paths" and arg_value is None:
        print(
            f"You need to define {arg_name} in the CLI call arguments or in complexipy.toml file"
        )
        raise Exit(code=1)

    return template_getter(toml_config, arg_name, default_value)


def get_arguments_value(
    toml_config: TOMLConfig | None,
    paths: List[str] | None,
    max_complexity_allowed: int | None,
    quiet: bool | None,
    ignore_complexity: bool | None,
    details: DetailTypes | None,
    color: ColorTypes | None,
    sort_arg: Sort | None,
    output_csv: bool | None,
    output_json: bool | None,
    exclude: List[str] | None,
) -> Tuple[
    List[str],
    int,
    bool,
    bool,
    DetailTypes,
    ColorTypes,
    Sort,
    bool,
    bool,
    List[str],
]:
    paths = get_argument_value(toml_config, "paths", paths, [])
    max_complexity_allowed = get_argument_value(
        toml_config, "max-complexity-allowed", max_complexity_allowed, 15
    )
    quiet = get_argument_value(toml_config, "quiet", quiet, False)
    ignore_complexity = get_argument_value(
        toml_config, "ignore-complexity", ignore_complexity, False
    )
    details = get_argument_value(
        toml_config, "details", details, DetailTypes.normal
    )
    color = get_argument_value(toml_config, "color", color, ColorTypes.auto)
    sort_arg = get_argument_value(toml_config, "sort", sort_arg, Sort.asc)
    output_csv = get_argument_value(
        toml_config, "output-csv", output_csv, False
    )
    output_json = get_argument_value(
        toml_config, "output-json", output_json, False
    )
    exclude = get_argument_value(toml_config, "exclude", exclude, [])

    return (
        paths,
        max_complexity_allowed,
        quiet,
        ignore_complexity,
        details,
        color,
        sort_arg,
        output_csv,
        output_json,
        exclude,
    )


def output_summary(
    console: Console,
    files: List[FileComplexity],
    details: DetailTypes,
    sort: Sort,
    ignore_complexity: bool,
    max_complexity: int,
) -> bool:
    table, has_success, total_complexity, total_functions = create_table(
        files, details, sort, ignore_complexity, max_complexity
    )

    if details == DetailTypes.low and table.row_count < 1:
        console.print(
            f"No function{'s' if len(files) > 1 else ''} were found with complexity greater than {max_complexity}."
        )
    else:
        if total_functions == 0:
            console.print(
                Align.center(
                    "No files were found with functions. No complexity was calculated."
                )
            )
        else:
            console.print(Align.center(table))
            brain_icon = get_brain_icon()
            console.print(
                f"{brain_icon} Total Cognitive Complexity: {total_complexity}"
            )

    return has_success


def create_table(
    files: List[FileComplexity],
    details: DetailTypes,
    sort: Sort,
    ignore_complexity: bool,
    max_complexity: int,
) -> Tuple[Table, bool, int, int]:
    has_success = True
    all_functions: List[Tuple[str, str, FunctionComplexity]] = []
    total_complexity = 0

    table = Table(
        title="Summary",
        show_header=True,
        header_style="bold magenta",
        show_lines=True,
    )
    table.add_column("Path")
    table.add_column("File")
    table.add_column("Function")
    table.add_column("Complexity")

    for file in files:
        for function in file.functions:
            total_complexity += function.complexity
            all_functions.append((file.path, file.file_name, function))

    if sort != Sort.file_name:
        all_functions.sort(key=lambda x: x[2].complexity)

        if sort == Sort.desc:
            all_functions.reverse()

    for function in all_functions:
        if function[2].complexity > max_complexity:
            table.add_row(
                f"{function[0]}",
                f"[green]{function[1]}[/green]",
                f"[green]{function[2].name}[/green]",
                f"[red]{function[2].complexity}[/red]",
            )
            has_success = False
        elif details != DetailTypes.low:
            table.add_row(
                f"{function[0]}",
                f"[green]{function[1]}[/green]",
                f"[green]{function[2].name}[/green]",
                f"[blue]{function[2].complexity}[/blue]",
            )

    if ignore_complexity:
        has_success = True

    return table, has_success, total_complexity, len(all_functions)


def print_failed_paths(console: Console, quiet: bool, failed_paths: List[str]):
    has_success = True

    if failed_paths:
        has_success = False

    if quiet:
        return has_success

    for failed_path in failed_paths:
        text = Text()
        text.append("error", style="bold red")
        text.append(f": Failed to process {failed_path}", style="bold white")
        text.append(" - Please check syntax")
        console.print(text)

    return has_success


def has_success_functions(
    files: List[FileComplexity], max_complexity: int
) -> bool:
    return all(
        all(
            function.complexity <= max_complexity for function in file.functions
        )
        for file in files
    )
