from __future__ import annotations

from typing import List

from complexipy._complexipy import FileComplexity, output_json


def store_json(
    output_path: str,
    files_complexities: List[FileComplexity],
    show_details: bool,
    max_complexity: int,
    suggest_refactors: bool = False,
) -> None:
    output_json(
        output_path,
        files_complexities,
        show_details,
        max_complexity,
        suggest_refactors,
    )
