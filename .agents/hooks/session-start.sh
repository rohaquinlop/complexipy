#!/bin/sh
# agent-hooks: accept the tree as the session finds it, unless the gate already
# reported it failing, so the gate judges only later changes. Then warm the
# build caches in the background, so the first gate does not pay for a cold
# build.
. "$(dirname "$0")/lib.sh"
reported || judge
(
	cargo clippy -q --workspace --all-targets
	cargo check -q -p complexipy-core --no-default-features
	cargo test -q --workspace --no-run
	uv run --no-sync maturin develop
) </dev/null >/dev/null 2>&1 &
