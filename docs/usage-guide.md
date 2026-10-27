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

=== "uv tool"

    ```bash
    uv tool install complexipy
    ```

=== "poetry"

    ```bash
    poetry add complexipy
    ```

The `complexipy` command is a native implementation written in Rust; the
whole analysis pipeline runs without a Python interpreter. The same wheel
also provides the Python API for embedding analysis in your own tools.

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

Show only the top N most complex functions across the analyzed files:

```bash
complexipy . --top 10
```

Show deterministic refactor suggestions for failing functions:

```bash
complexipy . --failed --suggest-refactors
```

When `--top` is set, results are globally re-sorted by complexity descending
before truncation.

Suppress analysis output (useful for CI pipelines):

```bash
complexipy . --quiet
```

### Sorting Results

Sort by complexity score:

```bash
complexipy . --sort asc   # Ascending (default)
complexipy . --sort desc  # Descending
complexipy . --sort file_name  # Alphabetically by file name
```

### Excluding Files and Directories

Exclude specific paths from analysis:

```bash
# Exclude a directory recursively
complexipy . --exclude "tests/**"

# Exclude multiple directories recursively
complexipy . --exclude "tests/**" --exclude "migrations/**" --exclude "build/**"

# Exclude specific files
complexipy . --exclude "src/legacy/old_code.py"
```

!!! note "How exclusion works"

    - Exclusions are glob patterns evaluated relative to each provided root path, and relative to the directory the command runs in, where the configuration file lives
    - A pattern that starts with `/`, `./`, or `../`, or ends with `/`, never matches; complexipy warns about it
    - Use `directory/**` to exclude a directory recursively
    - Use an exact relative file path, such as `src/legacy/old_code.py`, to exclude one file
    - Gitignore rules are still respected during file discovery, and so are `.ignore` files and hidden files and directories. `--report-ignored` lists every `*.py` file these rules skip, and a run that selects no file at all prints a warning on stderr, even under `--quiet`
    - A `*.py` symlink is analyzed only when it resolves to a regular file inside the analyzed directory. Links that leave it, dangling links, devices, and directories named `*.py` are skipped with a warning
    - A source file larger than 64 MiB is reported as failed instead of read

### Output Formats

Save results to JSON, CSV, GitLab Code Quality, or SARIF:

```bash
# JSON output (saved to complexipy-results.json)
complexipy . --output-format json

# CSV output (saved to complexipy-results.csv)
complexipy . --output-format csv

# Both
complexipy . --output-format json --output-format csv

# Explicit destination for a single format
complexipy . --output-format gitlab --output complexipy-code-quality.json

# Multiple formats written into a directory
complexipy . --output-format json --output-format sarif --output reports/

# GitLab Code Quality report
complexipy . --output-format gitlab

# SARIF output
complexipy . --output-format sarif

# Add --suggest-refactors to any of the above to also carry refactor rule
# findings (C001, C007, ...) as their own SARIF/GitLab rule IDs, alongside
# the always-present cognitive-complexity findings.
complexipy . --output-format sarif --suggest-refactors
```

JSON, SARIF, and GitLab Code Quality all follow the same rule: refactor plan
data (`refactor_plans` in JSON, per-rule findings in SARIF/GitLab) is only
emitted when `--suggest-refactors` is also passed. Without it, only the
cognitive-complexity threshold findings appear -- matching the CLI's rich
output, which also hides refactor suggestions unless the flag is set.

With `--diff`, `--diff-only`, or `--staged`, every SARIF result, refactor
findings included, carries the `baselineState` of its function against the
reference: `new` for a function the reference does not have, `updated` when
its complexity changed, and `unchanged` when it did not or when the diff
left its file alone. A function the gate pairs with one it replaces is
compared with that function. When the diff decides the exit code, a
complexity result's `level` is `error` for a function that fails the run,
through the gate or the snapshot, and `note` for one the run lets through,
and the GitLab Code Quality report rates the same functions `major` and
`info`. Otherwise the levels stay `warning` and `minor`. Fingerprints do not
change.

### Complexity Diff

Compare current results against any git reference:

```bash
complexipy . --diff HEAD~1
complexipy . --diff main
complexipy src/ --max-complexity-allowed 10 --diff HEAD~1
```

By default `--diff` **enforces** the complexity threshold: the run exits with code `1` only when a change breaches the contract relative to `--max-complexity-allowed`:

- A **new** function introduced above the threshold.
- A **modified** function whose complexity increased **and** ends above the threshold (including already-over functions that get worse).

Functions that regress but stay at or below the threshold (e.g. `3 → 4` with `--max-complexity-allowed 15`) are **not** failures.

To see the diff visually without affecting the exit code, use `--diff-only` instead:

```bash
complexipy . --diff-only HEAD~1
```

To compare the **staged** (git index) content instead of the working tree - "what complexity am I about to commit?" - add `--staged`:

```bash
# Staged changes vs HEAD (the default baseline for --staged)
complexipy . --staged

# Staged changes vs a specific ref
complexipy . --diff main --staged
```

Like `--diff`, `--staged` enforces the threshold against the staged content and fails when a staged change pushes a function above `--max-complexity-allowed`. It compares only the staged files the run selects: the path arguments, `paths`, and `exclude` apply as they do without `--staged`, to the index rather than the working tree, so a staged regular file is compared whatever the working tree holds at its path. Staged deletions that discovery would have selected produce `REMOVED` entries: under a selected directory, outside hidden paths, and not matched by `exclude`, `.gitignore`, `.ignore`, or git's exclude file. Staged additions produce `NEW` entries. Use `--diff-only` with `--staged` for a visual-only view.

A rename or a move passes the gate as long as the function's body stays as it was. A file that git pairs as renamed compares with its old path when the run covers that path; a file moved in from outside the selected paths compares as added, and a file moved out of them reports nothing. A path that was not a regular file at the reference, such as a symbolic link, is not read there: it compares as added, and its deletion reports nothing. A function above the threshold that the change adds passes only when it replaces a `REMOVED` function with the same body and at least its complexity, each removed function replaced once, and the compared files do not hold more functions with that body than they did at the reference. The complexity matters because a call to a function's own name scores as recursion, so a rename can make the same body more complex. The compared files are the ones the diff compares: modified, renamed from a covered path, deleted from a covered path, and added. Bodies compare structurally, ignoring formatting and comments, and leave out the name, the decorators, and the parameters, so a function rename, a class rename, a move to another file, and a method that becomes a function all pass, while a function that moves and changes fails. Functions that an ignore comment hides count on both sides, so a function copied from one that is now hidden fails. Bodies count and pair only within the repository that holds each file, since each repository is compared at its own reference. The diff table names, in a `Gate` column, the `REMOVED` function with the same body that each passing function replaces. A function that an ignore comment now hides was not removed, so it does not report as `REMOVED`. A deleted Python file that discovery would have selected reports its functions as `REMOVED`, so a function moved out of it passes as well.

When `--diff` or `--staged` decides the exit code, the report shows the gate's verdict. A function that fails the run, REGRESSED or NEW above the threshold, is listed as `FAILED` and marked `FAILS` in the diff table's `Gate` column, and the table ends with a `Gate:` line that counts them. A function above the threshold that the gate lets through is listed as `ALLOWED`, and `--failed` lists only the functions that fail the run. A snapshot run lists the functions its snapshot lets through as `ALLOWED` too.

This requires `git` and a repository-backed path. `--diff` and
`--diff-only` read each file from the repository that holds it, so a run
from outside the repository compares against the same history as a run
from inside it. A submodule, meaning a repository that the repository
around it tracks as a gitlink in its index, is read at the commit that the
outer repository pins for it at the reference, and deeper nesting follows
each pin in turn. When the run starts inside the submodule, it is read at
the reference itself. A repository inside another that the outer one does
not track is read at the reference, like any other repository. `--staged`
uses the repository of the working directory; outside a repository it
prints a warning and compares nothing, so the report lists functions above
the threshold as `FAILED` but the threshold does not decide the exit code.
When a file sits outside any repository, the reference names no commit in
the file's repository, or the outer repository pins no commit for the
submodule at the reference, or pins a commit the submodule does not have,
every function of that file reports as `NEW` and the run prints a warning
that names the cause. The functions of a file that does not parse at the
reference report as `NEW` too.

Instead of repeating the reference on every call, declare the comparison
policy once in a configuration file - see [Diff Configuration](#diff-configuration).

### Plain Output

Use plain output when you need one machine-friendly line per function:

```bash
complexipy . --plain
complexipy . --plain --top 5
complexipy . --plain --failed --max-complexity-allowed 10
```

Each line is emitted as:

```text
<path> <function> <complexity>
```

`--plain` is CLI-only and cannot be combined with `--quiet`.

### Module-Level Script Complexity

Use `--check-script` to include module-level code in the results as
`<module>`:

```bash
complexipy path/to/script.py --check-script
complexipy path/to/script.py --check-script --max-complexity-allowed 5
```

This is useful for scripts with complex top-level control flow outside
functions.

Every function a module defines is scored on its own, wherever the `def`
sits: under a module-level `if`, `try`, `with`, `for`, `while`, or `match`,
and in nested classes, whose methods are named `Outer::Inner::method`. A
function defined inside another function counts toward the outer one.

A source file with a PEP 263 coding cookie is read in that encoding: UTF-8
and Latin-1 always, and an ASCII-compatible encoding such as `cp1252` when
the file is pure ASCII. A file that declares any other encoding, such as
`utf-7`, is reported as failed, because reading it as UTF-8 would not show
the code Python runs.

### Refactor Suggestions

Use `--suggest-refactors` to print a small, ranked set of deterministic refactor plans next to rich CLI results:

```bash
complexipy . --failed --suggest-refactors
```

Sample output (abbreviated -- the real output also shows a caret-underlined span, the surrounding source, and a documentation link):

```text
      [1] C007 Merge nested if statements
          --> sample.py:4:9
          Category: ◆ Readability | Applicability: * Safe to apply
          Lines 4-6 -> Estimated reduction: -2 complexity (6 -> 4)

          Suggestion: * Safe to apply
          Merge nested conditions into `if item.active and item.ready:`
```

Plans are based on the Rust AST analysis only; no AI is used and no code is rewritten automatically. Estimated reductions are approximate, ranked, and limited, so treat them as guidance rather than exact future scores. `--plain --suggest-refactors` keeps plain output unchanged.

### Applying Fixable Suggestions

Use `--fix` to write the fixable suggestions to the source files. A suggestion is fixable when its applicability is **Safe to apply** and its replacement is a faithful splice (`"spliceable": true` in the JSON output). That covers C002 (loop guards) and C007 (collapsible if) today. C005 is safe to apply but its replacement contains a placeholder body and a second edit at the call site, so `--fix` skips it.

```bash
complexipy . --fix
```

`--fix` changes only functions whose complexity is above `max-complexity-allowed`, the same rule the gate uses. Every pass checks the threshold again, so a function stops changing once it passes. A function at or under the threshold keeps its suggestions in the report and is never rewritten. One run applies every fixable plan of those functions in every analyzed file and repeats pass by pass, up to 8 passes, until no fixable plan remains; a later pass labels its diff `pass 2:`. For each fixed file the run prints a syntax-highlighted unified diff of the applied changes, with line numbers, one context line per hunk, and the rule that fired, and then a summary of fixed and skipped fixes. The summary shows the reduction per fix as `(-3 complexity)`, with a tilde before the number (`(- ~3 complexity)`) when the number is estimated. Summary lines use the file's line numbers from before the run, in every pass; a fix inside a region that an earlier pass rewrote reports that region's lines. When nothing is fixable the run prints `No fixes to apply.` The report and every gate measure the state after the fixes. Re-analyzing a fixed file shows the finding is gone and the complexity dropped by the plan's measured reduction.

`--fix --dry-run` previews the same changes: it prints only the unified diff and writes nothing, not even the cache or the snapshot. `--fix` runs wherever you are: uncommitted changes, untracked files, and a missing git repository never block it, and no extra flag is needed. A fix keeps the file's line endings and its trailing newline. Per-rule filtering rides on `--select` and `--ignore`.

`--fix` rewrites only regular files inside the directory it runs from. It skips a symlink, and a file that changed on disk after complexipy read it. It writes a new file next to the original and renames it into place, keeping the permission bits and the group but not setuid, setgid, extended attributes, or ACLs.

**JSON Output Structure:**

```json
[
    {
        "path": "src",
        "file_name": "main.py",
        "function_name": "process_data",
        "complexity": 6,
        "refactor_plans": [
            {
                "rule_id": "C007",
                "kind": "collapsible_if",
                "title": "Merge nested if statements",
                "line_start": 4,
                "line_end": 6,
                "column_start": 9,
                "current_complexity": 6,
                "estimated_reduction": 2,
                "estimated_complexity_after": 4,
                "category": "Readability",
                "applicability": "MachineApplicable",
                "description": "Merge nested if statements into a single if with combined conditions",
                "explanation": "Nested if statements with a single body can be merged into a single if with combined conditions using 'and'. This reduces nesting and improves readability.",
                "references": [],
                "suggestion": {
                    "replacement": "        if item.active and item.ready:\n            total += item.value",
                    "applicability": "MachineApplicable",
                    "description": "Merge nested conditions into `if item.active and item.ready:`"
                },
                "help": null,
                "doc_url": "https://complexipy.com/refactoring-rules/#c007-collapsible-if"
            }
        ]
    }
]
```

JSON output contains one entry per emitted function. The `refactor_plans` list is only populated when `--suggest-refactors` is also passed -- otherwise it's `[]`, matching the CLI's rich-output behavior. CSV output does not include plans, and a cell that a spreadsheet would run as a formula (one that starts with `=`, `+`, `-`, or `@`) is written with a leading `'`. Function line ranges are available through the Python API (`line_start`, `line_end`), but are not included in the machine-readable CLI JSON/CSV function entries.

### Color Output

Control color output:

```bash
complexipy . --color auto  # Default: auto-detect terminal support
complexipy . --color yes   # Force colors
complexipy . --color no    # Disable colors
```

`auto` styles a stream only when it is a terminal and `NO_COLOR` is unset or empty. `--color` covers every styled line, including the `--fix` diff and the warnings on stderr, and `--plain` output is never styled.

Text that comes from the analyzed files is printed with its control characters escaped: a path, a source line, or a configuration value shows an escape character as `\u{1b}` and a line break as `\u{a}`, so it cannot move the cursor, rewrite earlier lines, set the window title, or print a hidden link.

## Configuration Files

### Configuration Priority

complexipy loads configuration in this order (highest to lowest priority):

1. Command-line arguments
1. `complexipy.toml`
1. `.complexipy.toml`
1. `pyproject.toml` (under `[tool.complexipy]`)

complexipy reads the first of these files that parses and has valid values,
and never merges files. When a higher file shadows a lower one that also
holds settings, a warning names the file that is not applied.

`--isolated` ignores every configuration file, and `--config <path>` reads
only the named file: its `[tool.complexipy]` section when the file is named
`pyproject.toml`, and the whole file otherwise. Use either one in CI to pin
the gate, so a change to the repository's configuration cannot change it.

A configuration file that is not a regular file, that is larger than 4 MiB,
or that is a symlink resolving outside the directory is skipped with a
warning. A configuration error names the file, the line and column, or the
key, and never repeats the file's contents.

Values read from a configuration file are checked before use: `paths`,
`output`, and `cache-dir` must stay inside the directory complexipy runs
from, symlinks included, and a diff `branch` cannot start with `-`. Pass a
path outside the project on the command line instead.

### Example Configurations

=== "complexipy.toml"

    ```toml
    paths = ["src", "tests"]
    max-complexity-allowed = 10
    exclude = ["migrations/**", "build/**"]
    snapshot-create = false
    snapshot-ignore = false
    quiet = false
    ignore-complexity = false
    failed = false
    color = "auto"
    sort = "asc"
    output-format = ["json", "gitlab"]
    output = "reports/"
    check-script = false
    no-ignore = false
    report-ignored = false
    cache-dir = ".complexipy_cache"
    ```

=== "pyproject.toml"

    ```toml
    [tool.complexipy]
    paths = ["src", "tests"]
    max-complexity-allowed = 10
    exclude = ["migrations/**", "build/**"]
    failed = true
    sort = "desc"
    check-script = true
    cache-dir = ".complexipy_cache"
    ```

=== ".complexipy.toml"

    ```toml
    # Hidden config file for team-specific settings
    max-complexity-allowed = 15
    exclude = ["venv/**", ".venv/**", "node_modules/**"]
    cache-dir = ".complexipy_cache"
    ```

`check-script` is supported in TOML. `--top` and `--plain` are CLI-only flags.

### JSON Schema

`complexipy.schema.json`, at the root of the repository, describes every key
of the configuration, with its type, default, and description. A TOML
language server such as [Taplo](https://taplo.tamasfe.dev/) (Even Better TOML
in VS Code) uses it for completion, hover text, and validation. To use it in
`complexipy.toml` or `.complexipy.toml`, add this line at the top of the file:

```toml
#:schema https://raw.githubusercontent.com/rohaquinlop/complexipy/main/complexipy.schema.json
```

The schema rejects unknown keys, so the editor marks a typo such as
`max-complexity-alowed`. complexipy itself ignores unknown keys.

### Diff Configuration

The comparison policy can be declared once in the repository instead of
passing the same flags on every call. Add a `[tool.complexipy.diff]`
section to the same configuration file:

=== "complexipy.toml"

    ```toml
    [diff]
    branch = "main"
    staged = true
    ```

=== "pyproject.toml"

    ```toml
    [tool.complexipy.diff]
    branch = "main"
    staged = true
    ```

- `branch` sets the default reference for `--diff` and `--diff-only`. A
  plain `complexipy .` then behaves like `complexipy . --diff main`,
  enforcement included. Pass `--diff <ref>` or `--diff-only <ref>` to
  override the reference for a single run.
- `staged` enables staged comparison by default, like passing `--staged`
  on every call.
- CLI flags always take precedence over the section values.
- `branch = ""` disables the diff for the current repository (opt-out),
  and the section's `staged` value with it.
- A `[diff]` section changes what the exit code checks, so every run it
  applies to prints a warning on stderr that names the file. Pass `--diff`,
  `--isolated`, or `--config <path>` to pin the gate in CI.
- Resolution order: CLI flag, then the `diff` section.
- If the configured `branch` does not exist in the local clone (for
  example a fresh or shallow clone), every function reports as `NEW`, the
  enforcement still applies, and the run prints a warning that names the
  reference and the repository. Fetch the branch or pass `--diff <ref>`
  with an existing reference instead.

### Language Server Settings

The [language server](editors.md) reads the same configuration files as the
CLI, from the workspace root, plus an `lsp` section that controls what the
editor shows:

=== "complexipy.toml"

    ```toml
    max-complexity-allowed = 15
    exclude = ["migrations/**"]

    [lsp]
    inlay-hints = "threshold"
    per-line-hints = false
    diagnostics = true
    ```

=== "pyproject.toml"

    ```toml
    [tool.complexipy]
    max-complexity-allowed = 15
    exclude = ["migrations/**"]

    [tool.complexipy.lsp]
    inlay-hints = "threshold"
    per-line-hints = false
    diagnostics = true
    ```

| Key | Description | Default |
| -- | -- | -- |
| `inlay-hints` | `threshold` shows the per-function hint only above `max-complexity-allowed`, `always` shows it for every function, `never` suppresses every hint | `threshold` |
| `per-line-hints` | Also show the complexity added by each line, as `+2`. Lines that add nothing are skipped | `false` |
| `diagnostics` | Publish a warning for every function above `max-complexity-allowed` | `true` |

The server reads the top-level `max-complexity-allowed`, `exclude`, `select`,
`ignore`, and `no-ignore` keys, and it ignores every other key of the same
file: `paths`, `quiet`, `failed`, `sort`, `color`, `output`, `output-format`,
`cache-dir`, `snapshot-create`, `snapshot-ignore`, `ignore-complexity`,
`check-script`, `report-ignored`, and `diff`. A function whose complexity
equals the threshold passes, as it does on the command line.

## CLI Options

| Flag | Description | Default |
| -- | -- | -- |
| `--exclude` | Exclude glob patterns relative to each provided path. Use patterns like `tests/**` for directories or `src/legacy/file.py` for specific files. |  |
| `--select <ids>` | Report only the named refactoring rules, e.g. `--select C001,C007`. Repeatable and comma-separated | all rules |
| `--ignore <ids>` | Never report the named refactoring rules, e.g. `--ignore C007`. Wins over `--select`. Repeatable and comma-separated |  |
| `--max-complexity-allowed` | Complexity threshold | `15` |
| `--snapshot-create` | Save the current violations above the threshold into `complexipy-snapshot.json` | `false` |
| `--snapshot-ignore` | Skip comparing against the snapshot even if it exists | `false` |
| `--failed` | Show only functions above the complexity threshold | `false` |
| `--suggest-refactors` | Show deterministic Rust AST-based refactor plans in rich CLI output. Ignored by `--plain` | `false` |
| `--fix` | Apply every fixable suggestion (Safe to apply and spliceable) of the functions above `max-complexity-allowed` to the source files, then report the post-fix state (see [Applying Fixable Suggestions](#applying-fixable-suggestions)). Implies `--suggest-refactors` | `false` |
| `--dry-run` | Print only the unified diff of the fixes that `--fix` would apply, and write nothing. Implies `--fix` | `false` |
| `--color <auto\|yes\|no>` | Use color | `auto` |
| `--sort <asc\|desc\|file_name>` | Sort results | `asc` |
| `--quiet` | Suppress output | `false` |
| `--ignore-complexity` | Don't exit with error on threshold breach | `false` |
| `--version` | Show installed complexipy version and exit | - |
| `--top <n>` | Show only the `n` most complex functions, globally sorted by complexity descending | - |
| `--plain` | Emit plain text lines as `<path> <function> <complexity>`. Cannot be combined with `--quiet` | `false` |
| `--output-format <format>` | Select a machine-readable output format. Repeat the flag to request multiple formats (`json`, `csv`, `gitlab`, `sarif`) | - |
| `--output <path>` | Write machine-readable output to a file or directory. Use a directory when emitting multiple formats | - |
| `--diff <ref>` | Show a complexity diff against a git reference and enforce the threshold. Fails on regressions above `--max-complexity-allowed` (see [Complexity Diff](#complexity-diff)) | - |
| `--diff-only <ref>` | Show a complexity diff visually without affecting the exit code (see [Complexity Diff](#complexity-diff)) | - |
| `--staged` | Compare staged (git index) changes against the `--diff` ref (default `HEAD`). Answers "what complexity am I about to commit?" (see [Complexity Diff](#complexity-diff)) | `false` |
| `--check-script` | Report module-level (script) complexity as a synthetic `<module>` entry | `false` |
| `--no-ignore` | Analyze every function, disregarding inline ignore comments (`# complexipy: ignore`, `# noqa: complexipy`) | `false` |
| `--report-ignored` | List every file:line where an ignore comment suppresses a function. `--quiet` hides the list | `false` |
| `--isolated` | Ignore every configuration file. Cannot be combined with `--config` | `false` |
| `--config <path>` | Read the configuration from this file only, instead of the invocation directory's files | - |

Boolean flags take an explicit value only in the attached form
`--flag=<true|false>`. A bare flag means `true`, and the following
argument is never consumed as its value.

Example:

```
# Exclude a directory recursively under the provided root
complexipy . --exclude "tests/**"
# Exclude a specific file
complexipy . --exclude "src/legacy/old_code.py"
```

## Cache Configuration

complexipy stores per-function complexity data between runs to power
delta/diff comparisons. By default this cache lives in
`.complexipy_cache/` inside your invocation directory. Instead of the
default location you can redirect it to any directory which is useful
for CI, shared cache directories or avoiding polluting the project
tree.

```bash
# CLI flag (highest priority)
complexipy . --cache-dir .cache/complexipy
```

Or declare it once in a configuration file:

=== "complexipy.toml"

    ```toml
    cache-dir = ".cache/complexipy"
    ```

=== "pyproject.toml"

    ```toml
    cache-dir = ".cache/complexipy"
    ```

=== ".complexipy.toml"

    ```toml
    cache-dir = ".cache/complexipy"
    ```

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
its old path is not among the files you pass. `invocation_path`, the working
directory by default, anchors relative file paths and plays the role of the
directory a CLI run starts in.

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

## Snapshot Baselines

Snapshots allow you to adopt complexipy gradually in large, existing codebases.

### Creating a Snapshot

```bash
complexipy . --snapshot-create --max-complexity-allowed 15
```

This creates `complexipy-snapshot.json` in your working directory, recording all functions that currently exceed the threshold.

Create the baseline from the command line. `snapshot-create = true` in a
configuration file re-records the baseline on every run, so every current
violation passes; such a run prints a warning on stderr that names the file.

### How Snapshots Work

Once a snapshot exists, complexipy will:

- ✅ **Pass**: Functions that were already in the snapshot and haven't gotten worse
- ✅ **Pass**: Functions that improved (automatically removed from snapshot)
- ❌ **Fail**: New functions that exceed the threshold
- ❌ **Fail**: Functions above the threshold that moved to another file or got a new name. The snapshot keys each entry by file and name, so run `--snapshot-create` after such a change to record them again
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
[
    {
        "path": "src/legacy.py",
        "file_name": "legacy.py",
        "functions": [
            {
                "name": "old_function",
                "complexity": 23
            }
        ]
    }
]
```

Snapshots are stored as a JSON array of analyzed files. Each entry contains only functions above the threshold at the time the snapshot was written. The file is rewritten after successful snapshot checks, so improved functions are removed automatically. Updates only touch the files analyzed in the run - entries for files outside the analysis are preserved, so running on a subset of files (for example through a pre-commit hook) never shrinks the baseline. Entries for files that no longer exist are dropped when the file is rewritten. Snapshots created by older complexipy versions may need to be regenerated with `--snapshot-create`.

Functions of one file that share a name, such as a property's getter and setter, get separate entries: the first in source order keeps the name, and the next ones are stored as `name#2`, `name#3`. `--diff` keys its comparison the same way.

## Rule Selection

Refactoring rules carry ids like `C001` (see
[Refactoring Rules](refactoring-rules.md)). Two flags choose which rules are
active, and both accept comma-separated lists and repeat:

| Flag | Effect |
| -- | -- |
| `--select C001,C007` | Report only the listed rules |
| `--ignore C007` | Never report the listed rules |

With no flag every rule is active. When both flags name the same rule,
`--ignore` wins. Rule ids are case-insensitive. An unknown id prints a warning
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

Each location prints as `path:line  # comment-text`, followed by the count. When `--output-format json` is also active, ignored locations are exported to `complexipy-ignored.json`. `--quiet` hides the list, and the JSON export is still written.

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

## CI/CD Integration

### GitHub Actions

Use the official action:

```yaml
- uses: rohaquinlop/complexipy-action@v8
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

### GitHub Code Scanning (SARIF)

Upload complexity violations as inline PR annotations using SARIF:

```yaml
- name: Run complexipy
  run: complexipy . --output-format sarif --output complexipy-results.sarif --ignore-complexity

- name: Upload SARIF results
  uses: github/codeql-action/upload-sarif@v3
  with:
      sarif_file: complexipy-results.sarif
```

### Pre-commit Hook

Add to `.pre-commit-config.yaml`:

```yaml
repos:
    - repo: https://github.com/rohaquinlop/complexipy-pre-commit
      rev: v8.1.0
      hooks:
          - id: complexipy
            args: [--max-complexity-allowed=15]
```

### GitLab CI

```yaml
.complexipy_code_quality:
    image: python:3.11
    script:
        - pip install complexipy
        - complexipy . --output-format gitlab --output complexipy-code-quality.json --ignore-complexity --max-complexity-allowed 15
    artifacts:
        when: always
        reports:
            codequality: complexipy-code-quality.json

complexity:
    extends: .complexipy_code_quality
    rules:
        - if: $CI_PIPELINE_SOURCE == "merge_request_event"
        - if: $CI_COMMIT_BRANCH == $CI_DEFAULT_BRANCH
```

Use `--ignore-complexity` when your main goal is to upload the report even if violations exist. GitLab will still display the findings in the merge request widget and pipeline UI.

If you also want the job to fail when the threshold is exceeded, split the workflow into two jobs:

```yaml
complexity_check:
    image: python:3.11
    script:
        - pip install complexipy
        - complexipy . --max-complexity-allowed 15

complexity_report:
    image: python:3.11
    script:
        - pip install complexipy
        - complexipy . --output-format gitlab --output complexipy-code-quality.json --ignore-complexity --max-complexity-allowed 15
    artifacts:
        when: always
        reports:
            codequality: complexipy-code-quality.json
```

## VS Code Integration

Install the [complexipy extension](https://marketplace.visualstudio.com/items?itemName=rohaquinlop.complexipy) from the Visual Studio Marketplace, or from [OpenVSX](https://open-vsx.org/extension/rohaquinlop/complexipy). The extension is a client of the [language server](editors.md#vs-code) and needs the `complexipy` command installed.

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
complexipy . --output-format json
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
complexipy . --exclude "venv/**" --exclude ".venv/**" --exclude "node_modules/**"
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
