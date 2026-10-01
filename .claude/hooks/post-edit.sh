#!/bin/sh
# claude-hooks: fast checks on the file Claude just edited. Python is formatted
# in place. Rust is formatted by the gate: rustfmt also rewrites child modules,
# so parallel edits would race. Failures reach Claude as context and never
# block the edit.
path=$(jq -r '.tool_input.file_path // empty')
cd "$CLAUDE_PROJECT_DIR" || exit 0
. .agents/hooks/lib.sh
rel=${path#"$ROOT"/}
case $rel in
/*) ;;
*.py | *.pyi)
	uv run --no-sync ruff format --force-exclude -q "$rel" >/dev/null 2>&1
	check ruff-check uv run --no-sync ruff check --force-exclude -q "$rel"
	check ty uv run --no-sync ty check --force-exclude "$rel"
	case $rel in
	complexipy/* | crates/*) check complexipy uv run --no-sync complexipy "$rel" ;;
	esac
	;;
crates/*/*.rs)
	crate=${rel#crates/}
	crate=${crate%%/*}
	check "clippy-$crate" cargo clippy -q -p "$crate" --all-targets -- -D warnings
	;;
esac
[ -n "$report" ] || exit 0
jq -n --arg ctx "$report" \
	'{hookSpecificOutput: {hookEventName: "PostToolUse", additionalContext: $ctx}}'
