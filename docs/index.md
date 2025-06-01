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

| Option | Long form          | Description                            | Default |
| ------ | ------------------ | -------------------------------------- | ------- |
| `-c`   | `--max-complexity` | Maximum cognitive complexity threshold | 15      |
| `-o`   | `--output`         | Output results to CSV file             | False   |
| `-d`   | `--details`        | Detail level (normal/low)              | normal  |
| `-q`   | `--quiet`          | Disable console output                 | False   |
| `-s`   | `--sort`           | Sort order (asc/desc/name)             | asc     |

**Notes:**

- If a file's complexity exceeds the maximum complexity, the program will exit with error code 1. Set to 0 to disable this behavior.
- CSV output is saved to `complexipy.csv` in the current directory.
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
