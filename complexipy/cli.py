"""Console-script bootstrap for the complexipy CLI.

The entire CLI pipeline (configuration, analysis, snapshot, export
formats, diff and ratchet gates) runs in Rust. This module only
bootstraps the process and hands the arguments to the extension.

The ``lsp`` argument starts the language server instead. That branch must
never write to stdout, because stdout carries the protocol frames. The
word reserves the command line, so a path named ``lsp`` needs
``complexipy -- lsp``.
"""

from __future__ import annotations

import sys
from pathlib import Path

from complexipy._complexipy import run_cli, run_lsp

LSP_ARGUMENT = "lsp"


def lsp_shadow_warning() -> str | None:
    """Return a warning when the reserved word hides a path.

    The warning appears only on an interactive terminal, because an editor
    starts the server with a pipe on stdin and never needs it.
    """
    if not Path(LSP_ARGUMENT).exists():
        return None

    stdin = sys.stdin
    if stdin is None or not stdin.isatty():
        return None

    return (
        f"complexipy is starting the language server; run "
        f"'complexipy -- {LSP_ARGUMENT}' to analyze ./{LSP_ARGUMENT}"
    )


def main() -> None:
    """Run the Rust CLI, or the language server for ``complexipy lsp``."""
    arguments = sys.argv[1:]

    if arguments[:1] == [LSP_ARGUMENT]:
        warning = lsp_shadow_warning()
        if warning is not None:
            print(warning, file=sys.stderr)

        sys.stdout.flush()
        sys.stderr.flush()
        sys.exit(run_lsp())

    sys.exit(run_cli(arguments))


if __name__ == "__main__":
    main()
