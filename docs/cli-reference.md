# CLI Options

| Flag | Description | Default |
| -- | -- | -- |
| `--exclude` | Exclude glob patterns relative to each provided path. Use patterns like `tests/**` for directories or `src/legacy/file.py` for specific files. |  |
| `--select <ids>` | Report only the named refactoring rules or rule groups, e.g. `--select C001,C007` or `--select complexity`. Repeatable and comma-separated | all rules |
| `--ignore <ids>` | Never report the named refactoring rules or rule groups, e.g. `--ignore C007` or `--ignore readability`. Wins over `--select`. Repeatable and comma-separated |  |
| `--max-complexity-allowed` | Complexity threshold | `15` |
| `--snapshot-create` | Save the current violations above the threshold into `complexipy-snapshot.json` | `false` |
| `--snapshot-ignore` | Skip comparing against the snapshot even if it exists | `false` |
| `--failed` | Show only functions above the complexity threshold | `false` |
| `--suggest-refactors` | Show deterministic Rust AST-based refactor plans in rich CLI output. Ignored by `--plain` | `false` |
| `--fix` | Apply every fixable suggestion (Safe to apply and spliceable) of the functions above `max-complexity-allowed` to the source files, then report the post-fix state (see [Applying Fixable Suggestions](refactoring-rules.md#applying-fixable-suggestions)). Implies `--suggest-refactors` | `false` |
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
| `--diff <ref>` | Show a complexity diff against a git reference and enforce the threshold. Fails on regressions above `--max-complexity-allowed` (see [Complexity Diff](diff-and-snapshots.md#complexity-diff)) | - |
| `--diff-only <ref>` | Show a complexity diff visually without affecting the exit code (see [Complexity Diff](diff-and-snapshots.md#complexity-diff)) | - |
| `--staged` | Compare staged (git index) changes against the `--diff` ref (default `HEAD`). Answers "what complexity am I about to commit?" (see [Complexity Diff](diff-and-snapshots.md#complexity-diff)) | `false` |
| `--check-script` | Report module-level (script) complexity as a synthetic `<module>` entry | `false` |
| `--no-ignore` | Analyze every function, disregarding inline ignore comments (`# complexipy: ignore`, `# noqa: complexipy`) | `false` |
| `--report-ignored` | List every file:line where an ignore comment suppresses a function. `--quiet` hides the list | `false` |
| `--force-exclude` | Apply `exclude` to files passed explicitly as arguments, such as the file names pre-commit passes | `false` |
| `--isolated` | Ignore every configuration file. Cannot be combined with `--config` | `false` |
| `--config <path>` | Read the configuration from this file only, instead of the invocation directory's files | - |
| `--explain <rule>` | Print a refactoring rule's id, name, category, applicability, description, and documentation link, then exit without analyzing anything. An unknown id exits with an error that lists the valid ids | - |

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
