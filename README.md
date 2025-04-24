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

# Set maximum cognitive complexity (default: 15)
complexipy path/to/file.py -c 20

# Disable exit with error (by setting max complexity to 0)
complexipy path/to/directory -c 0

# Output results to a CSV file
complexipy path/to/directory -o

# Show only files exceeding maximum complexity
complexipy path/to/directory -d low

# Disable console output
complexipy path/to/directory -q

# Sort results in descending order
complexipy path/to/directory -s desc
```

### Options

- `-c` or `--max-complexity`: Set the maximum cognitive complexity (default: 15).
  If the cognitive complexity of a file exceeds this value, the program will exit with error code 1.
  Set to 0 to disable this behavior.

- `-o` or `--output`: Output results to a CSV file named `complexipy.csv` in the current directory.

- `-d` or `--details`: Set detail level:
  - `normal` (default): Show all files and functions
  - `low`: Show only files/functions exceeding the maximum complexity

- `-q` or `--quiet`: Disable console output.

- `-s` or `--sort`: Set sort order:
  - `asc` (default): Sort by complexity (ascending)
  - `desc`: Sort by complexity (descending)
  - `name`: Sort by name (ascending)

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
───────────────────────────── 🐙 complexipy 2.1.1 ──────────────────────────────
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
