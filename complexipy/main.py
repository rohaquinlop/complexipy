from complexipy import rust
import typer

app = typer.Typer(name="complexipy")

@app.command()
def main(
    path: str,
    is_dir: str = typer.Option(None, "--is-dir", "-d", help="Is the path a directory?"),
    max_complexity: int = typer.Option(15, "--max-complexity", "-m", help="The maximum complexity allowed"),
):
    ans = None
    if is_dir:
        ans = rust.evaluate_dir(path, max_complexity)
    else:
        ans = rust.file_cognitive_complexity(path, max_complexity)
    ans = rust.file_cognitive_complexity(path, max_complexity)
    print(ans.path)
    print(ans.complexity)

if __name__ == "__main__":
    app()
