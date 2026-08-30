#!/bin/sh
# pi-hooks: ruff lint + format check after a .py edit. Print failures; never block.
out=$(uv run ruff check . 2>&1)
code=$?
if [ "$code" -ne 0 ]; then
	printf '%s\n' "$out"
fi
out=$(uv run ruff format --check . 2>&1)
code=$?
if [ "$code" -ne 0 ]; then
	printf '%s\n' "$out"
fi
exit 0
