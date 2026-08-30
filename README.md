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
    <a href="#learn-more">Learn More</a> •
    <a href="https://rohaquinlop.github.io/complexipy/">Documentation</a> •
    <a href="https://rohaquinlop.github.io/complexipy/changelog/">Changelog</a> •
    <a href="https://www.complexipy-teams.com/">Complexipy Teams</a>
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
result = file_complexity("app.py", check_script=True)
print(f"File complexity: {result.complexity}")

for func in result.functions:
    print(f"{func.name}: {func.complexity}")
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
<summary><strong>🪝 Pre-commit Hook</strong></summary>

```yaml
repos:
    - repo: https://github.com/rohaquinlop/complexipy-pre-commit
      rev: v5.1.0
      hooks:
          - id: complexipy
```

</details>

<details>
<summary><strong>🔌 VS Code Extension</strong></summary>

Install from the [marketplace](https://marketplace.visualstudio.com/items?itemName=rohaquinlop.complexipy) for real-time complexity analysis with visual indicators.

</details>

## Learn More

- [Usage Guide](https://rohaquinlop.github.io/complexipy/usage-guide/) - every CLI flag, configuration files, snapshots, complexity diff, and inline ignores
- [API Reference](https://rohaquinlop.github.io/complexipy/api-reference/) - the complete Python API
- [Understanding Scores](https://rohaquinlop.github.io/complexipy/understanding-scores/) - how the scoring algorithm works
- [Comparison with Ruff](https://rohaquinlop.github.io/complexipy/comparison-with-ruff/) - cognitive vs cyclomatic complexity
- [Refactoring Rules](https://rohaquinlop.github.io/complexipy/refactoring-rules/) - the rules behind `--suggest-refactors`
- [Changelog](https://rohaquinlop.github.io/complexipy/changelog/) - what changed in each release

______________________________________________________________________

<div align="center">

<sub>Inspired by the <a href="https://www.sonarsource.com/resources/cognitive-complexity/">Cognitive Complexity</a> research by G. Ann Campbell</sub><br>
<sub>complexipy is an independent project and is not affiliated with or endorsed by SonarSource</sub>

**[Documentation](https://rohaquinlop.github.io/complexipy/) • [PyPI](https://pypi.org/project/complexipy/) • [GitHub](https://github.com/rohaquinlop/complexipy)**

<sub>Built with ❤️ by <a href="https://github.com/rohaquinlop">@rohaquinlop</a> and <a href="https://github.com/rohaquinlop/complexipy/graphs/contributors">contributors</a></sub>

</div>
