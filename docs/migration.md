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

## What each replacement does

- `--output-format <format>` selects the machine-readable output format
  (`json`, `csv`, `gitlab`, `sarif`). Repeat the flag for multiple formats.
- `--diff <ref>` shows a complexity diff against a git reference and
  enforces the threshold by default.
- `[tool.complexipy.diff] branch` sets the default diff reference in TOML;
  `[tool.complexipy.diff] staged = true` enables staged comparison by
  default.
- `failed = true` shows only functions above the complexity threshold.
