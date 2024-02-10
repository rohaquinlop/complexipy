from complexipy import rust
import typer

app = typer.Typer(name="complexipy")

@app.command()
def main(
    path: str,
    max_complexity: int = typer.Option(15, "--max-complexity", "-c", help="The maximum complexity allowed"),
    is_dir: str = typer.Option(None, "--is-dir", "-d", help="The path is a directory."),
):
    ans = None
    if is_dir:
        # ans = rust.evaluate_dir(path, max_complexity)
        print("Your path is a directory")
    else:
        ans = rust.file_cognitive_complexity(path, max_complexity)
    if ans:
        print(f"Cognitive complexity: {ans.complexity}")
        print(f"Max complexity: {max_complexity}")

if __name__ == "__main__":
    app()
