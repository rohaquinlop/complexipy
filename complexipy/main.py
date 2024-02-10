from pathlib import Path
from complexipy import rust
from rich.console import Console
from rich.table import Table
import toml
import typer

root_dir = Path(__file__).resolve().parent.parent
app = typer.Typer(name="complexipy")
toml_file = toml.load(root_dir / "Cargo.toml")
console = Console()
version = toml_file["package"]["version"]

@app.command()
def main(
    path: str,
    max_complexity: int = typer.Option(15, "--max-complexity", "-c", help="The maximum complexity allowed, set this value as 0 to set it as unlimited."),
    is_dir: bool = typer.Option(False, "--is-dir", "-d", help="Flag that indicates if the path is a directory. If this flag is set, the path will be evaluated as a directory. Otherwise, it will be evaluated as a file."),
):
    has_success = True
    console.rule(f"complexipy {version} :octopus:")
    with console.status("Analyzing the complexity of the code...", spinner="dots"):
        files = rust.main(path, is_dir, max_complexity)
    console.print("Analysis completed! :tada:")
    # Summary
    table = Table(title="Summary", show_header=True, header_style="bold magenta", show_lines=True)
    table.add_column("File")
    table.add_column("Cognitive Complexity")
    for file in files:
        if file.complexity > max_complexity and max_complexity != 0:
            table.add_row(f"[green]{file.file_name}[/green]", f"[red]{file.complexity}[/red]")
            has_success = False
        else:
            table.add_row(f"[green]{file.file_name}[/green]", f"[blue]{file.complexity}[/blue]")
    console.print(table)
    console.print(f"Total files: {len(files)}")

    if not has_success:
        raise typer.Exit(code=1)

if __name__ == "__main__":
    app()
