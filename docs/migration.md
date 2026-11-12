# Migration Guide

This page tracks removed flags and keys and their replacements. Check it
before upgrading between major versions.

## Removed in 7.0.0

The following CLI flags and TOML keys were removed. Use their replacements
instead:

| Removed flag/key | Replacement |
| -- | -- |
| `--output-json` / `-j` | `--output-format json` |
| `--output-csv` / `-c` | `--output-format csv` |
| `--output-gitlab` | `--output-format gitlab` |
| `--output-sarif` / `-sr` | `--output-format sarif` |
| `--ratchet` / `-R` | `--diff <ref>` (enforces by default) |
| `output-json = true` | `output-format = ["json"]` |
| `output-csv = true` | `output-format = ["csv"]` |
| `output-gitlab = true` | `output-format = ["gitlab"]` |
| `output-sarif = true` | `output-format = ["sarif"]` |
| `ratchet = true` | `[tool.complexipy.diff] branch` or `--diff <ref>` |
| `staged = true` | `[tool.complexipy.diff] staged = true` |
| `details = "low"` | `failed = true` |

## Removed in 8.0.0

The CLI was rewritten in Rust. The following old-version compatibility
features were removed:

- **Git-URL analysis** (`complexipy <repository-url>`) - local paths only.
- **Legacy cache migration** - the previous
  `.complexipy_cache/<hash>.json` layout is no longer migrated; existing
  legacy files are ignored and the delta history starts fresh.
- **Legacy snapshot files** - snapshots created with older versions are no
  longer detected; recreate them with `--snapshot-create`.
- **`-mx` short flag** - use `--max-complexity-allowed` instead.

## Changes in 9.0.0

No flag or key was removed. Scores and some output changed:

- **Scores** - this release applies more of the cognitive complexity paper's
  rules: comprehensions, loop and `try` `else`, mixed boolean operators,
  recursion cycles, nested definitions, and conditional expressions in more
  places. On the django source tree, 1,946 of 30,813 functions change score:
  1,481 go down and 465 go up, and the total of all files drops by 7%. Re-run
  `complexipy` and review `max-complexity-allowed`. If you use a snapshot,
  recreate it with `--snapshot-create`, since a function whose score rose
  fails the check. The [changelog](changelog.md) lists each rule.
- **`--plain` output** - each line is now
  `<path>:<line> <function> <complexity>`. A script that reads the first
  field as the path must remove the `:<line>` suffix.
- **Warnings** - every CLI warning prints to stderr, including the `--diff`
  and `--staged` warnings, and `--quiet` no longer hides them. A script that
  reads these warnings from stdout must read stderr.
- **C005 helper** - the extracted predicate no longer carries a `-> bool`
  annotation.

## What each replacement does

- `--output-format <format>` selects the machine-readable output format
  (`json`, `csv`, `gitlab`, `sarif`). Repeat the flag for multiple formats.
- `--diff <ref>` shows a complexity diff against a git reference and
  enforces the threshold by default.
- `[tool.complexipy.diff] branch` sets the default diff reference in TOML;
  `[tool.complexipy.diff] staged = true` enables staged comparison by
  default.
- `failed = true` shows only functions above the complexity threshold.
