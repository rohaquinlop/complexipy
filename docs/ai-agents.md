# AI Agent Workflows

Coding agents such as Claude Code and Codex CLI write code in a loop: edit
files, run tools, and continue from the output. Cognitive complexity measures
how hard that code is to review, and a function over the limit is cheapest to
fix while the agent still has the file in context, if something tells the agent
about it.

Hooks are commands an agent runs at lifecycle events. A hook that runs
complexipy puts the analysis in the same turn, before a human reads the diff.
This page shows two hooks for both agents: one after each file edit, one when
the agent tries to finish.

## What the hooks use from complexipy

complexipy exits with status 1 when a function is above
`max-complexity-allowed`, and 0 when the code passes. A hook maps that exit code
to the agent's feedback protocol, so the hook itself stays small.

The recipes check the diff instead of the whole repository:

- `complexipy . --diff HEAD` checks the uncommitted changes on each edit.
- `complexipy . --diff main` checks the branch when the turn ends.

`--quiet` keeps the exit code and drops the terminal report, because the hook
prints its own message. When you want the function list in the feedback
instead, `--failed --plain` prints one line per function over the limit:

```bash
complexipy src/app.py --failed --plain --max-complexity-allowed 15
```

The output is one line per function over the limit:

```text
src/app.py handle_order 22
```

The scripts are POSIX `sh` and expect `complexipy` on the `PATH`. They work in
git repositories, because `--diff` needs a reference to compare against.

Both hooks compare a git reference, not the file that just changed: the edit
hook checks every uncommitted change, and the Stop hook checks the whole branch
against `main`. Start the agent from a clean tree, or commit or stash your own
work, so the feedback is about the agent's changes.

## The edit hook

Save this script in the hook directory of your agent and make it executable:

```sh
#!/bin/sh
# Reports the functions above the cognitive complexity limit that the working
# tree adds or worsens, so the agent fixes them before the turn continues.
limit=${COMPLEXIPY_MAX_COMPLEXITY:-15}
ref=${COMPLEXIPY_EDIT_DIFF_REF:-HEAD}
root=$(git rev-parse --show-toplevel 2>/dev/null) || exit 0
cd "$root" || exit 0
complexipy . --diff "$ref" --max-complexity-allowed "$limit" --quiet
status=$?
if [ "$status" -eq 0 ]; then
    exit 0
fi
if [ "$status" -ne 1 ]; then
    printf '%s\n' "complexipy could not check the working tree (exit $status)." >&2
    exit 1
fi
printf '%s\n' "complexipy: the working tree adds or worsens a function above the cognitive complexity limit of $limit." >&2
printf '%s\n' "Run 'complexipy . --diff $ref --failed' to list those functions, then refactor them." >&2
exit 2
```

The script runs after each edit, so `--diff HEAD` limits the analysis to the
functions the edits introduced or worsened. Exit status 2 is the feedback signal
for both agents: Claude Code shows the stderr text to the model, and Codex CLI
replaces the tool result with it.

## The end-of-turn hook

Save this script next to the edit hook:

```sh
#!/bin/sh
# Reports the functions above the cognitive complexity limit that the branch
# adds or worsens, so the agent keeps working instead of stopping.
limit=${COMPLEXIPY_MAX_COMPLEXITY:-15}
ref=${COMPLEXIPY_DIFF_REF:-main}
input=$(cat)
if printf '%s' "$input" | grep -Eq '"stop_hook_active"[[:space:]]*:[[:space:]]*true'; then
    exit 0
fi
root=$(git rev-parse --show-toplevel 2>/dev/null) || exit 0
cd "$root" || exit 0
complexipy . --diff "$ref" --max-complexity-allowed "$limit" --quiet
status=$?
if [ "$status" -eq 0 ]; then
    exit 0
fi
if [ "$status" -ne 1 ]; then
    printf '%s\n' "complexipy could not check the branch (exit $status)." >&2
    exit 1
fi
printf '%s\n' "complexipy: the branch adds or worsens a function above the cognitive complexity limit of $limit." >&2
printf '%s\n' "Run 'complexipy . --diff $ref --failed' to list those functions, then refactor them." >&2
exit 2
```

It compares the branch with `main` and runs again whenever the agent tries to
stop. A failing status sends the agent back to work with the message; a
passing one lets the turn end. The script reads `stop_hook_active`, so it
blocks at most once per turn. Set `COMPLEXIPY_DIFF_REF` when your base branch
is not `main`, and `COMPLEXIPY_MAX_COMPLEXITY` when you use a different limit.
The edit hook takes its own reference from `COMPLEXIPY_EDIT_DIFF_REF` (default
`HEAD`), which helps when the agent commits after each edit.

A Stop hook fires at the end of every response, not only when the task is
complete, so the check runs on each turn and blocks when the branch adds or
worsens a function above the limit.

Both agents pass `stop_hook_active` in the hook input. The script exits 0 while
that flag is true, which keeps a Stop hook from blocking forever. Claude Code
also cuts a Stop hook after eight consecutive blocks.

## Claude Code

Copy both scripts to `.claude/hooks/` in the repository and make them
executable:

```bash
chmod +x .claude/hooks/complexipy-edit.sh .claude/hooks/complexipy-stop.sh
```

Add both events to `.claude/settings.json`. If the file already has a `hooks`
object, add these entries as siblings of the existing event keys:

```json
{
  "hooks": {
    "PostToolUse": [
      {
        "matcher": "Edit|Write",
        "hooks": [
          {
            "type": "command",
            "command": "\"$CLAUDE_PROJECT_DIR\"/.claude/hooks/complexipy-edit.sh"
          }
        ]
      }
    ],
    "Stop": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "\"$CLAUDE_PROJECT_DIR\"/.claude/hooks/complexipy-stop.sh",
            "timeout": 120
          }
        ]
      }
    ]
  }
}
```

Claude Code normally reloads hooks when the settings file changes; restart
the session if a new hook does not appear. Run `/hooks` to confirm both appear
under `PostToolUse` and `Stop`.

Claude Code runs project hooks only after you trust the workspace.

To test a hook without an agent turn, pipe the input the event sends on stdin
and check the exit code:

```bash
echo '{"hook_event_name":"Stop","stop_hook_active":false}' | .claude/hooks/complexipy-stop.sh
```

Exit 0 lets the turn end, and exit 2 blocks the stop and passes the message to
the agent.
See the [Claude Code hooks reference](https://code.claude.com/docs/en/hooks)
for the event fields and the exit code rules, and the
[hooks guide](https://code.claude.com/docs/en/hooks-guide) for more examples.

## Codex CLI

Copy both scripts to `.codex/hooks/`, then create `.codex/hooks.json`:

```json
{
  "hooks": {
    "PostToolUse": [
      {
        "matcher": "apply_patch|Edit|Write",
        "hooks": [
          {
            "type": "command",
            "command": "sh \"$(git rev-parse --show-toplevel)/.codex/hooks/complexipy-edit.sh\"",
            "timeout": 120,
            "statusMessage": "Checking cognitive complexity"
          }
        ]
      }
    ],
    "Stop": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "sh \"$(git rev-parse --show-toplevel)/.codex/hooks/complexipy-stop.sh\"",
            "timeout": 120,
            "statusMessage": "Checking cognitive complexity"
          }
        ]
      }
    ]
  }
}
```

The commands resolve the scripts from the git root because Codex can start in a
subdirectory. Codex runs a project hook only after you trust the project
`.codex` layer and review the hook with `/hooks`, and it asks again when the
hook changes. The same hooks can be declared inline in `.codex/config.toml`
under `[[hooks.PostToolUse]]` and `[[hooks.Stop]]` if you prefer one file. See
the [Codex hooks reference](https://learn.chatgpt.com/docs/hooks) for the full
schema and the trust rules.

## Recipes without hooks

The same verdict works outside the agent loop:

- Gate the branch in pre-commit or CI with
  `complexipy . --diff main --max-complexity-allowed 15`; see [CI/CD](ci.md).
- Ask for a plan with `complexipy <path> --suggest-refactors`, review the patch
  with `complexipy <path> --fix --dry-run`, and apply it with `--fix`.
- Record the rule in the instruction file the agent reads (`AGENTS.md` or
  `CLAUDE.md`): "Keep every function at or below 15 cognitive complexity. Before
  you finish, run `complexipy . --diff main --max-complexity-allowed 15` and
  refactor anything it flags."
- Point the editor at the language server when the agent reads diagnostics; see
  [Editor Integration](editors.md).

## Notes

- `--plain` prints `<path> <function> <complexity>` without line numbers
  ([#326](https://github.com/rohaquinlop/complexipy/issues/326) is still open).
  Machine-readable output on stdout is not part of 8.3.0; `--output -` is on
  `main` now and ships in the next release. The hooks here use only the exit
  code, so they work with 8.3.0 and later.
- The hooks treat exit 1 as a limit breach, and complexipy also exits 1 for
  some configuration errors. When the feedback does not match a function in
  the diff, run the same command without `--quiet` to read the message.
- `--diff` needs Git 2.30 or newer. If the reference does not exist, for
  example when the default branch is `master`, complexipy treats every
  function as new and the hook reports a failure; set the reference on the
  first run.
- Hooks run with your permissions. Review the scripts before you commit them to
  a repository, like any other command a tool executes for you.
- The scripts need a POSIX shell. On Windows, install Git for Windows, which
  provides `sh`, and make sure the agent finds it on the `PATH`.
- Hook formats change between agent releases. The examples follow the
  references linked above, and the scripts need no change when the
  configuration schema moves.
