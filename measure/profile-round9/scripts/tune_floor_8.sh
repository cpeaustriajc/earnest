#!/usr/bin/env bash
# The narrower tuning at 8 wheels (100M), one pass, after tune_floor.sh at 7 showed FILTER=1 never helps.
S=${S:-/SCRATCH}; B=$S/bin; w=8
echo "# loadavg_before=$(cat /proc/loadavg) utc=$(date -u +%FT%TZ)"
one() { local out; out=$(env "$@" 2>&1 | grep -E '^(states|levels|set_checksum|bad_parents|time_s|run_cpu_s|run_sys_s|step1_s|step2_s|serial_s|step3_s|expand_s|copy_s|grow_s)=' | tr '\n' ' '); echo "$* :: $out"; }
for t in 4 2; do
  for h in 1 0; do
    one HUGE=$h SHARD_BITS=6 PART=4096 FILTER=1 $B/floor_par $w $t
    for sb in 4 6 8; do for part in 4096 16384 65536; do one HUGE=$h SHARD_BITS=$sb PART=$part FILTER=0 $B/floor_par $w $t; done; done
    for c in 256 1024 4096; do for d in 16 32; do one HUGE=$h CHUNK=$c DIST=$d $B/floor_par $w shared $t; done; done
  done
done
echo "# loadavg_after=$(cat /proc/loadavg) utc=$(date -u +%FT%TZ)"
