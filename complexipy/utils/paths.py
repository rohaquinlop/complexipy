from __future__ import annotations

import os
from typing import Dict, List, Optional

import typer

from complexipy.types import OutputFormat
from complexipy.utils.constants import DEFAULT_OUTPUT_FILENAMES


def resolve_output_paths(
    output_formats: List[OutputFormat],
    output: Optional[str],
    invocation_path: str,
) -> Dict[OutputFormat, str]:
    if not output_formats:
        return {}

    if output is None:
        return build_output_paths(invocation_path, output_formats)

    if output == "-":
        raise typer.BadParameter(
            "Writing machine-readable output to stdout is not supported."
        )

    destination = os.path.abspath(output)
    is_directory_hint = is_directory_output_hint(output)

    if len(output_formats) > 1:
        ensure_directory_destination(destination, is_directory_hint)
        return build_output_paths(destination, output_formats)

    output_format = output_formats[0]
    if os.path.isdir(destination) or is_directory_hint:
        os.makedirs(destination, exist_ok=True)
        return build_output_paths(destination, [output_format])

    parent_dir = os.path.dirname(destination)
    if parent_dir:
        os.makedirs(parent_dir, exist_ok=True)

    return {output_format: destination}


def build_output_paths(
    destination: str, output_formats: List[OutputFormat]
) -> Dict[OutputFormat, str]:
    return {
        output_format: os.path.join(
            destination,
            DEFAULT_OUTPUT_FILENAMES[output_format],
        )
        for output_format in output_formats
    }


def is_directory_output_hint(output: str) -> bool:
    return output.endswith(os.sep) or (
        os.altsep is not None and output.endswith(os.altsep)
    )


def ensure_directory_destination(
    destination: str, is_directory_hint: bool
) -> None:
    if os.path.exists(destination) and not os.path.isdir(destination):
        raise typer.BadParameter(
            "When multiple output formats are selected, --output "
            "must point to a directory."
        )
    elif not is_directory_hint:
        raise typer.BadParameter(
            "When multiple output formats are selected, --output must "
            "point to a directory or end with a path separator."
        )

    os.makedirs(destination, exist_ok=True)
