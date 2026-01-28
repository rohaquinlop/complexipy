# Usage Guide

This guide covers everything you need to know to effectively use complexipy in your Python projects.

## Installation

=== "pip"
    ```bash
    pip install complexipy
    ```

=== "uv"
    ```bash
    uv add complexipy
    ```

=== "poetry"
    ```bash
    poetry add complexipy
    ```

## Command Line Usage

### Basic Analysis

Analyze your entire project:

```bash
complexipy .
```

Analyze specific files or directories:

```bash
complexipy src/
complexipy src/main.py
complexipy src/ tests/
```

### Setting Complexity Threshold

The default threshold is 15. Functions exceeding this value will be highlighted:

```bash
complexipy . --max-complexity-allowed 10
```

### Filtering Results

Show only functions that exceed the threshold:

```bash
complexipy . --failed
```

Suppress all output (useful for CI pipelines):

```bash
complexipy . --quiet
```

### Sorting Results

Sort by complexity score:

```bash
complexipy . --sort asc   # Ascending (default)
complexipy . --sort desc  # Descending
complexipy . --sort name  # Alphabetically by function name
```

### Excluding Files and Directories

Exclude specific paths from analysis:

```bash
# Exclude a directory
complexipy . --exclude tests

# Exclude multiple paths
complexipy . --exclude tests --exclude migrations --exclude build

# Exclude specific files
complexipy . --exclude src/legacy/old_code.py
```

!!! note "How exclusion works"
    - Entries resolve to existing directories (prefix match) or files (exact match)
    - Non-existent entries are silently ignored
    - Paths are relative to each provided root path

### Output Formats

Save results to JSON or CSV:

```bash
# JSON output (saved to complexipy-results.json)
complexipy . --output-json

# CSV output (saved to complexipy-results.csv)
complexipy . --output-csv

# Both
complexipy . --output-json --output-csv
```

**JSON Output Structure:**
```json
{
  "files": [
    {
      "path": "src/main.py",
      "complexity": 42,
      "functions": [
        {
          "name": "process_data",
          "complexity": 18,
          "line_start": 10,
          "line_end": 45
        }
      ]
    }
  ],
  "total_complexity": 42
}
```

### Color Output

Control color output:

```bash
complexipy . --color auto  # Default: auto-detect terminal support
complexipy . --color yes   # Force colors
complexipy . --color no    # Disable colors
```

## Configuration Files

### Configuration Priority

complexipy loads configuration in this order (highest to lowest priority):

1. Command-line arguments
2. `complexipy.toml`
3. `.complexipy.toml`
4. `pyproject.toml` (under `[tool.complexipy]`)

### Example Configurations

=== "complexipy.toml"
    ```toml
    paths = ["src", "tests"]
    max-complexity-allowed = 10
    exclude = ["migrations", "build"]
    snapshot-create = false
    snapshot-ignore = false
    quiet = false
    ignore-complexity = false
    failed = false
    color = "auto"
    sort = "asc"
    output-csv = false
    output-json = false
    ```

=== "pyproject.toml"
    ```toml
    [tool.complexipy]
    paths = ["src", "tests"]
    max-complexity-allowed = 10
    exclude = ["migrations", "build"]
    failed = true
    sort = "desc"
    ```

=== ".complexipy.toml"
    ```toml
    # Hidden config file for team-specific settings
    max-complexity-allowed = 15
    exclude = ["venv", ".venv", "node_modules"]
    ```

## Python API

### Analyzing Files

```python
from complexipy import file_complexity

# Analyze a file
result = file_complexity("src/main.py")

print(f"Total complexity: {result.complexity}")
print(f"File path: {result.path}")

# Iterate over functions
for func in result.functions:
    print(f"{func.name}:")
    print(f"  Complexity: {func.complexity}")
    print(f"  Lines: {func.line_start}-{func.line_end}")
```

### Analyzing Code Strings

```python
from complexipy import code_complexity

# Analyze code snippet
code = """
def calculate_discount(price, customer):
    if customer.is_premium:
        if price > 100:
            return price * 0.8
        else:
            return price * 0.9
    return price
"""

result = code_complexity(code)
print(f"Complexity: {result.complexity}")

for func in result.functions:
    print(f"{func.name}: {func.complexity}")
```

### Practical API Usage

**Example: Pre-commit Hook**

```python
#!/usr/bin/env python3
"""Check complexity of staged Python files."""
import sys
from pathlib import Path
from complexipy import file_complexity

MAX_COMPLEXITY = 15

def main():
    # Get staged Python files (integrate with git)
    staged_files = get_staged_python_files()

    violations = []
    for filepath in staged_files:
        result = file_complexity(str(filepath))

        for func in result.functions:
            if func.complexity > MAX_COMPLEXITY:
                violations.append({
                    'file': filepath,
                    'function': func.name,
                    'complexity': func.complexity,
                    'line': func.line_start
                })

    if violations:
        print("Complexity violations found:")
        for v in violations:
            print(f"  {v['file']}:{v['line']} - "
                  f"{v['function']} (complexity: {v['complexity']})")
        sys.exit(1)

    print("All functions pass complexity check!")
    sys.exit(0)

if __name__ == "__main__":
    main()
```

**Example: Code Quality Dashboard**

```python
from pathlib import Path
from complexipy import file_complexity
import json

def analyze_project(root_path: str):
    """Generate complexity report for entire project."""
    project = Path(root_path)
    results = []

    for py_file in project.rglob("*.py"):
        if "venv" in str(py_file) or ".venv" in str(py_file):
            continue

        try:
            result = file_complexity(str(py_file))
            results.append({
                'file': str(py_file),
                'complexity': result.complexity,
                'functions': [
                    {
                        'name': f.name,
                        'complexity': f.complexity,
                        'line_start': f.line_start,
                        'line_end': f.line_end
                    }
                    for f in result.functions
                ]
            })
        except Exception as e:
            print(f"Error analyzing {py_file}: {e}")

    # Sort by complexity
    results.sort(key=lambda x: x['complexity'], reverse=True)

    # Save report
    with open("complexity-report.json", "w") as f:
        json.dump(results, f, indent=2)

    # Print summary
    total_files = len(results)
    total_complexity = sum(r['complexity'] for r in results)
    avg_complexity = total_complexity / total_files if total_files else 0

    print(f"Analyzed {total_files} files")
    print(f"Total complexity: {total_complexity}")
    print(f"Average complexity: {avg_complexity:.2f}")

    # Top 10 most complex files
    print("\nTop 10 most complex files:")
    for r in results[:10]:
        print(f"  {r['file']}: {r['complexity']}")

if __name__ == "__main__":
    analyze_project("./src")
```

## Snapshot Baselines

Snapshots allow you to adopt complexipy gradually in large, existing codebases.

### Creating a Snapshot

```bash
complexipy . --snapshot-create --max-complexity-allowed 15
```

This creates `complexipy-snapshot.json` in your working directory, recording all functions that currently exceed the threshold.

### How Snapshots Work

Once a snapshot exists, complexipy will:

- ✅ **Pass**: Functions that were already in the snapshot and haven't gotten worse
- ✅ **Pass**: Functions that improved (automatically removed from snapshot)
- ❌ **Fail**: New functions that exceed the threshold
- ❌ **Fail**: Tracked functions that got more complex

### Using Snapshots in CI

```yaml
# .github/workflows/complexity.yml
name: Complexity Check

on: [push, pull_request]

jobs:
  check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install complexipy
        run: pip install complexipy

      - name: Check complexity
        run: complexipy . --max-complexity-allowed 15
```

The snapshot file (`complexipy-snapshot.json`) should be committed to version control.

### Ignoring Snapshots

Temporarily disable snapshot checking:

```bash
complexipy . --snapshot-ignore
```

Use this when:
- Refactoring multiple files at once
- Regenerating the baseline
- Testing different thresholds

### Snapshot File Format

```json
{
  "version": "1.0",
  "threshold": 15,
  "functions": {
    "src/legacy.py::old_function": {
      "complexity": 23,
      "line_start": 10,
      "line_end": 50
    }
  }
}
```

## Inline Ignores

Suppress complexity warnings for specific functions:

```python
def complex_legacy_function():  # noqa: complexipy
    # Complex logic that can't be refactored yet
    pass

# Or with a reason
def another_complex_function():  # noqa: complexipy (technical debt: issue #123)
    pass
```

The ignore comment can also be on the line above:

```python
# noqa: complexipy
def complex_function():
    pass
```

!!! warning "Use Sparingly"
    Inline ignores should be temporary. Document why the complexity is necessary and track technical debt.

## CI/CD Integration

### GitHub Actions

Use the official action:

```yaml
- uses: rohaquinlop/complexipy-action@v2
  with:
    paths: src tests
    max_complexity_allowed: 15
    output_json: true
```

Or run directly:

```yaml
- name: Check complexity
  run: |
    pip install complexipy
    complexipy . --max-complexity-allowed 15
```

### Pre-commit Hook

Add to `.pre-commit-config.yaml`:

```yaml
repos:
  - repo: https://github.com/rohaquinlop/complexipy-pre-commit
    rev: v4.2.0
    hooks:
      - id: complexipy
        args: [--max-complexity-allowed=15]
```

### GitLab CI

```yaml
complexity:
  image: python:3.11
  script:
    - pip install complexipy
    - complexipy . --max-complexity-allowed 15
  only:
    - merge_requests
    - main
```

## VS Code Integration

Install the [complexipy extension](https://marketplace.visualstudio.com/items?itemName=rohaquinlop.complexipy) for real-time complexity analysis:

- Inline complexity scores
- Hover tooltips with details
- Color-coded indicators
- Quick-fix suggestions

## Tips and Best Practices

### 1. Start with High Thresholds

When introducing complexipy to an existing codebase:

```bash
# Create baseline
complexipy . --snapshot-create --max-complexity-allowed 25

# Gradually lower threshold over time
complexipy . --max-complexity-allowed 20
complexipy . --max-complexity-allowed 15
```

### 2. Focus on High-Traffic Code

Not all complex code needs immediate refactoring:

```bash
# Focus on frequently changed files
complexipy src/core/ --max-complexity-allowed 10
complexipy src/legacy/ --max-complexity-allowed 25
```

### 3. Use with Code Reviews

```bash
# Check only files in current branch
git diff --name-only main | grep '.py$' | xargs complexipy
```

### 4. Combine with Test Coverage

High complexity + low coverage = high risk

```bash
# Check coverage for complex functions
pytest --cov=src --cov-report=term-missing
complexipy src/ --failed
```

### 5. Track Trends

```bash
# Generate historical data
complexipy . --output-json
# Commit complexipy-results.json to track changes over time
```

## Troubleshooting

### No Python files found

Ensure you're in the correct directory and your files have `.py` extensions.

### Syntax errors in analyzed files

complexipy requires valid Python syntax. Fix syntax errors first:

```bash
python -m py_compile file.py
```

### Performance issues on large codebases

Exclude unnecessary directories:

```bash
complexipy . --exclude venv --exclude .venv --exclude node_modules
```

### Different results than expected

Check configuration file precedence. Use `--help` to see active configuration:

```bash
complexipy --help
```

## Next Steps

- Read [Understanding Complexity Scores](understanding-scores.md) to interpret results
- See [Comparison with Ruff](comparison-with-ruff.md) for complementary tooling
- Check out [About complexipy](about.md) to learn more about the project
