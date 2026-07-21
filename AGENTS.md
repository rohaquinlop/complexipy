# complexipy

> Cognitive complexity analyzer for Python — measures how hard code is for humans to understand.

## Tech Stack

- **Language:** Python 3.8+ (CLI/API) + Rust (core engine via PyO3/maturin)
- **Framework:** Typer (CLI), Rich (terminal output)
- **Package Manager:** uv (Python), Cargo (Rust)
- **Build:** maturin (Rust → Python extension), wasm-pack (Rust → WASM)
- **Docs:** MkDocs Material (EN + ES)

## Project Structure

```
complexipy/
├── src/                          # Rust core engine
│   ├── cognitive_complexity.rs   # AST walking + complexity scoring algorithm
│   ├── classes.rs                # Data types (FunctionComplexity, RefactorPlan, etc.)
│   ├── refactor_plans.rs         # Deterministic refactor plan generation
│   ├── runner.rs                 # File/dir/URL processing, git clone
│   ├── lib.rs                    # PyO3 module entry
│   └── helpers/exclude.rs        # Glob-based file exclusion
│
├── complexipy/                   # Python CLI + API wrapper
│   ├── main.py                   # Typer CLI app (entrypoint)
│   ├── api.py                    # Python API (code_complexity, file_complexity)
│   ├── types.py                  # Enums (ColorTypes, Sort, OutputFormat)
│   ├── utils/                    # Output, diff, snapshot, config, export formats
│   └── _complexipy.pyi           # Type stubs for Rust extension
│
├── tests/                        # pytest test suite
│   ├── main.py                   # Core tests + paper conformance
│   ├── src/                      # Test fixture .py files
│   └── test_*.py                 # Utility module tests
│
├── docs/                         # MkDocs content (EN + es/)
├── web/                          # Browser demo (WASM + CodeMirror)
└── .github/workflows/            # CI, PR title check, release
```

## Commands

### Setup

```bash
uv sync
```

### Build (Rust extension)

```bash
uv run maturin develop
```

### Test

```bash
uv run pytest tests/
```

### Lint, Format & Type Check

```bash
uv run ruff check .
uv run ruff format .
uv run ty check .
```

### Run

```bash
uv run complexipy <path>
uv run complexipy . --diff main --max-complexity-allowed 15
```

### Docs

```bash
uv run mkdocs serve
```

## Code Style

- No comments in code. The code must speak for itself.
- Docstrings only when necessary, and only about what the function does — never changelog or history notes.
- Conventional Commits: `type(scope): description` (e.g., `fix(diff): resolve path for nested invocation`).
- Pre-commit hooks: complexipy (self-dogfooding), mdformat, yamlfix.
- Ruff for linting and formatting (line-length 80, indent-width 4).

## Key Files

- `src/cognitive_complexity.rs` — Core algorithm: parses Python AST via ruff, computes cognitive complexity with nesting/structural/boolean increments
- `complexipy/main.py` — CLI entrypoint: argument handling, TOML config, output formatting, snapshot, diff logic
- `complexipy/utils/output.py` — Rich console output formatting
- `complexipy/utils/diff.py` — Git-based complexity diff with `--diff` enforcement
- `complexipy/utils/snapshot.py` — Baseline snapshot management for regression tracking
- `complexipy/_complexipy.pyi` — Type stubs for the Rust extension module
- `tests/main.py` — Core test suite including SonarSource paper conformance tests

## Conventions

- **Package manager:** Always use `uv` — `uv run pytest`, `uv run ruff`, `uv run complexipy`
- **Commits:** Only commit when explicitly asked. Never auto-commit.
- **PR titles:** Must follow Conventional Commits (enforced by CI).
- **Tests:** pytest with `CliRunner` for CLI tests, `unittest.mock.patch` for git operations in diff tests.
- **Dual-target Rust:** `#[cfg(feature = "python")]` for PyO3, `#[cfg(feature = "wasm")]` for wasm-bindgen. Shared logic uses `#[cfg(any(feature = "python", feature = "wasm"))]`.
- **GitHub CLI:** When available, use `gh` to retrieve context before making changes — check linked issues for requirements (`gh issue view <number>`), review open PRs for related work (`gh pr list`, `gh pr view <number>`), inspect CI status (`gh run list`, `gh run view <id>`), and search the repo (`gh search issues`, `gh search prs`). Always check the relevant issue or PR before implementing to understand the full scope and any prior discussion. If `gh` is not installed, skip these checks and work from the code and local context only.

## Anti-Patterns

- Do not add comments to code. Use descriptive variable/function names instead.
- Do not add docstrings that describe changelog or history. Docstrings describe what a function does.
- Do not commit without explicit user instruction.
- Do not use `pip` or `python -m` — use `uv run` for all commands.
- Do not modify `src/` (Rust) without understanding the PyO3/wasm-bindgen dual-target setup.
