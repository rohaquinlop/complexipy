from .types import (
    DetailTypes,
)
from complexipy.rust import (
    FileComplexity,
)
from rich.align import (
    Align,
)
from rich.console import (
    Console,
)
from rich.table import Table


def output_summary(
    console: Console,
    file_level: bool,
    files: list[FileComplexity],
    max_complexity: int,
    details: DetailTypes,
    path: str,
    execution_time: float,
) -> bool:
    if file_level:
        table, has_success, total_complexity = create_table_file_level(
            files, max_complexity, details
        )
    else:
        table, has_success, total_complexity = create_table_function_level(
            files, max_complexity, details
        )
    console.print(Align.center(table))
    console.print(f":brain: Total Cognitive Complexity in {path}: {total_complexity}")
    console.print(
        f"{len(files)} file{'s' if len(files)> 1 else ''} analyzed in {execution_time:.4f} seconds"
    )

    return has_success


def create_table_file_level(
    files: list[FileComplexity], max_complexity: int, details: DetailTypes
) -> tuple[Table, bool, int]:
    has_success = True

    table = Table(
        title="Summary", show_header=True, header_style="bold magenta", show_lines=True
    )
    table.add_column("Path")
    table.add_column("File")
    table.add_column("Complexity")
    total_complexity = 0
    for file in files:
        total_complexity += file.complexity
        if file.complexity > max_complexity and max_complexity != 0:
            table.add_row(
                f"{file.path}",
                f"[green]{file.file_name}[/green]",
                f"[red]{file.complexity}[/red]",
            )
            has_success = False
        elif details != DetailTypes.low or max_complexity == 0:
            table.add_row(
                f"{file.path}",
                f"[green]{file.file_name}[/green]",
                f"[blue]{file.complexity}[/blue]",
            )
    return table, has_success, total_complexity


def create_table_function_level(
    files: list[FileComplexity], complexity: int, details: DetailTypes
) -> tuple[Table, bool, int]:
    has_success = True

    table = Table(
        title="Summary", show_header=True, header_style="bold magenta", show_lines=True
    )
    table.add_column("Path")
    table.add_column("File")
    table.add_column("Function")
    table.add_column("Complexity")
    total_complexity = 0
    for file in files:
        total_complexity += file.complexity
        for function in file.functions:
            total_complexity += function.complexity
            if function.complexity > complexity and complexity != 0:
                table.add_row(
                    f"{file.path}",
                    f"[green]{file.file_name}[/green]",
                    f"[green]{function.name}[/green]",
                    f"[red]{function.complexity}[/red]",
                )
                has_success = False
            elif details != DetailTypes.low or complexity == 0:
                table.add_row(
                    f"{file.path}",
                    f"[green]{file.file_name}[/green]",
                    f"[green]{function.name}[/green]",
                    f"[blue]{function.complexity}[/blue]",
                )
    return table, has_success, total_complexity


def has_success_file_level(files: list[FileComplexity], max_complexity: int) -> bool:
    for file in files:
        if file.complexity > max_complexity and max_complexity != 0:
            return False
    return True


def has_success_function_level(files: list[FileComplexity], complexity: int) -> bool:
    for file in files:
        for function in file.functions:
            if function.complexity > complexity and complexity != 0:
                return False
    return True
