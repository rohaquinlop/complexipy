# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project instructions

@AGENTS.md

Everything tool-agnostic — commands, architecture, invariants, code style, conventions —
lives in `AGENTS.md` and is imported above. Edit that file, not this one. This file holds
only what is specific to Claude Code.

## Claude Code specifics

- **Skills:** load `git-commit` before running any `git commit`, and `create-pr` before
  opening a PR or writing a PR description. `release-notes` covers release write-ups.
  `.claude/skills` is a symlink to `../.agents/skills` — edit skills there, and never
  replace the symlink with copies.
- **Commit attribution:** `.claude/settings.json` blanks the commit and PR attribution
  footers — do not add Claude as co-author or re-add an attribution trailer.
- **Temporary files:** use the session scratchpad, never the repo working tree and never
  `/tmp`. Nothing scratch should ever end up in `git status`.
- **Verify Rust changes yourself:** `uv run maturin develop` is easy to skip when
  delegating to a subagent. If a subagent reports pytest results after touching
  `src/**/*.rs`, confirm it rebuilt first — otherwise the run means nothing.
