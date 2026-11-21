#!/usr/bin/env bash
# Block 6: three profiles of the 100-million-state model. Run from the repository root.
P=/usr/lib/linux-tools-6.8.0-151/perf
for combo in B:1 C:1 C:4; do
  X=${combo%%:*}; J=${combo##*:}
  tag=$X-j$J
  log=measure/raw/block6_profile_$tag.txt
  {
    echo "# profile build=$X jobs=$J event=cpu-clock -F 999 (the hardware event is not available on this machine)"
    echo "# loadavg_before=$(cat /proc/loadavg)"
    echo "# started=$(date -u +%FT%TZ)"
    echo "\$ perf record -e cpu-clock -F 999 -o /tmp/$tag.data /tmp/earnest-$X verify --no-cache --json --jobs $J --max-states 200000000 measure/grid_100m.model.ts"
  } > "$log"
  /usr/bin/time -v $P record -e cpu-clock -F 999 -o /tmp/$tag.data /tmp/earnest-$X verify --no-cache --json --jobs $J --max-states 200000000 measure/grid_100m.model.ts >> "$log" 2>&1
  {
    echo "# loadavg_after=$(cat /proc/loadavg)"
    echo "# finished=$(date -u +%FT%TZ)"
    echo "\$ perf report -i /tmp/$tag.data --no-children --sort symbol --stdio --percent-limit 0.3"
  } >> "$log"
  $P report -i /tmp/$tag.data --no-children --sort symbol --stdio --percent-limit 0.3 >> "$log" 2>&1
done
echo ALLDONE > /tmp/profiles.done
