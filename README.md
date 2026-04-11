# complexipy

<div align="center">
  <img src="https://raw.githubusercontent.com/rohaquinlop/complexipy/refs/heads/main/docs/img/complexipy_icon.svg" alt="complexipy" width="120" height="120">

  <p><em>Blazingly fast cognitive complexity analysis for Python, written in Rust.</em></p>

  <p>
    <a href="https://pypi.org/project/complexipy"><img src="https://img.shields.io/pypi/v/complexipy?color=blue&style=flat-square" alt="PyPI"></a>
    <a href="https://pepy.tech/project/complexipy"><img src="https://static.pepy.tech/badge/complexipy" alt="Downloads"></a>
    <a href="https://github.com/rohaquinlop/complexipy/blob/main/LICENSE"><img src="https://img.shields.io/github/license/rohaquinlop/complexipy?style=flat-square" alt="License"></a>
  </p>

  <p>
    <a href="#installation">Installation</a> •
    <a href="#quick-start">Quick Start</a> •
    <a href="#integrations">Integrations</a> •
    <a href="https://rohaquinlop.github.io/complexipy/">Documentation</a>
  </p>
</div>

## What is Cognitive Complexity?

> Cognitive complexity measures how hard code is to understand by humans, not machines.

Unlike traditional metrics like cyclomatic complexity, cognitive complexity accounts for nesting depth and control flow patterns that affect human comprehension. Inspired by [G. Ann Campbell's research](https://www.sonarsource.com/resources/cognitive-complexity/) at SonarSource, complexipy provides a fast, accurate implementation for Python.

**Key benefits:**

- **Human-focused** - Penalizes nesting, flow breaks, and human-unfriendly logic
- **Actionable insights** - Identifies genuinely hard-to-maintain code
- **Different from cyclomatic** - Measures readability while cyclomatic measures structural, testing, and branch density

## Common Questions

**[How is complexity calculated?](https://rohaquinlop.github.io/complexipy/understanding-scores/)**
Learn about the scoring algorithm, what each control structure contributes, and how nesting affects the final score.

**[How does this compare to Ruff's PLR0912?](https://rohaquinlop.github.io/complexipy/comparison-with-ruff/)**
Understand the key differences between cyclomatic complexity (Ruff) and cognitive complexity (complexipy), and why you might want to use both.

**[Is this a SonarSource/Sonar product?](https://rohaquinlop.github.io/complexipy/about/)**
No. complexipy is an independent project inspired by G. Ann Campbell's research, but it's not affiliated with or endorsed by SonarSource.

## Installation

```bash
pip install complexipy
# or
uv add complexipy
```

## Quick Start

### Command Line

```bash
# Analyze current directory
complexipy .

# Analyze specific file/directory
complexipy path/to/code.py

# Analyze with custom threshold
complexipy . --max-complexity-allowed 10

# Save results to JSON/CSV
complexipy . --output-format json --output-format csv

# Show the top 5 most complex functions
complexipy . --top 5

# Emit plain text for scripting/AI agents
complexipy . --plain

# Include module-level script complexity as <module>
complexipy . --check-script

# Write a GitLab report to a deterministic path
complexipy . --output-format gitlab --output complexipy-code-quality.json

# Compare complexity against a git reference
complexipy . --diff HEAD~1

# Include module-level script complexity as <module>
complexipy . --check-script

# Analyze current directory while excluding specific files
complexipy . --exclude path/to/exclude.py --exclude path/to/other/exclude.py
```

### Python API

```python
from complexipy import file_complexity, code_complexity

# Analyze a file
result = file_complexity("app.py", check_script=True)
print(f"File complexity: {result.complexity}")

for func in result.functions:
    print(f"{func.name}: {func.complexity}")

# Analyze code string
snippet = """
def complex_function(data):
    if data:
        for item in data:
            if item.is_valid():
                process(item)
"""

result = code_complexity(snippet, check_script=True)
print(f"Complexity: {result.complexity}")
```

## Integrations

<details>
<summary><strong>🔧 GitHub Actions</strong></summary>

```yaml
- uses: rohaquinlop/complexipy-action@v2
  with:
      paths: .
      max_complexity_allowed: 10
      output_format: json
```

</details>

<details>
<summary><strong>🔍 GitHub Code Scanning (SARIF)</strong></summary>

Upload complexity violations as inline PR annotations using SARIF:

```yaml
- name: Run complexipy
  run: complexipy . --output-format sarif --output complexipy-results.sarif --ignore-complexity

- name: Upload SARIF results
  uses: github/codeql-action/upload-sarif@v3
  with:
      sarif_file: complexipy-results.sarif
```

</details>

<details>
<summary><strong>🦊 GitLab Code Quality</strong></summary>

Publish complexity violations as a GitLab Code Quality artifact:

```yaml
complexity:
    image: python:3.11
    script:
        - pip install complexipy
        - complexipy . --output-format gitlab --output complexipy-code-quality.json --ignore-complexity
    artifacts:
        when: always
        reports:
            codequality: complexipy-code-quality.json
```

</details>

<details>
<summary><strong>🪝 Pre-commit Hook</strong></summary>

```yaml
repos:
    - repo: https://github.com/rohaquinlop/complexipy-pre-commit
      rev: v4.2.0
      hooks:
          - id: complexipy
```

</details>

<details>
<summary><strong>🔌 VS Code Extension</strong></summary>

Install from the [marketplace](https://marketplace.visualstudio.com/items?itemName=rohaquinlop.complexipy) for real-time complexity analysis with visual indicators.

</details>

## Configuration

### TOML Configuration Files

complexipy supports configuration via TOML files. Configuration files are loaded in this order of precedence:

1. `complexipy.toml` (project-specific config)
2. `.complexipy.toml` (hidden config file)
3. `pyproject.toml` (under `[tool.complexipy]` section)

#### Example Configuration

```toml
# complexipy.toml or .complexipy.toml
paths = ["src", "tests"]
max-complexity-allowed = 10
snapshot-create = false
snapshot-ignore = false
quiet = false
ignore-complexity = false
failed = false
color = "auto"
sort = "asc"
exclude = []
check-script = false
output-format = ["json", "sarif"]
output = "reports/"
check-script = false
```

```toml
# pyproject.toml
[tool.complexipy]
paths = ["src", "tests"]
max-complexity-allowed = 10
snapshot-create = false
snapshot-ignore = false
quiet = false
ignore-complexity = false
failed = false
color = "auto"
sort = "asc"
exclude = []
check-script = false
output-format = ["json"]
output = "complexipy-results.json"
check-script = false
```

Legacy TOML keys such as `output-json = true` and CLI flags such as
`--output-json` still work for now, but they are deprecated in favor of
`output-format` and `--output-format`.

`check-script` is supported in TOML. `--top` and `--plain` are CLI-only flags.

### CLI Options

| Flag                            | Description                                                                                                                                                      | Default |
| ------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------- |
| `--exclude`                     | Exclude entries relative to each provided path. Entries resolve to existing directories (prefix match) or files (exact match). Non-existent entries are ignored. |         |
| `--max-complexity-allowed`      | Complexity threshold                                                                                                                                             | `15`    |
| `--snapshot-create`             | Save the current violations above the threshold into `complexipy-snapshot.json`                                                                                  | `false` |
| `--snapshot-ignore`             | Skip comparing against the snapshot even if it exists                                                                                                            | `false` |
| `--failed`                      | Show only functions above the complexity threshold                                                                                                               | `false` |
| `--color <auto\|yes\|no>`       | Use color                                                                                                                                                        | `auto`  |
| `--sort <asc\|desc\|file_name>` | Sort results                                                                                                                                               | `asc`   |
| `--quiet`                       | Suppress output                                                                                                                                                  | `false` |
| `--ignore-complexity`           | Don't exit with error on threshold breach                                                                                                                        | `false` |
| `--version`                     | Show installed complexipy version and exit                                                                                                                       | -       |
| `--top <n>`                     | Show only the `n` most complex functions, globally sorted by complexity descending                                                                               | —       |
| `--plain`                       | Emit plain text lines as `<path> <function> <complexity>`. Cannot be combined with `--quiet`                                                                    | `false` |
| `--output-format <format>`      | Select a machine-readable output format. Repeat the flag to request multiple formats (`json`, `csv`, `gitlab`, `sarif`)                                          | —       |
| `--output <path>`               | Write machine-readable output to a file or directory. Use a directory when emitting multiple formats                                                             | —       |
| `--diff <ref>`                  | Show a complexity diff against a git reference (e.g. `HEAD~1`, `main`)                                                                                           | —       |
| `--check-script`                | Report module-level (script) complexity as a synthetic `<module>` entry                                                                                          | `false` |
| `--output-json`                 | Deprecated alias for `--output-format json`                                                                                                                      | `false` |
| `--output-csv`                  | Deprecated alias for `--output-format csv`                                                                                                                       | `false` |
| `--output-gitlab`               | Deprecated alias for `--output-format gitlab`                                                                                                                    | `false` |
| `--output-sarif`                | Deprecated alias for `--output-format sarif`          

Example:

```
# Exclude only top-level 'tests' directory under the provided root
complexipy . --exclude tests
# This will not exclude './complexipy/utils.py' if you pass '--exclude utils' at repo root,
# because there is no './utils' directory or file at that level.
```

### Snapshot Baselines

Use snapshots to adopt complexipy in large, existing codebases without touching every legacy function at once.

```bash
# Record the current state (creates complexipy-snapshot.json in the working directory)
complexipy . --snapshot-create --max-complexity-allowed 15

# Block regressions while allowing previously-recorded functions
complexipy . --max-complexity-allowed 15

# Temporarily skip the snapshot gate
complexipy . --snapshot-ignore
```

The snapshot file only stores functions whose complexity exceeds the configured threshold. When a snapshot file exists, complexipy will automatically:

- fail if a new function crosses the threshold,
- fail if a tracked function becomes more complex, and
- pass (and update the snapshot) when everything is stable or improved, automatically removing entries that now meet the standard.

Use `--snapshot-ignore` if you need to temporarily bypass the snapshot gate (for example during a refactor or while regenerating the baseline).

### Complexity Diff

Compare complexity against any git reference to see at a glance whether a branch or commit made things better or worse:

```bash
# Compare the working tree against the previous commit
complexipy . --diff HEAD~1

# Compare against a named branch
complexipy . --diff main

# Combine with other flags
complexipy src/ --max-complexity-allowed 10 --diff HEAD~1
```

Sample output:

```
Complexity diff  (vs HEAD~1)
────────────────────────────────────────────────────────────────────────
  REGRESSED   src/api.py::handle_request                        12 → 19  (+7)
  IMPROVED    src/utils.py::flatten_tree                        22 → 14  (-8)
  NEW         src/auth.py::validate_token                       17        (new)
────────────────────────────────────────────────────────────────────────
Net: 1 regressed, 1 improved, 1 new
```

The diff is appended after the normal analysis output and does not affect the exit code. Requires `git` to be available and the analysed paths to be inside a git repository.

### Script Complexity

Use `--check-script` when you also want to score module-level control flow, not just functions:

```bash
# Report top-level script logic as <module>
complexipy scripts/bootstrap.py --check-script
```

The same capability is available in the Python API via `check_script=True` on both `file_complexity()` and `code_complexity()`.

### Inline Ignores

You can explicitly ignore a known complex function inline, similar to Ruff's `C901` ignores:

```python
def legacy_adapter(x, y):  # complexipy: ignore
    if x and y:
        return x + y
    return 0
```

Place `# complexipy: ignore` on the function definition line (or the line immediately above). An optional reason can be provided in parentheses or plain text, it’s ignored by the parser.

## API Reference

```python
# Core functions
file_complexity(path: str, check_script: bool = False) -> FileComplexity
code_complexity(source: str, check_script: bool = False) -> CodeComplexity

# Return types
FileComplexity:
  ├─ path: str
  ├─ complexity: int
  └─ functions: List[FunctionComplexity]

FunctionComplexity:
  ├─ name: str
  ├─ complexity: int
  ├─ line_start: int
  └─ line_end: int
```

---

<div align="center">

<sub>Inspired by the <a href="https://www.sonarsource.com/resources/cognitive-complexity/">Cognitive Complexity</a> research by G. Ann Campbell</sub><br>
<sub>complexipy is an independent project and is not affiliated with or endorsed by SonarSource</sub>

**[Documentation](https://rohaquinlop.github.io/complexipy/) • [PyPI](https://pypi.org/project/complexipy/) • [GitHub](https://github.com/rohaquinlop/complexipy)**

<sub>Built with ❤️ by <a href="https://github.com/rohaquinlop">@rohaquinlop</a> and <a href="https://github.com/rohaquinlop/complexipy/graphs/contributors">contributors</a></sub>

</div>
