# Benchmarks

This page documents how the 8.0.0 Rust CLI compares to the 7.0.1 Python
CLI. Both versions share the same Rust analysis engine; the difference is
the CLI pipeline: argument parsing, config resolution, path walking, and
output rendering now run in Rust, and the Python interpreter no longer
does any analysis work.

## Methodology

- **New CLI (pre-release):** built from the release branch with
  `uv run maturin develop --release`, invoked as `.venv/bin/complexipy`.
  It reports version 7.0.1 because the version bump happens at release
  time; the exact commit is recorded in the environment block below.
- **Baseline CLI:** `complexipy==7.0.1` installed from PyPI with
  `uv add` into an isolated project, invoked as its venv's
  `bin/complexipy`.
- **Invocation parity:** both CLIs run as bare console scripts with
  byte-identical flags. The `uv run` wrapper is excluded from timing so
  neither side pays uv process overhead.
- **Corpus:** real open-source repos, shallow-cloned at pinned commits,
  plus a single-file probe that isolates interpreter and CLI startup from
  tree analysis.
- **Metrics:** wall time via hyperfine (warmup 3, 5 runs; probe warmup 5,
  20 runs) with stdout discarded; peak RSS via `/usr/bin/time -l`, 3 runs,
  maximum reported.
- **Parity gate:** before timing, both CLIs export JSON for every corpus
  repo. The exports are byte-identical and the exit codes match, so the
  comparison measures the same work on both sides.

The benchmark is repeatable:

```bash
bash benchmarks/benchmark-cli.sh
```

--8<-- "benchmarks/results.md"

## What the numbers say

- **Startup-dominated workloads win the most.** The single-file probe is
  about 4x faster, which is the Python interpreter + typer/rich import
  cost removed from the hot path.
- **The scaling table** at the bottom of the results measures the Rust
  engine on a synthetic fixture at 1x, 2x, and 4x sizes. Ratios near 2
  mean linear scoring; ratios near 4 mean quadratic behavior. The recorded
  ratios sit near 1.1-1.2.
- **Small and medium trees** (requests, flask) are roughly 5-6.4x
  faster.
- **Large trees** (django, ~2900 files) are roughly 16-24x faster: the
  Rust engine analyzes the tree in parallel across all cores, where the
  Python CLI processed it on one.
- **Output rendering is cheaper too.** Rendering the full results table
  on django costs ~0.2 s on the Rust CLI against ~1.6 s on the Python CLI
  (the default minus --quiet delta), so the wall-clock gap widens on large
  trees when results are actually printed - the [render] rows measure that
  path end to end.
- **Peak memory is lower across the board**, by roughly 15-40 MB,
  because the Python interpreter and its display libraries are gone from
  the process.
