from __future__ import annotations

import os
from typing import (
    List,  # It's important to use this to make it compatible with python 3.8, don't remove it
    Literal,
    Tuple,
    overload,
)

from typer import Exit

from complexipy.types import (
    ColorTypes,
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
    value: int | bool | str | List[str] | Sort | ColorTypes | TOMLType,
) -> int | bool | List[str] | ColorTypes | Sort | TOMLType: ...


def load_values_from_toml_key(
    key: str,
    value: int | bool | str | List[str] | Sort | ColorTypes | TOMLType,
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
    snapshot_create: bool | None,
    snapshot_ignore: bool | None,
    quiet: bool | None,
    ignore_complexity: bool | None,
    failed: bool | None,
    color: ColorTypes | None,
    sort_arg: Sort | None,
    output_csv: bool | None,
    output_json: bool | None,
    output_sarif: bool | None,
    exclude: List[str] | None,
) -> Tuple[
    List[str],
    int,
    bool,
    bool,
    bool,
    bool,
    bool,
    ColorTypes,
    Sort,
    bool,
    bool,
    bool,
    List[str],
]:
    paths = get_argument_value(toml_config, "paths", paths, [])
    max_complexity_allowed = get_argument_value(
        toml_config, "max-complexity-allowed", max_complexity_allowed, 15
    )
    snapshot_create = get_argument_value(
        toml_config, "snapshot-create", snapshot_create, False
    )
    snapshot_ignore = get_argument_value(
        toml_config, "snapshot-ignore", snapshot_ignore, False
    )
    quiet = get_argument_value(toml_config, "quiet", quiet, False)
    ignore_complexity = get_argument_value(
        toml_config, "ignore-complexity", ignore_complexity, False
    )
    if (
        failed is None
        and toml_config is not None
        and "failed" not in toml_config
    ):
        legacy_details = toml_config.get("details")
        if legacy_details is not None:
            failed = str(legacy_details).lower() == "low"
    failed = get_argument_value(toml_config, "failed", failed, False)
    color = get_argument_value(toml_config, "color", color, ColorTypes.auto)
    sort_arg = get_argument_value(toml_config, "sort", sort_arg, Sort.asc)
    output_csv = get_argument_value(
        toml_config, "output-csv", output_csv, False
    )
    output_json = get_argument_value(
        toml_config, "output-json", output_json, False
    )
    output_sarif = get_argument_value(
        toml_config, "output-sarif", output_sarif, False
    )
    exclude = get_argument_value(toml_config, "exclude", exclude, [])

    return (
        paths,
        max_complexity_allowed,
        snapshot_create,
        snapshot_ignore,
        quiet,
        ignore_complexity,
        failed,
        color,
        sort_arg,
        output_csv,
        output_json,
        output_sarif,
        exclude,
    )
