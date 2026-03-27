#!/usr/bin/env bash
# Unified Benchmark Runner
#
# Runs all benchmarks in all languages and produces a comparison table.
# Each benchmark is run BENCH_RUNS times (default 5) and the median
# time is recorded, reducing noise from OS/CI scheduling jitter.
#
# Usage:
#   ./run.sh             # Run all benchmarks
#   ./run.sh fibonacci   # Run only fibonacci benchmark

set -e
cd "$(dirname "$0")"

# Configuration
BENCHMARKS="fibonacci collections primes skynet pingpong fanout"
LANGUAGES="seq python go rust"
RESULTS_DIR="results"
SEQC="../target/release/seqc"
BENCH_RUNS="${BENCH_RUNS:-5}"  # Number of runs per benchmark; override with env var

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m'

# Parse arguments
FILTER="${1:-}"

# Setup
mkdir -p "$RESULTS_DIR"
rm -f "$RESULTS_DIR"/*.txt

echo -e "${GREEN}${BOLD}=== Seq Benchmark Suite ===${NC}"
echo -e "Runs per benchmark: ${BENCH_RUNS} (median selected)"
echo

# Check dependencies
HAS_SEQ=true
HAS_PYTHON=true
HAS_GO=true
HAS_RUST=true

if [ ! -f "$SEQC" ]; then
    echo -e "${CYAN}Building seqc...${NC}"
    (cd .. && cargo build --release -p seq-compiler 2>/dev/null) || HAS_SEQ=false
fi

command -v python3 &>/dev/null || { echo -e "${YELLOW}Warning: python3 not found${NC}"; HAS_PYTHON=false; }
command -v go &>/dev/null || { echo -e "${YELLOW}Warning: go not found${NC}"; HAS_GO=false; }
command -v rustc &>/dev/null || { echo -e "${YELLOW}Warning: rustc not found${NC}"; HAS_RUST=false; }

echo

# Build a single benchmark binary (compile once, run many times)
build_bench() {
    local bench=$1
    local lang=$2

    case $lang in
        seq)
            [ "$HAS_SEQ" = false ] && return 1
            local src="$bench/seq.seq"
            local bin="/tmp/bench_${bench}_seq"
            [ -f "$src" ] && "$SEQC" build "$src" -o "$bin" 2>/dev/null
            ;;
        python)
            [ "$HAS_PYTHON" = false ] && return 1
            [ -f "$bench/python.py" ]
            ;;
        go)
            [ "$HAS_GO" = false ] && return 1
            local src="$bench/go.go"
            local bin="/tmp/bench_${bench}_go"
            [ -f "$src" ] && go build -o "$bin" "$src" 2>/dev/null
            ;;
        rust)
            [ "$HAS_RUST" = false ] && return 1
            local src="$bench/rust.rs"
            local bin="/tmp/bench_${bench}_rust"
            [ -f "$src" ] && rustc -O -o "$bin" "$src" 2>/dev/null
            ;;
    esac
}

# Run an already-built benchmark binary once, output to stdout
run_bench_once() {
    local bench=$1
    local lang=$2

    case $lang in
        seq)    /tmp/bench_${bench}_seq 2>&1 ;;
        python) python3 "$bench/python.py" 2>&1 ;;
        go)     /tmp/bench_${bench}_go 2>&1 ;;
        rust)   /tmp/bench_${bench}_rust 2>&1 ;;
    esac
}

# Given N result files (one per run), compute the median time for each
# BENCH: line and write a single output file with median times.
#
# For each unique test key (BENCH:category:test:result), collect all
# times across runs, sort them, and pick the middle value.
compute_median_results() {
    local output_file=$1
    shift
    local run_files=("$@")

    # Collect all unique test keys (everything except the trailing time)
    local keys_file
    keys_file=$(mktemp)
    for f in "${run_files[@]}"; do
        grep "^BENCH:" "$f" 2>/dev/null | while IFS= read -r line; do
            # Key = first 4 colon-separated fields: BENCH:cat:test:result
            echo "$line" | rev | cut -d: -f2- | rev
        done
    done | sort -u > "$keys_file"

    # For each key, collect times and pick median
    > "$output_file"
    while IFS= read -r key; do
        local times=()
        for f in "${run_files[@]}"; do
            local t
            t=$(grep "^${key}:" "$f" 2>/dev/null | rev | cut -d: -f1 | rev)
            if [[ "$t" =~ ^[0-9]+$ ]]; then
                times+=("$t")
            fi
        done

        if [ ${#times[@]} -eq 0 ]; then
            continue
        fi

        # Sort numerically and pick the middle element
        local sorted
        sorted=($(printf '%s\n' "${times[@]}" | sort -n))
        local mid=$(( ${#sorted[@]} / 2 ))
        local median=${sorted[$mid]}

        echo "${key}:${median}" >> "$output_file"
    done < "$keys_file"

    rm -f "$keys_file"
}

# Run benchmarks
for bench in $BENCHMARKS; do
    [ -n "$FILTER" ] && [ "$bench" != "$FILTER" ] && continue

    echo -e "${CYAN}Running $bench benchmark...${NC}"
    for lang in $LANGUAGES; do
        printf "  %-8s " "$lang"

        output_file="$RESULTS_DIR/${bench}_${lang}.txt"

        # Build once
        if ! build_bench "$bench" "$lang" 2>/dev/null; then
            echo "SKIP:$bench:$lang:not available" > "$output_file"
            echo -e "${YELLOW}skipped${NC}"
            continue
        fi

        # Run BENCH_RUNS times, collect per-run results
        run_files=()
        run_ok=true
        for i in $(seq 1 "$BENCH_RUNS"); do
            run_file=$(mktemp)
            if run_bench_once "$bench" "$lang" > "$run_file" 2>&1; then
                if grep -q "^BENCH:" "$run_file" 2>/dev/null; then
                    run_files+=("$run_file")
                else
                    rm -f "$run_file"
                fi
            else
                rm -f "$run_file"
            fi
        done

        if [ ${#run_files[@]} -eq 0 ]; then
            echo "ERROR:$bench:$lang:failed" > "$output_file"
            echo -e "${RED}✗${NC}"
            continue
        fi

        # Compute median across runs
        compute_median_results "$output_file" "${run_files[@]}"

        # Clean up temp files
        rm -f "${run_files[@]}"

        if grep -q "^BENCH:" "$output_file" 2>/dev/null; then
            echo -e "${GREEN}✓${NC} (${#run_files[@]}/${BENCH_RUNS} runs)"
        else
            echo -e "${RED}✗${NC}"
        fi
    done
    echo
done

# Generate report
echo -e "${GREEN}${BOLD}=== Results (median of ${BENCH_RUNS} runs) ===${NC}"
echo

# Helper to get time from results
get_time() {
    local suite=$1 test=$2 lang=$3
    local file="$RESULTS_DIR/${suite}_${lang}.txt"
    [ -f "$file" ] || { echo "-"; return; }
    local time=$(grep "^BENCH:${suite}:${test}:" "$file" 2>/dev/null | cut -d: -f5)
    [ -n "$time" ] && echo "${time} ms" || echo "-"
}

# Print table
print_table() {
    local suite=$1
    shift
    local tests=("$@")

    echo -e "${BOLD}$suite${NC}"
    printf "%-25s %12s %12s %12s %12s\n" "Test" "Seq" "Python" "Go" "Rust"
    printf "%-25s %12s %12s %12s %12s\n" "------------------------" "----------" "----------" "----------" "----------"

    for test in "${tests[@]}"; do
        printf "%-25s %12s %12s %12s %12s\n" \
            "$test" \
            "$(get_time "$suite" "$test" seq)" \
            "$(get_time "$suite" "$test" python)" \
            "$(get_time "$suite" "$test" go)" \
            "$(get_time "$suite" "$test" rust)"
    done
    echo
}

print_table "fibonacci" "fib-naive-30" "fib-naive-35" "fib-fast-30" "fib-fast-50" "fib-naive-20-x1000" "fib-fast-20-x1000"
print_table "collections" "build-100k" "map-double" "filter-evens" "fold-sum" "chain"
print_table "primes" "count-10k" "count-100k"
print_table "skynet" "spawn-100k"
print_table "pingpong" "roundtrip-100k"
print_table "fanout" "throughput-100k"

echo -e "${CYAN}Note: Python concurrency uses asyncio (cooperative, single-threaded).${NC}"
echo -e "${CYAN}      Go/Seq/Rust use lightweight threads or OS threads.${NC}"

# Update LATEST_RUN.txt for CI freshness check
cat > LATEST_RUN.txt << EOF
# Benchmark run record - DO NOT EDIT MANUALLY
# This file is checked by CI to ensure benchmarks are run regularly
timestamp: $(date -u +"%Y-%m-%dT%H:%M:%SZ")
commit: $(git rev-parse --short HEAD 2>/dev/null || echo "unknown")
benchmarks_run: ${FILTER:-all}
runs_per_benchmark: ${BENCH_RUNS}
EOF

echo
echo -e "${GREEN}Updated LATEST_RUN.txt${NC}"
