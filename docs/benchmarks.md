# Benchmarks

This page documents how the current Rust CLI compares to the 7.0.1 Python
CLI. Both versions share the same Rust analysis engine; the difference is
the CLI pipeline: argument parsing, config resolution, path walking, and
output rendering now run in Rust, and the Python interpreter no longer
does any analysis work. The environment block below names the exact
version and commit.

## Methodology

- **New CLI:** built from the repository with
  `uv run maturin develop --release`, invoked as `.venv/bin/complexipy`.
- **Baseline CLI:** `complexipy==7.0.1` installed from PyPI with
  `uv add` into an isolated project, invoked as its venv's
  `bin/complexipy`.
- **Invocation parity:** both CLIs run as bare console scripts with
  byte-identical flags, each workload from its own empty directory. The
  `uv run` wrapper is excluded from timing so neither side pays uv process
  overhead.
- **Corpus:** real open-source repos, shallow-cloned at pinned commits,
  plus a single-file probe that isolates interpreter and CLI startup from
  tree analysis.
- **Metrics:** wall time via hyperfine (warmup 3, 5 runs; probe warmup 5,
  20 runs) with stdout discarded; peak RSS via `/usr/bin/time -l`, 3 runs,
  maximum reported.
- **Parity gate:** before timing, both CLIs export JSON for the requests
  repo and the single-file probe. The exit codes match, and every function
  the baseline reports has the same score in the new CLI, so both sides
  score the same code. The new CLI also scores functions defined under a
  module-level `if` or `try`, which 7.0.1 skips.

The benchmark is repeatable:

```bash
bash benchmarks/benchmark-cli.sh
```

--8<-- "benchmarks/results.md"

## What the numbers say

- **Large trees win the most.** django (~2900 files, its test suite
  included) is roughly 68-91x faster: the Rust engine analyzes the tree
  in parallel across all cores, where the Python CLI processed it on one.
- **Small and medium trees** (requests, flask) are roughly 9-15x faster.
- **Startup is cheaper.** The single-file probe is about 3.3x faster,
  which is the Python interpreter + typer/rich import cost removed from
  the hot path.
- **The scaling table** at the bottom of the results measures the Rust
  engine on a synthetic fixture at 1x, 2x, and 4x sizes. Ratios near 2
  mean linear scoring; ratios near 4 mean quadratic behavior. The recorded
  ratios sit near 1.0-1.2.
- **Output rendering is cheaper too.** Rendering the full results table
  on django costs ~0.02 s on the Rust CLI against ~5.7 s on the Python CLI
  (the default minus --quiet delta), so the wall-clock gap widens on large
  trees when results are actually printed - the [render] rows measure that
  path end to end.
- **Peak memory is lower on small trees and higher on large ones.** The
  Rust CLI uses 3-5 MB less on requests and flask and about 13 MB less on
  the single-file probe, because the Python interpreter and its display
  libraries are gone from the process. On django it peaks about 60 MB
  higher: it analyzes files in parallel and allocates with mimalloc, which
  trade memory for speed.
