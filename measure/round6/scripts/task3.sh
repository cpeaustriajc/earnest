#!/usr/bin/env bash
# Task 3: the floor, one thread, three ways, 5 rounds at 10M (7 wheels) and 100M (8 wheels).
# i = measure/floor.rs unchanged (its prefetch is ARM only, so nothing on x86-64)
# ii = with the x86-64 _mm_prefetch (measure/raw/block7_floor_x86.diff)
# iii = a plain load, std::hint::black_box(table.slots[table.home(h)]), in fill
# Built with rustc 1.97.0 -O. One uncounted run (repeat 0) before the counted rounds. Run from the repository root.
S=measure/round6/scripts
for v in i ii iii; do rustup run 1.97.0 rustc -O -o /tmp/bin/floor_$v $S/floor_$v.rs || exit 1; done
log=measure/round6/raw/task3_floor.log
{ echo "# floor variants i ii iii, rustc 1.97.0 -O"; sha256sum /tmp/bin/floor_i /tmp/bin/floor_ii /tmp/bin/floor_iii | sed 's/^/# /'
  echo "# loadavg_before=$(cat /proc/loadavg)"; echo "# started=$(date -u +%FT%TZ)"; } > $log
echo "### run binary=floor_i args='7 serial' repeat=0" >> $log; /tmp/bin/floor_i 7 serial >> $log 2>&1; echo "### end exit=$?" >> $log
for w in 7 8; do
  for i in 1 2 3 4 5; do
    for v in i ii iii; do
      echo "### run binary=floor_$v args='$w serial' repeat=$i" >> $log
      /tmp/bin/floor_$v $w serial >> $log 2>&1
      echo "### end exit=$?" >> $log
    done
  done
done
{ echo "# loadavg_after=$(cat /proc/loadavg)"; echo "# finished=$(date -u +%FT%TZ)"; } >> $log
