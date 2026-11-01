# complexipy

<div align="center">
  <img src="https://raw.githubusercontent.com/rohaquinlop/complexipy/refs/heads/main/docs/img/complexipy_icon.svg" alt="complexipy" width="120" height="120">

<p><em>Blazingly fast cognitive complexity analysis for Python, written in Rust.</em></p>

<p>Scores all 2,929 files of Django in about 0.2 s on an Apple M1 Pro. <a href="https://complexipy.com/benchmarks/">Benchmarks</a></p>

<p>
    <a href="https://pypi.org/project/complexipy"><img src="https://img.shields.io/pypi/v/complexipy?color=blue&style=flat-square" alt="PyPI"></a>
    <a href="https://pepy.tech/project/complexipy"><img src="https://static.pepy.tech/badge/complexipy" alt="Downloads"></a>
    <a href="https://github.com/rohaquinlop/complexipy/blob/main/LICENSE"><img src="https://img.shields.io/github/license/rohaquinlop/complexipy?style=flat-square" alt="License"></a>
  </p>

<p>
    <a href="#installation">Installation</a> •
    <a href="#quick-start">Quick Start</a> •
    <a href="#features">Features</a> •
    <a href="#integrations">Integrations</a> •
    <a href="#learn-more">Learn More</a> •
    <a href="https://complexipy.com/">Documentation</a> •
    <a href="https://complexipy.com/changelog/">Changelog</a> •
    <a href="https://www.complexipy-teams.com/">Complexipy Teams</a>
  </p>
</div>

<p align="center">
  <img src="https://raw.githubusercontent.com/rohaquinlop/complexipy/refs/heads/main/docs/img/demo.gif" alt="complexipy scores each function in checkout.py, fails apply_discount at 14 against a limit of 10, and suggests extracting a helper function">
</p>

## Installation

```bash
uv tool install complexipy
# or
uv add complexipy
# or
pip install complexipy
```

The `--diff`, `--diff-only`, and `--staged` options require Git 2.30 or newer
on your `PATH`. Check your version with `git --version`. Git is not required
for analysis without these options.

## What is Cognitive Complexity?

> Cognitive complexity measures how hard code is to understand by humans, not machines.

These two functions have the same cyclomatic complexity, 4. Their cognitive complexity is different:

```python
# Cognitive complexity: 3
def example1(a, b, c):
    if a:        # +1
        return 1
    if b:        # +1
        return 2
    if c:        # +1
        return 3
    return 4
```

```python
# Cognitive complexity: 6
def example2(a, b, c):
    if a:        # +1
        if b:    # +2 (1 + nesting_level)
            if c:  # +3 (1 + nesting_level)
                return 1
    return 4
```

Nested code makes the reader hold more context, so each nesting level adds to the score. complexipy implements the [Cognitive Complexity](https://www.sonarsource.com/resources/cognitive-complexity/) metric by G. Ann Campbell for Python. [How scores are calculated](https://complexipy.com/understanding-scores/)

## Quick Start

### Command Line

```bash
# Analyze the current directory
complexipy .

# Set a custom threshold
complexipy . --max-complexity-allowed 10

# Show failing functions with refactor suggestions
complexipy . --failed --suggest-refactors

# Save results to JSON
complexipy . --output-format json

# Block regressions against a git reference
complexipy . --diff main

# Exclude paths with glob patterns
complexipy . --exclude "tests/**"
```

### Python API

```python
from complexipy import file_complexity

# Analyze a file
result = file_complexity("app.py")
print(f"File complexity: {result.complexity}")

for func in result.functions:
    print(f"{func.name}: {func.complexity}")
```

## Features

- **Cognitive complexity scores** that follow G. Ann Campbell's paper. [How scores work](https://complexipy.com/understanding-scores/)
- **Gate only what a change makes worse**: `--diff` and `--staged` compare each function with a git reference. [Complexity diff](https://complexipy.com/diff-and-snapshots/#complexity-diff)
- **Snapshot baselines**: record the current state of an existing codebase, then fail only on new or worse functions. [Snapshots](https://complexipy.com/diff-and-snapshots/#snapshot-baselines)
- **Refactor suggestions and `--fix`**: rules such as guard clauses and extract helper, and automatic fixes where they are safe. [Refactoring rules](https://complexipy.com/refactoring-rules/)
- **Output formats**: JSON, CSV, SARIF for GitHub code scanning, and GitLab Code Quality. [Output formats](https://complexipy.com/usage-guide/#output-formats)
- **Language server**: inlay hints, hover, and warnings in VS Code, Neovim, and Zed. [Editor integration](https://complexipy.com/editors/)
- **GitHub Action and pre-commit hook** for CI. [Integrations](#integrations)

## Common Questions

**[How is complexity calculated?](https://complexipy.com/understanding-scores/)**
Learn about the scoring algorithm, what each control structure contributes, and how nesting affects the final score.

**[How does this compare to Ruff's PLR0912?](https://complexipy.com/comparison-with-ruff/)**
Understand the key differences between cyclomatic complexity (Ruff) and cognitive complexity (complexipy), and why you might want to use both.

**[Is this a SonarSource/Sonar product?](https://complexipy.com/about/)**
No. complexipy is an independent project inspired by G. Ann Campbell's research, but it's not affiliated with or endorsed by SonarSource.

## Integrations

<details>
<summary><strong>🔧 GitHub Actions</strong></summary>

```yaml
- uses: rohaquinlop/complexipy-action@v8
  with:
      paths: .
      max_complexity_allowed: 10
      output_json: true
```

</details>

<details>
<summary><strong>🪝 Pre-commit Hook</strong></summary>

```yaml
repos:
    - repo: https://github.com/rohaquinlop/complexipy-pre-commit
      rev: v8.3.0
      hooks:
          - id: complexipy
```

</details>

<details>
<summary><strong>🧩 Editors</strong></summary>

Every editor uses `complexipy lsp`, a language server with inlay hints, hover, and warnings that update as you type. Install `complexipy` first (`uv tool install complexipy`).

- **VS Code**: install the extension from the [Visual Studio Marketplace](https://marketplace.visualstudio.com/items?itemName=rohaquinlop.complexipy) or [OpenVSX](https://open-vsx.org/extension/rohaquinlop/complexipy).
- **Neovim and Zed**: see [Editor Integration](https://complexipy.com/editors/).

</details>

## Learn More

- [Usage Guide](https://complexipy.com/usage-guide/) - installation, the basic CLI workflow, and troubleshooting
- [Adopting in an Existing Codebase](https://complexipy.com/adoption/) - baseline, gate new code, then lower the threshold
- [Configuration](https://complexipy.com/configuration/) - configuration files and the cache
- [Diff and Snapshots](https://complexipy.com/diff-and-snapshots/) - complexity diff and snapshot baselines
- [Ignores and Rule Selection](https://complexipy.com/ignores/) - inline ignores and rule selection
- [CLI Options](https://complexipy.com/cli-reference/) - every CLI flag
- [Editor Integration](https://complexipy.com/editors/) - the language server, inlay hints, and the VS Code, Neovim, and Zed setup
- [API Reference](https://complexipy.com/api-reference/) - the complete Python API
- [Understanding Scores](https://complexipy.com/understanding-scores/) - how the scoring algorithm works
- [Comparison with Ruff](https://complexipy.com/comparison-with-ruff/) - cognitive vs cyclomatic complexity
- [Refactoring Rules](https://complexipy.com/refactoring-rules/) - the rules behind `--suggest-refactors`
- [Changelog](https://complexipy.com/changelog/) - what changed in each release

______________________________________________________________________

<div align="center">

<sub>Inspired by the <a href="https://www.sonarsource.com/resources/cognitive-complexity/">Cognitive Complexity</a> research by G. Ann Campbell</sub><br>
<sub>complexipy is an independent project and is not affiliated with or endorsed by SonarSource</sub>

**[Documentation](https://complexipy.com/) • [PyPI](https://pypi.org/project/complexipy/) • [GitHub](https://github.com/rohaquinlop/complexipy)**

<sub>Built with ❤️ by <a href="https://github.com/rohaquinlop">@rohaquinlop</a> and <a href="https://github.com/rohaquinlop/complexipy/graphs/contributors">contributors</a></sub>

</div>
