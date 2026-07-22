from __future__ import annotations

import os
import sys
from typing import (
    Any,
    Dict,
    List,  # It's important to use this to make it compatible with python 3.8, don't remove it
    Literal,
    Optional,
    Tuple,
    cast,
    overload,
)

import typer
from typer import Exit

from complexipy.types import (
    ColorTypes,
    OutputFormat,
    Sort,
    TOMLBase,
    TOMLConfig,
    TOMLType,
    TOMLTypes,
)

if sys.version_info >= (3, 11):
    import tomllib as toml_library
else:
    import tomli as toml_library


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
    key: Literal["paths", "exclude", "output-format"],
    value: str | List[str],
) -> List[str]: ...
@overload
def load_values_from_toml_key(
    key: str,
    value: int
    | bool
    | str
    | List[str]
    | Sort
    | ColorTypes
    | OutputFormat
    | TOMLType,
) -> (
    int | bool | str | List[str] | ColorTypes | OutputFormat | Sort | TOMLType
): ...


def load_values_from_toml_key(
    key: str,
    value: int
    | bool
    | str
    | List[str]
    | Sort
    | ColorTypes
    | OutputFormat
    | TOMLType,
):
    """Normalize TOML values to expected runtime types.

    - Convert `sort` string values to their Enum variants.
    - Ensure `paths` and `exclude` are lists when provided as strings.
    """
    if key == "color":
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
    elif key in ("paths", "exclude", "output-format"):
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


def get_argument_value(
    toml_config: TOMLConfig | None,
    arg_name: str,
    arg_value: TOMLTypes | None,
    default_value: TOMLTypes,
) -> TOMLTypes:
    if arg_value is not None:
        return arg_value

    if toml_config is None:
        if arg_name == "paths":
            typer.echo(
                f"You need to define {arg_name} in the CLI call arguments or in complexipy.toml file"
            )
            raise Exit(code=1)
        return default_value

    arg_resolved = toml_config.get(arg_name, default_value)

    if arg_resolved is None:
        typer.echo(
            f"You need to define {arg_name} in the CLI call arguments or in complexipy.toml file"
        )
        raise Exit(code=1)

    return arg_resolved


BOOLEAN_FIELDS: List[Tuple[str, str, bool]] = [
    ("snapshot_create", "snapshot-create", False),
    ("snapshot_ignore", "snapshot-ignore", False),
    ("quiet", "quiet", False),
    ("ignore_complexity", "ignore-complexity", False),
    ("check_script", "check-script", False),
    ("no_ignore", "no-ignore", False),
    ("report_ignored", "report-ignored", False),
    ("output_csv", "output-csv", False),
    ("output_json", "output-json", False),
    ("output_gitlab", "output-gitlab", False),
    ("output_sarif", "output-sarif", False),
]


def get_arguments_value(
    toml_config: TOMLConfig | None,
    cli_args: Dict[str, Any],
) -> Dict[str, Any]:
    result: Dict[str, Any] = {}

    paths = cli_args.get("paths")
    paths = get_argument_value(toml_config, "paths", paths, [])
    result["paths"] = paths

    result["max_complexity_allowed"] = get_argument_value(
        toml_config,
        "max-complexity-allowed",
        cli_args.get("max_complexity_allowed"),
        15,
    )

    for field_name, toml_key, default in BOOLEAN_FIELDS:
        result[field_name] = cast(
            bool,
            get_argument_value(
                toml_config, toml_key, cli_args.get(field_name), default
            ),
        )

    failed = cli_args.get("failed")
    if (
        failed is None
        and toml_config is not None
        and "failed" not in toml_config
    ):
        legacy_details = toml_config.get("details")
        if legacy_details is not None:
            failed = str(legacy_details).lower() == "low"
    result["failed"] = cast(
        bool, get_argument_value(toml_config, "failed", failed, False)
    )

    result["color"] = cast(
        ColorTypes,
        get_argument_value(
            toml_config, "color", cli_args.get("color"), ColorTypes.auto
        ),
    )

    result["sort"] = cast(
        Sort,
        get_argument_value(toml_config, "sort", cli_args.get("sort"), Sort.asc),
    )

    output_format = cli_args.get("output_format")
    if output_format is None:
        if toml_config is None:
            output_format = []
        else:
            output_format = cast(
                List[str],
                load_values_from_toml_key(
                    "output-format",
                    toml_config.get("output-format", []),
                ),
            )
    result["output_format"] = output_format

    output = cli_args.get("output")
    if output is None and toml_config is not None:
        output = cast(Optional[str], toml_config.get("output"))
    result["output"] = output

    result["exclude"] = get_argument_value(
        toml_config, "exclude", cli_args.get("exclude"), []
    )

    return result
