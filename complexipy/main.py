from .types import (
    DetailTypes,
    Sort,
)
from .utils import (
    output_summary,
    has_success_functions,
    load_toml_config,
    get_arguments_value,
)
from complexipy import (
    _complexipy,
)
from complexipy._complexipy import FileComplexity
import os
import platform
from rich.console import (
    Console,
)
from typing import (
    List,  # It's important to use this to make it compatible with python 3.8, don't remove it
    Optional,
)
import time
import typer

app = typer.Typer(name="complexipy")
console = Console()
version = "3.3.0"
INVOCATION_PATH = os.getcwd()
TOML_CONFIG = load_toml_config(INVOCATION_PATH)


@app.command()
def main(
    paths: Optional[List[str]] = typer.Option(
        None,
        help="Paths to the directories or files to analyze, it can be a local paths or a git repository URL.",
    ),
    max_complexity_allowed: Optional[int] = typer.Option(
        None,
        "--max-complexity-allowed",
        "-mx",
        help="Max complexity allowed per function.",
    ),
    quiet: Optional[bool] = typer.Option(
        None,
        "--quiet",
        "-q",
        help="Suppress the output to the console.",
    ),
    ignore_complexity: Optional[bool] = typer.Option(
        None,
        "--ignore-complexity",
        "-i",
        help="Ignore the complexity and show all functions.",
    ),
    details: Optional[DetailTypes] = typer.Option(
        None,
        "--details",
        "-d",
        help="Specify how detailed should be output, it can be 'low' or 'normal'. Default is 'normal'.",
    ),
    sort: Optional[Sort] = typer.Option(
        None,
        "--sort",
        "-s",
        help="Sort the output by complexity, it can be 'asc', 'desc' or 'name'. Default is 'asc'.",
    ),
    output_csv: Optional[bool] = typer.Option(
        None,
        "--output-csv",
        "-c",
        help="Output the results to a CSV file.",
    ),
    output_json: Optional[bool] = typer.Option(
        None,
        "--output-json",
        "-j",
        help="Output the results to a JSON file.",
    ),
):
    (
        paths,
        max_complexity_allowed,
        quiet,
        ignore_complexity,
        details,
        sort,
        output_csv,
        output_json,
    ) = get_arguments_value(
        TOML_CONFIG,
        paths,
        max_complexity_allowed,
        quiet,
        ignore_complexity,
        details,
        sort,
        output_csv,
        output_json,
    )

    if platform.system() == "Windows":
        console.rule(f"complexipy {version}")
    else:
        console.rule(f":octopus: complexipy {version}")
    start_time = time.time()
    files_complexities: List[FileComplexity] = _complexipy.main(paths, quiet)
    execution_time = time.time() - start_time
    output_csv_path = f"{INVOCATION_PATH}/complexipy.csv"
    output_json_path = f"{INVOCATION_PATH}/complexipy.json"

    if output_csv:
        _complexipy.output_csv(
            output_csv_path,
            files_complexities,
            sort.value,
            details.value == DetailTypes.normal.value,
            max_complexity_allowed,
        )
        console.print(f"Results saved at {output_csv_path}")

    if output_json:
        _complexipy.output_json(
            output_json_path,
            files_complexities,
            details.value == DetailTypes.normal.value,
            max_complexity_allowed,
        )
        console.print(f"Results saved at {output_json_path}")

    if quiet:
        has_success = has_success_functions(files_complexities, max_complexity_allowed)
    else:
        has_success = output_summary(
            console,
            files_complexities,
            details,
            sort,
            ignore_complexity,
            max_complexity_allowed,
        )
        console.print(
            f"{len(files_complexities)} file{'s' if len(files_complexities) > 1 else ''} analyzed in {execution_time:.4f} seconds"
        )
        if platform.system() == "Windows":
            console.rule("Analysis completed!")
        else:
            console.rule(":tada: Analysis completed! :tada:")

    if not has_success:
        raise typer.Exit(code=1)


if __name__ == "__main__":
    app()
