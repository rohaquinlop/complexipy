from __future__ import annotations

from typing import (
    List,
    Optional,
    Tuple,
)

import typer

from complexipy.types import (
    ColorTypes,
    RunConfig,
    Sort,
    TOMLConfig,
)
from complexipy.utils.toml import (
    get_argument_value,
    get_arguments_value,
)


def _comma_separated_list(value: str) -> List[str]:
    return [v.strip() for v in value.split(",") if v.strip()]


_comma_separated_list.__name__ = "TEXT[,TEXT...]"


def _flatten_lists(value: Optional[list]) -> List[str]:
    if not value:
        return []
    result = []
    for item in value:
        if isinstance(item, list):
            result.extend(item)
        else:
            result.append(item)
    return result


def resolve_config(
    toml_config: Optional[TOMLConfig],
    paths: Optional[List[str]],
    max_complexity_allowed: Optional[int],
    snapshot_create: Optional[bool],
    snapshot_ignore: Optional[bool],
    quiet: Optional[bool],
    ignore_complexity: Optional[bool],
    failed: Optional[bool],
    color: Optional[ColorTypes],
    sort: Optional[Sort],
    output_format: Optional[List[str]],
    output: Optional[str],
    output_csv: Optional[bool],
    output_json: Optional[bool],
    output_gitlab: Optional[bool],
    output_sarif: Optional[bool],
    diff: Optional[str],
    diff_only: Optional[str],
    ratchet: Optional[bool],
    top: Optional[int],
    plain: Optional[bool],
    suggest_refactors: Optional[bool],
    exclude: Optional[List[str]],
    check_script: Optional[bool],
    no_ignore: Optional[bool],
    report_ignored: Optional[bool],
) -> RunConfig:
    cli_args = {
        "paths": paths,
        "max_complexity_allowed": max_complexity_allowed,
        "snapshot_create": snapshot_create,
        "snapshot_ignore": snapshot_ignore,
        "quiet": quiet,
        "ignore_complexity": ignore_complexity,
        "failed": failed,
        "color": color,
        "sort": sort,
        "output_format": output_format,
        "output": output,
        "output_csv": output_csv,
        "output_json": output_json,
        "output_gitlab": output_gitlab,
        "output_sarif": output_sarif,
        "exclude": exclude,
        "check_script": check_script,
        "no_ignore": no_ignore,
        "report_ignored": report_ignored,
    }

    resolved = get_arguments_value(toml_config, cli_args)

    paths = resolved["paths"]
    max_complexity_allowed = resolved["max_complexity_allowed"]
    snapshot_create = resolved["snapshot_create"]
    snapshot_ignore = resolved["snapshot_ignore"]
    quiet = resolved["quiet"]
    ignore_complexity = resolved["ignore_complexity"]
    failed = resolved["failed"]
    color = resolved["color"]
    sort = resolved["sort"]
    output_format = resolved["output_format"]
    output = resolved["output"]
    exclude = resolved["exclude"]
    check_script = resolved["check_script"]
    no_ignore = resolved["no_ignore"]
    report_ignored = resolved["report_ignored"]

    exclude = _flatten_lists(exclude)
    output_format = _flatten_lists(output_format)

    no_ignore = bool(no_ignore)
    report_ignored = bool(report_ignored)
    ratchet = bool(get_argument_value(toml_config, "ratchet", ratchet, False))

    plain, suggest_refactors = validate_cli_arguments(
        plain, suggest_refactors, top, quiet
    )

    return RunConfig(
        paths=paths,
        max_complexity_allowed=max_complexity_allowed,
        snapshot_create=snapshot_create,
        snapshot_ignore=snapshot_ignore,
        quiet=quiet,
        ignore_complexity=ignore_complexity,
        failed=failed,
        color=color,
        sort=sort,
        output_format=output_format,
        output=output,
        exclude=exclude,
        check_script=check_script,
        no_ignore=no_ignore,
        report_ignored=report_ignored,
        ratchet=ratchet,
        plain=plain,
        suggest_refactors=suggest_refactors,
        top=top,
        diff=diff,
        diff_only=diff_only,
    )


def validate_cli_arguments(
    plain: Optional[bool],
    suggest_refactors: Optional[bool],
    top: Optional[int],
    quiet: bool,
) -> Tuple[bool, bool]:
    if plain is None:
        plain = False
    if suggest_refactors is None:
        suggest_refactors = False

    if plain and quiet:
        raise typer.BadParameter("--plain and --quiet cannot be used together.")

    if top is not None and top < 1:
        raise typer.BadParameter("--top must be a positive integer.")

    return plain, suggest_refactors
