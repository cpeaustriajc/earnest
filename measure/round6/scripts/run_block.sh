#!/usr/bin/env bash
# Runs `earnest verify` round-robin over variants and keeps every output.
# usage: run_block.sh <name> <measured_rounds> <warmup_rounds> <model> <variant>...
# variant = label:build:jobs:cpus   build -> /tmp/bin/earnest-<build>; cpus empty = no taskset,
# else `taskset -c <cpus>` in front of the command. Round numbers below 1 are not counted.
# Run from the repository root.
set -u
name=$1; rounds=$2; warm=$3; model=$4; shift 4
log=measure/round6/raw/$name.log
{
  echo "# block=$name model=$model max_states=200000000 variants=$* warmup_rounds=$warm measured_rounds=$rounds"
  echo "# started=$(date -u +%FT%TZ)"
  echo "# loadavg_before=$(cat /proc/loadavg)"
} > "$log"
for r in $(seq $((1 - warm)) "$rounds"); do
  for v in "$@"; do
    IFS=: read -r L X J C <<< "$v"
    pre=(); [ -n "$C" ] && pre=(taskset -c "$C")
    echo "### run round=$r variant=$L build=$X jobs=$J cpus=${C:-all}" >> "$log"
    t0=$(date +%s%N)
    /usr/bin/time -v "${pre[@]}" /tmp/bin/earnest-"$X" verify --no-cache --json --jobs "$J" --max-states 200000000 "$model" >> "$log" 2>&1
    rc=$?
    t1=$(date +%s%N)
    echo "### end exit=$rc outer_wall_ns=$((t1 - t0))" >> "$log"
  done
done
{
  echo "# loadavg_after=$(cat /proc/loadavg)"
  echo "# finished=$(date -u +%FT%TZ)"
} >> "$log"
