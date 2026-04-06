## Follow-up Issue: Unify Machine-Readable Outputs Under `--output-format`

The new `--output-gitlab` flag is the lowest-risk way to add GitLab Code Quality support without breaking the current CLI.

There is still a follow-up API/design issue worth addressing:

- The CLI now has separate flags for `--output-json`, `--output-csv`, `--output-sarif`, and `--output-gitlab`.
- Competing tools like Ruff expose a single `--output-format` interface, which is easier to document and script against.
- Moving to a format-based API would also make it easier to add future formats without growing the top-level flag surface.

### Proposed Follow-up

Add a repeatable `--output-format` option and a matching TOML key.

Example:

```bash
complexipy . --output-format json --output-format gitlab
```

Possible supported values:

- `json`
- `csv`
- `sarif`
- `gitlab`

### Important Constraints

- Do not break existing boolean flags in the first step. Keep them as deprecated aliases for at least one release.
- Preserve the current ability to emit multiple report files in one invocation.
- Decide whether the option should support repeated flags, comma-separated values, or both.
- Keep output filenames deterministic and consistent with the selected format.
- Document the migration path from `--output-json`, `--output-csv`, `--output-sarif`, and `--output-gitlab`.

### Suggested Implementation Plan

1. Introduce an `OutputFormat` enum in Python.
2. Add repeatable `--output-format`.
3. Translate legacy booleans into the same internal format set.
4. Emit reports through a single storage dispatch function.
5. Warn when deprecated legacy flags are used.
6. Remove legacy flags only in a later major release.
