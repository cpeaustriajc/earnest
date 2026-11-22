#!/bin/bash
binary="$1"; shift
label="$1"; shift
out=$( { /usr/bin/time -l "$binary" verify --no-cache "$@" > /SCRATCH/loop/probe/tmp-out.$$ 2> /SCRATCH/loop/probe/tmp-err.$$; } 2>&1 )
line=$(head -1 /SCRATCH/loop/probe/tmp-out.$$ | cut -c1-90)
real=$(grep -E ' real ' /SCRATCH/loop/probe/tmp-err.$$ | awk '{print $1" s real, "$3" s user, "$5" s sys"}')
instr=$(grep 'instructions retired' /SCRATCH/loop/probe/tmp-err.$$ | awk '{printf "%.2f G", $1/1e9}')
rss=$(grep 'maximum resident set size' /SCRATCH/loop/probe/tmp-err.$$ | awk '{printf "%.0f MiB", $1/1048576}')
echo "$label | $real | $instr | $rss | $line"
rm -f /SCRATCH/loop/probe/tmp-out.$$ /SCRATCH/loop/probe/tmp-err.$$
