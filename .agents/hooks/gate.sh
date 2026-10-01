#!/bin/sh
# agent-hooks: the full quality gate. It runs only when the tree changed since
# the last pass, and the Rust steps only when Rust changed. It formats in place
# and prints the failures with exit 1.
. "$(dirname "$0")/lib.sh"
[ "$(all_fp)" = "$(sed -n 1p "$STATE/judged" 2>/dev/null)" ] && exit 0
built=yes
if [ "$(rust_fp)" != "$(sed -n 2p "$STATE/judged" 2>/dev/null)" ]; then
	check cargo-fmt cargo fmt
	check clippy cargo clippy -q --workspace --all-targets -- -D warnings
	check no-default-features cargo check -q -p complexipy-core --no-default-features
	check cargo-test cargo test -q --workspace
	check maturin uv run --no-sync maturin develop || built=no
fi
check ruff-format uv run --no-sync ruff format -q .
check ruff-check uv run --no-sync ruff check -q .
check ty uv run --no-sync ty check .
check complexipy uv run --no-sync complexipy
[ "$built" = no ] || check pytest uv run --no-sync pytest -q --tb=short
[ -z "$report" ] || {
	printf '%s' "$report"
	exit 1
}
judge
