import os
import platform
import time
from datetime import datetime
from importlib.metadata import (
    PackageNotFoundError,
)
from importlib.metadata import (
    version as pkg_version,
)
from typing import (
    List,  # It's important to use this to make it compatible with python 3.8, don't remove it
    Optional,
    Tuple,
)

import typer
from rich.console import (
    Console,
)

from complexipy import (
    _complexipy,
)
from complexipy._complexipy import FileComplexity

from .types import (
    ColorTypes,
    DetailTypes,
    Sort,
)
from .utils.csv import store_csv
from .utils.json import store_json
from .utils.output import (
    has_success_functions,
    output_summary,
    print_invalid_paths,
)
from .utils.snapshot import (
    handle_snapshot_file_creation,
    handle_snapshot_functions_load,
    handle_snapshot_watermark,
)
from .utils.toml import (
    get_arguments_value,
    get_complexipy_toml_config,
)

app = typer.Typer(name="complexipy")
console = Console(color_system="auto")
INVOCATION_PATH = os.getcwd()
TOML_CONFIG = get_complexipy_toml_config(INVOCATION_PATH)


def _version_callback(value: bool):
    if value:
        try:
            console.print(pkg_version("complexipy"))
        except PackageNotFoundError:
            console.print("Unknown version")
        raise typer.Exit()


@app.command()
def main(
    paths: Optional[List[str]] = typer.Argument(
        None,
        help="Paths to the directories or files to analyze, it can be a local paths or a git repository URL.",
    ),
    exclude: Optional[List[str]] = typer.Option(
        None,
        "--exclude",
        "-e",
        help="Paths to the directories or files to exclude.",
    ),
    max_complexity_allowed: Optional[int] = typer.Option(
        None,
        "--max-complexity-allowed",
        "-mx",
        help="Max complexity allowed per function.",
    ),
    snapshot_create: Optional[bool] = typer.Option(
        None,
        "--snapshot-create",
        "-spc",
        help="Creates a snapshot of the current project state.",
    ),
    snapshot_ignore: Optional[bool] = typer.Option(
        None,
        "--snapshot-ignore",
        "-spi",
        help="Skip comparing against the existing snapshot file.",
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
    color: Optional[ColorTypes] = typer.Option(
        None,
        "--color",
        "-C",
        help="Whether the output should be in color: either 'auto', 'yes' or 'no'. Default is 'auto'.",
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
    version: bool = typer.Option(  # type: ignore[assignment]
        False,
        "--version",
        help="Show the complexipy version and exit.",
        callback=_version_callback,
        is_eager=True,
    ),
):
    global console

    (
        paths,
        max_complexity_allowed,
        snapshot_create,
        snapshot_ignore,
        quiet,
        ignore_complexity,
        details,
        color,
        sort,
        output_csv,
        output_json,
        exclude,
    ) = get_arguments_value(
        TOML_CONFIG,
        paths,
        max_complexity_allowed,
        snapshot_create,
        snapshot_ignore,
        quiet,
        ignore_complexity,
        details,
        color,
        sort,
        output_csv,
        output_json,
        exclude,
    )
    if color == ColorTypes.no:
        console = Console(color_system=None)
    elif color == ColorTypes.yes:
        console = Console(color_system="standard")

    if not quiet:
        if platform.system() == "Windows":
            console.rule("complexipy")
        else:
            console.rule(":octopus: complexipy")
    start_time = time.time()
    result: Tuple[List[FileComplexity], List[str]] = _complexipy.main(
        paths, quiet, exclude
    )
    files_complexities, failed_paths = result
    execution_time = time.time() - start_time
    current_time = datetime.today().strftime("%Y_%m_%d__%H:%M:%S")
    output_csv_path = f"{INVOCATION_PATH}/complexipy_results_{current_time}.csv"
    output_json_path = (
        f"{INVOCATION_PATH}/complexipy_results_{current_time}.json"
    )
    output_snapshot_path = f"{INVOCATION_PATH}/complexipy-snapshot.json"

    handle_snapshot_file_creation(
        snapshot_create,
        output_snapshot_path,
        max_complexity_allowed,
        files_complexities,
    )

    snapshot_file_exists = os.path.exists(output_snapshot_path)
    snapshot_files = handle_snapshot_functions_load(output_snapshot_path)
    should_run_snapshot_watermark = snapshot_file_exists and not snapshot_ignore
    watermark_success, watermark_messages = handle_snapshot_watermark(
        should_run_snapshot_watermark,
        snapshot_file_exists,
        output_snapshot_path,
        files_complexities,
        snapshot_files,
        max_complexity_allowed,
    )

    handle_results_storage(
        output_csv,
        output_csv_path,
        output_json,
        output_json_path,
        files_complexities,
        sort.value,
        details.value == DetailTypes.normal.value,
        max_complexity_allowed,
    )

    if quiet:
        has_success = has_success_functions(
            files_complexities, max_complexity_allowed
        )
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

    has_success = (
        handle_snapshot(
            should_run_snapshot_watermark,
            quiet,
            watermark_messages,
            output_snapshot_path,
            watermark_success,
        )
        and has_success
    )

    has_success = (
        print_invalid_paths(console, quiet, failed_paths) and has_success
    )
    if not has_success:
        raise typer.Exit(code=1)


def handle_results_storage(
    output_csv: bool,
    output_csv_path: str,
    output_json: bool,
    output_json_path: str,
    files_complexities: List[FileComplexity],
    sort: str,
    show_details: bool,
    max_complexity: int,
) -> None:
    global console

    if output_csv:
        store_csv(
            output_csv_path,
            files_complexities,
            sort,
            show_details,
            max_complexity,
        )
        console.print(f"Results saved at {output_csv_path}")

    if output_json:
        store_json(
            output_json_path,
            files_complexities,
            show_details,
            max_complexity,
        )
        console.print(f"Results saved at {output_json_path}")


def handle_snapshot(
    should_run_snapshot_watermark: bool,
    quiet: bool,
    watermark_messages: List[str],
    output_snapshot_path: str,
    watermark_success: bool,
) -> bool:
    global console

    if should_run_snapshot_watermark:
        if not quiet:
            if watermark_messages:
                for message in watermark_messages:
                    console.print(
                        f"[bold red]Snapshot watermark[/bold red]: {message}"
                    )
            else:
                console.print(
                    f"Snapshot watermark passed. Baseline stored at {output_snapshot_path}"
                )
        return watermark_success
    return False


if __name__ == "__main__":
    app()
