from typing import List

from complexipy._complexipy import FileComplexity, output_csv


def store_csv(
    output_path: str,
    files_complexities: List[FileComplexity],
    sort: str,
    show_details: bool,
    max_complexity: int,
) -> None:
    output_csv(
        output_path,
        files_complexities,
        sort,
        show_details,
        max_complexity,
    )
