# agent-hooks: helpers shared by the Claude and pi hooks. Source it from
# inside the repository; it moves to the root.
ROOT=$(git rev-parse --show-toplevel) || exit 0
cd "$ROOT" || exit 0
STATE=$(git rev-parse --absolute-git-dir)/agent-hooks
mkdir -p "$STATE"
report=""

check() {
	name=$1
	shift
	out=$("$@" 2>&1) && return 0
	lines=$(($(printf '%s\n' "$out" | wc -l)))
	if [ "$lines" -gt 80 ]; then
		log=$STATE/$name.log
		printf '%s\n' "$out" >"$log"
		out="$(printf '%s\n' "$out" | head -n 40)
[... $((lines - 80)) lines cut, full output in $log ...]
$(printf '%s\n' "$out" | tail -n 40)"
	fi
	report="$report== $name failed ==
$out
"
	return 1
}

fingerprint() {
	paths=$(git ls-files -co --exclude-standard -- "$@" | while IFS= read -r f; do
		[ -f "$f" ] && printf '%s\n' "$f"
	done)
	{
		printf '%s\n' "$paths"
		[ -z "$paths" ] || printf '%s\n' "$paths" | git hash-object --stdin-paths
	} | git hash-object --stdin
}

all_fp() { fingerprint '*.py' '*.pyi' '*.rs' '*.toml' '*.lock'; }
rust_fp() { fingerprint '*.rs' '*Cargo.toml' 'Cargo.lock'; }
judge() { { all_fp; rust_fp; } >"$STATE/judged"; }
reported() { [ "$(all_fp)" = "$(cat "$STATE/reported" 2>/dev/null)" ]; }
mark_reported() { all_fp >"$STATE/reported"; }
