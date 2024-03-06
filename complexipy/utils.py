from .types import (
    DetailTypes,
)
from complexipy.rust import (
    FileComplexity,
)
from rich.table import Table


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
