#!/usr/bin/env bash
# Task 1: a = --jobs 1, b = --jobs 4, c = taskset -c 0 with --jobs 4. Run from the repository root.
measure/round6/scripts/run_block.sh task1_grid_10m 7 1 tasks/benchmark/models/grid_10m.model.ts a:tip:1: b:tip:4: c:tip:4:0
measure/round6/scripts/run_block.sh task1_grid_100m 3 0 measure/grid_100m.model.ts a:tip:1: b:tip:4: c:tip:4:0
