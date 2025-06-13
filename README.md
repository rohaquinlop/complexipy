# complexipy

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


An extremely fast Python library to calculate the cognitive complexity of Python files, written in Rust.

## Table of Contents
- [complexipy](#complexipy)
  - [Table of Contents](#table-of-contents)
  - [What is Cognitive Complexity?](#what-is-cognitive-complexity)
  - [Documentation](#documentation)
  - [Requirements](#requirements)
  - [Installation](#installation)
  - [Usage](#usage)
    - [Command Line Interface](#command-line-interface)
    - [GitHub Action](#github-action)
      - [Action Inputs](#action-inputs)
      - [Examples](#examples)
    - [VSCode Extension](#vscode-extension)
    - [Options](#options)
  - [Use the library from python code](#use-the-library-from-python-code)
    - [Example](#example)
  - [Example](#example-1)
    - [Analyzing a file](#analyzing-a-file)
      - [From the CLI](#from-the-cli)
      - [Using the library](#using-the-library)
      - [Understanding the Analysis](#understanding-the-analysis)
    - [Output to a CSV file](#output-to-a-csv-file)
    - [Analyzing a directory](#analyzing-a-directory)
    - [Analyzing a git repository](#analyzing-a-git-repository)
  - [Contributors](#contributors)
  - [License](#license)
  - [Acknowledgments](#acknowledgments)
  - [References](#references)

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

## Documentation

**Documentation**: <a href="https://rohaquinlop.github.io/complexipy/" target="_blank">https://rohaquinlop.github.io/complexipy/</a>

**Source Code**: <a href="https://github.com/rohaquinlop/complexipy" target="_blank">https://github.com/rohaquinlop/complexipy</a>

**PyPI**: <a href="https://pypi.org/project/complexipy/" target="_blank">https://pypi.org/project/complexipy/</a>

## Requirements

- Python >= 3.8
- Git (optional) - required only if you want to analyze a git repository

## Installation

```bash
pip install complexipy
```

## Usage

### Command Line Interface

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
complexipy path/to/directory -o

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

| Input   | Description                                                   | Required | Default                 |
| ------- | ------------------------------------------------------------- | -------- | ----------------------- |
| paths   | Paths to analyze. Can be local paths or a git repository URL. | Yes      | ${{ github.workspace }} |
| output  | Generate results in a CSV file.                               | No       | false                   |
| details | Output detail level (low or normal).                          | No       | normal                  |
| quiet   | Suppress console output.                                      | No       | false                   |
| sort    | Sort results by complexity (asc, desc, or name).              | No       | asc                     |

#### Examples

Basic Usage:
```yaml
- uses: rohaquinlop/complexipy-action@v1
  with:
    paths: '.'
```

Custom Maximum Complexity:
```yaml
- uses: rohaquinlop/complexipy-action@v1
  with:
    paths: './src'
    max_complexity: 20
```

Generate CSV Report:
```yaml
- uses: rohaquinlop/complexipy-action@v1
  with:
    paths: '.'
    output: true
```

Analyze Specific Directory with Low Detail Output:
```yaml
- uses: rohaquinlop/complexipy-action@v1
  with:
    paths: './src/python'
    details: 'low'
    sort: 'desc'
```

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

- `-i` or `--ignore-complexity`: Ignore the complexity threshold and show all functions.

- `-o` or `--output`: Output results to a CSV file named `complexipy.csv` in the current directory.

- `-d` or `--details`: Set detail level:
  - `normal` (default): Show all files and functions
  - `low`: Show only files/functions exceeding the maximum complexity

- `-q` or `--quiet`: Disable console output.

- `-s` or `--sort`: Set sort order:
  - `asc` (default): Sort by complexity (ascending)
  - `desc`: Sort by complexity (descending)
  - `name`: Sort by name (ascending)

**Note:** The program will exit with error code 1 if any function has a cognitive complexity greater than or equal to 15. Use the `-i` option to ignore this behavior.

## Use the library from python code

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

## Example

### Analyzing a file

Given the following Python file:

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

#### From the CLI

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

#### Using the library

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

#### Understanding the Analysis

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

The cognitive complexity of the file is 1, with function `b_decorator` having a complexity of 1 due to the if statement.

### Output to a CSV file

Using the `-o` option outputs results to `complexipy.csv` in the current directory:

```bash
complexipy path/to/file.py -o
```

CSV content:
```csv
Path,File Name,Function Name,Cognitive Complexity
test_decorator.py,test_decorator.py,a_decorator,0
test_decorator.py,test_decorator.py,b_decorator,1
```

### Analyzing a directory

Analyze all Python files in the current directory and subdirectories:

```bash
complexipy .
```

### Analyzing a git repository

Analyze a remote repository:

```bash
# Analyze repository
complexipy https://github.com/rohaquinlop/complexipy

# With CSV output
complexipy https://github.com/rohaquinlop/complexipy -o
```

## Contributors

<p align="center">
    <a href = "https://github.com/rohaquinlop/complexipy/graphs/contributors">
    <img src = "https://contrib.rocks/image?repo=rohaquinlop/complexipy"/>
    </a>
</p>

Made with [contributors-img](https://contrib.rocks)

## License

This project is licensed under the MIT License - see the [LICENSE](https://github.com/rohaquinlop/complexipy/blob/main/LICENSE) file for details.

## Acknowledgments

- Thanks to G. Ann Campbell for publishing the paper "Cognitive Complexity a new way to measure understandability".
- This project is inspired by the Sonar way to calculate cognitive complexity.

## References

- [Cognitive Complexity](https://www.sonarsource.com/resources/cognitive-complexity/)
