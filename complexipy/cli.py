"""Console-script entry point for the complexipy CLI.

The entire CLI pipeline (configuration, analysis, snapshot, export
formats, diff and ratchet gates) runs in Rust. This module only
bootstraps the process and hands the arguments to the extension.

The ``lsp`` argument starts the language server instead. That branch must
never write to stdout, because stdout carries the protocol frames.
"""

from __future__ import annotations

import sys

from complexipy._complexipy import run_cli, run_lsp


def main() -> None:
    """Run the Rust CLI, or the language server for ``complexipy lsp``."""
    arguments = sys.argv[1:]

    if arguments[:1] == ["lsp"]:
        sys.stdout.flush()
        sys.stderr.flush()
        sys.exit(run_lsp())

    sys.exit(run_cli(arguments))


if __name__ == "__main__":
    main()
