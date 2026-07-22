from __future__ import annotations

from typing import (
    List,
    Optional,
    Tuple,
)

import typer

from complexipy.types import (
    OutputFormat,
    RunConfig,
)
from complexipy.utils.toml import (
    get_argument_value,
    get_arguments_value,
)


def _comma_separated_list(value: str) -> List[str]:
    return [v.strip() for v in value.split(",") if v.strip()]


_comma_separated_list.__name__ = "TEXT[,TEXT...]"


def _flatten_lists(value) -> List[str]:
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
    toml_config,
    paths,
    max_complexity_allowed,
    snapshot_create,
    snapshot_ignore,
    quiet,
    ignore_complexity,
    failed,
    color,
    sort,
    output_format,
    output,
    output_csv,
    output_json,
    output_gitlab,
    output_sarif,
    diff,
    diff_only,
    ratchet,
    top,
    plain,
    suggest_refactors,
    exclude,
    check_script,
    no_ignore,
    report_ignored,
    version,
) -> RunConfig:
    legacy_cli_output_flags = {
        OutputFormat.csv: output_csv,
        OutputFormat.json: output_json,
        OutputFormat.gitlab: output_gitlab,
        OutputFormat.sarif: output_sarif,
    }

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
        legacy_cli_output_flags=legacy_cli_output_flags,
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
