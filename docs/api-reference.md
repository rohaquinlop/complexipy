# API Reference

The complete Python API exposed by complexipy. Import everything from the
`complexipy` package. For usage examples, see the
[Python API](usage-guide.md#python-api) section of the usage guide.

```text
# Core functions
file_complexity(path: str, check_script: bool = False, no_ignore: bool = False) -> FileComplexity
code_complexity(source: str, check_script: bool = False, no_ignore: bool = False) -> CodeComplexity
collect_all_ignored_locations(paths: List[str], exclude: List[str] = [], invocation_path: str = "") -> Tuple[List[IgnoredLocation], List[str]]
compute_diff(files: List[FileComplexity], git_ref: str, invocation_path: Optional[str] = None) -> List[DiffEntry]
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
  └─ description: str

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
  ├─ status: DiffStatus (property)
  └─ delta: Optional[int] (property)

DiffStatus: REGRESSED | IMPROVED | UNCHANGED | NEW | REMOVED
```
