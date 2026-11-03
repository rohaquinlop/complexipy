# Usage Guide

This guide covers everything you need to know to effectively use complexipy in your Python projects.

!!! info "Moved sections"

    Some sections of this guide now have their own pages:

    - Configuration files and the cache: [Configuration](configuration.md)
    - Complexity diff and snapshot baselines: [Diff and Snapshots](diff-and-snapshots.md)
    - Rule selection and inline ignores: [Ignores and Rule Selection](ignores.md)
    - Refactor suggestions and `--fix`: [Refactoring Rules](refactoring-rules.md#using-refactoring-rules)
    - CI/CD integration: [CI/CD Integration](ci.md)
    - Every CLI option: [CLI Options](cli-reference.md)
    - Python API examples: [API Reference](api-reference.md#python-api)
    - VS Code: [Editor Integration](editors.md)

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

The `--diff`, `--diff-only`, and `--staged` options require Git 2.30 or newer
on your `PATH`. Check your version with `git --version`. Git is not required
for analysis without these options.

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

`--quiet` hides the report only. Every warning still prints to stderr, so a CI
log explains an unexpected result.

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
    - A file passed explicitly as an argument is analyzed even when a pattern matches it. With `--force-exclude`, or `force-exclude = true` in the configuration, patterns also apply to file arguments, relative to the directory the command runs in only, not to the file's own directory. Gitignore rules and hidden paths never apply to file arguments
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

### Plain Output

Use plain output when you need one machine-friendly line per function:

```bash
complexipy . --plain
complexipy . --plain --top 5
complexipy . --plain --failed --max-complexity-allowed 10
```

Each line is emitted as:

```text
<path>:<line> <function> <complexity>
```

`<line>` is the line where the function starts, so editors and terminals can link to it. With `--suggest-refactors`, each plan adds an indented line with its rule id, title, and reduction. A `~` marks an estimated reduction, and `complexipy --explain <rule>` prints the full rule text:

```text
src/main.py:1 nested 6
  C007 Merge 3 nested if statements (-4)
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

### Color Output

Control color output:

```bash
complexipy . --color auto  # Default: auto-detect terminal support
complexipy . --color yes   # Force colors
complexipy . --color no    # Disable colors
```

`auto` styles a stream only when it is a terminal and `NO_COLOR` is unset or empty. `--color` covers every styled line, including the `--fix` diff and the warnings on stderr, and `--plain` output is never styled.

Text that comes from the analyzed files is printed with its control characters escaped: a path, a source line, or a configuration value shows an escape character as `\u{1b}` and a line break as `\u{a}`, so it cannot move the cursor, rewrite earlier lines, set the window title, or print a hidden link.

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
