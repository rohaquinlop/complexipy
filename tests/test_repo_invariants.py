"""Repository invariants that `AGENTS.md` states and no other test enforces.

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
RULE_DOCS = (
    ROOT / "docs" / "refactoring-rules.md",
    ROOT / "docs" / "es" / "refactoring-rules.md",
)
AGENTS_MD = ROOT / "AGENTS.md"
AGENTS_MAX_LINES = 150
AGENTS_MAX_WORDS = 1300

LONG_DASHES = re.compile(
    "[" + "".join(map(chr, (0x2013, 0x2014, 0x2015))) + "]"
)
RULE_ID_DECLARATION = re.compile(r'\bid: "(C\d{3})"')
RULE_ID_HEADING = re.compile(r"^### (C\d{3}):", re.MULTILINE)
MATCH_ON_RULE_ID = re.compile(r"\bmatch\b[^{]*\brule_id\b")


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
