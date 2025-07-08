from .types import (
    DetailTypes,
    Sort,
)
from .utils import (
    check_os,
    output_summary,
    has_success_functions,
)
from complexipy import (
    _complexipy,
)
from complexipy._complexipy import FileComplexity, CodeComplexity
import os
from pathlib import (
    Path,
)
from rich.console import (
    Console,
)
from typing import (
    List,  # It's important to use this to make it compatible with python 3.8, don't remove it
)
import time
import typer

app = typer.Typer(name="complexipy")
console = Console()
version = "3.1.0"


@app.command()
def main(
    paths: List[str] = typer.Argument(
        help="Paths to the directories or files to analyze, it can be a local paths or a git repository URL.",
    ),
    quiet: bool = typer.Option(
        False, "--quiet", "-q", help="Suppress the output to the console."
    ),
    ignore_complexity: bool = typer.Option(
        False,
        "--ignore-complexity",
        "-i",
        help="Ignore the complexity and show all functions.",
    ),
    details: DetailTypes = typer.Option(
        DetailTypes.normal.value,
        "--details",
        "-d",
        help="Specify how detailed should be output, it can be 'low' or 'normal'. Default is 'normal'.",
    ),
    sort: Sort = typer.Option(
        Sort.asc.value,
        "--sort",
        "-s",
        help="Sort the output by complexity, it can be 'asc', 'desc' or 'name'. Default is 'asc'.",
    ),
    output_csv: bool = typer.Option(
        False, "--output-csv", "-c", help="Output the results to a CSV file."
    ),
    output_json: bool = typer.Option(
        False, "--output-json", "-j", help="Output the results to a JSON file."
    ),
):
    check_os()
    invocation_path = os.getcwd()
    console.rule(f":octopus: complexipy {version}")
    start_time = time.time()
    files_complexities: List[FileComplexity] = _complexipy.main(paths)
    execution_time = time.time() - start_time
    output_csv_path = f"{invocation_path}/complexipy.csv"
    output_json_path = f"{invocation_path}/complexipy.json"

    if output_csv:
        _complexipy.output_csv(
            output_csv_path,
            files_complexities,
            sort.value,
            details.value == DetailTypes.normal.value,
        )
        console.print(f"Results saved at {output_csv_path}")

    if output_json:
        _complexipy.output_json(
            output_json_path,
            files_complexities,
            details.value == DetailTypes.normal.value,
        )
        console.print(f"Results saved at {output_json_path}")

    if quiet:
        has_success = has_success_functions(files_complexities)
    else:
        has_success = output_summary(
            console, files_complexities, details, sort, ignore_complexity
        )

    console.print(
        f"{len(files_complexities)} file{'s' if len(files_complexities) > 1 else ''} analyzed in {execution_time:.4f} seconds"
    )
    console.rule(":tada: Analysis completed! :tada:")

    if not has_success:
        raise typer.Exit(code=1)


def code_complexity(
    code: str,
) -> CodeComplexity:
    return _complexipy.code_complexity(code)


def file_complexity(file_path: str) -> FileComplexity:
    path = Path(file_path)
    base_path = path.parent
    return _complexipy.file_complexity(
        file_path=path.resolve().as_posix(),
        base_path=base_path.resolve().as_posix(),
    )


if __name__ == "__main__":
    app()
