"""Console-script entry point for the complexipy CLI.

The entire CLI pipeline (configuration, analysis, snapshot, export
formats, diff and ratchet gates) runs in Rust. This module only
bootstraps the process and hands the arguments to the extension.
"""

from __future__ import annotations

import sys

from complexipy._complexipy import run_cli


def main() -> None:
    """Run the Rust CLI with the current process arguments."""
    sys.exit(run_cli(sys.argv[1:]))


if __name__ == "__main__":
    main()
