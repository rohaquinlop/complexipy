# Proposed Features & Ideas

This document outlines potential new features and improvements for complexipy, based on an analysis of the current codebase, existing integrations, and common pain points in developer tooling.

---

## 1. SARIF Output Format

**Priority: High | Effort: Medium**

### Problem

complexipy currently outputs results as JSON, CSV, or console text. While these are useful, they are not recognized by GitHub's Code Scanning infrastructure, which uses the [SARIF (Static Analysis Results Interchange Format)](https://sarifweb.azurewebsites.net/) standard.

### Proposal

Add a `--output-sarif` / `-sr` CLI flag (and `output-sarif` config key) that writes results to a `complexipy-results.sarif` file. Each function exceeding the complexity threshold becomes a SARIF `result` with:

- `ruleId`: `"complexipy/C001"`
- `level`: `"warning"` (or `"error"` when above threshold)
- `message`: human-readable complexity score with refactoring guidance
- `locations`: exact file path, `line_start`, and `line_end`

### Value

- Native GitHub Code Scanning integration (upload SARIF via `github/codeql-action/upload-sarif`)
- Works with GitLab SAST, Azure DevOps, and any SARIF-aware tool
- Positions complexipy as a first-class static analysis tool, not just a reporter

### Example

```yaml
# .github/workflows/complexity.yml
- name: Run complexipy
  run: complexipy . --output-sarif

- name: Upload SARIF
  uses: github/codeql-action/upload-sarif@v3
  with:
    sarif_file: complexipy-results.sarif
```

Inline annotations would appear directly in GitHub pull request diffs.

---

## 2. Comprehension Complexity Tracking

**Priority: High | Effort: Medium**

### Problem

Python list/dict/set comprehensions and generator expressions are syntactically compact, but deeply nested comprehensions with multiple `for`-clauses and `if`-filters are among the hardest constructs to read. The current algorithm handles control flow in statements but does not score comprehension complexity distinctly.

**Example of hard-to-read code currently scored as 0:**

```python
result = [
    transform(x)
    for x in outer
    if x.is_valid()
    for y in x.children
    if y.active and y.type == "leaf"
]
```

### Proposal

Extend the complexity algorithm in `cognitive_complexity/mod.rs` to score comprehension expressions:

| Construct | Score |
|-----------|-------|
| First `for` clause in comprehension | `+0` (base, expected) |
| Each additional `for` clause | `+1 + nesting_level` |
| Each `if` filter in a comprehension | `+1 + nesting_level` |
| Nested comprehension (comprehension inside comprehension) | `+1 + nesting_level` extra |

This follows the spirit of the cognitive complexity specification, where "breaks in linear flow" and nesting both carry cost.

### Value

- Catches legitimately complex comprehensions that developers often overlook
- More accurate scores for data-heavy Python code (data science, ETL pipelines)
- Aligns with the project's stated goal of measuring human cognitive load

---

## 3. Complexity Diff — Git-aware Change Reporting

**Priority: High | Effort: Medium-High**

### Problem

Developers want to know: *"Did my PR make things better or worse?"* Currently, the snapshot mechanism provides a binary pass/fail check against a baseline, but it does not produce a readable diff of what changed.

### Proposal

Add a `--diff` / `-d` flag that compares the current state against either:

1. A named git ref: `complexipy . --diff HEAD~1`
2. The existing snapshot file: `complexipy . --diff snapshot`

Output format (console):

```
complexipy diff report
──────────────────────────────────────────────────
  src/parser.py::parse_expression     14 → 19  (+5) ✗ REGRESSED
  src/utils.py::flatten_tree          22 → 17  (-5) ✓ IMPROVED
  src/auth.py::validate_token         8  → 8   (=)  · UNCHANGED (new)
──────────────────────────────────────────────────
Net change: +0 functions above threshold
```

Implementation approach:
- For git refs: check out the ref into a temp directory (reusing the existing git-clone infrastructure already present for remote URL analysis), run analysis, compare
- For snapshot: reuse the existing `load_snapshot_file` / `build_snapshot_map` logic

### Value

- Transforms complexipy from a pass/fail gate into an actionable code review tool
- Developers see concrete impact of their changes
- CI systems can post the diff as a PR comment via `gh pr comment`

---

## 4. HTML Report with Complexity Heat Map

**Priority: Medium | Effort: Medium**

### Problem

The CSV and JSON outputs are machine-readable, but there's no human-friendly report for managers, architects, or team reviews.

### Proposal

Add `--output-html` / `-H` flag that generates a self-contained HTML file with:

- **Summary dashboard**: total functions analyzed, % above threshold, average/median/p95 complexity
- **File tree view**: each file is a row; click to expand function list
- **Heat map coloring**: green (0–7) → yellow (8–14) → orange (15–24) → red (25+)
- **Sorting/filtering**: by file, by complexity, show only failures
- **Trend section** (optional): if a snapshot file exists, show improvement/regression arrows

The HTML should be self-contained (inlined CSS/JS, no external dependencies) so it can be attached to CI artifacts or emailed.

### Value

- Non-developer stakeholders can review complexity without tooling setup
- Great for quarterly code quality reviews
- Can be published as a GitHub Pages artifact in CI

---

## 5. Watch Mode — Real-time Analysis

**Priority: Medium | Effort: Medium**

### Problem

The VS Code extension provides real-time feedback during editing, but developers who prefer terminal-centric workflows or use other editors (Neovim, Emacs, Helix) have no equivalent.

### Proposal

Add a `--watch` / `-w` flag that:

1. Runs the initial analysis
2. Watches the target paths for file changes (using `notify` crate for cross-platform file system events)
3. Re-analyzes only the changed file on each save
4. Prints a compact diff of what changed since last run

```bash
complexipy . --watch --failed
# Watching for changes... (Ctrl+C to stop)
# [14:32:01] src/api.py changed
#   handle_request: 12 → 18 (+6) ✗ EXCEEDED THRESHOLD
```

Editors can integrate this via terminal pane or `efm-langserver`-style generic LSP wrappers.

### Value

- Editor-agnostic real-time feedback
- Particularly valuable for refactoring sessions
- Low implementation cost by reusing existing analysis path

---

## 6. Class-level Aggregated Complexity

**Priority: Medium | Effort: Low**

### Problem

The current output is function-centric. When reviewing a class with 15 methods, it's hard to get a quick read on whether the class as a whole is problematic (possibly violating the Single Responsibility Principle).

### Proposal

Extend `FileComplexity` / output to optionally group results by class:

```
src/services/auth.py
  AuthService (class total: 47)
    ├─ __init__           2
    ├─ authenticate      18  ✗
    ├─ validate_token    15  ✗
    └─ refresh           12
```

Add a `--group-by-class` / `-g` CLI flag. Internally, the Rust AST walker already visits class bodies; this requires tracking the enclosing class name and propagating it through `FunctionComplexity`.

Add `class_name: Option<str>` to `FunctionComplexity` so the Python API consumers can also access this information.

### Value

- Surfaces God Classes that distribute their complexity across many small-but-complex methods
- Better supports object-oriented Python codebases
- Low algorithm risk: purely additive data, no scoring changes

---

## 7. Per-path Complexity Thresholds

**Priority: Medium | Effort: Low**

### Problem

A single `max-complexity-allowed` value for the entire codebase is too blunt. Tests often contain intentionally complex setup code. Legacy modules may have a temporarily higher tolerance while being refactored.

### Proposal

Extend the TOML configuration to support per-path overrides:

```toml
# complexipy.toml
max-complexity-allowed = 10

[[overrides]]
paths = ["tests/"]
max-complexity-allowed = 20

[[overrides]]
paths = ["src/legacy/"]
max-complexity-allowed = 25
snapshot-ignore = true
```

The CLI would not expose this (too verbose for command line), making it a config-file-only feature, consistent with how tools like Ruff and mypy handle per-file overrides.

### Value

- Enables gradual adoption in large codebases without blanket `snapshot-ignore`
- Lets teams set stricter standards for core business logic vs. test helpers
- No user friction: purely additive to existing config schema

---

## 8. Pytest Plugin

**Priority: Medium | Effort: Medium**

### Problem

Pre-commit hooks and GitHub Actions are external gates, but many teams run `pytest` as their primary CI check. There's currently no way to fail a `pytest` run based on complexity.

### Proposal

Publish a separate `pytest-complexipy` package (or add it as an optional extra) that provides a pytest plugin:

```ini
# pytest.ini or pyproject.toml
[tool.pytest.ini_options]
addopts = "--complexipy-max 15 --complexipy-paths src/"
```

The plugin hooks into `pytest_sessionfinish` to analyze complexity after tests pass and fail the session if thresholds are violated, with a summary table in the pytest output.

### Value

- Zero additional CI configuration for teams already running pytest
- Complexity check runs in the same process as tests, no subprocess overhead
- Complexity failures appear in the same output as test failures

---

## 9. Complexity Statistics & Percentile Report

**Priority: Low | Effort: Low**

### Problem

The current output is function-by-function. There's no way to understand the distribution of complexity across a codebase without exporting to CSV and doing manual analysis.

### Proposal

Add a `--stats` / `-S` flag that appends a statistical summary after the function list:

```
─── Complexity Statistics ────────────────────────
  Functions analyzed:     342
  Above threshold (15):   23  (6.7%)
  Mean complexity:        6.4
  Median complexity:      4.0
  95th percentile:        18.0
  Max complexity:         47  (src/parser.py::parse_all)
  Functions with 0:       89  (26.0%)
──────────────────────────────────────────────────
```

### Value

- Gives teams a quick health score without reading every row
- Useful for tracking code quality trends over time when combined with `--output-json`
- Very low implementation cost: pure Python post-processing of existing output

---

## 10. SARIF Rule Documentation & `--explain` Flag

**Priority: Low | Effort: Low**

### Problem

When a function exceeds the threshold, developers know *that* it's complex but may not know *which specific lines* are driving the score. The `line_complexities` data is already computed and available in the API but is not exposed in the default CLI output.

### Proposal

Add an `--explain` / `-x` flag that, for each function above the threshold, prints a line-by-line breakdown:

```
src/auth.py::authenticate  complexity: 18  ✗
  Line 42:  if token and not expired:           +2  (bool op + nesting 1)
  Line 45:    for scope in required_scopes:     +2  (loop + nesting 1)
  Line 46:      if scope not in user.scopes:    +3  (if + nesting 2)
  Line 48:        if strict_mode:               +4  (if + nesting 3)
```

The `line_complexities: List[LineComplexity]` data is already populated in the Rust backend; this is purely a Python-side formatting change in `complexipy/utils/output.py`.

### Value

- Turns "your function is too complex" into actionable, line-specific guidance
- Developers immediately know which nested block to extract into a helper
- Zero algorithm changes required; the data already exists

---

## Implementation Priority Summary

| # | Feature | Priority | Effort | Algorithm Change |
|---|---------|----------|--------|-----------------|
| 1 | SARIF output | High | Medium | No |
| 2 | Comprehension complexity | High | Medium | Yes |
| 3 | Complexity diff (`--diff`) | High | Medium-High | No |
| 4 | HTML heat map report | Medium | Medium | No |
| 5 | Watch mode | Medium | Medium | No |
| 6 | Class-level aggregation | Medium | Low | No |
| 7 | Per-path thresholds | Medium | Low | No |
| 8 | Pytest plugin | Medium | Medium | No |
| 9 | Complexity statistics | Low | Low | No |
| 10 | Line-by-line `--explain` | Low | Low | No |

Features 1, 3, 6, 9, and 10 require no changes to the core Rust complexity algorithm and are the lowest-risk starting points. Feature 2 (comprehension complexity) is the most algorithmically interesting and would meaningfully improve score accuracy for data-heavy Python code.
