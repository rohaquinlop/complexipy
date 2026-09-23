# complexipy

Cognitive complexity analyzer for Python: it measures how hard code is for
humans to understand. Scoring follows G. Ann Campbell's SonarSource cognitive
complexity paper. The engine is Rust under `crates/`; `complexipy/` is the thin
Python package over the compiled `_complexipy` extension.

## Commands

```bash
uv sync                                             # install dependencies
uv run maturin develop                              # build the Rust extension
uv run pytest                                       # Python suite (testpaths = tests/)
cargo test --workspace                              # Rust tests, whole workspace
uv run ruff check .                                 # lint
uv run ruff format .                                # format
uv run ty check .                                   # type check
cargo check -p complexipy-core                      # default features
cargo check -p complexipy-core --no-default-features
cargo check -p complexipy-wasm --target wasm32-unknown-unknown
uv run complexipy <path>                            # run the tool
uv run complexipy . --diff main --max-complexity-allowed 15
uv run complexipy complexipy --failed               # dogfood it on itself
./benchmarks/benchmark-cli.sh                       # corpus and scaling guard
./build-wasm.sh                                     # wasm-pack -> web/wasm/, vscode/complexipy/wasm/
./serve-web-version.sh                              # serve web/ on :8080
complexipy lsp                                      # stdio LSP server
cargo run -p complexipy-lsp                         # the same server from the tree
uv run mkdocs serve                                 # preview the docs site
```

- Rebuild after any `crates/**/*.rs` edit: `uv run maturin develop`. Without it,
  pytest exercises the `.so` built from the previous source, and both a pass and
  a failure are meaningless.
- CI runs `cargo test --workspace`, clippy, fmt, and the wasm check, but nothing
  checks `--no-default-features`: run that one by hand whenever you touch a
  feature gate. Each crate declares its own target and features, so read its
  `Cargo.toml` before adding a dependency.
- One test: `uv run pytest tests/main.py::TestFiles::test_match`, or
  `cargo test -p complexipy-core rules::registry`.
- `cargo test -p complexipy-core` alone does not compile the `config` feature,
  so its tests are skipped. Use `cargo test --workspace`, which is what CI runs.

## Invariants

- A shared type that crosses into Python changes in four places in lockstep: the
  type declaration, the `#[pymodule]` export list, the stubs in
  `complexipy/_complexipy.pyi`, and `tests/test_stubs.py`, which reads the Rust
  declarations and fails when a stub disagrees.
- Rules consume `ComplexityRegion`s. A rule never re-parses source to find
  structure.
- `RuleRegistry::analyze()` drops inactive rules first, then sorts, resolves
  overlap, and caps the result at 5 plans per function. `effectiveness` in
  `RuleMetadata` is the single ranking source; no `match rule_id` exists
  anywhere.
- Rule selection lives in one `RuleSet`: `--ignore` wins over `--select`, a bare
  `# complexipy: ignore` drops the function, and `# complexipy: ignore[C007]`
  subtracts rules for one function only.
- A new rule is registered in `register_defaults()` and documented in both
  `docs/refactoring-rules.md` and `docs/es/refactoring-rules.md`.
- Never emit a suggestion the tool cannot stand behind. When a heuristic is not
  confident, emit `help` text rather than a wrong `suggestion`, and never print a
  complexity number the code knows is fabricated.
- Over the threshold means strictly greater than `max-complexity-allowed`, in the
  CLI and in the server alike.
- The server analyzes only `python` documents, or paths ending in `.py`.
- On stdout the server writes protocol frames and nothing else. Every log line
  goes to stderr, and the `lsp` branch of `complexipy/cli.py` never prints.
- Exclusion has two matchers: the walker's pattern program and
  `helpers::exclude::is_path_excluded`, which the server uses for open documents.
  `helpers/exclude/tests.rs` pins them together, so a change to one has to keep
  the other in agreement.
- The public Python API is a compatibility promise: `complexipy/__init__.py`, its
  `__all__`, and the `DiffStatus` values. A new export needs docs in `docs/` and
  `docs/es/`. `run_lsp` is a process entry point and stays out of `__all__`.

## Testing

- `tests/main.py` asserts the exact totals for the fixtures in `tests/src/`, plus
  paper conformance. An algorithm change renegotiates those numbers
  deliberately, never to make a run go green.
- `tests/fixtures/refactor_plans/` holds rule-behaviour fixtures and stays out of
  the `tests/src/` corpus, so rule work does not perturb the asserted totals.
- `pyproject.toml` sets `python_files = ["test_*.py", "main.py"]`, so
  `tests/main.py` is a test module, and `norecursedirs = ["tests/src"]` keeps the
  fixture files from being collected.
- Rust tests that need private items are a `mod tests;` child module in a sibling
  file, as in `utils.rs` and `utils/tests.rs`. A new test file is invisible until
  its owning module declares it, so no inline `mod tests { }` and no `#[path]`.
- Read the LSP child's stdout in `tests/test_lsp.py` with `os.read` on a raw fd.
  A buffered read swallows the frame, and the following `select` then reports an
  empty pipe.

## Conventions

- Use `uv` for everything: `uv run pytest`, `uv run ruff`, `uv run complexipy`.
  Never `pip`, and never `python -m`.
- Commit only when asked. Stage explicit paths, never `git add -A` or
  `git add .`.
- Commit subjects and PR titles follow Conventional Commits. `CONTRIBUTING.md`
  states the rule; CI enforces the PR side.
- Before implementing, check the linked issue or pull request with `gh` for the
  full scope and any earlier discussion.
- Run `pre-commit run --files` with quoted paths. Unquoted globs yield a bogus
  "no files to check".

## Code Style

- No comments in code. The code must speak for itself.
- Exception: YAML and shell scripts may carry a comment that records a build
  invariant or the reason for a step. Those files are read while editing CI,
  and the comment is often the rule's only record.
- Docstrings only when necessary, and only about what the function does. Never a
  changelog or a history note.
- ASCII punctuation only. No em dash, en dash, or horizontal bar in code,
  comments, docs, or commit messages.
- Ruff runs at line-length 80 and indent-width 4, with `tests/**` excluded from
  lint.
- Pre-commit runs complexipy on itself with `max-complexity-allowed = 15`,
  mdformat, and yamlfix. `SKILL.md` files are excluded from mdformat: it has no
  frontmatter support and rewrites the opening and closing `---`, which destroys
  the YAML that makes a skill loadable.

## Agent Configuration

- `AGENTS.md` (this file) is canonical. There is no `CLAUDE.md`.
- `.agents/skills/` holds the real skill files. `.claude/skills` is a symlink to
  `../.agents/skills`: do not replace it with a copy.
- `.pi/hooks.json` and `.pi/hook-scripts/` hold the pi hooks: ruff, ty, pytest,
  complexipy, and the Rust checks, each on its matching file type.
- `.sdd/` holds the specs and the change artifacts. Do not commit it.

## Keeping This File Current

A fact earns a line here only when an agent acting in good faith could break it.
If a test or a CI job catches the break, one short sentence is enough. If
nothing catches it, the line states the consequence. Rationale, history, and
anything the repository can be asked for directly fail that test: they go in the
commit message, or in the artifact under `.sdd/`.

A line costs about 15 tokens on every request here, so it has to earn its place.

This file stays at or under 150 lines (`wc -l AGENTS.md`) and about 1,300 words.
When a new fact no longer fits, delete one line to add one. Never raise the
limit: a rule that will not fit needs a real home, such as a workflow comment, a
test, a type, or a spec under `.sdd/`. Update this file in the same commit as a
change to a command, an invariant, or a convention.
