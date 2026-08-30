---
name: git-commit
description: >
    Generic git commit workflow and commit message conventions. MUST be loaded
    before running any git commit command. Invoke whenever the user asks to
    inspect, stage, split, or commit changes - even without an explicit slash
    command.
---

# Git Commits

Use this skill to create clean, reviewable git commits in any project.

## First: Inspect Local Conventions

Before staging or committing:

1. Run `git status --short` to see changed files.
1. Inspect recent commit style with `git log --oneline -n 10`.
1. Check project guidance files when present, such as `AGENTS.md`, `CONTRIBUTING.md`,
   `README.md`, or `.github/` templates.
1. Follow explicit project conventions over this generic default.
1. Derive the repo's commit shape from the log - subject-only vs bodies, scoped vs
   unscoped, bundled vs split - and match it.

If no stronger project convention exists, use Conventional Commits.

______________________________________________________________________

## Default Commit Format

```text
<type>(<scope>): <short description>
```

Scope is optional when the change is broad or no clear module exists:

```text
<type>: <short description>
```

______________________________________________________________________

## Types

| Type       | When to use                                             |
| ---------- | ------------------------------------------------------- |
| `feat`     | New user-visible functionality                          |
| `fix`      | Bug fix                                                 |
| `refactor` | Code change that neither adds a feature nor fixes a bug |
| `perf`     | Performance improvement                                 |
| `test`     | Adding or updating tests                                |
| `docs`     | Documentation only                                      |
| `build`    | Build system, packaging, or dependency changes          |
| `ci`       | CI/CD configuration                                     |
| `chore`    | Maintenance that does not affect runtime behavior       |
| `revert`   | Revert a previous commit                                |

______________________________________________________________________

## Scope

Choose a short, lowercase scope from the affected area:

- Directory or package: `api`, `web`, `cli`, `server`, `docs`
- Module or feature: `auth`, `billing`, `search`, `config`
- Tooling area: `deps`, `ci`, `lint`, `release`

Rules:

- Keep scope to one short token when possible.
- Prefer names that already appear in paths, package names, or recent commits.
- Omit scope for broad repository-wide changes.

______________________________________________________________________

## Short Description

- Lowercase unless using a proper noun
- Imperative mood: `add retry logic`, not `added retry logic`
- No period at the end
- Prefer 50 chars or less; hard maximum 72 chars
- Explain the intent, not the implementation detail

______________________________________________________________________

## Body

Add a body only when the subject cannot explain the why, risk, or migration impact.

If recent commits in this repo are subject-only, stay subject-only: write a subject
that carries the intent rather than compensating with a body. The exceptions are
breaking changes (`BREAKING CHANGE:`) and migration/rollout impact. When the repo
merges via PRs with descriptions, the PR description - not the commit body - is
where the explanation belongs.

Body rules:

- Wrap around 72 characters.
- Explain why the change exists and notable tradeoffs.
- Mention breaking changes with `BREAKING CHANGE:` when applicable.
- Reference issues only when useful and allowed by project convention.

______________________________________________________________________

## Examples

```text
feat(auth): add passkey login
fix(api): handle empty search results
refactor(cli): split command parsing from execution
test(billing): cover failed payment retries
docs(readme): document local setup
build(deps): update vite
ci(actions): cache package manager downloads
chore: remove unused assets
```

______________________________________________________________________

## Grouping Changes

When multiple files changed, do not blindly commit everything together. Create one
commit per logical concern.

Workflow:

1. Run `git status --short`.
1. Review diffs with `git diff` and, for staged changes, `git diff --staged`.
1. Identify logical groups. Each group should map to one type and optional scope.
1. Commit foundational changes before dependent changes.
1. Stage only files or hunks that belong to the current group.

Use explicit paths or interactive staging:

```bash
git add <file1> <file2>
git add -p <file>
git commit -m "<type>(<scope>): <description>"
```

Avoid `git add .` and `git add -A` unless the user explicitly asks for a single
all-changes commit and the diff has been reviewed.

### Dependency and Lockfile Changes

Commit package manifest and lockfile updates together for the same package manager.
Examples:

- `package.json` with `package-lock.json`, `pnpm-lock.yaml`, `yarn.lock`, or `bun.lockb`
- `pyproject.toml` with `uv.lock` or compatible Python lockfile
- `Cargo.toml` with `Cargo.lock`
- `go.mod` with `go.sum`

Use `build(deps)` or the project's established dependency commit style.

______________________________________________________________________

## Commit Execution Rules

- Only commit when the user asks to commit.
- Never push unless the user explicitly asks.
- Never amend, rebase, reset, or discard changes without explicit approval.
- Preserve unrelated user changes.
- If staged changes already exist, inspect them before adding more.
- If hooks fail, report the failure and do not bypass hooks unless the user approves.
- Do not add AI tool co-author lines or generation footers unless explicitly requested.

______________________________________________________________________

## What to Avoid

```text
# Too vague
fix: bug fix
chore: changes
feat: stuff

# Not imperative
feat(auth): added passkey support

# Too much implementation detail
fix(api): change line 42 to check null before mapping response

# Mixed concerns
feat(api): add search endpoint and update CI cache
```
