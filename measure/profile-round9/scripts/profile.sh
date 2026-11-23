#!/usr/bin/env bash
# One profile: perf record (cpu-clock, 999 Hz, frame-pointer call chains for the kernel side) of N runs
# of the symbol build in one recording, from the model folder. Then the flat report and the script dump.
# Usage: profile.sh TAG MODEL JOBS N
S=${S:-/SCRATCH}
SYM=${SYM:-$S/bin/earnest-sym}
P=/usr/lib/linux-tools/6.8.0-151-generic/perf
tag=$1 model=$2 jobs=$3 n=$4
mkdir -p $S/prof
cd $S/models || exit 1
{
  $S/scripts/machine.sh before
  $P record -e cpu-clock -F 999 -g -o $S/prof/$tag.data -- sh -c "for i in \$(seq $n); do /usr/bin/time -f 'gnutime wall=%e user=%U sys=%S maxrss_kb=%M' $SYM verify --no-cache --json --jobs $jobs --max-states 200000000 $model.model.ts | grep -E '\"(status|states)\"'; done" 2>&1
  $S/scripts/machine.sh after
} > $S/raw/prof_$tag.log 2>&1
$P report -i $S/prof/$tag.data --no-children --sort symbol --stdio --percent-limit 0.3 -g none 2>/dev/null | rustfilt > $S/raw/prof_${tag}_report.txt
$P report -i $S/prof/$tag.data --no-children --sort dso --stdio -g none 2>/dev/null > $S/raw/prof_${tag}_dso.txt
$P script -i $S/prof/$tag.data -F tid,ip,sym,dso,dsoff 2>/dev/null > $S/prof/$tag.script
python3 $S/scripts/attr.py $S/prof/$tag.script $SYM > $S/raw/prof_${tag}_groups.json
