---
name: create-issue
description: >
    Create a well-formed GitHub issue with a clean title and body. Invoke
    whenever the user asks to create an issue, file a bug report, open a
    feature request, or report something via GitHub CLI. Encodes the lessons
    learned from bad issue drafts: GitHub renders every single newline as a
    hard line break, internal working-doc jargon does not belong in public
    issues, and titles must match the repo's existing conventions.
---

# Create GitHub Issue

Creates a GitHub issue using `gh issue create`. Produces a public, well-formatted
title and body, then verifies the rendered result.

## Before Writing

1. **Search for existing issues first.** `gh search issues --repo <owner>/<repo> "<topic>"` — never duplicate an open or closed issue that already covers the topic. Reference it instead.
1. **Check the repo's conventions.** `gh issue list --repo <owner>/<repo> --state all --limit 20` and skim titles/labels/body structure of recent issues. Match that style.

## Title Rules

- **No internal prefixes.** Never `TASK 12: ...`, `TASK-123: ...`, or working-document IDs — those mean nothing to a public audience.
- Descriptive, matching the repo's existing style (imperative or noun phrase, e.g. "Add --fix to apply machine-applicable suggestions", "Report when ignore comments can be removed").
- One short sentence; no trailing period.
- If the repo uses labels (bug / enhancement / feature request), add the matching ones with `--label`.

## Body Rules — the ones that actually bite

1. **GitHub renders EVERY single newline in an issue body as a hard line break (`<br>`).** There is no soft-wrap collapsing. Therefore:
   - Each paragraph is **one continuous line** — never wrap text at 80 columns.
   - List items each go on **one line** (no indented continuation lines).
   - Separate blocks (paragraphs, lists, headings) with a single blank line.
   - Only fenced code blocks may contain real newlines — use them for multi-line content.
2. **Public tone, no internal context.** No "Task N", no references to private working docs (HANDOFF.md, backlog files, agent notes), no abbreviations only the team understands. A stranger must be able to act on the issue with only the repo in front of them. Say what the rule/feature is by its public name (rule IDs like C007 are fine — they're in the docs).
3. **Structure the body** with `##` sections. A proven shape:

   ```markdown
   ## Problem
   <what is broken or missing, from a user perspective>

   ## The idea / Why this is nontrivial
   <proposed change; for complex work, the constraints that make it hard>

   ## Open questions
   - <decisions that need input before implementing>

   ## Acceptance criteria (draft)
   - <verifiable outcomes>
   ```

4. **Write the body to a temp file** (`/tmp/issue-<n>.md`) with the write tool and pass it via `--body-file`. Never inline long markdown in shell arguments — quoting will mangle it.
5. **If the body was extracted from a larger document** (sed/awk/head), verify the extraction boundaries: a section header from the NEXT section can leak into the end of your file. Check the last lines of the extracted file before submitting.

## Workflow

1. Write title + body file per the rules above.
2. Create:

   ```bash
   gh issue create --repo <owner>/<repo> --title "<title>" --body-file /tmp/issue-<n>.md
   ```

   Use the `gh_cli` tool where available (it validates against an allowlist and parses JSON).
3. **Verify after creation** — this step is mandatory, it is how the previous mistakes were caught:

   - Fetch the rendered HTML and confirm there are **zero `<br>` tags outside code blocks**:

     ```bash
     gh api graphql -f query='query { repository(owner:"<owner>", name:"<repo>") { issue(number:<n>) { bodyHTML } } }' \
       --jq '.data.repository.issue.bodyHTML' | grep -c "<br"
     ```

     Expect `0` (grep exits 1 with no matches — that is success).
   - Re-read the stored body (`gh issue view <n>`) and check: no stray headings at the end, no internal jargon, title clean.
4. Report the issue URL. Leave the temp body file in place for reference.

## Editing an Existing Issue

Same rules apply to `gh issue edit <n> --title ... --body-file ...` — unwrapped single-line paragraphs, clean title, then re-verify with the GraphQL check.

## Edge Cases

| Scenario | Action |
| --- | --- |
| Existing issue covers the topic | Don't create a duplicate; link/reference the existing one |
| Repo has label conventions | Add labels with `--label` matching existing usage |
| Body extracted from a draft doc | Check the tail of the extracted file for leaked headings |
| Rendered body shows `<br>` breaks | Unwrap paragraphs to single lines, re-edit, re-verify |
| User wants an internal/rough draft | Ask — public issues default to the public style above |
