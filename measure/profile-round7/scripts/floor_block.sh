#!/usr/bin/env bash
# The floor, one thread, variants i (no prefetch on x86-64), ii (_mm_prefetch), iii (plain load),
# in turn inside each round; one uncounted run, then 5 rounds at 7 wheels (10M) and 8 wheels (100M).
S=/SCRATCH
B=$S/bin
echo "# sha256"; sha256sum $B/floor_i $B/floor_ii $B/floor_iii | sed 's/^/# /'
echo "# loadavg_before=$(cat /proc/loadavg) utc=$(date -u +%FT%TZ)"
one() {
  echo "### run binary=floor_$1 args='$2 serial' repeat=$3"
  /usr/bin/time -f 'gnutime wall=%e user=%U sys=%S maxrss_kb=%M' $B/floor_$1 $2 serial 2>&1
  echo "### end exit=$?"
}
one i 7 0
for w in 7 8; do
  for r in 1 2 3 4 5; do
    for v in i ii iii; do one $v $w $r; done
  done
done
echo "# loadavg_after=$(cat /proc/loadavg) utc=$(date -u +%FT%TZ)"
