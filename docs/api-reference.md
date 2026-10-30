# API Reference

The complete Python API exposed by complexipy. Import everything from the
`complexipy` package. For usage examples, see the
[Python API](usage-guide.md#python-api) section of the usage guide.

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
