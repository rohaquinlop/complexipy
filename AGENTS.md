# complexipy

> Cognitive complexity analyzer for Python - measures how hard code is for humans to understand.

This is the single source of truth for agent instructions in this repository.
`CLAUDE.md` is a pointer to this file plus a short Claude-Code-specific section -
put anything tool-agnostic here, not there. See [Keeping This File Current](#keeping-this-file-current).

## Tech Stack

- **Language:** Python 3.8+ (package shell) + Rust (engine, CLI, diff)
- **Framework:** clap (CLI args), owo-colors/syntect/comfy-table (terminal output)
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
├── crates/                       # Cargo workspace (root Cargo.toml is virtual)
│   ├── complexipy-core/          # engine: algorithm, types, rules, runner, diff
│   │   └── src/
│   │       ├── cognitive_complexity.rs   # AST walking + scoring algorithm
│   │       ├── classes.rs                # Data types (FunctionComplexity, RefactorPlan, ...)
│   │       ├── refactor_plans.rs         # ComplexityRegion tree + build_refactor_plans()
│   │       ├── rules/                    # Clippy-style refactor rule system
│   │       │   ├── types.rs              # RefactorRule trait + RuleMetadata
│   │       │   ├── complexity.rs         # Concrete rules (C001-C007, C011)
│   │       │   └── registry.rs           # Registration, filtering, ranking, overlap
│   │       ├── runner.rs                 # File/dir/git-URL walk + shared entry points
│   │       ├── diff.rs                   # git-diff comparison (compute_diff, DiffEntry)
│   │       ├── api.rs                    # Rust-level code_complexity / file_complexity
│   │       ├── utils.rs                  # CSV/JSON writers, snapshot I/O, AST helpers
│   │       └── helpers/exclude.rs        # Glob-based file exclusion
│   ├── complexipy-cli/           # CLI: clap args, output rendering, run orchestration
│   ├── complexipy-python/        # PyO3 module (_complexipy) + py_diff wrappers
│   └── complexipy-wasm/          # wasm-bindgen entry point
│
├── complexipy/                   # Python package: thin re-export layer over Rust
│   ├── __init__.py               # Public API: imports _complexipy, file_complexity wrapper
│   ├── cli.py                    # Console-script bootstrap → _complexipy.run_cli
│   ├── py.typed                  # PEP 561 marker
│   └── _complexipy.pyi           # Type stubs for the Rust extension
│
├── tests/                        # pytest test suite
│   ├── main.py                   # Core tests + paper conformance
│   ├── src/                      # Test fixture .py files (excluded from collection)
│   ├── fixtures/refactor_plans/  # Rule-behaviour fixtures
│   └── test_*.py                 # Utility module tests
│
├── docs/                         # MkDocs content (EN + es/)
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

**After editing any `crates/**/*.rs`, rebuild before running pytest** - otherwise
pytest exercises the previously built `.so`, and both passing and failing results
are meaningless.

### Test

```bash
uv run pytest               # Python suite (testpaths = tests/)
cargo test --workspace      # Rust tests across all four crates
```

Single test:

```bash
uv run pytest tests/main.py::TestFiles::test_match
uv run pytest tests/test_refactor_plans.py::test_match_dispatcher_creates_dispatcher_plan
uv run pytest -k refactor
cargo test -p complexipy-core rules::registry
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
cargo check -p complexipy-core
cargo check -p complexipy-core --no-default-features
cargo check -p complexipy-wasm --target wasm32-unknown-unknown
```

### Run

```bash
uv run complexipy <path>
uv run complexipy . --diff main --max-complexity-allowed 15
uv run complexipy complexipy --failed          # dogfood the tool on itself
```

### Benchmarks

```bash
./benchmarks/benchmark-cli.sh # corpus comparison plus synthetic scaling guard
```

`benchmark-cli.sh` compares the current CLI against the 7.0.1 baseline on
pinned real repos, then times a generated synthetic fixture at 1x/2x/4x
sizes (generated into `~/.cache/complexipy-benchmarks/scaling/`, never
committed) and records the scaling ratios in `benchmarks/results.md`,
which the docs pages include via pymdownx snippets.

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
complexipy/cli.py        console-script bootstrap: sys.argv → _complexipy.run_cli()
complexipy/__init__.py   public API: re-exports _complexipy names + file_complexity wrapper
  └─ complexipy._complexipy  PyO3 module (crates/complexipy-python)
       ├─ run_cli → complexipy_cli::run::run_at()   clap args → RunConfig → display/exit
       ├─ code_complexity / file_complexity         engine entry points (complexipy-core)
       └─ compute_diff / has_regressions            diff ratchet (complexipy-core)
```

### The FFI contract

Every Rust-side type crosses into Python through
`crates/complexipy-core/src/classes.rs` (`FileComplexity`, `FunctionComplexity`,
`LineComplexity`, `RefactorPlan`, `CodeSuggestion`, `RuleCategory`,
`Applicability`, `IgnoredLocation`, `RemovableIgnore`, `CodeComplexity`).
Changing one of those structs means updating **three** places in lockstep:
`crates/complexipy-core/src/classes.rs` → the `#[pymodule]` export list in
`crates/complexipy-python/src/lib.rs` → the stubs in `complexipy/_complexipy.pyi`.
The core crate's `python` feature gates the `#[pyclass]` attributes on the shared
types.

`complexipy/__init__.py` is the public Python API surface: `code_complexity`,
`file_complexity`, `collect_all_ignored_locations`,
`collect_removable_ignored_locations`, `compute_diff`, `has_regressions`, and the
`DiffEntry` / `DiffStatus` types. Those exports, their signatures, and the
`DiffStatus` values are a compatibility promise - internal refactors must keep them
stable, and new exports belong in `__init__.py` + `__all__` with docs in `docs/` (EN + ES).

### Rust core

- `crates/complexipy-core/src/cognitive_complexity.rs` - the algorithm. Parses with
  `ruff_python_parser`, walks the AST, and accumulates structural / nesting / boolean
  increments. While scoring, it also records a tree of `ComplexityRegion`s.
- `crates/complexipy-core/src/refactor_plans.rs` - defines `ComplexityRegion` /
  `RegionKind` / `ComplexityResult` and `build_refactor_plans()`, which lazily builds
  a `OnceLock<RuleRegistry>` and delegates to it. Scoring produces regions; regions
  produce refactor plans. Keep that direction - rules never re-parse source to find
  structure, they consume regions.
- `crates/complexipy-core/src/rules/` - the refactor rule system (see below).
- `crates/complexipy-core/src/runner.rs` - path/dir/git-URL expansion, exclusion
  globs, and the shared entry points (`run_analysis_shared`, `file_complexity_shared`,
  the ignored-location collectors).
- `crates/complexipy-core/src/diff.rs` - git diff comparison, `DiffEntry` /
  `DiffStatus`, staged diff, regression ratchet.
- `crates/complexipy-core/src/api.rs` - Rust-level `code_complexity` /
  `file_complexity` (mirrors the Python public API).
- `crates/complexipy-core/src/utils.rs` - CSV/JSON writers, snapshot file I/O, and
  AST helpers (`count_bool_ops`, noqa/ignore-comment scanning).
- `crates/complexipy-wasm/src/lib.rs` - the browser entry point; calls the same
  `code_complexity_shared()` the Python path uses.

### Refactor rules (`--suggest-refactors`)

A rule is `RefactorRule::check(region, source, function_complexity) -> Option<RefactorPlan>`
plus a `&'static RuleMetadata`. `RuleMetadata::new_plan()` prefills the identity fields
so `id` / `category` / `applicability` / `description` / `doc_url` can only ever come
from metadata; rules fill in the dynamic fields via `..metadata().new_plan()`.

`RuleRegistry::analyze()` then, in order: collects plans over the region tree
recursively, drops any plan with `estimated_reduction < 1` as noise, sorts by
spliceable desc → `effectiveness` desc → reduction desc → line asc (a
machine-applicable replacement beats a help-only plan of higher
effectiveness), resolves overlapping line ranges by keeping the
higher-spliceable/higher-effectiveness/higher-reduction plan, and caps at 5
plans per function.

`effectiveness` in `RuleMetadata` is the single source of truth for ranking - the
registry reads it via `effectiveness_by_rule_id()`, so there is no `match rule_id`
anywhere. Adding a rule is therefore: write the struct + `impl RefactorRule` in
`crates/complexipy-core/src/rules/complexity.rs`, set its `effectiveness` tier, register it in
`RuleRegistry::register_defaults()`, and document it in **both**
`docs/refactoring-rules.md` and `docs/es/refactoring-rules.md`.

Guiding principle for rule output: never emit a suggestion the tool cannot stand
behind. If a heuristic isn't confident, emit `help` text rather than a wrong
`suggestion`, and never print a complexity number the code knows is fabricated.

### Dual-target Rust

The workspace splits the three build targets across crates instead of feature
flags:

- `complexipy-core` - target-agnostic engine. Features: `default = ["runner"]`,
  `runner` (file-walker deps `ignore`/`globset`/`wax`), `python` (pyo3 `#[pyclass]`
  attributes on shared types), `wasm` (adds `CodeComplexity.version`).
- `complexipy-cli` - clap args + output rendering; depends on core (default features).
- `complexipy-python` - PyO3 module; depends on core (`python`, `runner`) and the
  cli crate (for `run_cli`). Built by maturin via `manifest-path` in pyproject.toml.
- `complexipy-wasm` - wasm-bindgen entry; depends on core with
  `default-features = false` and `features = ["wasm"]`.

Dependency direction is one-way: python → cli → core, wasm → core. Never the
reverse. Adding a dependency means adding it to the crate that uses it.

## Testing

- `tests/main.py` - core suite: asserts exact complexity totals for the fixtures in
  `tests/src/`, plus SonarSource paper conformance. If you change the algorithm, these
  hardcoded numbers are the contract you're renegotiating - update them deliberately,
  never to make a run go green.
- `tests/test_*.py` - one file per Python-side concern; refactor-plan fixtures and
  behaviour tests against the public API.
- `tests/fixtures/refactor_plans/` - fixtures for rule behaviour, deliberately kept out
  of the `tests/src/` complexity corpus so rule work doesn't perturb the asserted
  totals.
- Rust tests live next to their module. Public-API tests go in the crate's
  `tests/` directory; tests that need private items are a `mod tests;` child module
  in a sibling file (e.g. `crates/complexipy-core/src/utils.rs` →
  `crates/complexipy-core/src/utils/tests.rs`). No `#[path]`
  wiring - a new test file is invisible until the owning module declares it.
- `pyproject.toml` sets `python_files = ["test_*.py", "main.py"]`, so `tests/main.py` is
  a test module (not a script), and `norecursedirs = ["tests/src"]` keeps the fixture
  `.py` files from being collected.

## Code Style

- No comments in code. The code must speak for itself.
- ASCII punctuation only. Never use Unicode dashes (em dash U+2014, en
  dash U+2013, horizontal bar U+2015) in code, comments, docs, or commit
  messages. Use ASCII `-`.
- Docstrings only when necessary, and only about what the function does. Never changelog or history notes.
- Conventional Commits: `type(scope): description` (e.g., `fix(diff): resolve path for nested invocation`).
- Pre-commit hooks: complexipy (self-dogfooding, `max-complexity-allowed = 15` from
  `[tool.complexipy]`), mdformat, yamlfix. Pass explicitly quoted paths to
  `pre-commit run --files`; unquoted globs yield a bogus "no files to check".
- Ruff for linting and formatting (line-length 80, indent-width 4; `tests/**` excluded from lint).

## Key Files

- `crates/complexipy-core/src/cognitive_complexity.rs` - Core algorithm: parses Python AST via ruff, computes cognitive complexity with nesting/structural/boolean increments
- `crates/complexipy-core/src/refactor_plans.rs` - `ComplexityRegion` tree, `build_refactor_plans()` registry entry point
- `crates/complexipy-core/src/rules/types.rs` - `RefactorRule` trait, `RuleMetadata`, `new_plan()`
- `crates/complexipy-core/src/rules/registry.rs` - Rule registration, noise filtering, effectiveness ranking, overlap resolution
- `crates/complexipy-core/src/diff.rs` - Git diff comparison, `DiffEntry`/`DiffStatus`, `compute_diff`, `has_regressions`
- `crates/complexipy-core/src/runner.rs` - Shared entry points: `run_analysis_shared`, `file_complexity_shared`, ignored-location collectors
- `crates/complexipy-python/src/lib.rs` - PyO3 module `_complexipy`, pyfunctions, `py_diff` wrappers
- `crates/complexipy-cli/src/run.rs` - `run_at()`: config → analysis → snapshot → display → exit code
- `crates/complexipy-cli/src/utils/config.rs` - `resolve_config()`: merges CLI args + TOML into `RunConfig`
- `crates/complexipy-cli/src/output.rs` - Console display, `handle_display`, `handle_results_storage`
- `crates/complexipy-cli/src/utils/paths.rs` - Output path resolution for CSV/JSON/GitLab/SARIF exports
- `crates/complexipy-cli/src/utils/snapshot.rs` - `evaluate_snapshot()`, `SnapshotEvaluation`, watermark logic
- `crates/complexipy-wasm/src/lib.rs` - wasm-bindgen entry over `code_complexity_shared`
- `complexipy/__init__.py` - Public Python API surface (`__all__` compatibility promise)
- `complexipy/_complexipy.pyi` - Type stubs for the Rust extension module
- `tests/main.py` - Core test suite including SonarSource paper conformance tests

## Conventions

- **Package manager:** Always use `uv` - `uv run pytest`, `uv run ruff`, `uv run complexipy`
- **Commits:** Only commit when explicitly asked. Never auto-commit. Stage explicit paths - never `git add -A` or `git add .`
- **PR titles:** Must follow Conventional Commits (enforced by CI).
- **GitHub CLI:** When available, use `gh` to retrieve context before making changes - check linked issues for requirements (`gh issue view <number>`), review open PRs for related work (`gh pr list`, `gh pr view <number>`), inspect CI status (`gh run list`, `gh run view <id>`), and search the repo (`gh search issues`, `gh search prs`). Always check the relevant issue or PR before implementing to understand the full scope and any prior discussion. If `gh` is not installed, skip these checks and work from the code and local context only.

## Agent Configuration Layout

Each piece of agent config has exactly one real copy; the other paths point at it.

- `AGENTS.md` (this file) is canonical. `CLAUDE.md` is a stub that imports it via
  `@AGENTS.md` and holds only Claude-Code-specific mechanics.
- `.agents/skills/` holds the real skill files, so any tool that reads `.agents/` sees
  plain files. `.claude/skills` is a symlink to `../.agents/skills` - **do not replace
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
  contract, the region → rule direction, Rust tests as `mod tests;` siblings), an **architectural
  boundary**, or a **convention**, update this file in the same commit - scope it
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
- Do not use `pip` or `python -m` - use `uv run` for all commands.
- Do not modify `crates/` (Rust) without understanding the per-crate target setup.
- Do not run pytest against stale Rust changes - `uv run maturin develop` first.
- Do not adjust asserted complexity totals in `tests/main.py` to make a run pass.
- Do not duplicate agent config. `CLAUDE.md` imports this file; `.claude/skills` is a
  symlink. Copies drift.
