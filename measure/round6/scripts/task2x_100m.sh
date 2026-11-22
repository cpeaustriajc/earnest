#!/usr/bin/env bash
# Extra to task 2 (not asked for): a cpu-clock profile of grid_100m with the tip, for the comparison with 10M.
# One perf.data per profile; perf records a shell loop of N runs, so the samples of all N runs
# are in one file. Every run's JSON goes into the log. Run from the repository root.
# usage: task2.sh <jobs> <runs>
P=/usr/lib/linux-tools-6.8.0-151/perf
J=$1; N=$2
tag=task2x_profile_100m_j$J
log=measure/round6/raw/$tag.txt
D=/tmp/$tag.data
CMD="/tmp/bin/earnest-tip verify --no-cache --json --jobs $J --max-states 200000000 measure/grid_100m.model.ts"
{
  echo "# profile tip jobs=$J runs_in_one_recording=$N event=cpu-clock -F 999 (no hardware event on this machine)"
  echo "# loadavg_before=$(cat /proc/loadavg)"
  echo "# started=$(date -u +%FT%TZ)"
  echo "\$ perf record -e cpu-clock -F 999 -o $D -- sh -c 'for i in \$(seq $N); do $CMD; done'"
} > "$log"
/usr/bin/time -v $P record -e cpu-clock -F 999 -o $D -- sh -c "for i in \$(seq $N); do echo '### run' \$i; $CMD; done" >> "$log" 2>&1
{
  echo "# loadavg_after=$(cat /proc/loadavg)"
  echo "# finished=$(date -u +%FT%TZ)"
  echo "\$ perf report -i $D --no-children --sort symbol --stdio --percent-limit 0"
} >> "$log"
$P report -i $D --no-children --sort symbol --stdio --percent-limit 0 >> "$log" 2>&1
echo "\$ perf report -i $D --no-children --sort dso --stdio" >> "$log"
$P report -i $D --no-children --sort dso --stdio >> "$log" 2>&1
