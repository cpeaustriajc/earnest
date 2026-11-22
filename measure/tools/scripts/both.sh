#!/bin/bash
N=/SCRATCH
OLD="$1"; NEW="$2"; ROUNDS="$3"; shift 3
for round in $(seq "$ROUNDS"); do
  "$N/loop/probe/measure.sh" "$OLD" "old" "$@"
  "$N/loop/probe/measure.sh" "$NEW" "new" "$@"
done
