# Diff and Snapshots

## Complexity Diff

The `--diff`, `--diff-only`, and `--staged` options require Git 2.30 or newer
on your `PATH`. Check your version with `git --version`. Git is not required
for analysis without these options.

Compare current results against any git reference:

```bash
complexipy . --diff HEAD~1
complexipy . --diff main
complexipy src/ --max-complexity-allowed 10 --diff HEAD~1
```

By default `--diff` **enforces** the complexity threshold: the run exits with code `1` only when a change breaches the contract relative to `--max-complexity-allowed`:

- A **new** function introduced above the threshold.
- A **modified** function whose complexity increased **and** ends above the threshold (including already-over functions that get worse).

Functions that regress but stay at or below the threshold (e.g. `3 → 4` with `--max-complexity-allowed 15`) are **not** failures.

To see the diff visually without affecting the exit code, use `--diff-only` instead:

```bash
complexipy . --diff-only HEAD~1
```

To compare the **staged** (git index) content instead of the working tree - "what complexity am I about to commit?" - add `--staged`:

```bash
# Staged changes vs HEAD (the default baseline for --staged)
complexipy . --staged

# Staged changes vs a specific ref
complexipy . --diff main --staged
```

Like `--diff`, `--staged` enforces the threshold against the staged content and fails when a staged change pushes a function above `--max-complexity-allowed`. It compares only the staged files the run selects: the path arguments, `paths`, and `exclude` apply as they do without `--staged`, to the index rather than the working tree, so a staged regular file is compared whatever the working tree holds at its path. Staged deletions that discovery would have selected produce `REMOVED` entries: under a selected directory, outside hidden paths, and not matched by `exclude`, `.gitignore`, `.ignore`, or git's exclude file. Staged additions produce `NEW` entries. Use `--diff-only` with `--staged` for a visual-only view.

A rename or a move passes the gate as long as the function's body stays as it was. A file that git pairs as renamed compares with its old path when the run covers that path; a file moved in from outside the selected paths compares as added, and a file moved out of them reports nothing. A path that was not a regular file at the reference, such as a symbolic link, is not read there: it compares as added, and its deletion reports nothing. A function above the threshold that the change adds passes only when it replaces a `REMOVED` function with the same body and at least its complexity, each removed function replaced once, and the compared files do not hold more functions with that body than they did at the reference. The complexity matters because a call to a function's own name scores as recursion, so a rename can make the same body more complex. The compared files are the ones the diff compares: modified, renamed from a covered path, deleted from a covered path, and added. Bodies compare structurally, ignoring formatting and comments, and leave out the name, the decorators, and the parameters, so a function rename, a class rename, a move to another file, and a method that becomes a function all pass, while a function that moves and changes fails. Functions that an ignore comment hides count on both sides, so a function copied from one that is now hidden fails. Bodies count and pair only within the repository that holds each file, since each repository is compared at its own reference. The diff table names, in a `Gate` column, the `REMOVED` function with the same body that each passing function replaces. A function that an ignore comment now hides was not removed, so it does not report as `REMOVED`. A deleted Python file that discovery would have selected reports its functions as `REMOVED`, so a function moved out of it passes as well.

When `--diff` or `--staged` decides the exit code, the report shows the gate's verdict. A function that fails the run, REGRESSED or NEW above the threshold, is listed as `FAILED` and marked `FAILS` in the diff table's `Gate` column, and the table ends with a `Gate:` line that counts them. A function above the threshold that the gate lets through is listed as `ALLOWED`, and `--failed` lists only the functions that fail the run. A snapshot run lists the functions its snapshot lets through as `ALLOWED` too.

This requires `git` and a repository-backed path. `--diff` and
`--diff-only` read each file from the repository that holds it, so a run
from outside the repository compares against the same history as a run
from inside it. A submodule, meaning a repository that the repository
around it tracks as a gitlink in its index, is read at the commit that the
outer repository pins for it at the reference, and deeper nesting follows
each pin in turn. When the run starts inside the submodule, it is read at
the reference itself. A repository inside another that the outer one does
not track is read at the reference, like any other repository. `--staged`
uses the repository of the working directory; outside a repository it
prints a warning and compares nothing, so the report lists functions above
the threshold as `FAILED` but the threshold does not decide the exit code.
When a file sits outside any repository, the reference names no commit in
the file's repository, or the outer repository pins no commit for the
submodule at the reference, or pins a commit the submodule does not have,
every function of that file reports as `NEW` and the run prints a warning
that names the cause. The functions of a file that does not parse at the
reference report as `NEW` too.

Instead of repeating the reference on every call, declare the comparison
policy once in a configuration file - see [Diff Configuration](configuration.md#diff-configuration).

## Snapshot Baselines

Snapshots allow you to adopt complexipy gradually in large, existing codebases.

### Creating a Snapshot

```bash
complexipy . --snapshot-create --max-complexity-allowed 15
```

This creates `complexipy-snapshot.json` in your working directory, recording all functions that currently exceed the threshold.

Create the baseline from the command line. `snapshot-create = true` in a
configuration file re-records the baseline on every run, so every current
violation passes; such a run prints a warning on stderr that names the file.

### How Snapshots Work

Once a snapshot exists, complexipy will:

- ✅ **Pass**: Functions that were already in the snapshot and haven't gotten worse
- ✅ **Pass**: Functions that improved (automatically removed from snapshot)
- ❌ **Fail**: New functions that exceed the threshold
- ❌ **Fail**: Functions above the threshold that moved to another file or got a new name. The snapshot keys each entry by file and name, so run `--snapshot-create` after such a change to record them again
- ❌ **Fail**: Tracked functions that got more complex

### Using Snapshots in CI

```yaml
# .github/workflows/complexity.yml
name: Complexity Check

on: [push, pull_request]

jobs:
    check:
        runs-on: ubuntu-latest
        steps:
            - uses: actions/checkout@v4

            - name: Install complexipy
              run: pip install complexipy

            - name: Check complexity
              run: complexipy . --max-complexity-allowed 15
```

The snapshot file (`complexipy-snapshot.json`) should be committed to version control.

### Ignoring Snapshots

Temporarily disable snapshot checking:

```bash
complexipy . --snapshot-ignore
```

Use this when:

- Refactoring multiple files at once
- Regenerating the baseline
- Testing different thresholds

### Snapshot File Format

```json
[
    {
        "path": "src/legacy.py",
        "file_name": "legacy.py",
        "functions": [
            {
                "name": "old_function",
                "complexity": 23
            }
        ]
    }
]
```

Snapshots are stored as a JSON array of analyzed files. Each entry contains only functions above the threshold at the time the snapshot was written. The file is rewritten after successful snapshot checks, so improved functions are removed automatically. Updates only touch the files analyzed in the run - entries for files outside the analysis are preserved, so running on a subset of files (for example through a pre-commit hook) never shrinks the baseline. Entries for files that no longer exist are dropped when the file is rewritten. Snapshots created by older complexipy versions may need to be regenerated with `--snapshot-create`.

Functions of one file that share a name, such as a property's getter and setter, get separate entries: the first in source order keeps the name, and the next ones are stored as `name#2`, `name#3`. `--diff` keys its comparison the same way.
