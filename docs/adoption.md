# Adopting complexipy in an Existing Codebase

A codebase that already has complex functions fails the first run. This guide adds complexipy without fixing everything first: record what exists, block anything new or worse, then reduce the backlog over time.

The output on this page comes from a run on the [Flask](https://github.com/pallets/flask) source tree with the default threshold of 15.

## 1. See the current state

List the most complex functions:

```bash
complexipy src --top 5
```

```text
src/flask/sansio/blueprints.py
    Blueprint::register 29  ❌  FAILED

src/flask/app.py
    Flask::make_response 26  ❌  FAILED
    Flask::url_for 22  ❌  FAILED

src/flask/cli.py
    find_best_app 18  ❌  FAILED
    find_app_by_string 17  ❌  FAILED
```

A plain `complexipy src` exits with code 1 here, so it cannot run in CI yet.

## 2. Record a baseline

```bash
complexipy src --snapshot-create
```

```text
No function fails the run. 10 functions above 15 are ALLOWED: no worse than the baseline the gate compares with.
Snapshot watermark passed. Baseline stored at ./complexipy-snapshot.json
```

Commit `complexipy-snapshot.json`. From now on, every run reads it: a function in the snapshot passes while it does not get more complex, and a function that improves leaves the snapshot. See [Snapshot Baselines](diff-and-snapshots.md#snapshot-baselines).

## 3. Block new and worse functions

Run the same command in CI, without `--snapshot-create`. A new function above the threshold fails the run:

```text
src/flask/helpers.py
    merge_headers 23 (new, Δ = +23)  ❌  FAILED
```

To review only what a branch changes, compare it with the target branch:

```bash
complexipy src --diff origin/main
```

```text
+--------+-------------------------------------+-----------+-------+
| Status | Location                            | Change    | Gate  |
+==================================================================+
| NEW    | src/flask/helpers.py::merge_headers | 23  (new) | FAILS |
+--------+-------------------------------------+-----------+-------+

Net: 1 new
Gate: 1 function fails: REGRESSED or NEW above the threshold.
```

See [CI/CD Integration](ci.md) for the GitHub Action, the pre-commit hook, and GitLab, and [Complexity Diff](diff-and-snapshots.md#complexity-diff) for the diff options.

## 4. Show scores in the editor

The language server shows the score of each function above the threshold as you type, so a function gets attention before it reaches CI. Set `inlay-hints = "always"` in the [language server settings](configuration.md#language-server-settings) to show the score of every function. See [Editor Integration](editors.md) for VS Code, Neovim, and Zed.

## 5. Reduce the backlog

Ask for refactor suggestions on the failing functions:

```bash
complexipy src --failed --suggest-refactors
```

Preview the fixes that complexipy can apply by itself, then apply them:

```bash
complexipy src --fix --dry-run
complexipy src --fix
```

```text
Fixed C002 at src/flask/helpers.py:684-694 (-4 complexity)
Skipped C001 at src/flask/helpers.py:684-694 (not safe to auto-apply)
```

Here `merge_headers` went from 23 to 19. The skipped suggestion needs a manual change. See [Refactoring Rules](refactoring-rules.md) for what each rule does and which ones `--fix` applies.

## 6. Lower the threshold

When the backlog is small, set a lower `max-complexity-allowed` in your configuration file and record the baseline again with `--snapshot-create`. Repeat in small steps. See [Configuration](configuration.md) and [Understanding Scores](understanding-scores.md) for the recommended thresholds.
