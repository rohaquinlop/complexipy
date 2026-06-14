# Changelog

All notable changes to this project will be documented in this file.

n## [Unreleased]
## [5.6.0] - 2026-06-14

### Features

- Add `--no-ignore` flag to disregard `# complexipy: ignore` and `# noqa: complexipy` comments
- Add `--report-ignored` flag to list all suppressed functions with optional JSON export to `complexipy-ignored.json`
- Add `IgnoredLocation` type and `collect_all_ignored_locations()` to the Python API

## [5.5.0] - 2026-05-22

### Bug Fixes

- **cache:** Bound target-set entries

### Documentation

- Document recursive exclude globs

### Features

- Add deterministic refactor plans
- **cli:** Show refactor plans in rich output
- **refactor-plans:** Include plans in json output

### Miscellaneous

- **ci:** Deploy docs only on releases

### Refactor

- Replace untyped dicts with dataclasses in output.py
- **output:** Move FunctionRow/FileEntry to dataclasses submodule
- **core:** Extract exclude logic and fix glob pattern support## [5.4.1] - 2026-05-05

### Bug Fixes

- **snapshot:** Stabilize snapshot output

### CI/CD

- Speed up pull request workflow

### Documentation

- Update stale CLI and API docs## [5.4.0] - 2026-04-25

### Bug Fixes

- Remove comprehension complexity scoring

### Documentation

- Add complexipy teams link

### Miscellaneous

- Remove warnings on cfg's feature wasm## [5.3.0] - 2026-04-16

### Bug Fixes

- Snapshot watermark now determines exit code when active
- Show snapshot-allowed functions as PASSED in output
- Correct test assertions for comprehension complexity scoring
- Replace FileComplexity constructor with \_main() in diff tests
- Use \_complexipy.main() instead of FileComplexity constructor in tests
- **utils:** Improve status output and ignore docs
- **utils:** Support ignore markers on multiline defs
- Add PyO3 default for check_script and validate --script-strict requires --check-script
- Sort --top N globally by complexity and validate N > 0
- Consistent type annotations and add --top + --failed test
- Preserve global descending order in --top for multi-file runs
- Resolve merge conflicts with upstream main
- Ratchet only fails when regressions breach the threshold
- **ci:** Handle bad CRC-32 entries in wheels during repack

### CI/CD

- Retrigger after transient ppc64le build failure

### Documentation

- Sync CLI and API documentation
- Polish documentation formatting
- **config:** Document new cli parameters
- Sync CLI and API documentation
- Polish documentation formatting
- **config:** Resolve merge conflicts in docs
- **readme:** Document --ratchet flag

### Features

- Score cognitive complexity of comprehension expressions
- Add complexity diff against a git reference (--diff)
- Add SARIF 2.1.0 output format (--output-sarif)
- Add glob pattern support to exclude configuration
- Add gitlab code quality output
- **output:** Unify report formats and destinations
- Add --ratchet flag for regression-only CI failures
- Add check_script parameter to Rust engine for module-level complexity
- Add --check-script and --script-strict CLI flags
- Add --plain flag for machine-readable output (closes #121)
- Add --top N flag to show the N most complex functions

### Miscellaneous

- Remove sonar keyword from project metadata
- Update maturin from 1.10.2 to 1.12.6
- **rust:** Remove comments
- **deps:** Refresh lockfiles
- **config:** Align ty config with ci checks
- Remove local issue and pr notes

### Refactor

- Extract \_is_function_passing to reduce build_output_rows complexity
- Reduce cognitive complexity of main and format_diff
- Flatten rust module layout
- Make module-level complexity a first-class analysis entry
- Extract handle_display to reduce main() cognitive complexity

### Testing

- Add script-level complexity fixture files
- Add script-level complexity tests and update existing tests for check_script param
- Add --script-strict CLI integration tests## [5.1.0] - 2025-12-08

### Bug Fixes

- Results output paths## [5.0.0] - 2025-11-26

### Bug Fixes

- #110 reduce error verbosity

### Features

- #104 Use failed flag to modify output
- #108 Show change since last run## [4.2.0] - 2025-09-21

### Bug Fixes

- **py38:** Postpone annotations to support Python 3.8\\n\\n- Add to and \\n- Fixes TypeError: unsupported operand type(s) for |: 'TypeVar' and 'NoneType' on 3.8\\n- Keeps behavior unchanged on Python 3.9–3.12 (no-op on 3.11+)\\n\\nFixes #95

### CI/CD

- Test across Python 3.8–3.12 on all OSes\\n\\n- Add matrix for python-version 3.8, 3.9, 3.10, 3.11, 3.12\\n- Run uv/maturin/pytest and CLI smoke under each version using: uv run -p <version>\\n- Ensures import-time annotation issues are caught on older Python

### Documentation

- **vscode:** Add 0.2.0 changelog to README

### Features

- **cli:** Add --version flag to print installed version
- #94 support inline ignore
- **exclude:** Path-aware directory/file excludes relative to roots

### Update

- Docs and cli order## [3.1.1] - 2025-07-06

### Documentation

- Update README and main.py to enhance command-line options and usage instructions
- Revise documentation for command-line options and usage examples
- Center badges in README and documentation
- Update complexity thresholds in documentation and code

### Miscellaneous

- Bump version to 3.1.0 in Cargo files and documentation## [3.0.0] - 2025-06-15

### Bug Fixes

- **rust:** Fix decorator detection
- **web:** Restart UI when no code is provided
- **web:** Improve error handling in code analysis

### Documentation

- Add GitHub Actions badge to README and index documentation
- Remove web interface section from README.md
- Add VSCode extension badge to README and documentation
- Add detailed section for VSCode extension in README and documentation
- Update README and documentation to reflect changes in complexity analysis options
- Update README and documentation to reflect new output options for complexity analysis
- Add pre-commit badge to README and documentation

### Features

- **rust:** Refactor to reduce code duplication
- **wasm:** Fix values on elif else clauses
- **rust:** Fix wasm version
- **wasm:** Use the same implementation for both Python and WASM
- **web:** Enhance the frontend
- **web:** Add active line highlighting
- **build:** Add scripts for building and serving WebAssembly module
- **wasm:** Add WebAssembly module and TypeScript definitions
- **vscode:** Initial commit for complexipy extension
- **vscode:** Enhance cognitive complexity analysis with line decorations
- Introduce complexipy VSCode extension for cognitive complexity analysis
- Update branding assets for complexipy extension
- **cli:** #70 Add JSON output option for complexity analysis

### Miscellaneous

- **deps:** Update dependencies and version for complexipy
- **pyproject:** Remove Python 3.13 classifier from pyproject.toml
- **deps:** Update dependency versions and classifiers
- **web:** Update documentation and setup for complexipy
- **build:** Update build-wasm.sh to copy generated files to complexipy-vscode directory
- Update .gitignore to include WASM directories
- **web:** Add integrity attributes to CodeMirror script tags for security
- Add ruff subproject as a new dependency
- **ci:** Enable recursive submodule checkout in CI workflow
- Update ruff dependencies to use Git sources
- Update dependencies and exclude additional directories
- Add repository information to package.json for complexipy extension
- Update .gitignore and add LICENSE file for complexipy extension
- Update CHANGELOG for version 0.0.2
- Add pre-commit configuration and documentation for complexity checks

### Refactor

- **wasm:** Simplify code complexity analysis and remove unused line number tracking
- **vscode:** Simplify complexity decoration logic and remove medium complexity handling
- **cognitive_complexity:** Remove unnecessary Option wrapping for code parameter
- **cognitive_complexity:** Optimize statement handling by removing unnecessary cloning
- **vscode:** Update complexity decoration colors and enhance decoration text
- **vscode:** Update WASM module loading in extension activation
- **web:** Enhance real-time analysis and UI updates in the code editor
- **vscode:** Update display name in package.json for clarity
- **vscode:** Remove obsolete WASM files and TypeScript definitions
- Update dependencies and improve code structure
- **tests:** Clean up extension test file by removing unused imports and messages
- Rename Rust module and update imports throughout the codebase
- Update command name and enhance README for clarity

### WIP

- Added wasm support on the same complexipy library## [2.1.1] - 2025-04-24

### Bug Fixes

- **python:** Fix compatibility with python 3.8## [2.1.0] - 2025-04-23

### Bug Fixes

- **rust:** #63 Fix dictionary expression count bool ops## [2.0.0] - 2025-04-18

### Features

- **rust:** Change parser from rustpython to ruff_python_parser## [1.2.0] - 2024-12-15

### Bug Fixes

- **python:** #57 fix output_summary function call

### Features

- **build:** Bump version to 1.2.0## [1.1.0] - 2024-12-14

### Features

- **rust:** Version 1.1.0## [0.5.0] - 2024-10-28

### Features

- **CI:** #50 fix CI workflow
- **build:** #52 improve usability of the package
- **docs:** #52 update documentation
- **docs:** Fix typo## [0.4.0] - 2024-06-21

### Features

- **back:** #40 update cognitive complexity algorithm## [0.3.3] - 2024-04-27

### Bug Fixes

- **rust:** #41 update ci and remove unused dependencies
- **ci:** Update ci
- **ci:** Update ci
- **ci:** Update ci
- **ci:** Update ci
- **ci:** Update ci
- **ci:** Update ci

### Features

- **cli|rust:** Update version to 0.3.3## [0.3.2] - 2024-03-22

### Bug Fixes

- **doc:** Fix typo
- **ci:** Update ci

### Features

- **cli:** #37 no files found
- **cli|rust:** Update version to 0.3.2## [0.3.1] - 2024-03-13

### Bug Fixes

- **algorithm:** #16 fix cognitive complexity

### Features

- **build|report|docs:** #27 Add sort option to the output
- **build:** #31 require python >= 3.8
- **docs:** Update complexipy version## [0.3.0] - 2024-03-07

### Bug Fixes

- **docs:** Fix typo
- **docs:** Fix links

### Features

- **test:** #16 update test_try
- **test:** #16 add test_try_nested
- **test:** #16 update test_try_nested
- **test:** #16 add test_nested_func
- **test:** #16 add test_kubernetes
- **build:** #16 enhance try cognitive complexity
- **test:** #16 add test_for_assign
- **test:** #16 add unit testing
- **build|docs:** #15 add function level complexity analysis
- **docs:** #15 add cognitive complexity explanation
- **build:** #23 add progress bars
- **docs:** Add contributors section
- **docs:** Fix spaces
- **docs:** Fix links
- **build|docs:** #25 add quiet option## [0.2.2] - 2024-02-27

### Features

- **build:** Update readme and version## [0.2.1] - 2024-02-27

### Features

- **build|docs:** Update version## [0.2.0] - 2024-02-27

### Features

- **build:** #3 ignore paths
- **build:** #1 add support for git repositories
- **report:** #4 change the output format to CSV
- **build|docs:** #4 update readme
- **docs:** #4 readme update
- **build:** Fix cognitive complexity algorithm
- **docs:** Create docs and update readme## [1.0.0] - 2024-02-11

### Features

- **build:** Initial setup
- **back:** Add ast and first calculations
- **back:** Add tests
- **back:** Add count_bool_ops function
- **back:** Improve cognitive complexity algorithm \[**BREAKING**\]
- **test:** Add new tests
- **test:** Add new tests
- **back:** Add directory analysis \[**BREAKING**\]
- **cli:** Add parameters information to the help message
- **docs:** Update readme
- **ci:** Add workflow
- **ci:** Update to use pypi publish
- **build:** Set python version to 3.11
- **test:** Delete test
- **build:** Set python minimum version to 3.11
- **build:** Update python version to 3.11 \[**BREAKING**\]
- **ci:** Setup rust-toolchain
- **build:** Set rustpython-parser version
- **build:** Pin rustpython-parser
- **ci:** Update release job
- **gui:** Update gui information
- **back:** #2 delete directory flag
- **docs:** Update readme
- **docs:** #2 update readme
- **docs:** #2 update readme
- **ci:** Cancel previous run
- **back:** Add execution time to summary
- **back:** Add output to XML file option
- **docs:** Update readme
- **docs:** Update readme
- **build:** Set version
- **ci:** Remove skip existing
- **ci:** Add skip existing
- **build:** Update version

### Revert

- **build:** Set python version to 3.11
