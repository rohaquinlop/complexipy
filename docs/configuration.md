# Configuration

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
    force-exclude = false
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
    workspace-diagnostics = false
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
    workspace-diagnostics = false
    ```

| Key | Description | Default |
| -- | -- | -- |
| `inlay-hints` | `threshold` shows the per-function hint only above `max-complexity-allowed`, `always` shows it for every function, `never` suppresses every hint | `threshold` |
| `per-line-hints` | Also show the complexity added by each line, as `+2`. Lines that add nothing are skipped | `false` |
| `diagnostics` | Publish a warning for every function above `max-complexity-allowed` | `true` |
| `workspace-diagnostics` | Also publish warnings for files that are not open, from a scan of the workspace folder | `false` |

The server reads the top-level `max-complexity-allowed`, `paths`, `exclude`,
`select`, `ignore`, and `no-ignore` keys, and it ignores every other key of the same
file: `quiet`, `failed`, `sort`, `color`, `output`, `output-format`,
`cache-dir`, `snapshot-create`, `snapshot-ignore`, `ignore-complexity`,
`check-script`, `report-ignored`, and `diff`. A function whose complexity
equals the threshold passes, as it does on the command line.

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
