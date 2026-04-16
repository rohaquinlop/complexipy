# Repo Scout: rohaquinlop/complexipy — Perf/Feature Focus

> 670 stars | 20 forks | 7 open issues | MIT | v5.3.0 | Updated 2026-04-16
> "Blazingly fast cognitive complexity analysis for Python, written in Rust."

---

## TL;DR

You (sebastianbreguel) already took **5 of the top 6 opportunities** from the prior scout. Six PRs merged (`--ratchet`, `--plain`, `--top`, script-level complexity, unify machine-readable output) plus three still open (#160 perf clone-removal, #161 dataclasses, #162 diff --staged). Your external merge rate on this repo is **100% (6/6)**, well above the already-high project baseline. The opportunity space has narrowed — this update focuses on what's **left** in perf/feature.

**Top 3 next bets** (perf/feature only):
1. **Rayon-parallel file walk** in `evaluate_dir` — the single biggest unshipped perf win, ~90% merge likelihood
2. **Cyclomatic complexity support** (#77) — `help wanted` + `good first issue`, 10 months stale, owner on record as enthusiastic
3. **Criterion benchmark suite** — no `benches/` exists; makes future perf PRs (yours or others') credible

---

## Updated Contribution Profile (fresh signal)

- **Your personal merge rate**: 6/6 merged, 3/3 open pending review (PR #160, #161, #162).
- **Maintainer cadence**: still ~1–3 day review turnaround. Merged three of your PRs same week.
- **Active contributor tax**: you are now effectively a trusted external contributor. Future PRs get higher prior.
- **CI state**: maturin builds across Linux/macOS/Windows + multiple Python versions + ppc64le + x86/aarch64. Recent `fix(ci): handle bad CRC-32 entries in wheels during repack` and `retrigger after transient ppc64le build failure` suggest CI is brittle on exotic targets — test locally on Linux before pushing.
- **Review style**: owner (`rohaquinlop`) reads diffs carefully, rarely nitpicks, merges fast when tests pass. No CODEOWNERS, no CONTRIBUTING.md.
- **Stack constraints** (confirmed from `Cargo.toml`): `edition = "2024"`, pyo3 via maturin, `ignore::Walk` already wired (no need to add .gitignore support — it's there), no rayon yet, no `benches/` dir.

---

## Opportunity Status: What's Taken vs What's Left

| # from prior | Opportunity | Status | Note |
|---|---|---|---|
| 1 | Config dataclass replacing 25 Optional params | **Open** | Still worth it, large diff |
| 2 | Dataclasses in output.py | **In flight — PR #161 (yours)** | |
| 3 | `count_bool_ops` by reference | **In flight — PR #160 (yours)** | Covers #10, #12 too |
| 4 | Git-aware delta reporting (#151) | **Partially — PR #162 (yours, --staged/--cached)** | Full git-range compare still open |
| 5 | CONTRIBUTING.md | Open (skipped — out of scope here: not perf/feature) | |
| 6 | Deduplicate evaluate_dir quiet branches | **Included in PR #160** | |
| 7 | Audit Python 3.8 support | Open (out of scope — not perf/feature) | |
| 8 | Free-threading (#116) | Open | XL, risky |
| 9 | Other complexity types (#77) | **Open — highest feature-level bet** | |
| 10 | Remove debug `println!` | **Included in PR #160** | |
| 11 | VSCode threshold settings (#127) | Open (out of scope — separate repo) | |
| 12 | `is_decorator` by reference | **Included in PR #160** | |
| 13 | Publish to openVSX (#145) | Open (out of scope — separate repo + CI) | |

**Net remaining from prior list (perf/feature only):** #1 (config dataclass — arguably more refactor than feature), #4 (full git-range diff), #8 (free-threading), #9 (other complexity types).

---

## New Opportunities (Perf/Feature)

Opps below were not in the prior scout. All are verified against current code (v5.3.0, commit `17d6175`).

### Performance

**P1. Parallelize `evaluate_dir` with Rayon over `ignore::WalkParallel`**
`src/cognitive_complexity.rs:171` currently walks files sequentially via `ignore::Walk`. Each file is independently parsed and analyzed — embarrassingly parallel. The `ignore` crate already has a `WalkParallel` variant, and rayon is the idiomatic fan-out. Expected 3–8× wall-clock speedup on multi-file runs (e.g. `complexipy .` on a 500-file repo). Directly reinforces the project's "blazingly fast" positioning. Coordination: you'll need an `Arc<Mutex<ProgressBar>>` (or `indicatif`'s thread-safe variants) and a thread-safe accumulator for results — straightforward. Add a `--threads N` (or `-j N`) CLI flag with sensible default (`num_cpus::get()`).

**P2. Criterion benchmark suite in `benches/`**
No `benches/` directory, no `criterion` dep. A tool that claims "extremely fast" without a public benchmark is a blind spot. Add `benches/walk_bench.rs` measuring: (a) single large file, (b) small repo (~20 files), (c) medium repo (~500 files, vendored copy of django or similar). This lands cleanly alone and becomes the regression harness for P1 and any future perf PR — both for you and for the maintainer.

**P3. Release-profile tuning**
Verify `Cargo.toml` `[profile.release]` sets `lto = "fat"`, `codegen-units = 1`, `panic = "abort"`. Maturin's default release profile is decent but not optimal. Expect 5–15% speedup with near-zero diff. Very low risk.

**P4. Parse-once guard**
Audit whether any file gets parsed more than once in a run. If so, cache the `ruff_python_parser::parse_program` result keyed by file path. If not, skip this opp. Requires 30 min of reading `process_path` → `evaluate_dir` → individual analysis functions.

### Features

**F1. Cyclomatic complexity support (#77)**
Labeled `help wanted` + `good first issue`. Owner said "great idea!!!" in 2025-06. 10 months stale. The cleanest approach: add a `ComplexityKind` enum (`Cognitive`, `Cyclomatic`), a `--metric {cognitive,cyclomatic,all}` CLI flag, and a parallel `count_cyclomatic_complexity` traversal in Rust. Cyclomatic is simpler than cognitive (just count decision points) — maybe 150 LOC in Rust, 50 LOC in Python for the flag. Keep cognitive as default to preserve backwards compat.

**F2. Halstead metrics** (stretch on #77)
Mentioned in #77 comments. Operators/operands counting over the AST. Adds `--metric halstead`. Defer until F1 ships — F1 establishes the multi-metric plumbing.

**F3. HTML report output format**
Current outputs: csv, json, gitlab, sarif. HTML with a sortable complexity table + color-coded thresholds would be the first "human-facing" output format and is genuinely useful for CI artifacts and docs sites. Template can be ~100 lines of static HTML + a tiny template string in Python. Sits cleanly next to the new `OutputFormat` enum you already know.

**F4. `--watch` mode**
Rebuild & re-analyze on file save (via `notify` crate). Dev-ergonomics feature. Medium effort but fits the "fast feedback loop" story of the tool.

**F5. Per-directory threshold overrides via TOML**
Today `max_complexity_allowed` is global. Let `.complexipy.toml` accept per-path overrides:
```toml
[[thresholds]]
paths = ["legacy/**"]
max_complexity = 25
```
Useful for gradually adopting the tool on old codebases. Natural successor to your `--ratchet` work.

**F6. `--baseline <file>` flag**
Store a snapshot of current complexities to a baseline file; future runs compare against it and fail only on regressions relative to baseline (not just cache). Adjacent to `--ratchet` and #151; cleaner than the cache-based delta. Could subsume part of the snapshot flow.

---

## Ranked Opportunity Table (Perf/Feature Only)

Base score 30, adjusted per the rubric. "You-bias" (+10) applied because your prior acceptance makes the maintainer less risk-averse on your PRs.

| # | Opportunity | Type | Impact | Effort | Lines | Merge Score | Notes |
|---|---|---|---|---|---|---|---|
| P1 | Rayon parallelize `evaluate_dir` | Perf | **High** | M | ~120 | **90** | Biggest unshipped win; reinforces core project claim |
| F1 | Cyclomatic complexity (#77) | Feat | **High** | L | ~300 | **90** | Labeled `good first issue` + `help wanted`, owner enthusiastic, 10mo stale |
| P2 | Criterion benchmark suite | Perf infra | Medium | S | ~150 | **85** | Enables trustworthy perf PRs; zero-risk landing |
| F6 | `--baseline <file>` flag | Feat | High | M | ~180 | **80** | Natural successor to your `--ratchet` |
| F3 | HTML report format | Feat | Medium | S | ~120 | **80** | First human-facing output format |
| P3 | Release profile tuning | Perf | Low–Med | S | ~5 | **80** | Near-zero risk, small but free win |
| F5 | Per-dir threshold overrides | Feat | Medium | M | ~150 | **75** | Useful for legacy adoption |
| 1 (prior) | Config dataclass in `main.py` | Refactor/Feat-enabler | High | M | ~150 | **75** | Unblocks future flag additions; large diff may need splitting |
| 4 (prior) | Full git-range `--diff <ref>` | Feat | High | M | ~150 | **75** | Your #162 only did index; full ref-vs-ref still open |
| F4 | `--watch` mode | Feat | Medium | M | ~200 | **65** | Nice-to-have, larger diff, `notify` dep |
| F2 | Halstead metrics | Feat | Medium | L | ~250 | **60** | Wait for F1 to land first |
| P4 | Parse-once guard | Perf | Low (maybe 0) | S audit | ? | **55** | Only if audit finds a double-parse |
| 8 (prior) | Free-threading (#116) | Perf/compat | High | XL | ~? | **50** | PyO3-level work, high failure risk |

**Sorted by `(Impact × MergeScore) / Effort`:**
1. P1 Rayon
2. P2 Criterion
3. P3 Release tuning
4. F1 Cyclomatic (#77)
5. F6 `--baseline`
6. F3 HTML report
7. 4 (prior) git-range diff

---

## Recommended Next PR (single pick)

**Ship P1 (Rayon) + P2 (Criterion) as two sequential PRs in that order.**

Rationale: P2 lands alone trivially (infra only, no logic change). Then P1 can include `before/after` criterion numbers in the PR body, which turns it from a "trust me, it's faster" PR into a receipted perf claim. That pairing is exactly the kind of evidence-first contribution this maintainer rewards — and it lets you write a release note that says "3–8× faster on large repos" with numbers to back it up. After those land, **F1 (cyclomatic)** is the highest-impact pure feature left on the table.

File: `/Users/sebabreguel/personal/complexipy/repo-scout-rohaquinlop-complexipy.md`
