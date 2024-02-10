from complexipy import rust
import typer

app = typer.Typer(name="complexipy")

@app.command()
def main(
    path: str,
    max_complexity: int = typer.Option(15, "--max-complexity", "-c", help="The maximum complexity allowed, set this value as 0 to set it as unlimited."),
    is_dir: bool = typer.Option(False, "--is-dir", "-d", help="Flag that indicates if the path is a directory. If this flag is set, the path will be evaluated as a directory. Otherwise, it will be evaluated as a file."),
):
    print("==== Complexipy Summary ====")

    if is_dir:
        files = rust.evaluate_dir(path, max_complexity)

        print(f"Directory: {path}")
        print(f"Max complexity: {max_complexity}")
        print(f"Total files: {len(files)}")
        for file in files:
            print(f"File: {file.file_name} - Cognitive Complexity: {file.complexity}")
    else:
        file = rust.file_cognitive_complexity(path, max_complexity)

        print(f"Max complexity: {max_complexity}")
        print(f"File: {file.file_name} - Cognitive complexity: {file.complexity}")

if __name__ == "__main__":
    app()
