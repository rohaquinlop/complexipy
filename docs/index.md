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
    <a href="https://github.com/rohaquinlop/complexipy-pre-commit" target="_blank">
        <img src="https://img.shields.io/badge/pre--commit-complexipy-2088FF?logo=pre-commit&logoColor=white" alt="Pre-commit">
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
# Analyze the current directory (recursively)
complexipy .

# Analyze a specific directory (recursively)
complexipy path/to/directory

# Analyze a remote Git repository
complexipy https://github.com/user/repo.git

# Analyze a single file
complexipy path/to/file.py

# Suppress console output
complexipy path/to/directory --quiet      # or -q

# List every function, ignoring the 15-point complexity threshold
complexipy path/to/file.py --ignore-complexity   # or -i

# Show only files / functions whose complexity exceeds the threshold
complexipy path/to/directory --details low       # or -d low

# Sort results (asc: ascending complexity, desc: descending complexity, name: A→Z)
complexipy path/to/directory --sort desc         # or -s desc

# Save results
complexipy path/to/directory --output-csv        # -c, writes complexipy.csv
complexipy path/to/directory --output-json       # -j, writes complexipy.json
```

#### Command-line options

| Short | Long                     | Parameters | Description                                                                                                                                                     | Default  |
| ----- | ------------------------ | ---------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------- |
| `-c`  | `--output-csv`           | –          | Write the report to `complexipy.csv` in the current working directory.                                                                                          | false    |
| `-j`  | `--output-json`          | –          | Write the report to `complexipy.json` in the current working directory.                                                                                         | false    |
| `-i`  | `--ignore-complexity`    | –          | Do not stop with an error when a function's cognitive complexity is > 15. All functions are still listed in the output.                                         | off      |
| `-d`  | `--details <normal∣low>` | required   | Control the verbosity of the output.<br>• `normal` – show every file and function (default)<br>• `low` – show only entries that exceed the complexity threshold | `normal` |
| `-q`  | `--quiet`                | –          | Suppress console output. Exit codes are still returned.                                                                                                         | false    |
| `-s`  | `--sort <asc∣desc∣name>` | required   | Order the results.<br>• `asc` – complexity ascending (default)<br>• `desc` – complexity descending<br>• `name` – alphabetical A→Z                               | `asc`    |

> **Note**  The CLI exits with code **1** when at least one function exceeds the threshold of **15** points. Pass `--ignore-complexity` (`-i`) to disable this behaviour.

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
      uses: rohaquinlop/complexipy-action@v2
      with:
        paths: .  # Analyze the entire repository
```

#### Action Inputs

| Input             | Type / Allowed Values                 | Required |
| ----------------- | ------------------------------------- | -------- |
| paths             | string (single path or list of paths) | Yes      |
| quiet             | boolean                               | No       |
| ignore_complexity | boolean                               | No       |
| details           | normal, low                           | No       |
| sort              | asc, desc, name                       | No       |
| output_csv        | boolean                               | No       |
| output_json       | boolean                               | No       |

#### Examples

Basic Usage:
```yaml
- uses: rohaquinlop/complexipy-action@v1
  with:
    paths: |
      .
      project_path
```

Generate CSV Report:
```yaml
- uses: rohaquinlop/complexipy-action@v1
  with:
    paths: .
    output_csv: true
```

Generate JSON Report:
```yaml
- uses: rohaquinlop/complexipy-action@v1
  with:
    paths: .
    output_json: true
```

Analyze Specific Directory with Low Detail Output:
```yaml
- uses: rohaquinlop/complexipy-action@v1
  with:
    paths: ./src/python
    details: low
    sort: desc
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
    - Green: Low complexity (functions ≤ 15, lines ≤ 5)
    - Red: High complexity (functions > 15, lines > 5)
- Automatic updates on:
  - File save
  - Active editor change
  - Text changes

You can also trigger a manual analysis by:
1. Opening the Command Palette (Ctrl+Shift+P / Cmd+Shift+P)
2. Typing "complexipy"
3. Selecting the "complexipy" command

## Python API

Complexipy can also be used directly from your Python code. The high-level helper functions below wrap the Rust core and return lightweight Python classes that behave like regular dataclasses.

- `complexipy.file_complexity(path: str) -> FileComplexity` – analyse a Python file on disk.
- `complexipy.code_complexity(src: str) -> CodeComplexity` – analyse a string that contains Python source.

Both helpers return objects whose public attributes you can freely access:

```text
FileComplexity
├─ path: str                   # Relative path of the analysed file
├─ file_name: str              # Filename without the directory part
├─ complexity: int             # Cognitive complexity of the whole file
└─ functions: List[FunctionComplexity]

FunctionComplexity
├─ name: str
├─ complexity: int
├─ line_start: int
├─ line_end: int
└─ line_complexities: List[LineComplexity]

LineComplexity
├─ line: int
└─ complexity: int
```

### Quick-start

```python
from complexipy import file_complexity, code_complexity

# Analyse a file
fc = file_complexity("path/to/your/file.py")
print(f"Total file complexity: {fc.complexity}")

for fn in fc.functions:
    print(f"{fn.name}:{fn.line_start}-{fn.line_end} → {fn.complexity}")

# Analyse an in-memory snippet
snippet = """
def example_function(x):
    if x > 0:
        for i in range(x):
            print(i)
"""
cc = code_complexity(snippet)
print(f"Snippet complexity: {cc.complexity}")
```

## End-to-End Example

The following walk-through shows how to use **Complexipy** from both the **command line** *and* the **Python API**, how to interpret the scores it returns, and how to save them for later use.

### 1.  Prepare a sample file

Create `example.py` with two simple functions:

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

### 2.  Run the CLI

Analyse the file from your terminal:

```bash
complexipy example.py
```

Typical output (shortened):

```text
───────────────────────────── 🐙 complexipy 3.1.0 ──────────────────────────────
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

**What do those columns mean?**

* **Path / File** – location of the analysed source file
* **Function** – function or method name that was measured
* **Complexity** – the cognitive complexity score of that function *(lower is better)*

### 3.  Use the Python API

```python
from complexipy import file_complexity, code_complexity

# Analyse the file on disk
fc = file_complexity("example.py")
print(fc.complexity)   # → 1

# Analyse an in-memory snippet
snippet = "for x in range(10):\n    print(x)"
cc = code_complexity(snippet)
print(cc.complexity)   # → 1
```

### 4.  Why is the score 1?

```python
def b_decorator(a, b):  # 0
  def inner(func):      # 0
    if func:            # +1 – decision point
      return None       # 0
    return func         # 0
  return inner          # 0
```

Only a single `if` branch is encountered, therefore the file's total complexity is **1**.

### 5.  Persisting the results

* **CSV** – `complexipy example.py -c` → creates `complexipy.csv`
* **JSON** – `complexipy example.py -j` → creates `complexipy.json`

### 6.  Scaling up your analysis

* **Entire folder (recursively):** `complexipy .`
* **Specific directory:** `complexipy ~/projects/my_app`
* **Remote Git repository:**
  ```bash
  complexipy https://github.com/rohaquinlop/complexipy          # print to screen
  complexipy https://github.com/rohaquinlop/complexipy -c       # save as CSV
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
