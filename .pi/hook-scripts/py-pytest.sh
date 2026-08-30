#!/bin/sh
# pi-hooks: run pytest after a .py edit. Print failures; never block.
out=$(uv run pytest 2>&1)
code=$?
if [ "$code" -ne 0 ]; then
	printf '%s\n' "$out"
fi
exit 0
