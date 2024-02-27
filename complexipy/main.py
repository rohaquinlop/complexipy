from pathlib import Path
from complexipy import rust
import csv
import os
import re
from rich.console import Console
from rich.table import Table
import time
import typer

root_dir = Path(__file__).resolve().parent.parent
app = typer.Typer(name="complexipy")
console = Console()
version = "0.2.1"


@app.command()
def main(
    path: str = typer.Argument(
        help="Path to the directory or file to analyze, it can be a local path or a git repository URL.",
    ),
    max_complexity: int = typer.Option(
        15,
        "--max-complexity",
        "-c",
        help="The maximum complexity allowed per file, set this value as 0 to set it as unlimited.",
    ),
    output: bool = typer.Option(
        False, "--output", "-o", help="Output the results to a CSV file."
    ),
):
    has_success = True
    is_dir = Path(path).is_dir()
    _url_pattern = (
        r"^(https:\/\/|http:\/\/|www\.|git@)(github|gitlab)\.com(\/[\w.-]+){2,}$"
    )
    is_url = bool(re.match(_url_pattern, path))
    invocation_path = os.getcwd()

    console.rule(f"complexipy {version} :octopus:")
    with console.status("Analyzing the complexity of the code...", spinner="dots"):
        start_time = time.time()
        files = rust.main(path, is_dir, is_url, max_complexity)
    execution_time = time.time() - start_time
    console.rule(":tada: Analysis completed!:tada:")

    if output:
        with open(f"{invocation_path}/complexipy.csv", "w", newline="") as file:
            writer = csv.writer(file)
            writer.writerow(["Path", "File Name", "Cognitive Complexity"])
            for file in files:
                writer.writerow([file.path, file.file_name, file.complexity])
        console.print(f"Results saved to {invocation_path}/complexipy.csv")

    # Summary
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
        else:
            table.add_row(
                f"{file.path}",
                f"[green]{file.file_name}[/green]",
                f"[blue]{file.complexity}[/blue]",
            )
    console.print(table)
    console.print(f":brain: Total Cognitive Complexity in {path}: {total_complexity}")
    console.print(f"{len(files)} files analyzed in {execution_time:.4f} seconds")

    if not has_success:
        raise typer.Exit(code=1)


if __name__ == "__main__":
    app()
