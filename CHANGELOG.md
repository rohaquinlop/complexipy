# Changelog

All notable changes to complexipy are documented here, newest first. Each
release section links to its GitHub release notes for the full details.

## Unreleased

### Changed

- The Python CLI is retired: `complexipy` is now a native Rust
  implementation exposed through a thin console-script shim, so the whole
  pipeline runs without a Python interpreter. The Python API is unchanged
  and now includes the diff comparison (`compute_diff`, `has_regressions`,
  `DiffEntry`, `DiffStatus`) backed by the same Rust engine as the CLI.
  The extension module keeps being distributed via maturin wheels.
  ([#224](https://github.com/rohaquinlop/complexipy/issues/224), [#243](https://github.com/rohaquinlop/complexipy/issues/243))

### Fixed

- The C007 collapsible-if rule no longer suggests merging a nested `if`
  when a same-level statement with side effects precedes it — the merge
  would change program behavior. Comments in multiline headers and
  trailing comments are handled instead of being misplaceable by the
  replacement.
  ([#228](https://github.com/rohaquinlop/complexipy/issues/228), [#236](https://github.com/rohaquinlop/complexipy/issues/236))
- The C007 preceding-statement guard no longer fails when blank or
  comment-only lines sit between the outer and inner `if`: the indent
  step detection now skips such lines, so the merge is still rejected
  instead of producing a replacement that drops code.
  ([#245](https://github.com/rohaquinlop/complexipy/issues/245))
- The C002 loop-guards suggestion keeps statements that sit between the
  chained `if`s: they now appear in the replacement between the
  corresponding guards instead of being dropped. Guard conditions are now
  parenthesized (`if not (<cond>):`), so inverting conditions with `and`
  or `or` no longer silently changes what the guard skips.
- The C005 extract-predicate suggestion keeps the statement keyword:
  `while` conditions stay `while` loops, and `elif` conditions get help
  text only instead of a broken standalone `if` replacement. The
  predicate body indent now follows the file's indent step.
- C002 and C007 refuse machine suggestions when the shifted body holds a
  multi-line string literal: dedenting would change the string's value.
  The plans carry help text instead.
- C002 loop guards strip a redundant top-level `not` from the guard
  condition (`if not a:` becomes guard `if a:`), and C005 predicate names
  get an underscore suffix when the source already defines the generated
  name.
- C005 extract-predicate now emits a module-level helper whose parameters
  are the condition's free variables (attribute bases included, builtins
  excluded), so the helper is unit-testable. The snippet shows the
  enclosing context at the statement's real indentation. Conditions that
  bind a name (`:=`) or contain a lambda/comprehension get help text only.

## [7.0.1] - 2026-08-12

### Changed

- The release pipeline now retries artifact uploads with pinned workflow
  versions and only notifies downstream repositories on tag releases,
  keeping non-tag pushes silent.
- Pinned maturin version in the release workflow to avoid GitHub API
  rate limits when resolving the latest version inside Docker build
  containers.

### Fixed

- Snapshot entries for analyzed files are updated in place instead of
  being moved to the end of the file: partial runs (e.g. pre-commit hooks
  analyzing only staged files) no longer reorder the snapshot, so the
  snapshot stays byte-identical across commits that do not change
  complexity. ([#226](https://github.com/rohaquinlop/complexipy/issues/226))

See the [release notes](https://github.com/rohaquinlop/complexipy/releases/tag/7.0.1)
for the full details.

## [7.0.0] - 2026-08-10

!!! note "Migration"

    The deprecated `--output-json`, `--output-csv`, `--output-gitlab`,
    `--output-sarif`, and `--ratchet` flags and their TOML keys were
    removed; use `--output-format` and `--diff` instead. See the
    [migration guide](https://rohaquinlop.github.io/complexipy/migration/)
    for each removed flag and key with its replacement.

### Added

- `--suggest-refactors` is now a clippy-style lint system: stable rule IDs
  (C001–C005, C007, C011), category and applicability metadata,
  `path:line:col` anchors with caret spans, verbatim suggestion rendering,
  and a documentation link per rule. (#209)
- Refactor-plan findings are included in JSON, SARIF, and GitLab exports
  when `--suggest-refactors` is passed; the SARIF rule catalog is built
  dynamically from the plans encountered. (#209)
- `compute_diff`, `has_regressions`, `DiffEntry`, and `DiffStatus` are now
  part of the public Python API, so CI tools can consume diff results as
  objects instead of parsing terminal output; `DiffStatus` provides named
  constants such as `DiffStatus.REGRESSED`. (#210)
- `collect_removable_ignored_locations()` and `RemovableIgnore` are exported
  from the Python API. Every run now reports ignore comments that are no
  longer necessary (`path:line function=X complexity=N <comment>`) when the
  suppressed function is back under the allowed limit — the exit code is
  unaffected, the report is suppressed under `--quiet`, and it works under
  `--plain`. (#213)
- `--staged` flag for git-index comparison — answers "what complexity am I
  about to commit?" instead of only what changed in the working tree;
  `--staged` alone defaults the baseline to `HEAD` and enforces, while
  `--diff <ref> --staged` enforces against the ref. (#218)
- `[tool.complexipy.diff]` TOML section so the comparison policy lives in
  the repository config: `branch = "main"` makes a plain `complexipy .` run
  behave like `--diff main` (enforcement included), `staged = true` enables
  staged comparison by default, and `branch = ""` opts out. CLI flags take
  precedence over the section. (#219)
- Refactor reductions are now measured instead of estimated: for every
  machine-applicable suggestion (C002, C007) the replacement is spliced
  into the source, re-parsed, and re-scored, so `estimated_reduction` and
  `estimated_complexity_after` report the literal delta of applying the
  suggestion — and ranking, overlap resolution, and the noise filter all
  operate on measured values. The new `reduction_is_measured` flag on
  `RefactorPlan` separates measured plans from help-only formula
  estimates, which the CLI renders with a `~` qualifier
  (`Estimated reduction: ~-2`) while measured plans render plain
  (`Reduction: -2`). Guard suggestions are now faithful splices for loops
  with statements before or after the if-chain and for multi-line loop
  headers; measurement failures fall back to the formula estimate — never
  a panic, never a fabricated number. (#225)

### Changed

- Refactor-plan reduction estimates are now honest: the reduction math was
  rewritten and validated against measured before/after complexity, C004 no
  longer suggests splitting `match` statements, C006 was deleted because its
  gate could never fire, and C011 now fires on `try` → `with` → `try`
  chains. Overlapping plans are deduped against every overlap and capped at
  5, reporting dropped ones as "... and N more suggestions". (#209)
- Condition extraction in the refactor rules now tracks bracket depth,
  string literals, and walrus `:=` instead of a naive `rfind(':')`. (#209)
- `file_complexity()` now returns cwd-relative paths (matching
  `git diff --name-only`), and nested invocations resolve git paths via a
  `git ls-files` basename lookup — without this, diffing per-file results
  silently marked every function as NEW. (#210)
- Community standards files were added: `CODE_OF_CONDUCT.md`,
  `CONTRIBUTING.md`, `SECURITY.md`, issue templates, and a pull request
  template. (#201)

### Fixed

- Snapshot updates now merge with the existing snapshot instead of
  replacing it: only the files analyzed in a run are touched, so partial
  runs (e.g. pre-commit hooks analyzing only staged files) no longer erase
  the baseline for unanalyzed files. (#215)
- The docs footer now renders its links as styled links instead of raw
  markdown text, on both the English and Spanish landing pages. (#216)

### Removed

- `RefactorPlan.steps` — replaced by the concrete `suggestion` / `help`
  fields on the plan. (#209)
- `CodeSnippet` from the public API; `CodeSuggestion` is exported in its
  place. (#209)
- `--output-json` / `-j`, `--output-csv` / `-c`, `--output-gitlab`, and
  `--output-sarif` / `-sr` — use `--output-format` instead. (#221)
- `--ratchet` / `-R` — `--diff` enforces the threshold by default. (#221)
- TOML keys `output-json`, `output-csv`, `output-gitlab`, `output-sarif`,
  flat `staged`, flat `ratchet`, and the undocumented `details = "low"`
  alias. (#221)

These removals land in 7.0.0. See the
[migration guide](https://rohaquinlop.github.io/complexipy/migration/) for
each removed flag and key with its replacement.

See the [release notes](https://github.com/rohaquinlop/complexipy/releases/tag/7.0.0)
for the full details.

## [6.2.0] - 2026-07-23

### Added

- `--exclude` and `--output-format` now accept comma-separated values
  (`--exclude tests/**,src/**`), avoiding shell expansion issues. (#199)

### Fixed

- Output now shows the correct relative path from the current working
  directory when analyzing a single file or a directory. (#198)
- `--output-sarif` help text now carries a deprecation notice, clarifying
  that SARIF output is moving to a different mechanism. (#199)

### Changed

- Decomposed the monolithic 316-line `main()` function into a clean
  orchestrator (~80 lines) with extracted business logic in domain-specific
  `utils/` modules (`config.py`, `paths.py`, `ignored.py`). (#199)
- Introduced `RunConfig`, `ExitReport`, and `SnapshotEvaluation` dataclasses,
  replacing mutable accumulators and positional tuple unpacking. (#199)
- Eliminated all `global console` declarations — the `console` instance is
  constructed once and passed explicitly. (#199)
- Refactored `get_arguments_value` from a 20-parameter function returning a
  15-element tuple to a dict-based approach. (#199)
- Added 30 new tests covering config resolution and snapshot evaluation
  logic. (#199)

See the [release notes](https://github.com/rohaquinlop/complexipy/releases/tag/6.2.0)
for the full details.

## [6.1.0] - 2026-07-21

### Fixed

- Fixed git path resolution when running `--diff` from a nested subdirectory
  inside a repository, which previously caused "not a git repository"
  errors. (#196)
- Fixed ruff lint configuration to use a glob pattern instead of a directory
  exclusion for test files. (#196)

### Changed

- Simplified the diff CLI — `--diff` now always enforces
  `--max-complexity-allowed`, while `--diff-only` provides the visual-only
  comparison. `--ratchet` is deprecated in favour of this model. (#196)
- Integrated diff output into the main analysis flow instead of producing it
  as a separate post-processing step. (#196)
- Removed the redundant "Failed functions" output section — failed functions
  are now shown inline in the per-file summary. (#196)
- Added `AGENTS.md` for AI agent context.
- Added a CI workflow that notifies downstream repositories on new
  releases. (#197)

See the [release notes](https://github.com/rohaquinlop/complexipy/releases/tag/6.1.0)
for the full details.

## [6.0.1] - 2026-07-03

### Fixed

- Normalized Windows paths for wax glob compatibility in the exclude
  handling. (#194)

See the [release notes](https://github.com/rohaquinlop/complexipy/releases/tag/6.0.1)
for the full details.

## [6.0.0] - 2026-06-26

!!! note "Migration"

    This release brings the cognitive complexity algorithm into full
    conformance with the SonarSource white paper (v1.7). Scores change for
    files using `match`, `try`/`except`, `with`, comprehensions, lambdas,
    recursion, or nested ternaries — in most cases they increase. Re-run
    `complexipy` after upgrading and review the new scores; if you use
    `--max-complexity` / `--failed` in CI you will likely need to raise
    thresholds. See the [migration guide](https://rohaquinlop.github.io/complexipy/migration/)
    for upgrade guidance.

### Fixed

- `match` statements now apply a structural + nesting increment per the
  paper, instead of being scored as 0. (#192)
- `try`/`else`/`finally` blocks are now collected at the current nesting
  level instead of `+1`, matching the paper's rule. (#192)
- `except` handlers now charge `1 + nesting_level` instead of a flat
  `+1`. (#192)
- `with` blocks no longer incorrectly raise the nesting level. (#192)
- Direct recursion now emits `+1` for each self-call via a scope-aware
  `RecursionFinder`, correctly skipping nested function/class definitions
  and lambdas. (#192)
- Lambda expressions are now included in boolean-op counting, recursing
  into the body at `nesting_level + 1`. (#192)
- Comprehensions (`ListComp`, `SetComp`, `DictComp`, `Generator`) now charge
  `1 + nesting_level` per generator and `+1` per `if` filter. (#192)
- Nested ternaries now recurse at `nesting_level + 1`, so inner ternaries
  receive the correct nesting increment. (#192)
- `for`/`while` `else` clauses are now collected at the loop's own nesting
  level, not `+1`. (#192)
- Bare expression statements now count boolean ops (e.g.
  `foo(a and b)`). (#192)
- The `for` iterable now counts boolean ops, for parity with `while`. (#192)

### Changed

- Extracted all path orchestration (file I/O, directory traversal, URL
  cloning) from `cognitive_complexity.rs` into a new `src/runner.rs` module,
  gated `#[cfg(feature = "python")]`, which also resolves a pre-existing
  wasm build failure. (#192)
- Extracted `push_line`, `absorb`, `absorb_with_regions`, `finalize_region`,
  `count_line_bool_ops`, `loop_complexity`, `is_ignored`, and
  `analyze_function` helpers to eliminate the repeated fold/push pattern
  across all statement arms. (#192)
- Unified structurally identical `Stmt::For` and `Stmt::While` handling into
  a single `loop_complexity` call. (#192)
- Collapsed 7 near-identical statement arms through
  `count_line_bool_ops`. (#192)
- Derived `Default` on `ComplexityRegion` and `RegionKind`. (#192)
- Eliminated `merge_child` and its internal `child.regions.clone()`. (#192)
- Hardened the Windows release unit-test job against transient Cargo HTTP/2
  framing failures. (#189)

See the [release notes](https://github.com/rohaquinlop/complexipy/releases/tag/6.0.0)
for the full details.

## [5.6.1] - 2026-06-16

### Fixed

- Fixed a panic in `extract_comment_marker` when a multi-byte UTF-8
  character (em-dash, emoji, accented letters, CJK) straddles byte offset 16
  of a comment, which previously aborted analysis of the entire directory.
  Error-prone byte-slicing was replaced with regex-based matching. (#187)

### Removed

- Removed the unreliable `release-plz.yml` auto-release workflow and its
  supporting files (`CHANGELOG.md`, `cliff.toml`). Manual releases via
  `release.yml` (tag → build → test → publish) are now the standard
  path. (#188)

See the [release notes](https://github.com/rohaquinlop/complexipy/releases/tag/5.6.1)
for the full details.

## [5.6.0] - 2026-06-14

### Added

- `--no-ignore` flag to disregard `# complexipy: ignore` and
  `# noqa: complexipy` comments. (#182)
- `--report-ignored` flag to list all suppressed functions, with optional
  JSON export to `complexipy-ignored.json`. (#182)
- `IgnoredLocation` type and `collect_all_ignored_locations()` to the
  Python API. (#182)

### Fixed

- Fixed diff mode showing all files as NEW on Windows. (#177)

### Changed

- Migrated pyo3 to 0.29 and updated CI runners. (#185)
- Added musllinux_1_2 wheel builds for Alpine Linux support. (#180)

See the [release notes](https://github.com/rohaquinlop/complexipy/releases/tag/5.6.0)
for the full details.

## [5.5.0] - 2026-05-22

### Added

- Deterministic refactor plans: a new algorithm generates actionable,
  deterministic suggestions to reduce cognitive complexity. Plans are
  displayed in the rich CLI output and included in JSON output.
- Recursive exclude globs: exclude patterns now support `**` (e.g.
  `tests/**`). The glob engine was replaced with `wax` for correct recursive
  matching relative to the caller's working directory.

### Fixed

- Fixed unbounded growth of target-set entries in the cache that could
  degrade performance on large projects.

### Changed

- Output internals now use typed `FunctionRow` / `FileEntry` dataclasses
  instead of untyped `Dict` structures, extracted into
  `complexipy/utils/dataclasses.py`.
- Docs deploys now trigger only on release events, not on every push.

See the [release notes](https://github.com/rohaquinlop/complexipy/releases/tag/5.5.0)
for the full details.

## [5.4.1] - 2026-05-05

### Fixed

- Stabilized snapshot output by omitting transient line-level fields from
  Python-facing serialized Rust structures.
- Moved shared output constants into `complexipy.utils.constants` so output
  filenames, legacy CLI flags, and legacy TOML keys are defined in one
  place.
- Clarified deprecated output flag messaging for `--output-csv`,
  `--output-json`, and `--output-gitlab` to point users at
  `--output-format`.

### Changed

- Updated CI so pull requests run a faster quick-test matrix, while full
  wheel builds, source distribution builds, and full platform test jobs run
  for tags or manual workflow dispatches.
- Enabled uv dependency caching in CI jobs.
- Updated README and docs to match the current Python API return types and
  the current JSON array snapshot format.

See the [release notes](https://github.com/rohaquinlop/complexipy/releases/tag/5.4.1)
for the full details.

## [5.4.0] - 2026-04-25

### Removed

- Removed comprehension complexity scoring — rolls back the
  `ListComp`/`SetComp`/`Generator`/`DictComp` AST node handling from
  `count_bool_ops()` and the `count_comprehension_complexity()` helper added
  in v5.3.0. (#166)

### Fixed

- Fixed feature gating for WASM builds and included version metadata. (#164)

See the [release notes](https://github.com/rohaquinlop/complexipy/releases/tag/5.4.0)
for the full details.

## [5.3.0] - 2026-04-16

### Added

- `--ratchet` flag — CI fails only when complexity increases past the
  threshold; regressions are blocked, improvements always pass. (#159)
- `--plain` flag — machine-readable plain-text output for
  scripting/piping. (#158)
- `--top N` flag — display the N most complex functions, globally sorted
  across all files. (#157)
- `--check-script` / `--script-strict` flags — analyse module-level (script)
  complexity in addition to functions. (#156)
- Unified output destinations — consistent `--output-*` routing across all
  report formats. (#155)
- GitLab Code Quality report via `--output-gitlab`. (#153)
- SARIF 2.1.0 output via `--output-sarif` for IDE and GitHub Advanced
  Security integration. (#141)
- Git diff analysis — `--diff <ref>` reports complexity changes relative to
  any git reference. (#140)
- Comprehension complexity — list/dict/set comprehensions and generator
  expressions now contribute to cognitive complexity scores. (#139)
- `# complexipy: ignore` — new canonical inline suppression comment;
  `# noqa: complexipy` is deprecated. (#146)
- Glob patterns in the config `exclude` field. (#142)
- Spanish documentation. (#147)

### Fixed

- `--top` results now preserve global descending order across multi-file
  runs.
- `--top N` rejects `N ≤ 0` with a clear error.
- `--script-strict` now correctly requires `--check-script`.
- Ignore markers (`# complexipy: ignore`) now work on multiline function
  definitions.
- JSON output includes a final newline (POSIX compliance). (#148)
- Snapshot-allowed functions are now shown as `PASSED` in output.
- The snapshot watermark correctly drives the exit code when active.

See the [release notes](https://github.com/rohaquinlop/complexipy/releases/tag/5.3.0)
for the full details.

## [5.2.0] - 2026-01-28

### Fixed

- Fixed `# noqa: complexipy` comments on decorated functions. (#128)

### Changed

- Updated the pre-commit complexipy version in the docs. (#126)
- Updated the documentation. (#132)

See the [release notes](https://github.com/rohaquinlop/complexipy/releases/tag/5.2.0)
for the full details.

## [5.1.0] - 2025-12-09

### Fixed

- Fixed invalid results output paths on Windows. (#120)

See the [release notes](https://github.com/rohaquinlop/complexipy/releases/tag/5.1.0)
for the full details.

## [5.0.0] - 2025-11-26

!!! note "Migration"

    Conditional scoring now counts each `elif`/`else` branch as +1
    complexity (plus its boolean test), aligning with Sonar's
    cognitive-complexity rules. Expect higher scores for branching. See the
    [migration guide](https://rohaquinlop.github.io/complexipy/migration/)
    for upgrade guidance.

### Added

- Snapshots: `--snapshot-create` writes `complexipy-snapshot.json`;
  comparisons block regressions, auto-refresh on improvements, and can be
  bypassed with `--snapshot-ignore`. (#111)
- Change tracking: a per-target cache in `.complexipy_cache` shows
  deltas/new failures for over-threshold functions using stable BLAKE2
  keys. (#115)
- Output controls: `--failed` to show only violations (#114); `--color auto|yes|no`
  (#109); richer summaries of failing functions and invalid paths.
- Python 3.14 support. (#106)

### Changed

- Excludes and errors: exclude entries are resolved relative to the root and
  only applied when they match real files/dirs; missing paths are reported
  cleanly instead of panicking. (#113)

See the [release notes](https://github.com/rohaquinlop/complexipy/releases/tag/5.0.0)
for the full details.

## [4.2.0] - 2025-09-21

### Added

- Exclude files from analysis.
- Inline ignores to exclude functions from analysis.

### Fixed

- Python 3.8 support. (#96)

See the [release notes](https://github.com/rohaquinlop/complexipy/releases/tag/4.2.0)
for the full details.

## [4.1.0] - 2025-09-08

### Added

- Version support. (#93)

See the [release notes](https://github.com/rohaquinlop/complexipy/releases/tag/4.1.0)
for the full details.

## [4.0.2] - 2025-08-22

Patch release; no changes were recorded in its release notes.

See the [release notes](https://github.com/rohaquinlop/complexipy/releases/tag/4.0.2)
for the full details.

## [4.0.1] - 2025-08-21

### Fixed

- Fixed the PyPI README error.

See the [release notes](https://github.com/rohaquinlop/complexipy/releases/tag/4.0.1)
for the full details.

## [4.0.0] - 2025-08-21

!!! note "Migration"

    The logic for counting boolean operators in conditions was updated to
    align with the definition from the original paper. Existing functions
    may report higher complexities. See the
    [migration guide](https://rohaquinlop.github.io/complexipy/migration/)
    for upgrade guidance.

### Added

- Configuration support via `complexipy.toml` or `[tool.complexipy]` in
  `pyproject.toml` — users can now define default arguments.

### Fixed

- Fixed an infinite loop when analyzing modules with invalid Python
  syntax. (#85, resolved in #88)

### Changed

- Improved performance and Rust implementation details.
- Updated and improved documentation.

See the [release notes](https://github.com/rohaquinlop/complexipy/releases/tag/4.0.0)
for the full details.

## [3.3.0] - 2025-07-17

### Added

- `--max-complexity-allowed` (`-mx`) — customize the maximum allowed
  cognitive complexity threshold per function. The default remains 15 to
  maintain existing behavior.
- GitHub Actions integration for the custom threshold.

See the [release notes](https://github.com/rohaquinlop/complexipy/releases/tag/3.3.0)
for the full details.

## [3.2.0] - 2025-07-09

### Fixed

- Fixed an error when using complexipy on Windows, related to the `rich`
  library used to draw the console output with emojis.
- Fixed the `quiet` parameter still drawing output that wasn't handled in
  the Rust code.

See the [release notes](https://github.com/rohaquinlop/complexipy/releases/tag/3.2.0)
for the full details.

## [3.1.1] - 2025-07-06

### Changed

- The maximum complexity threshold is now `15`, matching the Sonar
  threshold to make adoption of the library easier. (#78)

See the [release notes](https://github.com/rohaquinlop/complexipy/releases/tag/3.1.1)
for the full details.

## [3.0.0] - 2025-06-16

!!! note "Migration"

    `--max-complexity` was removed; complexipy now uses a fixed cognitive
    complexity threshold of 15. The tool exits with an error when a function
    meets or exceeds the threshold. Use `--ignore-complexity` (`-i`) to show
    all functions regardless of their complexity score. See the
    [migration guide](https://rohaquinlop.github.io/complexipy/migration/)
    for upgrade guidance.

### Added

- WebAssembly (WASM) support — the core analysis engine can now be compiled
  to WebAssembly, enabling browser-based analysis and tools like the
  VSCode extension. (#72)
- JSON output via `--output-json` (`-j`) for machine-readable results. (#74)
- `--ignore-complexity` (`-i`) flag to display all functions regardless of
  whether they exceed the complexity threshold. (#73)
- `--details` (`-d`) now also affects CSV and JSON outputs. (#73)
- Sort results by complexity score (`asc`, `desc`) or by `name`. (#73)
- Pre-commit hook support with documentation for easy setup. (#75)

### Removed

- The `--max-complexity` argument. (#73)

See the [release notes](https://github.com/rohaquinlop/complexipy/releases/tag/3.0.0)
for the full details.

## [2.1.1] - 2025-04-24

### Fixed

- Fixed compatibility with Python 3.8. (#66)

See the [release notes](https://github.com/rohaquinlop/complexipy/releases/tag/2.1.1)
for the full details.

## [2.1.0] - 2025-04-23

### Fixed

- Fixed dictionary expression bool op counting. (#64)

See the [release notes](https://github.com/rohaquinlop/complexipy/releases/tag/2.1.0)
for the full details.

## [2.0.0] - 2025-04-18

### Changed

- Changed the parser from `rustpython` to `ruff_python_parser`. (#62)

See the [release notes](https://github.com/rohaquinlop/complexipy/releases/tag/2.0.0)
for the full details.

## [1.2.0] - 2024-12-15

### Fixed

- Fixed the `output_summary` function call that was missing the
  `files_complexities` argument. (#58)

See the [release notes](https://github.com/rohaquinlop/complexipy/releases/tag/1.2.0)
for the full details.

## [1.1.0] - 2024-12-14

### Added

- Multiple paths support — pass several paths to analyze at once. (#56)

### Removed

- The deprecated `-l` option, simplifying the command-line interface. (#56)
- File-level analysis, in order to focus on functions instead of the whole
  file. (#56)

See the [release notes](https://github.com/rohaquinlop/complexipy/releases/tag/1.1.0)
for the full details.

## [0.5.0] - 2024-10-28

### Added

- Python API — call complexipy from your own Python code with
  `file_complexity()` and `code_complexity()`. (#45)
- Library usage documented in the README. (#49)
- Improved package usability. (#53)
- Updated documentation. (#54)

See the [release notes](https://github.com/rohaquinlop/complexipy/releases/tag/0.5.0)
for the full details.

## [0.4.0] - 2024-06-21

### Added

- The cognitive complexity now considers the `If Expression` used in the
  code, including when used inside a `Call Expression` and so on. (#44)

### Fixed

- Fixed an edge case that could cause a memory overflow by filtering the
  cognitive complexity of the `orelse` values in an `If Statement` and
  keeping the nesting level subtraction propagation. (#44)

See the [release notes](https://github.com/rohaquinlop/complexipy/releases/tag/0.4.0)
for the full details.

## [0.3.3] - 2024-04-27

### Changed

- Updated CI and removed unused dependencies. (#43)

See the [release notes](https://github.com/rohaquinlop/complexipy/releases/tag/0.3.3)
for the full details.

## [0.3.2] - 2024-03-22

### Changed

- When using `--details low`, an empty summary table is no longer printed;
  an informational message is shown instead. (#38)

See the [release notes](https://github.com/rohaquinlop/complexipy/releases/tag/0.3.2)
for the full details.

## [0.3.1] - 2024-03-13

### Added

- `-s` / `--sort` optional parameter to sort the output. (#30)
- Python >= 3.8 requirement. (#35)

### Fixed

- Fixed the logic used to calculate cognitive complexity: assignment
  statements only add complexity when `IfExp` is used, and `BinOp` by
  itself does not add complexity. (#34)

See the [release notes](https://github.com/rohaquinlop/complexipy/releases/tag/0.3.1)
for the full details.

## [0.3.0] - 2024-03-07

### Added

- Function-level complexity analysis by default — the maximum complexity is
  evaluated for each function inside the Python files; per-file cognitive
  complexity is still available. (#21)
- New parameters. (#14)
- Progress bars. (#24)
- `--quiet` option. (#26)
- Unit testing. (#20)
- Cognitive complexity explanation in the docs. (#22)

### Changed

- Enhanced the algorithm to measure cognitive complexity — results are
  closer to the Sonar implementation. (#18)
- Reduced verbosity, with more information about the stages when running
  `complexipy` over git repositories (using the URL).
- CSV report generation is now implemented in Rust instead of Python,
  improving performance.

See the [release notes](https://github.com/rohaquinlop/complexipy/releases/tag/0.3.0)
for the full details.

## [0.2.2] - 2024-02-27

Patch release; no changes were recorded in its release notes.

See the [release notes](https://github.com/rohaquinlop/complexipy/releases/tag/0.2.2)
for the full details.

## [0.2.1] - 2024-02-27

### Changed

- Updated the README.

See the [release notes](https://github.com/rohaquinlop/complexipy/releases/tag/0.2.1)
for the full details.

## [0.2.0] - 2024-02-27

### Added

- Ignore paths support. (#6)
- Git repository URL support. (#7)
- CSV output format. (#8)
- Cognitive complexity algorithm fixes. (#10)
- Updated README. (#9)

See the [release notes](https://github.com/rohaquinlop/complexipy/releases/tag/0.2.0)
for the full details.
