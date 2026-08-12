Patch release: partial runs (e.g. pre-commit hooks analyzing only staged files) no longer reorder the snapshot, so commits no longer produce a modified complexipy-snapshot.json. The release pipeline also gained artifact-upload retries and quieter notifications.

## Fixed

- Snapshot entries for analyzed files are updated in place instead of being moved to the end of the file: partial runs (e.g. pre-commit hooks analyzing only staged files) no longer reorder the snapshot, so the snapshot stays byte-identical across commits that do not change complexity. (#226)

## Changed

- The release pipeline now retries artifact uploads with pinned workflow versions and only notifies downstream repositories on tag releases, keeping non-tag pushes silent.
- Pinned maturin version in the release workflow to avoid GitHub API rate limits when resolving the latest version inside Docker build containers.

## PRs

- fix(snapshot): keep analyzed entries in place on partial runs by @rohaquinlop in https://github.com/rohaquinlop/complexipy/pull/227

**Full Changelog**: https://github.com/rohaquinlop/complexipy/compare/7.0.0...7.0.1
