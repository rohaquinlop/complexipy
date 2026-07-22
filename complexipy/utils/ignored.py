from __future__ import annotations

import json
import os
from typing import List, Optional

from rich.console import Console

from complexipy import _complexipy
from complexipy.types import OutputFormat
from complexipy.utils.paths import resolve_output_paths


def handle_report_ignored(
    console: Console,
    report_ignored: bool,
    paths: Optional[List[str]],
    exclude: Optional[List[str]],
    output_formats: List[OutputFormat],
    output: Optional[str],
    no_ignore: bool,
    invocation_path: str,
) -> None:
    if not report_ignored:
        return

    paths = paths or []
    exclude = exclude or []
    ignored_locations, _ = _complexipy.collect_all_ignored_locations(
        paths, exclude, invocation_path
    )
    if ignored_locations:
        for loc in ignored_locations:
            console.print(f"{loc.path}:{loc.line}  {loc.comment}")
        console.print(
            f"\nFound {len(ignored_locations)} suppressed location(s)."
        )
        if no_ignore:
            console.print("(all markers ignored due to --no-ignore)")
    else:
        console.print("No ignore comments found.")

    if OutputFormat.json in output_formats and ignored_locations:
        ignored_output_paths = resolve_output_paths(
            [OutputFormat.json], output, invocation_path
        )
        ignored_json_path = os.path.join(
            os.path.dirname(ignored_output_paths[OutputFormat.json]),
            "complexipy-ignored.json",
        )
        ignored_data = [
            {"path": loc.path, "line": loc.line, "comment": loc.comment}
            for loc in ignored_locations
        ]
        with open(ignored_json_path, "w") as f:
            json.dump(ignored_data, f, indent=2)
            f.write("\n")
        console.print(f"Ignored locations saved at {ignored_json_path}")
