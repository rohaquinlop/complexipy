from complexipy import rust
import typer

app = typer.Typer(name="complexipy")

@app.command()
def main(
    path: str,
    max_complexity: int = typer.Option(15, "--max-complexity", "-c", help="The maximum complexity allowed"),
    is_dir: bool = typer.Option(False, "--is-dir", "-d", help="The path is a directory."),
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
