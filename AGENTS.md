# complexipy

> Cognitive complexity analyzer for Python — measures how hard code is for humans to understand.

This is the single source of truth for agent instructions in this repository.
`CLAUDE.md` is a pointer to this file plus a short Claude-Code-specific section —
put anything tool-agnostic here, not there. See [Keeping This File Current](#keeping-this-file-current).

## Tech Stack

- **Language:** Python 3.8+ (CLI/API) + Rust (core engine via PyO3/maturin)
- **Framework:** Typer (CLI), Rich (terminal output)
- **Package Manager:** uv (Python), Cargo (Rust)
- **Build:** maturin (Rust → Python extension), wasm-pack (Rust → WASM)
- **Docs:** MkDocs Material (EN + ES)

The analysis engine is Rust; the CLI and public Python API are thin wrappers over a
PyO3 extension module (`complexipy._complexipy`). The same Rust core also compiles to
WASM for the browser demo and the VS Code extension. Scoring follows G. Ann Campbell's
SonarSource cognitive complexity paper.

## Project Structure

```
complexipy/
├── src/                          # Rust core engine
│   ├── cognitive_complexity.rs   # AST walking + complexity scoring algorithm
│   ├── classes.rs                # Data types (FunctionComplexity, RefactorPlan, etc.)
│   ├── refactor_plans.rs         # ComplexityRegion tree + build_refactor_plans()
│   ├── rules/                    # Clippy-style refactor rule system
│   │   ├── types.rs              # RefactorRule trait + RuleMetadata
│   │   ├── complexity.rs         # Concrete rules (C001–C007, C011)
│   │   └── registry.rs           # Registration, filtering, ranking, overlap resolution
│   ├── runner.rs                 # File/dir/URL processing, git clone
│   ├── utils.rs                  # CSV/JSON writers, snapshot I/O, AST helpers
│   ├── wasm.rs                   # wasm-bindgen entry point
│   ├── lib.rs                    # PyO3 module entry
│   ├── helpers.rs                # Module declarations (helpers/)
│   ├── helpers/exclude.rs        # Glob-based file exclusion
│   ├── rules.rs                  # Module declarations (rules/)
│   └── tests/rules/              # Rust unit tests (wired in via #[path])
│
├── complexipy/                   # Python CLI + API wrapper
│   ├── main.py                   # Typer CLI app (entrypoint, pure orchestrator)
│   ├── api.py                    # Python API (code_complexity, file_complexity)
│   ├── types.py                  # Enums + dataclasses (RunConfig, ExitReport)
│   ├── utils/                    # Utilities organized by domain
│   │   ├── config.py             # CLI + TOML config resolution (resolve_config)
│   │   ├── output.py             # Rich console display + formatting
│   │   ├── paths.py              # Output path resolution for export formats
│   │   ├── diff.py               # Git diff computation + flag resolution
│   │   ├── snapshot.py           # Snapshot evaluation + watermark
│   │   ├── ignored.py            # Ignored-location reporting
│   │   ├── toml.py               # TOML loading + argument resolution
│   │   ├── cache.py              # Previous-run caching for delta reporting
│   │   ├── constants.py          # Output filenames, legacy flag maps
│   │   ├── csv.py, json.py,      # Export format writers
│   │   │   gitlab.py, sarif.py
│   │   └── dataclasses.py        # FunctionRow, FileEntry display types
│   └── _complexipy.pyi           # Type stubs for Rust extension
│
├── tests/                        # pytest test suite
│   ├── main.py                   # Core tests + paper conformance
│   ├── src/                      # Test fixture .py files (excluded from collection)
│   ├── fixtures/refactor_plans/  # Rule-behaviour fixtures
│   └── test_*.py                 # Utility module tests
│
├── docs/                         # MkDocs content (EN + es/)
├── pkg/                          # wasm-pack output (copied into web/ and vscode/)
├── web/                          # Browser demo (WASM + CodeMirror)
├── vscode/                       # VS Code extension (WASM module)
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

**After editing any `src/**/*.rs`, rebuild before running pytest** — otherwise pytest
exercises the previously built `.so`, and both passing and failing results are
meaningless.

### Test

```bash
uv run pytest                 # Python suite (testpaths = tests/)
cargo test --features python  # Rust unit tests
```

Single test:

```bash
uv run pytest tests/main.py::TestFiles::test_match
uv run pytest tests/test_refactor_plans.py::test_match_dispatcher_creates_dispatcher_plan
uv run pytest -k refactor
cargo test --features python rules::registry
```

### Lint, Format & Type Check

```bash
uv run ruff check .
uv run ruff format .
uv run ty check .
```

### Cross-target compile checks

CI builds wheels for both features but only type-checks and tests the Python one;
break the wasm target and *release* breaks, not CI.

```bash
cargo check --features python
cargo check --no-default-features --features wasm --target wasm32-unknown-unknown
```

### Run

```bash
uv run complexipy <path>
uv run complexipy . --diff main --max-complexity-allowed 15
uv run complexipy complexipy --failed          # dogfood the tool on itself
```

### WASM / web demo

```bash
./build-wasm.sh          # wasm-pack build → web/wasm/ + vscode/complexipy/wasm/
./serve-web-version.sh   # serve web/ on :8080
```

### Docs

```bash
uv run mkdocs serve
```

## Architecture

### Layering

```
complexipy/main.py       Typer CLI — pure orchestrator, no logic of its own
  ├─ utils/config.py     resolve_config(): CLI flags + TOML → RunConfig
  ├─ _complexipy.main()  ← Rust: walks paths/dirs/git URLs, returns Vec<FileComplexity>
  ├─ utils/output.py     Rich rendering, plain output, refactor-plan display
  ├─ utils/{csv,json,gitlab,sarif}.py  export writers
  ├─ utils/{snapshot,diff,cache,ignored}.py  gates that feed the exit code
  └─ types.ExitReport    aggregates display/snapshot/paths/diff → exit 0 or 1
```

### The FFI contract

Every Rust-side type crosses into Python through `src/classes.rs` (`FileComplexity`,
`FunctionComplexity`, `LineComplexity`, `RefactorPlan`, `CodeSuggestion`,
`RuleCategory`, `Applicability`, `IgnoredLocation`, `RemovableIgnore`,
`CodeComplexity`). Changing one of
those structs means updating **three** places in lockstep: `src/classes.rs` → the
`#[pymodule]` export list in `src/lib.rs` → the stubs in `complexipy/_complexipy.pyi`.

`complexipy/__init__.py` is the public Python API surface: `code_complexity`,
`file_complexity`, `collect_all_ignored_locations`,
`collect_removable_ignored_locations`, `compute_diff`, `has_regressions`, and the
`DiffEntry` / `DiffStatus` types. Those exports, their signatures, and the
`DiffStatus` values are a compatibility promise — internal refactors must keep them
stable, and new exports belong in `__init__.py` + `__all__` with docs in `docs/` (EN + ES).

### Rust core

- `src/cognitive_complexity.rs` — the algorithm. Parses with `ruff_python_parser`,
  walks the AST, and accumulates structural / nesting / boolean increments. While
  scoring, it also records a tree of `ComplexityRegion`s.
- `src/refactor_plans.rs` — defines `ComplexityRegion` / `RegionKind` /
  `ComplexityResult` and `build_refactor_plans()`, which lazily builds a
  `OnceLock<RuleRegistry>` and delegates to it. Scoring produces regions; regions
  produce refactor plans. Keep that direction — rules never re-parse source to find
  structure, they consume regions.
- `src/rules/` — the refactor rule system (see below).
- `src/runner.rs` — path/dir/git-URL expansion, exclusion globs, progress bar, and the
  `#[pyfunction]`s (`main`, `file_complexity`, `collect_all_ignored_locations`,
  `collect_removable_ignored_locations`).
- `src/utils.rs` — CSV/JSON writers, snapshot file I/O, and AST helpers
  (`count_bool_ops`, noqa/ignore-comment scanning).
- `src/wasm.rs` — the browser entry point; calls the same
  `function_level_cognitive_complexity_shared()` the Python path uses.

### Refactor rules (`--suggest-refactors`)

A rule is `RefactorRule::check(region, source, function_complexity) -> Option<RefactorPlan>`
plus a `&'static RuleMetadata`. `RuleMetadata::new_plan()` prefills the identity fields
so `id` / `category` / `applicability` / `description` / `doc_url` can only ever come
from metadata; rules fill in the dynamic fields via `..metadata().new_plan()`.

`RuleRegistry::analyze()` then, in order: collects plans over the region tree
recursively, drops any plan with `estimated_reduction < 2` as noise, sorts by
spliceable desc → `effectiveness` desc → reduction desc → line asc (a
machine-applicable replacement beats a help-only plan of higher
effectiveness), resolves overlapping line ranges by keeping the
higher-spliceable/higher-effectiveness/higher-reduction plan, and caps at 5
plans per function.

`effectiveness` in `RuleMetadata` is the single source of truth for ranking — the
registry reads it via `effectiveness_by_rule_id()`, so there is no `match rule_id`
anywhere. Adding a rule is therefore: write the struct + `impl RefactorRule` in
`src/rules/complexity.rs`, set its `effectiveness` tier, register it in
`RuleRegistry::register_defaults()`, and document it in **both**
`docs/refactoring-rules.md` and `docs/es/refactoring-rules.md`.

Guiding principle for rule output: never emit a suggestion the tool cannot stand
behind. If a heuristic isn't confident, emit `help` text rather than a wrong
`suggestion`, and never print a complexity number the code knows is fabricated.

### Dual-target Rust

`default = ["python"]`; the `wasm` feature swaps PyO3 for wasm-bindgen. Gate code
accordingly:

- `#[cfg(feature = "python")]` — PyO3-only (`runner.rs`, `utils.rs` pyfunctions)
- `#[cfg(feature = "wasm")]` — wasm-bindgen-only
- `#[cfg(any(feature = "python", feature = "wasm"))]` — shared analysis logic
- `#[cfg_attr(feature = "python", pyclass(...))]` on shared data types

Most dependencies are `optional = true` and pulled in by whichever feature needs them,
so adding a dependency usually means adding it to the feature list too.

## Testing

- `tests/main.py` — core suite: asserts exact complexity totals for the fixtures in
  `tests/src/`, plus SonarSource paper conformance. If you change the algorithm, these
  hardcoded numbers are the contract you're renegotiating — update them deliberately,
  never to make a run go green.
- `tests/test_*.py` — one file per Python util module; CLI behaviour via Typer's
  `CliRunner`, git operations via `unittest.mock.patch`.
- `tests/fixtures/refactor_plans/` — fixtures for rule behaviour, deliberately kept out
  of the `tests/src/` complexity corpus so rule work doesn't perturb the asserted
  totals.
- Rust tests live under `src/tests/` and are attached to their module with
  `#[cfg(test)] #[path = "../tests/rules/foo.rs"] mod tests;` — a new Rust test file is
  invisible until you add that `mod tests;` line in the module it tests.
- `pyproject.toml` sets `python_files = ["test_*.py", "main.py"]`, so `tests/main.py` is
  a test module (not a script), and `norecursedirs = ["tests/src"]` keeps the fixture
  `.py` files from being collected.

## Code Style

- No comments in code. The code must speak for itself.
- Docstrings only when necessary, and only about what the function does — never changelog or history notes.
- Conventional Commits: `type(scope): description` (e.g., `fix(diff): resolve path for nested invocation`).
- Pre-commit hooks: complexipy (self-dogfooding, `max-complexity-allowed = 15` from
  `[tool.complexipy]`), mdformat, yamlfix. Pass explicitly quoted paths to
  `pre-commit run --files`; unquoted globs yield a bogus "no files to check".
- Ruff for linting and formatting (line-length 80, indent-width 4; `tests/**` excluded from lint).

## Key Files

- `src/cognitive_complexity.rs` — Core algorithm: parses Python AST via ruff, computes cognitive complexity with nesting/structural/boolean increments
- `src/refactor_plans.rs` — `ComplexityRegion` tree, `build_refactor_plans()` registry entry point
- `src/rules/types.rs` — `RefactorRule` trait, `RuleMetadata`, `new_plan()`
- `src/rules/registry.rs` — Rule registration, noise filtering, effectiveness ranking, overlap resolution
- `complexipy/main.py` — CLI entrypoint: pure orchestrator that delegates to utils modules
- `complexipy/types.py` — `RunConfig`, `ExitReport` dataclasses + enums (`ColorTypes`, `Sort`, `OutputFormat`)
- `complexipy/utils/config.py` — `resolve_config()`: merges CLI args + TOML into `RunConfig`
- `complexipy/utils/output.py` — Rich console display, `handle_display`, `handle_results_storage`
- `complexipy/utils/paths.py` — Output path resolution for CSV/JSON/GitLab/SARIF exports
- `complexipy/utils/diff.py` — Git diff computation, `resolve_diff_flags`, `handle_diff_output`, staged (index) comparison via `compute_staged_diff`
- `complexipy/utils/snapshot.py` — `evaluate_snapshot()`, `SnapshotEvaluation`, watermark logic
- `complexipy/utils/toml.py` — TOML loading, data-driven `get_arguments_value`
- `complexipy/_complexipy.pyi` — Type stubs for the Rust extension module
- `tests/main.py` — Core test suite including SonarSource paper conformance tests

## Conventions

- **Package manager:** Always use `uv` — `uv run pytest`, `uv run ruff`, `uv run complexipy`
- **Commits:** Only commit when explicitly asked. Never auto-commit. Stage explicit paths — never `git add -A` or `git add .`
- **PR titles:** Must follow Conventional Commits (enforced by CI).
- **GitHub CLI:** When available, use `gh` to retrieve context before making changes — check linked issues for requirements (`gh issue view <number>`), review open PRs for related work (`gh pr list`, `gh pr view <number>`), inspect CI status (`gh run list`, `gh run view <id>`), and search the repo (`gh search issues`, `gh search prs`). Always check the relevant issue or PR before implementing to understand the full scope and any prior discussion. If `gh` is not installed, skip these checks and work from the code and local context only.

## Agent Configuration Layout

Each piece of agent config has exactly one real copy; the other paths point at it.

- `AGENTS.md` (this file) is canonical. `CLAUDE.md` is a stub that imports it via
  `@AGENTS.md` and holds only Claude-Code-specific mechanics.
- `.agents/skills/` holds the real skill files, so any tool that reads `.agents/` sees
  plain files. `.claude/skills` is a symlink to `../.agents/skills` — **do not replace
  it with copies.** Add a new skill once, under `.agents/skills/<name>/SKILL.md`.
  (Contributors on Windows need symlink support in their checkout, i.e.
  `git config core.symlinks true` with Developer Mode enabled.)
- `SKILL.md` files are excluded from the mdformat hook. mdformat has no frontmatter
  support: it rewrites the opening `---` as a thematic break and the closing `---` as a
  setext heading, which silently destroys the YAML that makes a skill loadable. Do not
  remove that exclusion, and do not hand-run `mdformat` over a `SKILL.md`.

## Keeping This File Current

Treat this file as part of the change, not as documentation to catch up on later.

- If a change alters a **command**, a **structural invariant** (the FFI three-place
  contract, the region → rule direction, `#[path]`-wired Rust tests), an **architectural
  boundary**, or a **convention**, update this file in the same commit — scope it
  `docs(agents)` when the doc edit stands alone.
- New refactor rule, export format, or CLI flag: check whether Project Structure, Key
  Files, or Architecture now describe something that no longer exists.
- Do not restate any of this in `CLAUDE.md`. That file imports this one via
  `@AGENTS.md`; anything duplicated there will drift. It holds only
  Claude-Code-specific mechanics (which skills to load, subagent and plan-mode habits).

## Anti-Patterns

- Do not add comments to code. Use descriptive variable/function names instead.
- Do not add docstrings that describe changelog or history. Docstrings describe what a function does.
- Do not commit without explicit user instruction.
- Do not use `pip` or `python -m` — use `uv run` for all commands.
- Do not modify `src/` (Rust) without understanding the PyO3/wasm-bindgen dual-target setup.
- Do not run pytest against stale Rust changes — `uv run maturin develop` first.
- Do not adjust asserted complexity totals in `tests/main.py` to make a run pass.
- Do not duplicate agent config. `CLAUDE.md` imports this file; `.claude/skills` is a
  symlink. Copies drift.
