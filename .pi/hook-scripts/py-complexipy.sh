#!/bin/sh
# pi-hooks: dogfood complexipy on every Python file edited this turn.
# Print failures; never block.
paths=$(jq -r '.inputs[]?.path // empty' 2>/dev/null)
if [ -z "$paths" ]; then
	exit 0
fi
for path in $paths; do
	out=$(uv run complexipy "$path" 2>&1)
	code=$?
	if [ "$code" -ne 0 ]; then
		printf '%s\n' "$out"
	fi
done
exit 0
