from complexipy.types import TOMLType
from typer import Exit


def error_handler(
    argument: TOMLType,
    arg_name: str,
) -> None:
    if argument is None:
        print(
            f"You need to define {arg_name} in the CLI call arguments or in complexipy.toml file"
        )
        raise Exit(code=1)
