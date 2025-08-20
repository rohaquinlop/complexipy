# complexipy

<div align="center">
  <img src="docs/img/complexipy_icon.svg" alt="complexipy" width="120" height="120">
  
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

Unlike traditional metrics, cognitive complexity considers the mental effort required to read and comprehend code. It identifies truly complex code that needs refactoring, making it perfect for code reviews and maintaining clean codebases.

**Key benefits:**
- **Human-focused** — Aligns with developer intuition
- **Actionable insights** — Pinpoints hard-to-understand code
- **Better maintenance** — Improves long-term code quality

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
complexipy . --output-json --output-csv
```

### Python API

```python
from complexipy import file_complexity, code_complexity

# Analyze a file
result = file_complexity("app.py")
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

result = code_complexity(snippet)
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
    output_json: true
```

</details>

<details>
<summary><strong>🪝 Pre-commit Hook</strong></summary>

```yaml
repos:
- repo: https://github.com/rohaquinlop/complexipy-pre-commit
  rev: v3.0.0
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
quiet = false
ignore-complexity = false
details = "normal"
sort = "asc"

[output]
csv = true
json = true
```

```toml
# pyproject.toml
[tool.complexipy]
paths = ["src", "tests"]
max-complexity-allowed = 10
quiet = false
ignore-complexity = false
details = "normal"
sort = "asc"

[tool.complexipy.output]
csv = true
json = true
```

### CLI Options

| Flag | Description | Default |
|------|-------------|---------|
| `--max-complexity-allowed` | Complexity threshold | `15` |
| `--output-json` | Save results as JSON | `false` |
| `--output-csv` | Save results as CSV | `false` |
| `--details <normal\|low>` | Output verbosity | `normal` |
| `--sort <asc\|desc\|name>` | Sort results | `asc` |
| `--quiet` | Suppress output | `false` |
| `--ignore-complexity` | Don't exit with error on threshold breach | `false` |

## API Reference

```python
# Core functions
file_complexity(path: str) -> FileComplexity
code_complexity(source: str) -> CodeComplexity

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

**[Documentation](https://rohaquinlop.github.io/complexipy/) • [PyPI](https://pypi.org/project/complexipy/) • [GitHub](https://github.com/rohaquinlop/complexipy)**

<sub>Built with ❤️ by <a href="https://github.com/rohaquinlop">@rohaquinlop</a> and <a href="https://github.com/rohaquinlop/complexipy/graphs/contributors">contributors</a></sub>

<sub>Inspired by the <a href="https://www.sonarsource.com/resources/cognitive-complexity/">Cognitive Complexity</a> research by SonarSource</sub>

</div>
