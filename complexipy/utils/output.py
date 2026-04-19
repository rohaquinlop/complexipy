from __future__ import annotations

from typing import (
    Dict,
    List,  # It's important to use this to make it compatible with python 3.8, don't remove it
    Optional,
    Tuple,
)

from rich.align import Align
from rich.console import Console
from rich.text import Text

from complexipy._complexipy import (
    FileComplexity,
    FunctionComplexity,
)
from complexipy.types import (
    Sort,
)
from complexipy.utils import dataclasses as dc


def output_summary(
    console: Console,
    files: List[FileComplexity],
    failed_only: bool,
    sort: Sort,
    ignore_complexity: bool,
    max_complexity: int,
    previous_functions: Optional[Dict[Tuple[str, str, str], int]],
    snapshot_map: Optional[Dict[Tuple[str, str, str], int]] = None,
    plain: bool = False,
    top: Optional[int] = None,
) -> bool:
    (
        file_entries,
        failing_functions,
        total_functions,
    ) = build_output_rows(
        files, failed_only, sort, max_complexity, snapshot_map
    )
    has_success = not failing_functions or ignore_complexity

    if top is not None:
        file_entries = truncate_top_n(file_entries, top)

    if plain:
        output_plain(console, file_entries)
        return has_success

    if failed_only and not file_entries:
        console.print(
            f"No function{'s' if len(files) > 1 else ''} were found with complexity greater than {max_complexity}."
        )
    elif total_functions == 0:
        console.print(
            Align.center(
                "No files were found with functions. No complexity was calculated."
            )
        )
    else:
        output_file_entries(
            console,
            file_entries,
            failing_functions,
            ignore_complexity,
            previous_functions,
            max_complexity,
        )
    return has_success


def output_plain(
    console: Console,
    file_entries: List[dc.FileEntry],
) -> None:
    for entry in file_entries:
        for function in entry.functions:
            path = normalize_path(function.path, function.file_name)
            console.print(f"{path} {function.name} {function.complexity}")


def truncate_top_n(
    file_entries: List[dc.FileEntry],
    n: int,
) -> List[dc.FileEntry]:
    all_functions: List[Tuple[str, dc.FunctionRow]] = []
    for entry in file_entries:
        for function in entry.functions:
            all_functions.append((entry.path, function))

    all_functions.sort(key=lambda x: x[1].complexity, reverse=True)
    top_functions = all_functions[:n]

    # Preserve global descending order across files: emit a new entry whenever
    # the path changes, rather than regrouping (which would collapse runs and
    # lose the global rank order for multi-file results).
    result: List[dc.FileEntry] = []
    for path, function in top_functions:
        if result and result[-1].path == path:
            result[-1].functions.append(function)
        else:
            result.append(dc.FileEntry(path=path, functions=[function]))
    return result


def output_file_entries(
    console: Console,
    file_entries: List[dc.FileEntry],
    failing_functions: Dict[str, List[str]],
    ignore_complexity: bool,
    previous_functions: Optional[Dict[Tuple[str, str, str], int]],
    max_complexity: int,
) -> None:
    for entry in file_entries:
        console.print(f"[bold]{entry.path}[/bold]")
        for function in entry.functions:
            status_text = format_status_text(function.passed)
            delta_text = output_delta_text(
                previous_functions, function, max_complexity
            )
            console.print(
                f"    {function.name} {function.complexity}{delta_text} {status_text}"
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


def format_status_text(passed: bool) -> str:
    if passed:
        return "[bold black on green] :white_heavy_check_mark: PASSED [/bold black on green]"
    return "[bold white on red] :cross_mark: FAILED [/bold white on red]"


def output_delta_text(
    previous_functions: Optional[Dict[Tuple[str, str, str], int]],
    function: dc.FunctionRow,
    max_complexity: int,
) -> str:
    if previous_functions is None:
        return ""

    if function.complexity <= max_complexity:
        return ""

    key = (function.path, function.file_name, function.name)
    previous = previous_functions.get(key)
    if previous is None:
        return f" (new, \u0394 = +{function.complexity})"
    if previous != function.complexity:
        delta = function.complexity - previous
        return f" (last: {previous}, \u0394 = {delta:+d})"
    return ""


def _is_function_passing(
    function: FunctionComplexity,
    file_path: str,
    file_name: str,
    max_complexity: int,
    snapshot_map: Optional[Dict[Tuple[str, str, str], int]],
) -> bool:
    if function.complexity <= max_complexity:
        return True
    if snapshot_map is None:
        return False
    prev = snapshot_map.get((file_path, file_name, function.name))
    return prev is not None and function.complexity <= prev


def build_output_rows(
    files: List[FileComplexity],
    failed_only: bool,
    sort: Sort,
    max_complexity: int,
    snapshot_map: Optional[Dict[Tuple[str, str, str], int]] = None,
) -> Tuple[List[dc.FileEntry], Dict[str, List[str]], int]:
    file_entries: List[dc.FileEntry] = []
    failing_functions: Dict[str, List[str]] = {}
    total_functions = 0

    for file in files:
        sorted_functions = sort_functions(file.functions, sort)
        displayable_functions: List[dc.FunctionRow] = []

        for function in sorted_functions:
            total_functions += 1
            passed = _is_function_passing(
                function,
                file.path,
                file.file_name,
                max_complexity,
                snapshot_map,
            )

            if failed_only and passed:
                continue

            if not passed:
                full_path = normalize_path(file.path, file.file_name)
                failing_functions.setdefault(full_path, []).append(
                    function.name
                )

            displayable_functions.append(
                dc.FunctionRow(
                    name=function.name,
                    complexity=function.complexity,
                    passed=passed,
                    path=file.path,
                    file_name=file.file_name,
                )
            )

        if displayable_functions:
            file_entries.append(
                dc.FileEntry(
                    path=normalize_path(file.path, file.file_name),
                    functions=displayable_functions,
                )
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
        text.append(" - Please check file/folder exists or check syntax")
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
