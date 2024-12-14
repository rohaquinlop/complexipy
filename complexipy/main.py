from .types import (
    DetailTypes,
    Sort,
)
from .utils import (
    output_summary,
    has_success_functions,
)
from complexipy import (
    rust,
)
from complexipy.rust import FileComplexity, CodeComplexity
import os
from pathlib import (
    Path,
)
from rich.console import (
    Console,
)
import time
import typer

root_dir = Path(__file__).resolve().parent.parent
app = typer.Typer(name="complexipy")
console = Console()
version = "1.1.0"


@app.command()
def main(
    paths: list[str] = typer.Argument(
        help="Paths to the directories or files to analyze, it can be a local paths or a git repository URL.",
    ),
    max_complexity: int = typer.Option(
        15,
        "--max-complexity",
        "-c",
        help="The maximum complexity allowed per file, set this value as 0 to set it as unlimited. Default is 15.",
    ),
    output: bool = typer.Option(
        False, "--output", "-o", help="Output the results to a CSV file."
    ),
    details: DetailTypes = typer.Option(
        DetailTypes.normal.value,
        "--details",
        "-d",
        help="Specify how detailed should be output, it can be 'low' or 'normal'. Default is 'normal'.",
    ),
    quiet: bool = typer.Option(
        False, "--quiet", "-q", help="Suppress the output to the console."
    ),
    sort: Sort = typer.Option(
        Sort.asc.value,
        "--sort",
        "-s",
        help="Sort the output by complexity, it can be 'asc', 'desc' or 'name'. Default is 'asc'.",
    ),
):
    invocation_path = os.getcwd()
    console.rule(f":octopus: complexipy {version}")
    start_time = time.time()
    files_complexities: list[FileComplexity] = rust.main(paths)
    execution_time = time.time() - start_time
    output_csv_path = f"{invocation_path}/complexipy.csv"

    if output:
        rust.output_csv(output_csv_path, files_complexities, sort.value)
        console.print(f"Results saved at {output_csv_path}")

    if not quiet:
        has_success = output_summary(
            console, max_complexity, details, paths, sort
        )
    if quiet:
        has_success = has_success_functions(files_complexities, max_complexity)

    console.print(
        f"{len(files_complexities)} file{'s' if len(files_complexities)> 1 else ''} analyzed in {execution_time:.4f} seconds"
    )
    console.rule(":tada: Analysis completed! :tada:")

    if not has_success:
        raise typer.Exit(code=1)


def code_complexity(
    code: str,
) -> CodeComplexity:
    return rust.code_complexity(code)


def file_complexity(file_path: str) -> FileComplexity:
    path = Path(file_path)
    base_path = path.parent
    return rust.file_complexity(
        file_path=path.resolve().as_posix(),
        base_path=base_path.resolve().as_posix(),
    )


if __name__ == "__main__":
    app()
