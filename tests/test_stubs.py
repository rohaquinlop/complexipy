"""Tests that the extension stub matches the extension module.

The `.pyi` file is hand-maintained, so it can drift from the extension in
either direction. These tests use the live module and the Rust declarations as
the oracles: they compare the declared top-level names, the declared
constructors and their signatures, the declared fields and their descriptor
kind, the declared enum members, and the field annotations against the Rust
field types. Unmapped Rust types and unrecognized shapes fail the test rather
than passing silently.
"""

from __future__ import annotations

import ast
import enum
import re
from pathlib import Path
from typing import Any

import pytest

import complexipy
import complexipy._complexipy as extension

STUB = Path(complexipy.__file__).parent / "_complexipy.pyi"
ROOT = Path(__file__).resolve().parents[1]
RUST_SOURCES = (
    ROOT / "crates" / "complexipy-core" / "src" / "classes.rs",
    ROOT / "crates" / "complexipy-python" / "src" / "lib.rs",
)

RUST_STRUCT = re.compile(r"pub struct (\w+) \{(.*?)\n\}", re.DOTALL)
RUST_FIELD = re.compile(r"pub (\w+): ([^,\n]+),")
RUST_GATED = re.compile(r"#\[cfg\(feature = \"(\w+)\"\)\]")

SCALARS = {
    "u8": "int",
    "u32": "int",
    "u64": "int",
    "usize": "int",
    "i32": "int",
    "i64": "int",
    "f32": "float",
    "f64": "float",
    "String": "str",
    "bool": "bool",
}
WRAPPERS = (
    ("Vec<", "List"),
    ("Option<Box<", "Optional"),
    ("Option<", "Optional"),
)

FIELD = "field"
PROPERTY = "property"
METHOD = "method"
ENUM_MEMBER = "enum_member"


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


def is_enum(cls: type) -> bool:
    return issubclass(cls, enum.Enum)


def is_property(member: ast.FunctionDef) -> bool:
    return any(
        isinstance(decorator, ast.Name) and decorator.id == "property"
        for decorator in member.decorator_list
    )


def member_kind(member: ast.stmt) -> tuple[str, str] | None:
    """Classify one stub member, or None when it is not a member."""
    if isinstance(member, ast.AnnAssign) and isinstance(
        member.target, ast.Name
    ):
        return member.target.id, FIELD
    if isinstance(member, ast.Assign) and isinstance(
        member.targets[0], ast.Name
    ):
        return member.targets[0].id, ENUM_MEMBER
    if isinstance(member, ast.FunctionDef) and member.name != "__init__":
        return member.name, PROPERTY if is_property(member) else METHOD

    return None


def declared_members(class_name: str) -> dict[str, str]:
    """Map each member the stub declares to its kind."""
    for node in stub_tree().body:
        if not isinstance(node, ast.ClassDef) or node.name != class_name:
            continue

        members: dict[str, str] = {}

        for member in node.body:
            kind = member_kind(member)
            if kind is not None:
                members[kind[0]] = kind[1]

        return members

    raise AssertionError(f"{class_name} is not declared in the stub")


def declared_field_annotations(class_name: str) -> dict[str, str]:
    """Map each declared field to the annotation text the stub writes."""
    source = STUB.read_text(encoding="utf-8")
    annotations: dict[str, str] = {}

    for node in ast.parse(source).body:
        if not isinstance(node, ast.ClassDef) or node.name != class_name:
            continue

        for member in node.body:
            if not isinstance(member, ast.AnnAssign):
                continue
            if (
                not isinstance(member.target, ast.Name)
                or member.annotation is None
            ):
                continue

            segment = ast.get_source_segment(source, member.annotation)
            annotations[member.target.id] = (segment or "").strip()

    return annotations


def runtime_members(cls: type) -> dict[str, str]:
    """Map each public attribute of a live class to its descriptor kind."""
    return {
        name: type(value).__name__
        for name, value in vars(cls).items()
        if not name.startswith("_")
    }


def rust_structs() -> dict[str, dict[str, str]]:
    """Map every Rust struct in the type sources to its field types."""
    structs: dict[str, dict[str, str]] = {}

    for source in RUST_SOURCES:
        text = source.read_text(encoding="utf-8")

        for name, body in RUST_STRUCT.findall(text):
            fields: dict[str, str] = {}
            gated = False

            for line in body.splitlines():
                gate = RUST_GATED.search(line)
                if gate is not None:
                    gated = gate.group(1) != "python"
                    continue

                match = RUST_FIELD.search(line)
                if match is not None and not gated:
                    fields[match.group(1)] = match.group(2).strip()

                gated = False

            structs[name] = fields

    return structs


def python_annotation(rust_type: str, classes: set[str]) -> str:
    """Translate one Rust field type to the annotation the stub must use."""
    for prefix, wrapper in WRAPPERS:
        if rust_type.startswith(prefix) and rust_type.endswith(">"):
            inner = rust_type[len(prefix) : -1]

            return f"{wrapper}[{python_annotation(inner, classes)}]"

    if rust_type in SCALARS:
        return SCALARS[rust_type]

    if rust_type in classes:
        return rust_type

    raise AssertionError(
        f"no Python annotation is mapped for the Rust type {rust_type!r}"
    )


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

        if is_enum(cls):
            continue

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
        if "__init__" not in members and not is_enum(getattr(extension, name))
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


def test_stub_fields_match_the_live_descriptors() -> None:
    for name in declared_classes():
        cls = getattr(extension, name)

        if is_enum(cls):
            continue

        live = runtime_members(cls)

        for member, kind in declared_members(name).items():
            assert member in live, (
                f"{name}.{member} is declared in the stub but absent from "
                f"the extension"
            )

            if kind in (FIELD, PROPERTY):
                assert live[member] == "getset_descriptor", (
                    f"{name}.{member} is declared as a {kind} but the "
                    f"extension exposes a {live[member]}"
                )
            else:
                assert live[member] != "getset_descriptor", (
                    f"{name}.{member} is declared as a {kind} but the "
                    f"extension exposes a field"
                )

        for member in live:
            assert member in declared_members(name), (
                f"{name}.{member} exists on the extension but is not "
                f"declared in the stub"
            )


def test_stub_enum_members_match_the_extension() -> None:
    for name in declared_classes():
        cls = getattr(extension, name)

        if not is_enum(cls):
            continue

        declared = {
            member
            for member, kind in declared_members(name).items()
            if kind == ENUM_MEMBER
        }

        assert declared == {member.name for member in cls}, (
            f"{name} declares {sorted(declared)} but the extension has "
            f"{sorted(member.name for member in cls)}"
        )


def test_stub_field_annotations_match_the_rust_fields() -> None:
    classes = set(declared_classes())
    compared = 0

    for name, fields in rust_structs().items():
        if name not in classes:
            continue

        declared = declared_field_annotations(name)

        for field, rust_type in fields.items():
            expected = python_annotation(rust_type, classes)

            assert field in declared, (
                f"{name}.{field} exists in Rust but is not declared in the stub"
            )
            assert declared[field] == expected, (
                f"{name}.{field} is annotated {declared[field]!r} in the "
                f"stub, but the Rust field type {rust_type!r} maps to "
                f"{expected!r}"
            )

            compared += 1

    assert compared, "no Rust struct field was compared"
