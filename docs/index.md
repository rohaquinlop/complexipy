# complexipy

<div align="center">

<a href="https://sonarcloud.io/summary/new_code?id=rohaquinlop_complexipy" target="_blank">
    <img src="https://sonarcloud.io/api/project_badges/measure?project=rohaquinlop_complexipy&metric=alert_status" alt="Quality Gate">
</a>
<a href="https://pypi.org/project/complexipy" target="_blank">
    <img src="https://img.shields.io/pypi/v/complexipy?color=%2334D058&label=pypi%20package" alt="Package version">
</a>
<a href="https://pepy.tech/project/complexipy" target="_blank">
    <img src="https://static.pepy.tech/badge/complexipy" alt="Downloads">
</a>
<a href="https://github.com/rohaquinlop/complexipy/blob/main/LICENSE" target="_blank">
    <img src="https://img.shields.io/github/license/rohaquinlop/complexipy" alt="License">
</a>
<a href="https://github.com/marketplace/actions/complexipy" target="_blank">
    <img src="https://img.shields.io/badge/GitHub%20Actions-complexipy-2088FF?logo=github-actions&logoColor=white" alt="GitHub Actions">
</a>
<a href="https://marketplace.visualstudio.com/items?itemName=rohaquinlop.complexipy" target="_blank">
    <img src="https://img.shields.io/visual-studio-marketplace/v/rohaquinlop.complexipy?color=%2334D058&label=vscode%20extension" alt="VSCode Extension">
</a>

</div>

An extremely fast Python library to calculate the cognitive complexity of Python files, written in Rust.

## What is Cognitive Complexity?

Cognitive Complexity breaks from using mathematical models to assess software
maintainability by combining Cyclomatic Complexity precedents with human
assessment. It yields method complexity scores that align well with how
developers perceive maintainability.

Unlike traditional complexity metrics, cognitive complexity focuses on how difficult code is to *understand* by humans, making it more relevant for maintaining and reviewing code.

**Key benefits:**

- Identifies hard-to-understand code sections
- Helps improve code quality and maintainability
- Provides a more intuitive metric than traditional complexity measures

📄 Read the white paper: [Cognitive Complexity, a new way of measuring understandability](https://www.sonarsource.com/resources/cognitive-complexity/)

## Resources

- **Source Code**: [GitHub Repository](https://github.com/rohaquinlop/complexipy)
- **PyPI Package**: [PyPI - complexipy](https://pypi.org/project/complexipy/)

## Requirements

- Python >= 3.8
- Git (optional) - required only if you want to analyze a git repository

## Installation

```bash
pip install complexipy
```

## Usage

### Command Line Interface

Here are the various ways you can use complexipy:

```shell
# Analyze the current directory and subdirectories
complexipy .

# Analyze a specific directory and subdirectories
complexipy path/to/directory

# Analyze a git repository
complexipy git_repository_url

# Analyze a specific file
complexipy path/to/file.py

# Ignore complexity threshold and show all functions
complexipy path/to/file.py -i

# Output results to a CSV file
complexipy path/to/directory -c

# Output results to a JSON file
complexipy path/to/directory -j

# Show only files exceeding maximum complexity
complexipy path/to/directory -d low

# Disable console output
complexipy path/to/directory -q

# Sort results in descending order
complexipy path/to/directory -s desc
```

### GitHub Action

You can use complexipy as a GitHub Action to automatically check code complexity in your CI/CD pipeline:

```yaml
name: Check Code Complexity
on: [push, pull_request]

jobs:
  complexity:
    runs-on: ubuntu-latest
    steps:
    - uses: actions/checkout@v4
    - name: Check Python Code Complexity
      uses: rohaquinlop/complexipy-action@v1
      with:
        paths: '.'  # Analyze the entire repository
```

#### Action Inputs

| Input       | Description                                                   | Required | Default                 |
| ----------- | ------------------------------------------------------------- | -------- | ----------------------- |
| paths       | Paths to analyze. Can be local paths or a git repository URL. | Yes      | ${{ github.workspace }} |
| output_csv  | Generate results in a CSV file.                               | No       | false                   |
| output_json | Generate results in a JSON file.                              | No       | false                   |
| details     | Output detail level (low or normal).                          | No       | normal                  |
| quiet       | Suppress console output.                                      | No       | false                   |
| sort        | Sort results by complexity (asc, desc, or name).              | No       | asc                     |

#### Examples

Basic Usage:
```yaml
- uses: rohaquinlop/complexipy-action@v1
  with:
    paths: '.'
```

Generate CSV Report:
```yaml
- uses: rohaquinlop/complexipy-action@v1
  with:
    paths: '.'
    output_csv: true
```

Generate JSON Report:
```yaml
- uses: rohaquinlop/complexipy-action@v1
  with:
    paths: '.'
    output_json: true
```

Analyze Specific Directory with Low Detail Output:
```yaml
- uses: rohaquinlop/complexipy-action@v1
  with:
    paths: './src/python'
    details: 'low'
    sort: 'desc'
```

### Pre-commit Hook

You can use complexipy as a pre-commit hook to automatically check code complexity before each commit. This helps maintain code quality by preventing complex code from being committed.

To use complexipy with pre-commit, add the following to your `.pre-commit-config.yaml`:

```yaml
repos:
- repo: https://github.com/rohaquinlop/complexipy-pre-commit
  rev: v3.0.0  # Use the latest version
  hooks:
    - id: complexipy
```

To exclude Jupyter Notebooks from the analysis, you can specify the file types:

```yaml
repos:
- repo: https://github.com/rohaquinlop/complexipy-pre-commit
  rev: v3.0.0
  hooks:
    - id: complexipy
      types_or: [ python, pyi ]  # Only analyze Python files
```

The pre-commit hook will:
- Run automatically before each commit
- Check the cognitive complexity of your Python files
- Prevent commits if any function exceeds the complexity threshold
- Help maintain code quality standards in your repository

### VSCode Extension

You can also use complexipy directly in Visual Studio Code through our official [extension](https://marketplace.visualstudio.com/items?itemName=rohaquinlop.complexipy):

1. Open VS Code
2. Go to the Extensions view (Ctrl+Shift+X / Cmd+Shift+X)
3. Search for "complexipy"
4. Click Install

The extension provides:
- Real-time complexity analysis as you type
- Visual complexity indicators:
  - Function complexity shown with ƒ symbol
  - Line-level complexity shown with + symbol
  - Color-coded indicators:
    - Green: Low complexity (functions < 15, lines ≤ 5)
    - Red: High complexity (functions ≥ 15, lines > 5)
- Automatic updates on:
  - File save
  - Active editor change
  - Text changes

You can also trigger a manual analysis by:
1. Opening the Command Palette (Ctrl+Shift+P / Cmd+Shift+P)
2. Typing "complexipy"
3. Selecting the "complexipy" command

### Options

| Option | Long form             | Description                                        | Default |
| ------ | --------------------- | -------------------------------------------------- | ------- |
| `-c`   | `--output-csv`        | Output results to CSV file                         | False   |
| `-j`   | `--output-json`       | Output results to JSON file                        | False   |
| `-i`   | `--ignore-complexity` | Ignore complexity threshold and show all functions | False   |
| `-d`   | `--details`           | Detail level (normal/low)                          | normal  |
| `-q`   | `--quiet`             | Disable console output                             | False   |
| `-s`   | `--sort`              | Sort order (asc/desc/name)                         | asc     |

**Notes:**

- The program will exit with error code 1 if any function has a cognitive complexity greater than or equal to 15. Use the `-i` option to ignore this behavior.
- CSV output is saved to `complexipy.csv` in the current directory.
- JSON output is saved to `complexipy.json` in the current directory.
- Detail level `low` shows only files/functions exceeding the maximum complexity.
- Sort options:
  - `asc`: Sort by complexity (ascending)
  - `desc`: Sort by complexity (descending)
  - `name`: Sort by name (ascending)

## Python API

The library provides two main functions:

- `complexipy.file_complexity`: Analyze a Python file at a specified path
- `complexipy.code_complexity`: Analyze a string containing Python code

### Example

```python
# Analyze a file
from complexipy import file_complexity
result = file_complexity("path/to/your/file.py")
print(f"File complexity: {result.complexity}")

# Analyze a code snippet
from complexipy import code_complexity
code = """
def example_function(x):
    if x > 0:
        for i in range(x):
            print(i)
"""
result = code_complexity(code)
print(f"Code complexity: {result.complexity}")
```

## Examples

### Analyzing a Python File

Let's analyze this sample Python file:

```python
def a_decorator(a, b):
    def inner(func):
        return func
    return inner

def b_decorator(a, b):
    def inner(func):
        if func:
            return None
        return func
    return inner
```

#### Command Line Output

Running `complexipy path/to/file.py` produces:

```txt
───────────────────────────── 🐙 complexipy 3.0.0 ──────────────────────────────
                                    Summary
      ┏━━━━━━━━━━━━━━━━━━━┳━━━━━━━━━━━━━━━━━━━┳━━━━━━━━━━━━━┳━━━━━━━━━━━━┓
      ┃ Path              ┃ File              ┃ Function    ┃ Complexity ┃
      ┡━━━━━━━━━━━━━━━━━━━╇━━━━━━━━━━━━━━━━━━━╇━━━━━━━━━━━━━╇━━━━━━━━━━━━┩
      │ test_decorator.py │ test_decorator.py │ a_decorator │ 0          │
      ├───────────────────┼───────────────────┼─────────────┼────────────┤
      │ test_decorator.py │ test_decorator.py │ b_decorator │ 1          │
      └───────────────────┴───────────────────┴─────────────┴────────────┘
🧠 Total Cognitive Complexity: 1
1 file analyzed in 0.0092 seconds
────────────────────────── 🎉 Analysis completed! 🎉 ───────────────────────────
```

#### Using the Python API

With `file_complexity`:
```python
>>> from complexipy import file_complexity
>>> fc = file_complexity("path/to/file.py")
>>> fc.complexity
1
```

With `code_complexity`:
```python
>>> from complexipy import code_complexity
>>> snippet = """for x in range(0, 10):
    print(x)
"""
>>> cc = code_complexity(snippet)
>>> cc.complexity
1
```

#### Understanding Complexity Calculation

```python
def a_decorator(a, b): # 0
    def inner(func): # 0
        return func # 0
    return inner # 0

def b_decorator(a, b): # 0
    def inner(func): # 0
        if func: # 1 (nested = 0), total 1
            return None # 0
        return func # 0
    return inner # 0
```

The cognitive complexity of the file is 1, with the function `b_decorator` having a complexity of 1 due to the if statement. This demonstrates how complexipy assesses code according to cognitive complexity principles that focus on how difficult the code is to understand by humans.

### Generating CSV Output

To export results to a CSV file, use the `-o` option:

```bash
complexipy path/to/file.py -o
```

The CSV file will contain:

```csv
Path,File Name,Function Name,Cognitive Complexity
test_decorator.py,test_decorator.py,a_decorator,0
test_decorator.py,test_decorator.py,b_decorator,1
```

This is useful for integrating complexipy into CI/CD pipelines or other automated analysis workflows.

### Analyzing a Directory

To analyze all Python files in a directory and its subdirectories:

```bash
complexipy /path/to/directory
```

### Analyzing a Git Repository

To analyze a remote Git repository:

```bash
# Clone and analyze
complexipy https://github.com/rohaquinlop/complexipy

# With CSV output
complexipy https://github.com/rohaquinlop/complexipy -o
```

## Contributors

<div align="center">
    <a href = "https://github.com/rohaquinlop/complexipy/graphs/contributors">
    <img src = "https://contrib.rocks/image?repo=rohaquinlop/complexipy"/>
    </a>
</div>

Made with [contributors-img](https://contrib.rocks)

## License

This project is licensed under the MIT License - see the [LICENSE](https://github.com/rohaquinlop/complexipy/blob/main/LICENSE) file for details.

## Acknowledgments

- Thanks to G. Ann Campbell for publishing the paper "Cognitive Complexity: A new way to measure understandability".
- This project is inspired by SonarSource's approach to calculating cognitive complexity.

## References

- [Cognitive Complexity](https://www.sonarsource.com/resources/cognitive-complexity/)
