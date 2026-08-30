---
name: release-notes
description: >
    Generate polished release notes for a new version by inspecting git history
    and past release formats. Keeps the project changelog in sync:
    `CHANGELOG.md` and its Spanish mirror (`docs/es/changelog.md`) accumulate
    entries under `## Unreleased` and move them into a dated section at release
    time. Supports creating the git tag and publishing the release via
    `gh release create`. Invoke when the user asks to write release notes,
    publish a release, or draft a changelog entry for a new version.
---

# Release Notes

Generate well-structured release notes for a new version, publish them as a GitHub
Release, and optionally create the git tag.

## Workflow

### 1. Determine the version and scope

- If the user provides a version (e.g. "5.6.1"), use it.

- If not, read `Cargo.toml` or `pyproject.toml` (or the project's primary
  version file) to find the current version. Compare with the latest git tag
  to confirm what's unreleased.

- Determine if this is a **major**, **minor**, or **patch** release based on
  semver: the last component of the version.

### 2. Gather the changelog

- Find the latest tag with:

    ```bash
    git tag --sort=-v:refname | head -5
    ```

    If the user supplied a previous version, use that as the base. Otherwise
    use the latest tag.

- Get all commits between the previous tag and HEAD:

    ```bash
    git log --oneline --no-merges <prev-tag>..HEAD
    ```

- Get the full commit log with conventional commit types and PR references:

    ```bash
    git log --format="%h %s" <prev-tag>..HEAD
    ```

- Get the **tag date** (needed to filter PRs by merge date):

    ```bash
    git log -1 --format="%ci" <prev-tag>
    ```

- Get all **merged PRs** in this range - prefer the merge-commit approach first
  (more reliable), then cross-reference with `gh pr list` for author/URL details:

    ```bash
    # List merge commits to identify PR numbers in range
    git log --merges --format="%h %s" <prev-tag>..HEAD

    # Then fetch full PR details for those numbers
    gh pr list --state merged --base main --json number,title,author,mergedAt,url \
      --jq '.[] | select(.mergedAt > "<prev-tag-date>")'
    ```

- **Verify version in source file matches intent** - if the version file shows a
  version different from what the user requested, check the git log for version
  bump commits to understand the actual state (e.g. a bumped-then-reverted scenario).

- Detect **new contributors** by checking if any PR author has no prior merged PRs:

    ```bash
    gh pr list --state merged --json author --jq '[.[].author.login] | unique'
    ```

### 3. Update the changelog

The project changelog lives in `CHANGELOG.md` (English, single source of
truth). `docs/changelog.md` embeds it and must stay a stub - never edit its
content; only the root file changes. `docs/es/changelog.md` is the Spanish
mirror.

Before writing the release notes, make sure `## Unreleased` in
`CHANGELOG.md` reflects every merged change since the last release:

- Group changes under `### Added`, `### Changed`, `### Fixed`, `### Removed`
  (Keep a Changelog order; omit empty subsections).
- Write each bullet in the release-notes style: what changed, why, and
  impact, with backticks for code and `(#PR)` references.
- Removed flags, keys, and API breaks go under `### Removed`; a major
  release with breaking changes gets a `!!! note "Migration"` callout
  linking to the migration guide
  (`https://rohaquinlop.github.io/complexipy/migration/`).
- Mirror every entry in `docs/es/changelog.md` under `## Sin publicar`
  (`### Añadido`, `### Cambiado`, `### Corregido`, `### Eliminado`); the ES
  migration link points to
  `https://rohaquinlop.github.io/complexipy/es/migracion/`.

If a change is missing from `## Unreleased`, add it before drafting the
notes. The release notes are drafted from this section, and at publish time
it moves into a dated release section (step 8).

### 4. Study past release style

Read the last 2-3 releases to detect the current format convention:

```bash
gh release view <prev-tag> --json body,tagName
gh release view <prev-tag-2> --json body,tagName
```

Identify:

- **Section naming**: `## Fixed`, `### Features`, `## 🚀 Features`, `## What's Changed`, etc.
- **Summary style**: whether a prose summary paragraph opens the notes (common
  for patch releases) or just goes straight into sections.
- **PR listing**: `## PRs`, `## PR's`, `## What's Changed`, or inline per-section.
- **New contributors**: whether `## New Contributors` is used.
- **Full Changelog format**: always ends with `**Full Changelog**: ...`

### 5. Categorise changes by conventional commit type

Group commits into sections based on their conventional commit prefix:

| Prefix     | Section header                     | Notes                                         |
| ---------- | ---------------------------------- | --------------------------------------------- |
| `feat`     | `## Features` or `### Added`       | New capabilities                              |
| `fix`      | `## Fixed`                         | Bug fixes                                     |
| `refactor` | `## Changed` or `## Refactoring`   | Code restructuring                            |
| `docs`     | `## Documentation`                 | Documentation changes                         |
| `chore`    | `## Chores`                        | Maintenance, version bumps, lockfile syncs    |
| `ci`       | `## Chores` or inline              | CI/CD changes (section depends on past style) |
| `test`     | `## Tests` or fold into `## Fixed` | Test additions tied to fixes                  |
| `perf`     | `## Performance`                   | Performance improvements                      |
| `build`    | `## Build` or `## Chores`          | Build system changes                          |
| `revert`   | `## Fixed` or `## Changed`         | Reverts                                       |

For **patch releases**, prefix sections with `##`. For **minor feature releases**,
`##` or `###` both appear in past practice - follow the most recent style.

### 6. Write the release notes body and save to file

Use this structure, adapting to the detected project style:

```
[Optional summary paragraph - one or two sentences summarising the release]

## [Section header matching past style]

- [Description of change with context, why, and impact. (#PR-number)]
- [Multi-line descriptions are indented two spaces on continuation lines.]

## Section 2
...

## PRs

- [conventional-commit(scope): message] by @author in https://github.com/[owner]/[repo]/pull/[number]

## New Contributors

- @user made their first contribution in https://github.com/...

**Full Changelog**: https://github.com/[owner]/[repo]/compare/[prev-tag]...[new-tag]
```

Rules:

- Each change bullet should say **what** changed, **why** (context/pain point),
  and optionally the **impact** - not just repeat the commit message.
- Patch releases (x.y.Z) should open with a concise summary paragraph.
- Formatting: use backticks for code, file paths, flags, and types.
- PR references: use `(#NN)` shorthand within sections, full link in PRs section.
- The Full Changelog link always compares the previous tag to the new one.

**Always save the notes to a file** in the project root so the user can easily
review, edit, and copy them:

```
RELEASE_NOTES_<version>.md
```

Present a summary of the notes to the user, then ask whether they want to
publish (finalize the changelog, create tag + GitHub release) or make edits
first.

### 7. Finalize the changelog

Once the user approves the notes, move `## Unreleased` in `CHANGELOG.md`
into a dated release section:

1. Rename `## Unreleased` to `## [<version>] - <date>` (date = the release
   date; use today unless a specific date is intended).
2. Reset `## Unreleased` to an empty section - it accumulates the next
   release's changes.
3. Append the release-link footer to the new section:
   `See the [release notes](https://github.com/<owner>/<repo>/releases/tag/<version>) for the full details.`
4. Mirror the new section in `docs/es/changelog.md` (`## [<version>] - <date>`,
   same date and PR references, translated) and reset the Spanish
   `## Sin publicar` to empty.
5. If this is a major release with breaking changes, add a
   `!!! note "Migration"` callout at the top of the new section linking to
   the migration guide (`https://rohaquinlop.github.io/complexipy/migration/`;
   Spanish: `https://rohaquinlop.github.io/complexipy/es/migracion/`).

Never edit `docs/changelog.md` - it embeds the root file via the
pymdownx.snippets include (`--8<-- "CHANGELOG.md"`).

### 8. Create the tag (if requested)

If the user asks to publish or create the release:

```bash
# Determine target commit - usually main HEAD
git log -1 --format="%H" main

# Create the tag on latest main
git tag <version> <commit-hash>
git push origin <version>
```

Verify the tag points to the latest main commit - never to a detached or
stale commit.

### 9. Create the GitHub Release

Draft the release body from the `## [<version>] - <date>` section that just
moved out of `## Unreleased`: keep the changelog's bullets, and add the
opening summary paragraph, the `## PRs` list (full
`[conventional-commit(scope): message] by @author in <pull-url>` lines), and
the `**Full Changelog**` footer per the project style.

```bash
gh release create <version> -F - <<'BODYEOF'
<release-notes-body>
BODYEOF
```

After creation, set the release title to match the version:

```bash
gh release edit <version> --title "<version>"
```

### 10. Verify

Confirm with:

```bash
gh release view <version> --json name,tagName,url --jq '{name, tagName, url}'
```

Confirm the changelog is finalized: `## [<version>] - <date>` exists in
`CHANGELOG.md` and `docs/es/changelog.md`, `## Unreleased` is empty, and
`docs/changelog.md` is still just the include stub.

## Edge Cases

| Scenario                                        | Action                                                                                                                                                     |
| ----------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------- |
| No previous tag                                 | Use `git log --oneline` from the beginning of git history                                                                                                  |
| Tag already exists locally but stale            | Delete local tag (`git tag -d <tag>`), delete remote (`git push origin :refs/tags/<tag>`), delete release (`gh release delete <tag> --yes`), then recreate |
| Tag exists but on wrong (stale) commit          | Delete tag and release, recreate on latest main                                                                                                            |
| User wants a draft release                      | Add `--draft` to the `gh release create` command                                                                                                           |
| User wants a prerelease                         | Add `--prerelease` to the `gh release create` command                                                                                                      |
| No PRs in the release range                     | Generate notes from raw commit messages, grouped by conventional commit type                                                                               |
| `## Unreleased` is missing entries              | Add the missing changes from the gathered PR list to `CHANGELOG.md` (and the Spanish mirror) before drafting the release notes (step 3)                   |
| `docs/changelog.md` was edited                  | Restore it to a stub containing only `--8<-- "CHANGELOG.md"` - the root file is the single source of truth                                               |
| Auto-release pipeline already created a release | Check with `gh release view <tag>`; if exists, prompt user before overwriting                                                                              |
| Multiple repos                                  | Use the current working directory's git remote to infer owner/repo                                                                                         |

## Downstream Release Verification

If the project has a **notify-downstream** workflow that dispatches release
events to other repos (e.g. pre-commit hooks, GitHub Actions, companion
packages), verify that those downstream repos' workflows actually **create
GitHub Releases**, not just push tags.

Common failure pattern: a downstream `update-version.yml` workflow does
`git tag && git push` but lacks `gh release create`. The tag gets pushed,
no release is created, and the dispatch appears "successful" in the upstream
pipeline logs.

**Check:** After publishing, inspect downstream repos:

```bash
gh release view <tag> --repo <downstream-owner/repo> --json tagName
```

If missing, add to the downstream workflow:

```yaml
- name: Create GitHub Release
  env:
    GH_TOKEN: ${{ secrets.GITHUB_TOKEN }}
  run: |
    VERSION="${{ github.event.client_payload.version }}"
    gh release create "v${VERSION}" --title "v${VERSION}" --generate-notes
```

## Style Reference

This project follows a professional tone without emoji section markers,
using `##` headers and descriptive bullet points that explain the "what",
"why", and impact of each change rather than just paraphrasing commit messages.

## Changelog Conventions

- `CHANGELOG.md` at the repo root is the single source of truth; the docs
  page embeds it via pymdownx.snippets (`--8<-- "CHANGELOG.md"`), so
  `docs/changelog.md` must stay a stub.
- Sections are newest first: `## Unreleased` on top, then
  `## [x.y.z] - YYYY-MM-DD`.
- Subsection order: Added, Changed, Fixed, Removed; omit empty ones.
- Every release section ends with a link back to its GitHub release notes.
- Major releases with breaking changes carry a `!!! note "Migration"`
  callout linking to the migration guide.
- `docs/es/changelog.md` mirrors the root file: same headings, dates, and
  PR references, translated.
- At release time, Unreleased content moves into the dated section and
  Unreleased resets empty - the changelog is the drafting ground for the
  GitHub release notes.
