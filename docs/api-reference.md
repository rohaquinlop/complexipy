# API Reference

The complete Python API exposed by complexipy. Import everything from the
`complexipy` package. For usage examples, see the
[Python API](#python-api) section of the usage guide.

```text
# Core functions
file_complexity(path: str, check_script: bool = False, no_ignore: bool = False, with_plans: bool = True) -> FileComplexity
code_complexity(source: str, check_script: bool = False, no_ignore: bool = False, with_plans: bool = True) -> CodeComplexity
collect_all_ignored_locations(paths: List[str], exclude: List[str], invocation_path: str = ".") -> Tuple[List[IgnoredLocation], List[str]]
collect_removable_ignored_locations(paths: List[str], exclude: List[str], max_complexity_allowed: int, invocation_path: str = ".") -> Tuple[List[RemovableIgnore], List[str]]
compute_diff(current_files: List[FileComplexity], git_ref: str, invocation_path: Optional[str] = None) -> List[DiffEntry]
has_regressions(entries: List[DiffEntry], max_complexity: int) -> bool

# Return types
FileComplexity:
  ├─ path: str (relative to the working directory, or absolute when outside it)
  ├─ file_name: str
  ├─ complexity: int
  └─ functions: List[FunctionComplexity]

FunctionComplexity:
  ├─ name: str
  ├─ complexity: int
  ├─ line_start: int
  ├─ line_end: int
  ├─ line_complexities: List[LineComplexity]
  └─ refactor_plans: List[RefactorPlan]

RefactorPlan:
  ├─ kind: str
  ├─ title: str
  ├─ line_start: int
  ├─ line_end: int
  ├─ column_start: int
  ├─ current_complexity: int
  ├─ estimated_reduction: int
  ├─ estimated_complexity_after: int
  ├─ rule_id: str
  ├─ category: RuleCategory
  ├─ applicability: Applicability
  ├─ description: str
  ├─ explanation: str
  ├─ references: List[str]
  ├─ suggestion: Optional[CodeSuggestion]
  ├─ help: Optional[str]
  └─ doc_url: str

CodeSuggestion:
  ├─ replacement: str
  ├─ applicability: Applicability
  ├─ description: str
  └─ insertions: List[CodeInsertion]

CodeInsertion:
  ├─ before_line: int
  └─ text: str

RuleCategory: Complexity | Readability
Applicability: MachineApplicable | MaybeIncorrect | Informational

LineComplexity:
  ├─ line: int
  └─ complexity: int

IgnoredLocation:
  ├─ path: str
  ├─ line: int
  └─ comment: str

CodeComplexity:
  ├─ complexity: int
  └─ functions: List[FunctionComplexity]

DiffEntry:
  ├─ file_path: str
  ├─ func_name: str
  ├─ old_complexity: Optional[int]
  ├─ new_complexity: Optional[int]
  └─ status: DiffStatus (property)

DiffStatus: REGRESSED | IMPROVED | UNCHANGED | NEW | REMOVED
```

## Errors

`file_complexity` raises `ValueError` when the file cannot be read or decoded as UTF-8, or does not parse. `code_complexity` raises `ValueError` when the code does not parse. An internal error inside any analysis function also surfaces as `ValueError`, so one `except ValueError` skips a file that cannot be analyzed.

## Python API

### Analyzing Files

```python
from complexipy import file_complexity

# Analyze a file
result = file_complexity("src/main.py", check_script=True)

print(f"Total complexity: {result.complexity}")

# Analyze without honoring inline ignore comments
result = file_complexity("src/main.py", no_ignore=True)
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

result = code_complexity(code, check_script=True)
print(f"Complexity: {result.complexity}")

# Analyze code string without honoring ignore comments
result = code_complexity(code, no_ignore=True)

for func in result.functions:
    print(f"{func.name}: {func.complexity}")
```

Both functions build refactor plans by default. Pass `with_plans=False` when
you only need the scores: `refactor_plans` stays empty and the analysis skips
the plan work.

```python
result = file_complexity("src/main.py", with_plans=False)
```

### Comparing Against a Git Reference

`compute_diff` compares current complexity results against a git reference
(commit, tag, or branch) and returns `DiffEntry` objects - one per function
that changed, appeared, or disappeared. `has_regressions` reports whether any
entry breaches a complexity threshold (a REGRESSED or NEW function above
*max_complexity*). The entries carry no function bodies, so a moved or renamed
function counts as NEW; only `--diff`, which compares the bodies, lets an exact
copy through. Each file is read from the git repository that holds it, at its
own path, and a submodule at the commit its outer repository pins, as with
`--diff`, so a file moved since the reference reports its functions as NEW:
its old path is not among the files you pass. Like `--diff`, it needs Git 2.30
or newer on your `PATH`. `invocation_path`, the working directory by default,
anchors relative file paths and plays the role of the directory a CLI run
starts in.

```python
from complexipy import (
    compute_diff,
    has_regressions,
    file_complexity,
    DiffEntry,
    DiffStatus,
)

# Analyze the current state of the files you care about
current = [file_complexity(p) for p in changed_files]

# Compare against a git reference (the working directory is the default cwd)
entries = compute_diff(current, "origin/main")

# Filter for regressions above your threshold
regressions = [
    e
    for e in entries
    if e.status == DiffStatus.REGRESSED and e.new_complexity > 15
]

# Or use the ratchet gate directly (fails on REGRESSED/NEW above threshold)
if has_regressions(entries, 15):
    raise SystemExit("Complexity regressions detected")
```

`DiffEntry` exposes `file_path`, `func_name`, `old_complexity`, and
`new_complexity` (either may be `None` for NEW / REMOVED functions), plus the
`status` property. `status` is a `DiffStatus` member - a
`str`-based enum, so it compares equal to its string value (e.g.
`DiffStatus.REGRESSED == "REGRESSED"`).

```python
for e in entries:
    if e.status != DiffStatus.UNCHANGED:
        print(f"{e.file_path}::{e.func_name}: {e.status} "
              f"{e.old_complexity} -> {e.new_complexity}")
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
