#!/bin/sh
# pi-hooks: sequential Rust checks after a .rs edit. Print failures per
# section; never block the edit.

run_section() {
	name=$1
	shift
	out=$("$@" 2>&1)
	code=$?
	if [ "$code" -ne 0 ]; then
		printf '%s\n' "== $name failed =="
		printf '%s\n' "$out"
	fi
}

run_section "cargo fmt" cargo fmt
run_section "cargo clippy" cargo clippy --workspace --all-targets -- -D warnings
run_section "cargo test" cargo test --workspace
run_section "maturin develop" uv run maturin develop

exit 0
