"""Tests that the extension stub matches the extension module.

The `.pyi` file is hand-maintained, so it can drift: five declarations
survived the 8.0.0 Python-CLI retirement pointing at functions the module no
longer exposes. These tests fail on that class of drift in either direction.
"""

from __future__ import annotations

import re
from pathlib import Path

import complexipy
import complexipy._complexipy as extension

STUB = Path(complexipy.__file__).parent / "_complexipy.pyi"

TOP_LEVEL_DECLARATION = re.compile(r"^(?:def|class) (\w+)", re.MULTILINE)


def declared_names() -> set[str]:
    return set(TOP_LEVEL_DECLARATION.findall(STUB.read_text()))


def exported_names() -> set[str]:
    return {name for name in dir(extension) if not name.startswith("__")}


def test_stub_declares_every_exported_name() -> None:
    missing = exported_names() - declared_names()

    assert not missing, (
        f"exposed by the module but absent from the stub: {missing}"
    )


def test_stub_declares_nothing_the_module_lacks() -> None:
    stale = declared_names() - exported_names()

    assert not stale, (
        f"declared in the stub but absent from the module: {stale}"
    )


def test_bootstrap_functions_stay_out_of_the_public_api() -> None:
    assert "run_lsp" in exported_names()
    assert "run_cli" in exported_names()

    for name in ("run_cli", "run_lsp"):
        assert name not in complexipy.__all__, f"{name} leaked into __all__"
        assert not hasattr(complexipy, name), f"{name} leaked into the package"


def test_public_api_names_are_all_importable() -> None:
    for name in complexipy.__all__:
        assert hasattr(complexipy, name), (
            f"__all__ lists a missing name: {name}"
        )
