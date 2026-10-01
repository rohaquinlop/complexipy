#!/bin/sh
# claude-hooks: run the gate when Claude ends a turn. A failure blocks the stop
# up to 3 times per prompt. After that the user is told, and the same tree is
# not blocked again until it changes.
input=$(cat)
cd "$CLAUDE_PROJECT_DIR" || exit 0
. .agents/hooks/lib.sh
reported && exit 0
attempts=$STATE/claude-attempts
[ "$(printf '%s' "$input" | jq -r '.stop_hook_active')" = true ] || echo 0 >"$attempts"
out=$(sh .agents/hooks/gate.sh) && exit 0
n=$(($(cat "$attempts" 2>/dev/null || echo 0) + 1))
echo "$n" >"$attempts"
if [ "$n" -le 3 ]; then
	jq -n --arg reason "Quality gate failed. Fix these before you finish:
$out" '{decision: "block", reason: $reason}'
	exit 0
fi
mark_reported
failed=$(printf '%s\n' "$out" | sed -n 's/^== \(.*\) failed ==$/\1/p' | paste -sd ' ' -)
jq -n --arg msg "Quality gate still fails after 3 fix attempts: $failed. Logs: $STATE" \
	'{systemMessage: $msg}'
