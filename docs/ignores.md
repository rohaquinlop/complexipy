# Ignores and Rule Selection

## Rule Selection

Refactoring rules carry ids like `C001` (see
[Refactoring Rules](refactoring-rules.md)). Two flags choose which rules are
active, and both accept comma-separated lists and repeat:

| Flag | Effect |
| -- | -- |
| `--select C001,C007` | Report only the listed rules |
| `--ignore C007` | Never report the listed rules |
| `--select complexity` | Report only the rules of one group |
| `--ignore readability` | Never report the rules of one group |

A group name stands for every rule of that category, as listed in
[Refactoring Rules](refactoring-rules.md#rule-id-reference): `complexity` or
`readability`. Group names mix with rule ids, as in
`--select complexity,C007`.

With no flag every rule is active. When both flags name the same rule,
`--ignore` wins. Rule ids and group names are case-insensitive. An unknown id prints a warning
on stderr that it matches no registered rule. A `select` list keeps narrowing
the active set even when one of its ids is unknown, and a second warning says
so when `select` and `ignore` leave no rule active.

The same keys work in `complexipy.toml`, `.complexipy.toml`, and
`pyproject.toml`:

```toml
select = ["C001", "C007"]
ignore = ["C002"]
```

A toml value may also be one comma-separated string, like the flags:
`select = "C001, C007"`.

A flag replaces the toml key it names. Rule selection never changes the exit
code: the exit code follows the complexity threshold, snapshots, paths, and
diff gating only. A rule that is not active never appears in the terminal, the
refactor plans, JSON, CSV, SARIF, or GitLab output.

## Inline Ignores

Suppress complexity warnings for specific functions using the `# complexipy: ignore` comment:

```python
def complex_legacy_function():  # complexipy: ignore
    # Complex logic that can't be refactored yet
    pass

# Or with a reason
def another_complex_function():  # complexipy: ignore (technical debt: issue #123)
    pass
```

The ignore comment can also be placed on the line above the function definition:

```python
# complexipy: ignore
def complex_function():
    pass
```

A marker counts only inside a comment: on the `def` line, on a line of its
signature or decorators, or as a comment on its own line directly above. The
same text in a string, a docstring, or a decorator argument, or a trailing
comment on the code line above, suppresses nothing. The words are matched
whole and separated by spaces or tabs only, so `# complexipy: ignored` is
not a marker.

### Suppressing Specific Rules

Add a bracketed rule list to keep the function but hide the named rules:

```python
def tangled(a, b):  # complexipy: ignore[C007]
    pass
```

The function is still analyzed and reported; only the listed rules are
suppressed for it. Separate several ids with commas:
`# complexipy: ignore[C001,C007]`. The `# noqa: complexipy[C007]` form behaves
the same way. A bare marker keeps suppressing the whole function. Bracketed
text that is not a list of rule ids is a reason, so
`# complexipy: ignore [technical debt]` keeps suppressing the whole function.
Group names are not rule ids here: `# complexipy: ignore[readability]` also
suppresses the whole function.

!!! note "Deprecated Syntax"

    The `# noqa: complexipy` syntax is deprecated and will be removed in a future version.
    Please migrate to `# complexipy: ignore` instead.

    **Why?** Tools like [yesqa](https://github.com/asottile/yesqa) automatically strip `# noqa`
    comments that aren't recognized by flake8, which would silently remove your complexipy
    suppressions. The new syntax avoids this conflict entirely.

!!! warning "Use Sparingly"

    Inline ignores should be temporary. Document why the complexity is necessary and track technical debt.

### Disabling Inline Ignores

Use `--no-ignore` to disregard all inline ignore comments and analyze every function:

```bash
complexipy . --no-ignore
```

Functions previously suppressed by `# complexipy: ignore` or `# noqa: complexipy` will be analyzed normally and may fail the threshold.

### Reporting Ignored Functions

Use `--report-ignored` to list every location where an ignore comment suppresses a function:

```bash
# List ignored functions
complexipy . --report-ignored

# Combine with --no-ignore to report while analyzing everything
complexipy . --report-ignored --no-ignore
```

Each location prints as `path:line  # comment-text`, followed by the count. When `--output-format json` is also active, ignored locations are exported to `complexipy-ignored.json`. `--quiet` hides the list, and the JSON export is still written. With `--output -`, the file is written to the directory complexipy runs from.

Both flags are also available in the Python API via `no_ignore=True`:

```python
from complexipy import file_complexity, code_complexity

# Analyze without honoring ignore comments
result = file_complexity("app.py", no_ignore=True)
```

To programmatically collect ignored locations, use `collect_all_ignored_locations()`:

```python
from complexipy import collect_all_ignored_locations

locations, failed = collect_all_ignored_locations(
    paths=["src"],
    exclude=["tests/**"],
)
for loc in locations:
    print(f"{loc.path}:{loc.line}  {loc.comment}")
```

### Removing Stale Ignore Comments

An ignore comment is only necessary while the suppressed function's complexity
exceeds `--max-complexity-allowed`. Once the function's complexity drops to or
below the limit, the comment is stale and can be removed. complexipy detects
this automatically on every run and reports the locations to clean up:

```bash
complexipy .
```

Example output:

```text
The following ignore comment(s) are no longer necessary (complexity is within the allowed limit) and can be removed:
src/legacy.py:42  function=parse_legacy_config complexity=8  # complexipy: ignore
```

The report is purely informational: it never affects the exit code, and it is
suppressed under `--quiet`. The same detection is available programmatically
via `collect_removable_ignored_locations()`:

```python
from complexipy import collect_removable_ignored_locations

removable, failed = collect_removable_ignored_locations(
    paths=["src"],
    exclude=["tests/**"],
    max_complexity_allowed=15,
)
for rem in removable:
    print(f"{rem.path}:{rem.line}  function={rem.function} complexity={rem.complexity}")
```
