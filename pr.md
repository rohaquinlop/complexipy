Title: feat: add GitLab Code Quality report output

## What

Add GitLab Code Quality report output via `--output-gitlab`, document how to use it in CI, and capture a follow-up issue for a future `--output-format` redesign.

## Why

Issue #124 asks for a CI-friendly GitLab report format so complexipy findings can appear directly in GitLab merge requests and pipeline reports without requiring a wrapper script.

## Changes

- add a new `--output-gitlab` CLI flag and TOML config support
- add a GitLab Code Quality formatter that emits the required JSON fields for failing functions
- add tests covering the new report format and CLI output file generation
- document the new flag in the README, docs index, and English and Spanish usage guides
- add GitLab CI examples for uploading Code Quality artifacts
- add `issue.md` describing the follow-up `--output-format` consolidation work
