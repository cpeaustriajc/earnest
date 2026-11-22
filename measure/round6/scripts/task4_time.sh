#!/usr/bin/env bash
# Task 4 timing: tip against the two-pass claim with batches of 16, 8 and 32, all --jobs 4, interleaved.
measure/round6/scripts/run_block.sh task4_grid_10m 7 1 tasks/benchmark/models/grid_10m.model.ts tip:tip:4: p16:p16:4: p8:p8:4: p32:p32:4:
measure/round6/scripts/run_block.sh task4_grid_100m 3 0 measure/grid_100m.model.ts tip:tip:4: p16:p16:4: p8:p8:4: p32:p32:4:
