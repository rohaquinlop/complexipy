"""Tests that the extension stub matches the extension module.

The `.pyi` file is hand-maintained, so it can drift from the extension in
either direction. These tests use the live module as the oracle: they compare
the declared top-level names, they check that a declared constructor exists
exactly where the extension can build an instance, and they call `DiffEntry`
through the parameter names the stub declares, so a renamed or reordered
parameter fails. Field annotations are not compared.
"""

from __future__ import annotations

import ast
from pathlib import Path
from typing import Any

import pytest

import complexipy
import complexipy._complexipy as extension

STUB = Path(complexipy.__file__).parent / "_complexipy.pyi"


def stub_tree() -> ast.Module:
    return ast.parse(STUB.read_text(encoding="utf-8"))


def declared_names() -> set[str]:
    return {
        node.name
        for node in stub_tree().body
        if isinstance(node, (ast.FunctionDef, ast.ClassDef))
    }


def declared_classes() -> dict[str, set[str]]:
    return {
        node.name: {
            member.name
            for member in node.body
            if isinstance(member, ast.FunctionDef)
        }
        for node in stub_tree().body
        if isinstance(node, ast.ClassDef)
    }


def constructor_parameters(class_name: str) -> dict[str, ast.expr | None]:
    """Map each declared constructor parameter to its annotation."""
    for node in stub_tree().body:
        if not isinstance(node, ast.ClassDef) or node.name != class_name:
            continue

        for member in node.body:
            if (
                isinstance(member, ast.FunctionDef)
                and member.name == "__init__"
            ):
                return {
                    argument.arg: argument.annotation
                    for argument in member.args.args
                    if argument.arg != "self"
                }

    raise AssertionError(f"{class_name} declares no constructor")


def declares_optional(annotation: ast.expr | None) -> bool:
    return (
        isinstance(annotation, ast.Subscript)
        and isinstance(annotation.value, ast.Name)
        and annotation.value.id == "Optional"
    )


def accepts_none(position: int) -> bool:
    constructor: Any = extension.DiffEntry
    arguments: list[Any] = [
        None if index == position else argument
        for index, argument in enumerate(["sample.py", "heavy", 1, 2])
    ]

    try:
        constructor(*arguments)
    except TypeError:
        return False

    return True


def exported_names() -> set[str]:
    return {name for name in dir(extension) if not name.startswith("__")}


def can_be_built(cls: type) -> bool:
    return cls.__new__ is not object.__new__


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


def test_stub_constructors_match_the_extension() -> None:
    for name, members in declared_classes().items():
        cls = getattr(extension, name)
        declared = "__init__" in members
        supported = can_be_built(cls)

        assert declared == supported, (
            f"{name} declares __init__={declared}, but the extension "
            f"answers can_be_built={supported}"
        )


def test_the_extension_refuses_classes_without_a_declared_constructor() -> None:
    refused = [
        name
        for name, members in declared_classes().items()
        if "__init__" not in members
    ]

    assert refused

    for name in refused:
        with pytest.raises(TypeError):
            getattr(extension, name)()


DIFF_ENTRY_VALUES = ("sample.py", "heavy", 1, 2)


def test_diff_entry_accepts_the_declared_arguments() -> None:
    names = list(constructor_parameters("DiffEntry"))
    entry = extension.DiffEntry(**dict(zip(names, DIFF_ENTRY_VALUES)))

    assert entry.file_path == "sample.py"
    assert entry.func_name == "heavy"
    assert entry.old_complexity == 1
    assert entry.new_complexity == 2
    assert entry.status is extension.DiffStatus.REGRESSED

    added = extension.DiffEntry("sample.py", "heavy", None, 2)

    assert added.old_complexity is None
    assert added.status is extension.DiffStatus.NEW


def test_stub_marks_the_optional_parameters_as_optional() -> None:
    parameters = constructor_parameters("DiffEntry")

    for position, (name, annotation) in enumerate(parameters.items()):
        expected = accepts_none(position)
        declared = declares_optional(annotation)

        assert declared == expected, (
            f"{name} is declared Optional={declared}, but the extension "
            f"answers accepts_none={expected}"
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
