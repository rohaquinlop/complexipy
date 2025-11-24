from __future__ import annotations

import platform
from typing import (
    List,  # It's important to use this to make it compatible with python 3.8, don't remove it
)

from rich.align import Align
from rich.console import Console
from rich.text import Text

from complexipy._complexipy import (
    FileComplexity,
    FunctionComplexity,
)
from complexipy.types import (
    DetailTypes,
    Sort,
)


def get_brain_icon():
    """Get platform-appropriate brain icon."""
    if platform.system() == "Windows":
        return "Brain"
    else:
        return ":brain:"


def output_summary(
    console: Console,
    files: List[FileComplexity],
    details: DetailTypes,
    sort: Sort,
    ignore_complexity: bool,
    max_complexity: int,
) -> bool:
    (
        file_entries,
        failing_functions,
        total_functions,
    ) = build_output_rows(files, details, sort, max_complexity)
    has_success = not failing_functions or ignore_complexity

    if details == DetailTypes.low and not file_entries:
        console.print(
            f"No function{'s' if len(files) > 1 else ''} were found with complexity greater than {max_complexity}."
        )
        return has_success

    if total_functions == 0:
        console.print(
            Align.center(
                "No files were found with functions. No complexity was calculated."
            )
        )
        return has_success

    output_file_entries(
        console, file_entries, failing_functions, ignore_complexity
    )

    return has_success


def output_file_entries(
    console: Console,
    file_entries: List[dict[str, str | List[dict[str, str | int | bool]]]],
    failing_functions: dict[str, List[str]],
    ignore_complexity: bool,
) -> None:
    for entry in file_entries:
        console.print(f"[bold]{entry['path']}[/bold]")
        for function in entry["functions"]:
            if isinstance(function, str):
                continue

            status_text = (
                "[green]PASSED[/green]"
                if function["passed"]
                else "[red]FAILED[/red]"
            )
            console.print(
                f"    {function['name']} {function['complexity']} {status_text}"
            )
        console.print()

    if failing_functions:
        console.print("[bold red]Failed functions:[/bold red]")
        for path, functions in failing_functions.items():
            console.print(f" - {path}: {', '.join(sorted(functions))}")
        if ignore_complexity:
            console.print(
                "[yellow]--ignore-complexity enabled: failures will not affect the exit code.[/yellow]"
            )
    else:
        console.print(
            "[bold green]All functions are within the allowed complexity.[/bold green]"
        )


def build_output_rows(
    files: List[FileComplexity],
    details: DetailTypes,
    sort: Sort,
    max_complexity: int,
) -> tuple[
    List[dict[str, str | List[dict[str, str | int | bool]]]],
    dict[str, List[str]],
    int,
]:
    file_entries: List[dict[str, str | List[dict[str, str | int | bool]]]] = []
    failing_functions: dict[str, List[str]] = {}
    total_functions = 0

    for file in files:
        sorted_functions = sort_functions(file.functions, sort)
        displayable_functions: List[dict[str, str | int | bool]] = []

        for function in sorted_functions:
            total_functions += 1
            passed = function.complexity <= max_complexity

            if details == DetailTypes.low and passed:
                continue

            if not passed:
                full_path = normalize_path(file.path, file.file_name)
                failing_functions.setdefault(full_path, []).append(
                    function.name
                )

            displayable_functions.append(
                {
                    "name": function.name,
                    "complexity": function.complexity,
                    "passed": passed,
                }
            )

        if displayable_functions:
            file_entries.append(
                {
                    "path": normalize_path(file.path, file.file_name),
                    "functions": displayable_functions,
                }
            )

    return file_entries, failing_functions, total_functions


def sort_functions(
    functions: List[FunctionComplexity], sort: Sort
) -> List[FunctionComplexity]:
    if sort == Sort.file_name:
        return sorted(functions, key=lambda f: f.name.lower())
    reverse = sort == Sort.desc
    return sorted(functions, key=lambda f: f.complexity, reverse=reverse)


def normalize_path(path: str, file_name: str) -> str:
    cleaned_path = path.rstrip("/")
    if cleaned_path.endswith(file_name):
        return cleaned_path
    if cleaned_path:
        return f"{cleaned_path}/{file_name}"
    return file_name


def print_invalid_paths(
    console: Console, quiet: bool, invalid_paths: List[str]
):
    has_success = True

    if invalid_paths:
        has_success = False

    if quiet:
        return has_success

    for failed_path in invalid_paths:
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
