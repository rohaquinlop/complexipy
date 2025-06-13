from .types import (
    DetailTypes,
    Sort,
)
from complexipy._complexipy import (
    FileComplexity,
    FunctionComplexity,
)
from rich.align import (
    Align,
)
from rich.console import (
    Console,
)
from rich.table import Table
from typing import (
    List,  # It's important to use this to make it compatible with python 3.8, don't remove it
    Tuple,
)


def output_summary(
    console: Console,
    files: List[FileComplexity],
    details: DetailTypes,
    sort: Sort,
    ignore_complexity: bool,
) -> bool:
    table, has_success, total_complexity, all_functions = create_table(
        files, details, sort, ignore_complexity
    )

    if details == DetailTypes.low and table.row_count < 1:
        console.print(
            f"No function{'s' if len(files) > 1 else ''} were found with complexity greater or equal to 15."
        )
    else:
        if len(all_functions) == 0:
            console.print(
                Align.center(
                    "No files were found with functions. No complexity was calculated."
                )
            )
        else:
            console.print(Align.center(table))
            console.print(
                f":brain: Total Cognitive Complexity: {total_complexity}"
            )

    return has_success


def create_table(
    files: List[FileComplexity],
    details: DetailTypes,
    sort: Sort,
    ignore_complexity: bool,
) -> Tuple[Table, bool, int, List[Tuple[str, str, FunctionComplexity]]]:
    has_success = True
    all_functions: List[Tuple[str, str, FunctionComplexity]] = []
    total_complexity = 0

    table = Table(
        title="Summary",
        show_header=True,
        header_style="bold magenta",
        show_lines=True,
    )
    table.add_column("Path")
    table.add_column("File")
    table.add_column("Function")
    table.add_column("Complexity")

    for file in files:
        for function in file.functions:
            total_complexity += function.complexity
            all_functions.append((file.path, file.file_name, function))

    if sort != Sort.name:
        all_functions.sort(key=lambda x: x[2].complexity)

        if sort == Sort.desc:
            all_functions.reverse()

    for function in all_functions:
        if function[2].complexity >= 15:
            table.add_row(
                f"{function[0]}",
                f"[green]{function[1]}[/green]",
                f"[green]{function[2].name}[/green]",
                f"[red]{function[2].complexity}[/red]",
            )
            has_success = False
        elif details != DetailTypes.low:
            table.add_row(
                f"{function[0]}",
                f"[green]{function[1]}[/green]",
                f"[green]{function[2].name}[/green]",
                f"[blue]{function[2].complexity}[/blue]",
            )

    if ignore_complexity:
        has_success = True

    return table, has_success, total_complexity, all_functions


def has_success_functions(files: List[FileComplexity]) -> bool:
    for file in files:
        for function in file.functions:
            if function.complexity >= 15:
                return False
    return True
