#!/bin/sh
# pi-hooks: run ty after a .py edit. Print failures; never block.
out=$(uv run ty check . 2>&1)
code=$?
if [ "$code" -ne 0 ]; then
	printf '%s\n' "$out"
fi
exit 0
