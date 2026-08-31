#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BASELINE_DIR="${BASELINE_DIR:-$HOME/.cache/complexipy-benchmarks/baseline}"
CORPUS_DIR="$REPO_ROOT/benchmarks/corpus"
NEW_CLI="$REPO_ROOT/.venv/bin/complexipy"
OLD_CLI="$BASELINE_DIR/.venv/bin/complexipy"
RESULTS_FILE="$REPO_ROOT/benchmarks/results.md"
PROBE="$CORPUS_DIR/requests/src/requests/__init__.py"

RUNS="${RUNS:-5}"
PROBE_RUNS="${PROBE_RUNS:-20}"
WARMUP=3
PROBE_WARMUP=5
SCALING_DIR="${SCALING_DIR:-$HOME/.cache/complexipy-benchmarks/scaling}"
BASE_FUNCTIONS="${BASE_FUNCTIONS:-285}"
SCALING_RUNS="${SCALING_RUNS:-5}"

workload_names="requests flask django"

corpus_url() {
    case "$1" in
        requests) echo "https://github.com/psf/requests.git" ;;
        flask) echo "https://github.com/pallets/flask.git" ;;
        django) echo "https://github.com/django/django.git" ;;
    esac
}

corpus_sha() {
    case "$1" in
        requests) echo "5460f467b02e49471c0fd6cfc9ca0adab6351f98" ;;
        flask) echo "d318b683471101618febed18996405ad26462110" ;;
        django) echo "0b40210e4808937a7c0922e8b7502bff4752faa3" ;;
    esac
}

fail() {
    echo "error: $1" >&2
    exit 1
}

[[ "$(uname)" == "Darwin" ]] || fail "this script targets macOS (/usr/bin/time -l)"

for tool in git hyperfine uv; do
    command -v "$tool" >/dev/null 2>&1 || fail "$tool is required"
done

provision_new_cli() {
    if [[ ! -x "$NEW_CLI" ]]; then
        fail "$NEW_CLI is missing; run: uv run maturin develop --release"
    fi
}

provision_baseline() {
    if [[ ! -x "$OLD_CLI" ]]; then
        mkdir -p "$(dirname "$BASELINE_DIR")"
        uv init --bare --name complexipy-baseline --python 3.12 "$BASELINE_DIR" >/dev/null
        (cd "$BASELINE_DIR" && uv add "complexipy==7.0.1" >/dev/null)
    fi
    [[ -x "$OLD_CLI" ]] || fail "baseline provisioning failed"
    "$OLD_CLI" --version 2>/dev/null | grep -q "7.0.1" || fail "baseline CLI must report 7.0.1"
}

provision_corpus() {
    mkdir -p "$CORPUS_DIR"
    for name in $workload_names; do
        dir="$CORPUS_DIR/$name"
        sha="$(corpus_sha "$name")"
        if [[ ! -d "$dir/.git" ]]; then
            git clone --quiet --depth 1 "$(corpus_url "$name")" "$dir"
        fi
        if [[ "$(git -C "$dir" rev-parse HEAD)" != "$sha"* ]]; then
            git -C "$dir" fetch --quiet --depth 1 origin "$sha"
            git -C "$dir" checkout --quiet "$sha"
        fi
    done
    [[ -f "$PROBE" ]] || fail "single-file probe missing: $PROBE"
}

parity_check() {
    local dir new_exit old_exit
    dir="$(mktemp -d)"
    for target in "$CORPUS_DIR/requests" "$PROBE"; do
        (cd "$dir" && "$NEW_CLI" "$target" --output-format json --max-complexity-allowed 1000 >/dev/null 2>&1)
        new_exit=$?
        [[ -f "$dir/complexipy-results.json" ]] && mv "$dir/complexipy-results.json" "$dir/new.json"
        (cd "$dir" && "$OLD_CLI" "$target" --output-format json --max-complexity-allowed 1000 >/dev/null 2>&1)
        old_exit=$?
        [[ -f "$dir/complexipy-results.json" ]] && mv "$dir/complexipy-results.json" "$dir/old.json"
        [[ "$new_exit" == "$old_exit" ]] || fail "parity: exit codes differ on $target ($new_exit vs $old_exit)"
        cmp -s "$dir/new.json" "$dir/old.json" || fail "parity: JSON exports differ on $target"
    done
    rm -rf "$dir"
    echo "  parity: byte-identical JSON exports and exit codes (requests + single-file probe)"
}

run_hyperfine() {
    local label="$1"
    local warmup="$2"
    local runs="$3"
    local out_json="$4"
    shift 4
    hyperfine \
        --warmup "$warmup" \
        --runs "$runs" \
        --style basic \
        --ignore-failure \
        --export-json "$out_json" \
        -n "8.0.0 rust cli (pre-release)" \
        -n "7.0.1 python cli" \
        "$@" \
        | tail -2 | head -1
    echo "  [hyperfine] $label done"
}

measure_rss() {
    local cli="$1"
    local target="$2"
    local mode="$3"
    local log="$4"
    local max=0
    local rss
    for _ in 1 2 3; do
        /usr/bin/time -l "$cli" "$target" $mode >/dev/null 2>"$log" || true
        rss="$(awk '/maximum resident set size/ {print $1}' "$log")"
        if [[ -n "$rss" && "$rss" -gt "$max" ]]; then
            max="$rss"
        fi
    done
    echo "$max"
}

scaling_mean() {
    local out_json="$1"
    shift
    hyperfine \
        --warmup "$WARMUP" \
        --runs "$SCALING_RUNS" \
        --style basic \
        --export-json "$out_json" \
        "$@" \
        >/dev/null 2>&1
    "$REPO_ROOT/.venv/bin/python" - "$out_json" <<'PY'
import json
import sys

with open(sys.argv[1]) as f:
    data = json.load(f)
print(f"{data['results'][0]['mean'] * 1000:.1f}")
PY
}

measure_scaling() {
    declare -ga scaling_rows
    local counts=()
    for size in 1 2 4; do
        local file="$SCALING_DIR/scaling_${size}x.py"
        local workdir
        workdir="$(mktemp -d)"
        (
            cd "$workdir"
            "$NEW_CLI" "$file" --output-format json --max-complexity-allowed 100000 \
                >/dev/null 2>&1
        ) || fail "scaling parity run failed for $file"
        counts+=("$(grep -c '"function_name"' "$workdir/complexipy-results.json")")
        rm -rf "$workdir"
    done
    [[ "${counts[1]}" -eq $((counts[0] * 2)) ]] || fail "2x function count ${counts[1]} != 2x ${counts[0]}"
    [[ "${counts[2]}" -eq $((counts[0] * 4)) ]] || fail "4x function count ${counts[2]} != 4x ${counts[0]}"

    local prev_ms=""
    local prev_qms=""
    for size in 1 2 4; do
        local file="$SCALING_DIR/scaling_${size}x.py"
        local lines
        lines="$(wc -l < "$file" | tr -d ' ')"
        local ms qms ratio qratio
        ms="$(scaling_mean "$tmp/hf-scaling-$size.json" "$NEW_CLI $file")"
        qms="$(scaling_mean "$tmp/hf-scaling-$size-quiet.json" "$NEW_CLI $file --quiet")"
        ratio="-"
        qratio="-"
        if [[ -n "$prev_ms" ]]; then
            ratio="$("$REPO_ROOT/.venv/bin/python" -c "print(f'{$ms / $prev_ms:.2f}')")"
            qratio="$("$REPO_ROOT/.venv/bin/python" -c "print(f'{$qms / $prev_qms:.2f}')")"
        fi
        prev_ms="$ms"
        prev_qms="$qms"
        local rss
        rss="$(measure_rss "$NEW_CLI" "$file" "--quiet" "$tmp/time-scaling.log")"
        scaling_rows+=("| ${size}x ($lines lines) | ${ms} ms | $ratio | ${qms} ms | $qratio | $((rss / 1048576)) MB |")
        echo "  [scaling] ${size}x: $ms ms (ratio $ratio), quiet $qms ms (ratio $qratio)"
    done
}

echo "== provisioning =="
provision_new_cli
echo "  new CLI: $("$NEW_CLI" --version 2>&1 | head -1)"
provision_baseline
echo "  baseline CLI: $("$OLD_CLI" --version 2>&1 | head -1) (python $("$BASELINE_DIR/.venv/bin/python" -c 'import sys; print(".".join(map(str, sys.version_info[:3])))'))"
provision_corpus
echo "  corpus: $(for n in $workload_names; do echo -n "$n@$(git -C "$CORPUS_DIR/$n" rev-parse --short HEAD) "; done)"
parity_check

uv run python benchmarks/generate_scaling_fixture.py "$BASE_FUNCTIONS" "$SCALING_DIR" >/dev/null
echo "  scaling fixture: $(wc -l < "$SCALING_DIR/scaling_1x.py" | tr -d ' ') lines (1x), sizes 1x/2x/4x"

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

declare -a rows
declare -a rss_rows
declare -a scaling_rows

for name in $workload_names; do
    target="$CORPUS_DIR/$name"
    file_count="$(find "$target" -name '*.py' -not -path '*/.git/*' | wc -l | tr -d ' ')"
    for mode in default quiet failed render; do
        case "$mode" in
            quiet) mode_flags="--quiet" ;;
            failed) mode_flags="--failed" ;;
            *) mode_flags="" ;;
        esac
        new_cmd="$NEW_CLI $target $mode_flags"
        old_cmd="$OLD_CLI $target $mode_flags"
        if [[ "$mode" == "render" ]]; then
            new_cmd="$new_cmd > \"$tmp/out-new.txt\""
            old_cmd="$old_cmd > \"$tmp/out-old.txt\""
        fi
        json="$tmp/hf-$name-$mode.json"

        echo "== $name ($file_count files) [$mode] =="
        run_hyperfine "$name [$mode]" "$WARMUP" "$RUNS" "$json" "$new_cmd" "$old_cmd"

        new_rss="$(measure_rss "$NEW_CLI" "$target" "$mode_flags" "$tmp/time-new.log")"
        old_rss="$(measure_rss "$OLD_CLI" "$target" "$mode_flags" "$tmp/time-old.log")"
        echo "  [rss] new=${new_rss}B old=${old_rss}B"

        stats="$("$REPO_ROOT/.venv/bin/python" - "$json" <<'PY'
import json
import sys

with open(sys.argv[1]) as f:
    data = json.load(f)
new = data["results"][0]
old = data["results"][1]


def fmt(seconds):
    if seconds is None:
        return "n/a"
    return f"{seconds:.3f} s"


speedup = old["mean"] / new["mean"]
print(
    f"{fmt(new['mean'])} ± {fmt(new['stddev'])}"
    f"|{fmt(old['mean'])} ± {fmt(old['stddev'])}"
    f"|{speedup:.2f}x"
)
PY
)"
        IFS='|' read -r new_wall old_wall speedup <<<"$stats"
        rows+=("| $name ($file_count files) [$mode] | $new_wall | $old_wall | $speedup |")
        rss_rows+=("| $name ($file_count files) [$mode] | $((new_rss / 1048576)) MB | $((old_rss / 1048576)) MB |")
    done
done

target="$PROBE"
for mode in default quiet failed render; do
    case "$mode" in
        quiet) mode_flags="--quiet" ;;
        failed) mode_flags="--failed" ;;
        *) mode_flags="" ;;
    esac
    new_cmd="$NEW_CLI $target $mode_flags"
    old_cmd="$OLD_CLI $target $mode_flags"
    if [[ "$mode" == "render" ]]; then
        new_cmd="$new_cmd > \"$tmp/out-new.txt\""
        old_cmd="$old_cmd > \"$tmp/out-old.txt\""
    fi
    json="$tmp/hf-probe-$mode.json"

    echo "== single-file probe [${mode}] =="
    run_hyperfine "probe [$mode]" "$PROBE_WARMUP" "$PROBE_RUNS" "$json" "$new_cmd" "$old_cmd"

    new_rss="$(measure_rss "$NEW_CLI" "$target" "$mode_flags" "$tmp/time-new.log")"
    old_rss="$(measure_rss "$OLD_CLI" "$target" "$mode_flags" "$tmp/time-old.log")"

    stats="$("$REPO_ROOT/.venv/bin/python" - "$json" <<'PY'
import json
import sys

with open(sys.argv[1]) as f:
    data = json.load(f)
new = data["results"][0]
old = data["results"][1]


def fmt(seconds):
    if seconds is None:
        return "n/a"
    return f"{seconds * 1000:.1f} ms"


speedup = old["mean"] / new["mean"]
print(
    f"{fmt(new['mean'])} ± {fmt(new['stddev'])}"
    f"|{fmt(old['mean'])} ± {fmt(old['stddev'])}"
    f"|{speedup:.2f}x"
)
PY
)"
    IFS='|' read -r new_wall old_wall speedup <<<"$stats"
    rows+=("| single-file probe [$mode] | $new_wall | $old_wall | $speedup |")
    rss_rows+=("| single-file probe [$mode] | $((new_rss / 1048576)) MB | $((old_rss / 1048576)) MB |")
done

measure_scaling

new_version="$("$NEW_CLI" --version 2>&1 | head -1)"
old_version="$("$OLD_CLI" --version 2>&1 | head -1)"
old_python="$("$BASELINE_DIR/.venv/bin/python" -c 'import sys; print(".".join(map(str, sys.version_info[:3])))')"
new_python="$("$REPO_ROOT/.venv/bin/python" -c 'import sys; print(".".join(map(str, sys.version_info[:3])))')"

{
    echo "<!-- Generated by benchmarks/benchmark-cli.sh. Do not edit. -->"
    echo
    echo "## Environment"
    echo
    echo "- Machine: $(sysctl -n hw.model 2>/dev/null || echo unknown) ($(uname -m))"
    echo "- OS: $(sw_vers -productName 2>/dev/null) $(sw_vers -productVersion 2>/dev/null)"
    echo "- CPU: $(sysctl -n machdep.cpu.brand_string 2>/dev/null)"
    echo "- RAM: $(( $(sysctl -n hw.memsize 2>/dev/null || echo 0) / 1073741824 )) GB"
    echo "- New CLI: $new_version (built from $(git -C "$REPO_ROOT" rev-parse --short HEAD), python $new_python)"
    echo "- Baseline CLI: $old_version (PyPI, python $old_python)"
    echo "- uv: $(uv --version)"
    echo "- hyperfine: $(hyperfine --version | head -1)"
    echo "- Runs: hyperfine warmup=$WARMUP runs=$RUNS (probe warmup=$PROBE_WARMUP runs=$PROBE_RUNS); RSS via /usr/bin/time -l, 3 runs, maximum"
    echo "- Modes: default, --quiet, --failed, and render (full output written to a file)"
    echo "- Scaling fixture: $BASE_FUNCTIONS base functions, sizes 1x/2x/4x, $SCALING_RUNS runs each"
    echo "- Date: $(date -u +%Y-%m-%d)"
    echo
    echo "Corpus (shallow clones at pinned commits):"
    echo
    for name in $workload_names; do
        echo "- $name @ $(corpus_sha "$name") ($(find "$CORPUS_DIR/$name" -name '*.py' -not -path '*/.git/*' | wc -l | tr -d ' ') files)"
    done
    echo "- single-file probe: requests/src/requests/__init__.py @ $(corpus_sha requests)"
    echo
    echo "## Wall time"
    echo
    echo "| Workload | 8.0.0 Rust CLI (pre-release) | 7.0.1 Python CLI | Speedup |"
    echo "| -- | -- | -- | -- |"
    for row in "${rows[@]}"; do echo "$row"; done
    echo
    echo "## Scaling (synthetic fixture)"
    echo
    echo "| Size | Mean | Ratio | --quiet mean | Ratio | Peak RSS |"
    echo "| -- | -- | -- | -- | -- | -- |"
    for row in "${scaling_rows[@]}"; do echo "$row"; done
    echo
    echo "The fixture is generated deterministically by"
    echo "benchmarks/generate_scaling_fixture.py ($BASE_FUNCTIONS base functions,"
    echo "sizes 1x/2x/4x, kept outside the repo under $SCALING_DIR; never"
    echo "committed). Ratios near 2 mean linear scoring; ratios near 4 mean"
    echo "quadratic behavior."
    echo
    echo "## Peak RSS"
    echo
    echo "| Workload | 8.0.0 Rust CLI (pre-release) | 7.0.1 Python CLI |"
    echo "| -- | -- | -- |"
    for row in "${rss_rows[@]}"; do echo "$row"; done
    echo
    echo "Notes: stdout is discarded during timing except the [render] mode,"
    echo "which writes the full output to a file; both CLIs run as bare console"
    echo "scripts with identical flags. The django workload exits 1 on"
    echo "both CLIs because its own test fixture"
    echo "(tests/test_runner_apps/tagged/tests_syntax_error.py) is intentionally"
    echo "unparseable."
} >"$RESULTS_FILE"

echo "== results written to $RESULTS_FILE =="
