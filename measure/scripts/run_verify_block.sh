#!/usr/bin/env bash
# Runs `earnest verify` round-robin over build:jobs combinations and keeps every output.
# usage: run_verify_block.sh <name> <measured_rounds> <warmup_rounds> <max_states> <model> <build:jobs>...
# Run from the repository root. Builds are /tmp/earnest-A, /tmp/earnest-B, /tmp/earnest-C.
# Round numbers below 1 are unmeasured warm-up rounds.
set -u
name=$1; rounds=$2; warm=$3; max=$4; model=$5; shift 5
log=measure/raw/$name.log
mkdir -p measure/raw
{
  echo "# block=$name model=$model max_states=$max combos=$* warmup_rounds=$warm measured_rounds=$rounds"
  echo "# started=$(date -u +%FT%TZ)"
  echo "# loadavg_before=$(cat /proc/loadavg)"
} > "$log"
for r in $(seq $((1 - warm)) "$rounds"); do
  for c in "$@"; do
    X=${c%%:*}; J=${c##*:}
    echo "### run round=$r build=$X jobs=$J" >> "$log"
    t0=$(date +%s%N)
    /usr/bin/time -v /tmp/earnest-"$X" verify --no-cache --json --jobs "$J" --max-states "$max" "$model" >> "$log" 2>&1
    rc=$?
    t1=$(date +%s%N)
    echo "### end exit=$rc outer_wall_ns=$((t1 - t0))" >> "$log"
  done
done
{
  echo "# loadavg_after=$(cat /proc/loadavg)"
  echo "# finished=$(date -u +%FT%TZ)"
} >> "$log"
