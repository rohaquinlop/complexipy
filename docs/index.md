# complexipy

<div align="center">
  <img src="img/complexipy_icon.svg" alt="complexipy" width="70" height="70">
  
  <p><strong>Blazingly fast cognitive complexity analysis for Python, written in Rust.</strong></p>

  <p>
    <a href="https://pypi.org/project/complexipy"><img src="https://img.shields.io/pypi/v/complexipy?color=blue&style=flat-square" alt="PyPI"></a>
    <a href="https://pepy.tech/project/complexipy"><img src="https://static.pepy.tech/badge/complexipy" alt="Downloads"></a>
    <a href="https://github.com/rohaquinlop/complexipy/blob/main/LICENSE"><img src="https://img.shields.io/github/license/rohaquinlop/complexipy?style=flat-square" alt="License"></a>
  </p>
</div>

## What is Cognitive Complexity?

> **Cognitive complexity measures how hard code is to understand by humans, not machines.**

Unlike traditional complexity metrics that focus on mathematical models, cognitive complexity aligns with human intuition about code difficulty. It helps identify code that truly needs refactoring and improves code review effectiveness.

**Why use complexipy?**

- **Human-focused** — Matches developer perception of difficulty
- **Actionable** — Identifies specific code that needs attention  
- **Practical** — Improves maintainability and code reviews
- **⚡ Fast** — Rust-powered analysis for large codebases

!!! info "Background Research"
    Read the original research: [Cognitive Complexity: A new way of measuring understandability](https://www.sonarsource.com/resources/cognitive-complexity/)

## Installation

Install complexipy using your preferred Python package manager:

=== "uv"

    ```bash
    uv add complexipy
    ```

=== "pip"

    ```bash
    pip install complexipy
    ```

**Requirements:**

- Python 3.8+
- Git (optional, for repository analysis)

## Quick Start

Get started with complexipy in seconds:

### Command Line

```bash
# Analyze current directory
complexipy .

# Analyze specific files or directories
complexipy src/main.py tests/

# Set complexity threshold
complexipy . --max-complexity-allowed 10

# Generate reports
complexipy . --output-json --output-csv
```

### Python API

```python
from complexipy import file_complexity, code_complexity

# Analyze a file
result = file_complexity("app.py")
print(f"Total complexity: {result.complexity}")

# Find complex functions
for func in result.functions:
    if func.complexity > 15:
        print(f"{func.name}: {func.complexity}")

# Analyze code directly
code = """
def complex_function(data):
    for item in data:
        if item.is_valid():
            if item.priority == 'high':
                process_urgent(item)
            else:
                process_normal(item)
"""

result = code_complexity(code)
print(f"Code complexity: {result.complexity}")
```

## CLI Reference

### Commands

```bash
# Basic usage
complexipy <path>                       # Analyze path (file/directory/URL)
complexipy .                            # Current directory
complexipy src/                         # Specific directory  
complexipy app.py                       # Single file
complexipy https://github.com/user/repo # Remote repository
```

### Configuration

complexipy supports configuration via TOML files for consistent settings across your project. Configuration files are loaded in this order of precedence:

1. `complexipy.toml` (project-specific config)
2. `.complexipy.toml` (hidden config file)  
3. `pyproject.toml` (under `[tool.complexipy]` section)

#### Configuration Options

All CLI options can be configured via TOML files:

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

For `pyproject.toml`, use the `[tool.complexipy]` section:

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

!!! tip "Configuration Precedence"
    CLI arguments always override TOML configuration values, allowing for flexible per-run customization.

### Options

| Option | Description | Default |
|--------|-------------|---------|
| `--max-complexity-allowed <N>` | Complexity threshold for warnings | `15` |
| `--output-json` | Export results to JSON | `false` |
| `--output-csv` | Export results to CSV | `false` |
| `--details <normal\|low>` | Output verbosity level | `normal` |
| `--sort <asc\|desc\|name>` | Sort results by complexity or name | `asc` |
| `--quiet` | Suppress terminal output | `false` |
| `--ignore-complexity` | Don't exit with error on threshold breach | `false` |

### Examples

```bash
# Find only high-complexity functions
complexipy . --details low --sort desc

# Generate reports with custom threshold
complexipy . --max-complexity-allowed 8 --output-json

# Silent analysis sorted by name
complexipy src/ --sort name --quiet
```

!!! tip "Exit Codes"
    complexipy returns exit code `1` when functions exceed the complexity threshold. Use `--ignore-complexity` to suppress this behavior in CI/CD pipelines.

## API Reference

### Functions

#### `file_complexity(path: str) -> FileComplexity`

Analyze the cognitive complexity of a Python file.

**Parameters:**
- `path` (str): Path to the Python file to analyze

**Returns:** `FileComplexity` object containing analysis results

#### `code_complexity(source: str) -> CodeComplexity`

Analyze the cognitive complexity of Python source code.

**Parameters:**
- `source` (str): Python source code as a string

**Returns:** `CodeComplexity` object containing analysis results

### Data Types

#### `FileComplexity`

```python
@dataclass
class FileComplexity:
    path: str                           # File path
    file_name: str                      # Filename only
    complexity: int                     # Total file complexity
    functions: List[FunctionComplexity] # Function-level details
```

#### `FunctionComplexity`

```python
@dataclass 
class FunctionComplexity:
    name: str                           # Function name
    complexity: int                     # Function complexity score
    line_start: int                     # Starting line number
    line_end: int                       # Ending line number
    line_complexities: List[LineComplexity] # Line-by-line breakdown
```

#### `LineComplexity`

```python
@dataclass
class LineComplexity:
    line: int                           # Line number
    complexity: int                     # Line complexity contribution
```

### Usage Example

```python
from complexipy import file_complexity

# Analyze a file
result = file_complexity("my_module.py")

print(f"{result.file_name}")
print(f"Total complexity: {result.complexity}")
print()

# Categorize functions by complexity
for func in result.functions:
    if func.complexity > 15:
        print(f"🔴 HIGH: {func.name} (lines {func.line_start}-{func.line_end}): {func.complexity}")
    elif func.complexity > 5:
        print(f"🟡 MEDIUM: {func.name} (lines {func.line_start}-{func.line_end}): {func.complexity}")
    else:
        print(f"🟢 LOW: {func.name} (lines {func.line_start}-{func.line_end}): {func.complexity}")
```

## Integrations

complexipy integrates seamlessly with your development workflow through multiple tools and platforms.

=== "GitHub Actions"

    Automatically check code complexity in your CI/CD pipeline:

    ```yaml title=".github/workflows/complexity.yml"
    name: Code Complexity Check
    on: [push, pull_request]

    jobs:
      complexity:
        runs-on: ubuntu-latest
        steps:
        - uses: actions/checkout@v4
        - uses: rohaquinlop/complexipy-action@v2
          with:
            paths: .
            max_complexity_allowed: 10
            output_json: true
    ```

    **Configuration Options:**

    | Input | Type | Description |
    |-------|------|-------------|
    | `paths` | string | Paths to analyze (required) |
    | `max_complexity_allowed` | number | Complexity threshold |
    | `output_json` | boolean | Generate JSON report |
    | `output_csv` | boolean | Generate CSV report |
    | `details` | normal/low | Output verbosity |

=== "Pre-commit Hook"

    Prevent complex code from being committed:

    ```yaml title=".pre-commit-config.yaml"
    repos:
    - repo: https://github.com/rohaquinlop/complexipy-pre-commit
      rev: v3.0.0
      hooks:
        - id: complexipy
    ```

    **What it does:**
    
    - Automatically runs before each commit
    - Analyzes only changed Python files  
    - Blocks commits that exceed complexity threshold
    - Shows detailed complexity analysis

=== "VS Code Extension"

    Get real-time complexity analysis in your editor:

    Install the [complexipy VS Code extension](https://marketplace.visualstudio.com/items?itemName=rohaquinlop.complexipy) for:

    - **Real-time analysis** as you type
    - **Visual indicators** for functions and lines
    - **Color-coded complexity**:
        - 🟢 Green: Low complexity (≤15 functions, ≤5 lines)
        - 🔴 Red: High complexity (>15 functions, >5 lines)
    - ⚡ **Auto-updates** on file save

    **Usage:** Open Command Palette (`Ctrl+Shift+P`) → Search "complexipy"

## Understanding Cognitive Complexity

Let's walk through an example to understand how cognitive complexity is calculated:

```python title="example.py"
def process_orders(orders):           # Base: 0
    processed = []                    # +0

    for order in orders:              # +1 (loop)
        if order.is_valid():          # +2 (nested condition)
            if order.priority == 'high':  # +3 (deeply nested)
                rush_process(order)   # +0
            elif order.amount > 1000: # +0 (elif at same level)
                bulk_process(order)   # +0
            else:                     # +0
                normal_process(order) # +0

        processed.append(order)       # +0

    return processed                  # +0
    # Total complexity: 6
```

### Complexity Breakdown

1. **`for` loop** (+1): Adds base loop complexity
2. **`if order.is_valid()`** (+2): Nested inside loop  
3. **`if order.priority == 'high'`** (+3): Deeply nested condition

### Running the Analysis

```bash
$ complexipy example.py

───────────────────────────── 🐙 complexipy 4.0.0 ──────────────────────────────
                                    Summary
           ┏━━━━━━━━━━━━┳━━━━━━━━━━━━┳━━━━━━━━━━━━━━━━┳━━━━━━━━━━━━┓
           ┃ Path       ┃ File       ┃ Function       ┃ Complexity ┃
           ┡━━━━━━━━━━━━╇━━━━━━━━━━━━╇━━━━━━━━━━━━━━━━╇━━━━━━━━━━━━┩
           │ example.py │ example.py │ process_orders │ 6          │
           └────────────┴────────────┴────────────────┴────────────┘
🧠 Total Cognitive Complexity: 6
1 file analyzed in 0.0042 seconds
────────────────────────── 🎉 Analysis completed! 🎉 ───────────────────────────
```

!!! tip "Refactoring Guidelines"
    - **Complexity ≤ 5:** Simple, easy to understand
    - **Complexity 6-15:** Moderate, acceptable for most cases
    - **Complexity > 15:** Complex, consider refactoring into smaller functions

## Links & Resources

- **📚 [Research Paper](https://www.sonarsource.com/resources/cognitive-complexity/)** - Original cognitive complexity research by SonarSource
- **🐙 [GitHub Repository](https://github.com/rohaquinlop/complexipy)** - Source code and issue tracking  
- **📦 [PyPI Package](https://pypi.org/project/complexipy/)** - Installation and version history
- **🛠️ [GitHub Action](https://github.com/marketplace/actions/complexipy)** - CI/CD integration
- **🪝 [Pre-commit Hook](https://github.com/rohaquinlop/complexipy-pre-commit)** - Git workflow integration
- **🔌 [VS Code Extension](https://marketplace.visualstudio.com/items?itemName=rohaquinlop.complexipy)** - Editor integration

## Contributing

We welcome contributions! Here's how you can help:

- **Report bugs** - Open an issue with details and reproduction steps
- **Suggest features** - Share your ideas for improvements
- **Improve docs** - Help make the documentation clearer
- **Submit code** - Fix bugs or implement new features

Visit our [GitHub repository](https://github.com/rohaquinlop/complexipy) to get started.

## License

complexipy is released under the [MIT License](https://github.com/rohaquinlop/complexipy/blob/main/LICENSE).

---

<div align="center">
<sub>Built with ❤️ by <a href="https://github.com/rohaquinlop">@rohaquinlop</a> and <a href="https://github.com/rohaquinlop/complexipy/graphs/contributors">contributors</a></sub>
</div>
