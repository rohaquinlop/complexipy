from .types import (
    DetailTypes,
    Sort,
)
from .utils import (
    output_summary,
    has_success_functions,
)
from complexipy import (
    _complexipy,
)
from complexipy._complexipy import FileComplexity, CodeComplexity
import os
import platform
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
version = "3.2.0"


@app.command()
def main(
    paths: List[str] = typer.Argument(
        help="Paths to the directories or files to analyze, it can be a local paths or a git repository URL.",
    ),
    max_complexity: int = typer.Option(
        15,
        "--max-complexity-allowed",
        "-mx",
        help="Max complexity allowed per function.",
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
    invocation_path = os.getcwd()
    if platform.system() == "Windows":
        console.rule(f"complexipy {version}")
    else:
        console.rule(f":octopus: complexipy {version}")
    start_time = time.time()
    files_complexities: List[FileComplexity] = _complexipy.main(paths, quiet)
    execution_time = time.time() - start_time
    output_csv_path = f"{invocation_path}/complexipy.csv"
    output_json_path = f"{invocation_path}/complexipy.json"

    if output_csv:
        _complexipy.output_csv(
            output_csv_path,
            files_complexities,
            sort.value,
            details.value == DetailTypes.normal.value,
            max_complexity,
        )
        console.print(f"Results saved at {output_csv_path}")

    if output_json:
        _complexipy.output_json(
            output_json_path,
            files_complexities,
            details.value == DetailTypes.normal.value,
            max_complexity,
        )
        console.print(f"Results saved at {output_json_path}")

    if quiet:
        has_success = has_success_functions(files_complexities, max_complexity)
    else:
        has_success = output_summary(
            console,
            files_complexities,
            details,
            sort,
            ignore_complexity,
            max_complexity,
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


def code_complexity(
    code: str,
) -> CodeComplexity:
    """
    Analyze cognitive complexity of Python code provided as a string.

    This function parses and analyzes Python source code from a string,
    identifying all function definitions and calculating their cognitive
    complexity scores. Perfect for analyzing code snippets, templates,
    or dynamically generated code.

    Args:
        code: A string containing valid Python source code. Must be
              syntactically correct Python code.

    Returns:
        CodeComplexity object containing the analysis results, including
        individual function complexity scores and total complexity.

    Raises:
        SyntaxError: If the provided code string contains invalid Python syntax.

    Example:
        >>> code = '''
        ... def fibonacci(n):
        ...     if n <= 1:
        ...         return n
        ...     else:
        ...         return fibonacci(n-1) + fibonacci(n-2)
        ... '''
        >>> result = code_complexity(code)
        >>> print(f"Total complexity: {result.complexity}")
    """
    return _complexipy.code_complexity(code)


def file_complexity(file_path: str) -> FileComplexity:
    """
    Analyze cognitive complexity of a single Python source file.

    This function reads and analyzes a Python file from the filesystem,
    identifying all function definitions and calculating their cognitive
    complexity scores. Useful for analyzing individual files or integrating
    complexity analysis into custom tools.

    Args:
        file_path: Path to the Python file to analyze. Can be relative or
                   absolute. The file must exist and be readable.

    Returns:
        FileComplexity object containing complete analysis results for the
        file, including all functions found and their complexity scores.

    Raises:
        FileNotFoundError: If the specified file does not exist.
        PermissionError: If the file cannot be read due to permissions.
        SyntaxError: If the Python file contains syntax errors.

    Example:
        >>> result = file_complexity('mymodule.py')
        >>> print(f"File: {result.file_name}")
        >>> print(f"Total complexity: {result.complexity}")
        >>> for func in result.functions:
        ...     if func.complexity > 10:
        ...         print(f"Complex function: {func.name} ({func.complexity})")
    """
    path = Path(file_path)
    base_path = path.parent
    return _complexipy.file_complexity(
        file_path=path.resolve().as_posix(),
        base_path=base_path.resolve().as_posix(),
    )


if __name__ == "__main__":
    app()
