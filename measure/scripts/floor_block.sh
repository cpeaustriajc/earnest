#!/usr/bin/env bash
# Block 7: floor runs. Variants alternate inside each repeat. Run from the repository root.
log=measure/raw/block7_floor.log
{ echo "# floor: /tmp/floor = measure/floor.rs (prefetch is a no-op on x86-64); /tmp/floor_x86 = copy with _mm_prefetch (see block7_floor_x86.diff)"
  echo "# both built with: rustc -O (rustc 1.97.0)"
  echo "# loadavg_before=$(cat /proc/loadavg)"; echo "# started=$(date -u +%FT%TZ)"; } > $log
run() { # wheels variant repeats
  for i in $(seq 1 "$3"); do
    for f in floor floor_x86; do
      echo "### run binary=$f args='$1 $2' repeat=$i" >> $log
      /tmp/$f "$1" "$2" >> $log 2>&1
      echo "### end exit=$?" >> $log
    done
  done
}
run 7 serial 5
run 8 serial 3
run 7 4 5
{ echo "# loadavg_after=$(cat /proc/loadavg)"; echo "# finished=$(date -u +%FT%TZ)"; } >> $log
echo done > /tmp/floor.done
