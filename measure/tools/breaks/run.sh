#!/bin/bash
N=/SCRATCH
B=/SCRATCH/loop/check/breaks
T=/WORKTREES/perf-check
mk() {
  rm -rf "$B/src-$1"; mkdir -p "$B/src-$1"
  rsync -a --exclude target --exclude .git --exclude node_modules "$T/" "$B/src-$1/"
  python3 -I "$B/patch.py" "$1" "$B/src-$1" && echo "prepared break $1 on $(git -C "$T" rev-parse --short HEAD)"
}
show='^test .*FAILED|left:|right:|test result|failed:$|^    [a-z_:]+$'
mk hold_after_swap_plain
(cd "$B/src-hold_after_swap_plain" && CARGO_TARGET_DIR=$B/target cargo test --locked -p earnest_check --lib explore::workers 2>&1 | grep -E "$show")
mk through_dropped
(cd "$B/src-through_dropped" && CARGO_TARGET_DIR=$B/target cargo test --locked -p earnest_check --test explore a_time_stop_in_the_last_pass 2>&1 | grep -E "$show")
mk merge_skips_a_task_on_ten_threads
(cd "$B/src-merge_skips_a_task_on_ten_threads" && CARGO_TARGET_DIR=$B/target cargo test --locked --no-fail-fast -p earnest_check 2>&1 | grep -E "$show")
rm -rf "$B/target" "$B"/src-*
