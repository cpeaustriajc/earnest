#!/bin/bash
N=/SCRATCH
S=/SCRATCH_OLDER
NEW="$1"; NAME="$2"; OLD="${OLD:-$N/rows/earnest-fix5}"
cd "$N/loop/probe" || exit 1
uptime
for pair in "$OLD before" "$NEW $NAME"; do
  set -- $pair; b=$1; n=$2
  for j in 1 4 10; do ./measure.sh "$b" "$n corridor j$j" --jobs $j --max-states 20000000 corridor.model.ts; done
done
cd "$N/loop/ir/lights" || exit 1
for pair in "$OLD before" "$NEW $NAME"; do
  set -- $pair; b=$1; n=$2
  for j in 1 4; do "$N/loop/probe/measure.sh" "$b" "$n lights j$j" --jobs $j --max-states 20000000 lights.model.ts; done
done
cd "$S/models-new" || exit 1
for pair in "$OLD before" "$NEW $NAME"; do
  set -- $pair; b=$1; n=$2
  for m in grid_10m grid_1m wide; do for j in 1 4; do "$N/loop/probe/measure.sh" "$b" "$n $m j$j" --jobs $j --max-states 20000000 $m.model.ts; done; done
  "$N/loop/probe/measure.sh" "$b" "$n ledger j1" --jobs 1 --max-states 20000000 ledger.model.ts
  "$N/loop/probe/measure.sh" "$b" "$n grid_10m j1 time limit" --jobs 1 --max-states 20000000 --time-limit 3600 grid_10m.model.ts
  "$N/loop/probe/measure.sh" "$b" "$n grid_10m j4 time limit" --jobs 4 --max-states 20000000 --time-limit 3600 grid_10m.model.ts
  for lim in 5000000 4700000 2000000; do for j in 1 4; do "$N/loop/probe/measure.sh" "$b" "$n grid_10m j$j limit $lim" --jobs $j --max-states $lim grid_10m.model.ts; done; done
done
uptime
