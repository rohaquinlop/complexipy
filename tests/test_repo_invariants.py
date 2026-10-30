"""Repository invariants that no other test enforces.

These tests read only tracked text and never import the extension, so the
`Repo Checks` workflow can run them without building Rust, including on the
docs-only changes that `CI.yml` skips.
"""

from __future__ import annotations

import ast
import re
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
RULES_DIR = ROOT / "crates" / "complexipy-core" / "src" / "rules"
SYNTAX_TREE = ROOT / "crates" / "complexipy-core" / "src" / "syntax_tree.rs"
RULE_DOCS = (
    ROOT / "docs" / "refactoring-rules.md",
    ROOT / "docs" / "es" / "refactoring-rules.md",
)
AGENTS_MD = ROOT / "AGENTS.md"
AGENTS_MAX_LINES = 150
AGENTS_MAX_WORDS = 1300
PYPROJECT = ROOT / "pyproject.toml"
RELEASE_WORKFLOW = ROOT / ".github" / "workflows" / "release.yml"
LICENSE = ROOT / "LICENSE"
CARGO_TOML = ROOT / "Cargo.toml"
ZED_LICENSE = ROOT / "zed" / "LICENSE"

LONG_DASHES = re.compile(
    "[" + "".join(map(chr, (0x2013, 0x2014, 0x2015))) + "]"
)
RULE_ID_DECLARATION = re.compile(r'\bid: "(C\d{3})"')
RULE_ID_HEADING = re.compile(r"^### (C\d{3}):", re.MULTILINE)
MATCH_ON_RULE_ID = re.compile(r"\bmatch\b[^{]*\brule_id\b")
PYTHON_VERSION_AXIS = re.compile(
    r"^\s*python-version: \[([^\]]*)\]", re.MULTILINE
)
QUOTED_VERSION = re.compile(r'"(\d+)\.(\d+)"')
REQUIRES_PYTHON_CAP = re.compile(
    r'^requires-python = "[^"]*<(\d+)\.(\d+)"', re.MULTILINE
)
PYTHON_VERSION_CLASSIFIER = re.compile(
    r'"Programming Language :: Python :: (\d+)\.(\d+)"'
)
WORKSPACE_VERSION = re.compile(
    r'^\[workspace\.package\]$(?:\n(?!\[).*)*?\nversion = "([^"]+)"',
    re.MULTILINE,
)
PRE_COMMIT_REV = re.compile(
    r"repo: https://github\.com/rohaquinlop/complexipy-pre-commit\s*\n\s*rev: (\S+)"
)


def tracked_files() -> list[Path]:
    output = subprocess.run(
        ["git", "ls-files", "-z"],
        cwd=ROOT,
        check=True,
        capture_output=True,
    ).stdout
    return [ROOT / name for name in output.decode().split("\0") if name]


def tracked_text(path: Path) -> str | None:
    try:
        return path.read_text(encoding="utf-8")
    except (UnicodeDecodeError, OSError):
        return None


def is_test_source(path: Path) -> bool:
    return path.name == "tests.rs" or "tests" in path.parts


def production_rust_sources() -> list[Path]:
    return [
        path
        for path in (ROOT / "crates").glob("*/src/**/*.rs")
        if not is_test_source(path)
    ]


def test_tracked_text_has_no_long_dashes() -> None:
    offenders = []
    for path in tracked_files():
        text = tracked_text(path)
        if text is None:
            continue
        for number, line in enumerate(text.splitlines(), start=1):
            if LONG_DASHES.search(line):
                offenders.append(f"{path.relative_to(ROOT)}:{number}")

    assert not offenders, (
        "use ASCII punctuation, not an em dash, en dash, or horizontal bar: "
        + ", ".join(offenders)
    )


def test_every_rule_is_documented_in_both_languages() -> None:
    declared = {
        rule_id
        for path in RULES_DIR.rglob("*.rs")
        if not is_test_source(path)
        for rule_id in RULE_ID_DECLARATION.findall(
            path.read_text(encoding="utf-8")
        )
    }

    assert declared, f"no rule ids declared under {RULES_DIR}"

    for doc in RULE_DOCS:
        documented = set(
            RULE_ID_HEADING.findall(doc.read_text(encoding="utf-8"))
        )

        assert documented == declared, (
            f"{doc.relative_to(ROOT)}: missing {sorted(declared - documented)}, "
            f"unknown {sorted(documented - declared)}"
        )


def test_no_production_code_matches_on_rule_id() -> None:
    offenders = []
    for path in production_rust_sources():
        for number, line in enumerate(
            path.read_text(encoding="utf-8").splitlines(), start=1
        ):
            if line.lstrip().startswith("//"):
                continue
            if MATCH_ON_RULE_ID.search(line):
                offenders.append(f"{path.relative_to(ROOT)}:{number}")

    assert not offenders, (
        "rank rules through RuleMetadata, not a match on rule_id: "
        + ", ".join(offenders)
    )


def test_only_the_syntax_tree_module_calls_the_parser() -> None:
    offenders = [
        str(path.relative_to(ROOT))
        for path in production_rust_sources()
        if path != SYNTAX_TREE
        and "ruff_python_parser" in path.read_text(encoding="utf-8")
    ]

    assert not offenders, (
        "parse through syntax_tree::parse_module or parse_expression, which "
        "free a long expression chain without deep recursion: "
        + ", ".join(offenders)
    )


def files_under_test() -> list[Path]:
    python = [*(ROOT / "tests").glob("test_*.py"), ROOT / "tests" / "main.py"]
    rust = [
        path
        for path in (ROOT / "crates").glob("*/tests/it/*.rs")
        if path.name != "main.rs"
    ]
    return sorted(python + rust)


def opens_with_doc_line(path: Path) -> bool:
    text = path.read_text(encoding="utf-8")
    if path.suffix == ".py":
        return bool(ast.get_docstring(ast.parse(text)))
    for line in text.splitlines():
        stripped = line.strip()
        if stripped and not stripped.startswith("#!["):
            return stripped.startswith("//!")
    return False


def test_every_test_file_opens_with_a_doc_line() -> None:
    missing = [
        str(path.relative_to(ROOT))
        for path in files_under_test()
        if not opens_with_doc_line(path)
    ]

    assert not missing, (
        'open each test file with a one-line """ or //! doc: '
        + ", ".join(missing)
    )


def test_agents_md_stays_within_its_budget() -> None:
    text = AGENTS_MD.read_text(encoding="utf-8")
    lines = text.count("\n")
    words = len(text.split())

    assert lines <= AGENTS_MAX_LINES and words <= AGENTS_MAX_WORDS, (
        f"AGENTS.md has {lines} lines and {words} words, over the "
        f"{AGENTS_MAX_LINES}-line, {AGENTS_MAX_WORDS}-word budget: delete a "
        "line to add one"
    )


def versions_in(pattern: re.Pattern[str], text: str) -> set[tuple[int, int]]:
    return {(int(major), int(minor)) for major, minor in pattern.findall(text)}


def release_python_axes() -> dict[int, set[tuple[int, int]]]:
    text = RELEASE_WORKFLOW.read_text(encoding="utf-8")
    return {
        text.count("\n", 0, axis.start()) + 1: versions_in(
            QUOTED_VERSION, axis.group(1)
        )
        for axis in PYTHON_VERSION_AXIS.finditer(text)
    }


def dotted(version: tuple[int, int]) -> str:
    return f"{version[0]}.{version[1]}"


def test_requires_python_cap_follows_the_release_matrix() -> None:
    axes = release_python_axes()

    assert axes, (
        f"no python-version axis in {RELEASE_WORKFLOW.relative_to(ROOT)}"
    )

    newest = {line: max(versions) for line, versions in axes.items()}

    assert len(set(newest.values())) == 1, (
        "release.yml python-version axes disagree on the newest version: "
        + ", ".join(f"line {line}: {dotted(v)}" for line, v in newest.items())
    )

    tested = next(iter(newest.values()))
    expected = (tested[0], tested[1] + 1)
    cap = REQUIRES_PYTHON_CAP.search(PYPROJECT.read_text(encoding="utf-8"))

    assert cap, "requires-python in pyproject.toml has no < upper bound"

    actual = (int(cap.group(1)), int(cap.group(2)))

    assert actual == expected, (
        f"requires-python caps at <{dotted(actual)}, but release.yml tests up "
        f"to {dotted(tested)}: set the cap to <{dotted(expected)}"
    )


def test_python_classifiers_match_the_release_matrix() -> None:
    tested = set().union(*release_python_axes().values())
    classified = versions_in(
        PYTHON_VERSION_CLASSIFIER, PYPROJECT.read_text(encoding="utf-8")
    )

    assert classified == tested, (
        "pyproject.toml Python classifiers: missing "
        f"{[dotted(v) for v in sorted(tested - classified)]}, untested "
        f"{[dotted(v) for v in sorted(classified - tested)]}"
    )


def test_zed_license_matches_the_root_license() -> None:
    assert ZED_LICENSE.read_text(encoding="utf-8") == LICENSE.read_text(
        encoding="utf-8"
    ), (
        f"{ZED_LICENSE.relative_to(ROOT)} differs from "
        f"{LICENSE.relative_to(ROOT)}: copy the root license into zed/"
    )


def documentation_files() -> list[Path]:
    return [
        path
        for path in tracked_files()
        if path.suffix == ".md"
        and (path == ROOT / "README.md" or ROOT / "docs" in path.parents)
    ]


def test_pre_commit_snippets_pin_the_current_release() -> None:
    version = WORKSPACE_VERSION.search(CARGO_TOML.read_text(encoding="utf-8"))

    assert version, "Cargo.toml has no [workspace.package] version"

    expected = f"v{version.group(1)}"
    revs = [
        (path.relative_to(ROOT).as_posix(), rev)
        for path in documentation_files()
        for rev in PRE_COMMIT_REV.findall(tracked_text(path) or "")
    ]

    assert revs, "no complexipy pre-commit snippet in README.md or docs/"
    stale = [f"{name}: {rev}" for name, rev in revs if rev != expected]
    assert not stale, (
        f"pre-commit snippets must pin rev: {expected}, the workspace "
        f"version in Cargo.toml: {stale}"
    )
