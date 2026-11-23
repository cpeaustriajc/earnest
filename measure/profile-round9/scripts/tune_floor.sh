#!/usr/bin/env bash
# Tuning of the two floors for several threads, one run per setting, two passes, at the given wheels.
# Usage: tune_floor.sh WHEELS  (prints one line per run)
S=${S:-/SCRATCH}; B=$S/bin; w=${1:-7}
echo "# loadavg_before=$(cat /proc/loadavg) utc=$(date -u +%FT%TZ)"
one() { # env... mode
  local out; out=$(env "$@" 2>&1 | grep -E '^(states|levels|set_checksum|bad_parents|time_s|run_cpu_s|run_sys_s|step1_s|step2_s|serial_s|step3_s|expand_s|copy_s|grow_s)=' | tr '\n' ' ')
  echo "$* :: $out"
}
for pass in 1 2; do
  for t in 2 4; do
    for h in 0 1; do
      for pf in 0 1; do one HUGE=$h PREFAULT=$pf $B/floor_par $w $t; done
      for sb in 4 6 8; do for part in 1024 4096 16384; do for f in 0 1; do
        one HUGE=$h SHARD_BITS=$sb PART=$part FILTER=$f $B/floor_par $w $t; done; done; done
      for c in 256 1024 4096; do for d in 0 8 16 32 48; do
        one HUGE=$h CHUNK=$c DIST=$d $B/floor_par $w shared $t; done; done
    done
  done
done
echo "# loadavg_after=$(cat /proc/loadavg) utc=$(date -u +%FT%TZ)"
