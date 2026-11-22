#!/bin/sh
# usage: compare.sh <new binary>
W=/SCRATCH/rows
S=/SCRATCH_OLDER
T=/WORKTREES/perf-search
rm -f $W/cmp/*.diff $W/cmp/*.new
files=$(cd $T && find tasks/benchmark/models tasks/models tasks/conformance \( -name '*.model.ts' -o -name '*.rs' \) | sort | sed "s|^|$T/|")
{
for f in $files $W/own/*.model.ts; do for j in 1 2 4 10; do for mode in verify verify50 verify7 report report50; do
  case "$f" in *slow.model.ts) case $mode in verify|report) continue;; esac;; esac
  echo "$f $mode $j"
done; done; done
for f in $(ls $S/models-new/*.model.ts | grep -v grid_100m) $S/models/*.model.ts; do for j in 1 2 4 10; do echo "$f verify $j"; echo "$f verify50 $j"; done; done
} > $W/cmp/list
xargs -P 4 -L 1 $W/compare_one.sh "$1" < $W/cmp/list > $W/cmp/log
runs=$(grep -c RUN $W/cmp/log); refd=$(grep -c "DIFF ref" $W/cmp/log)
self=0; selfruns=0
for one in $W/cmp/*.1.new; do k=${one%.1.new}; for j in 2 4 10; do [ -f $k.$j.new ] || continue; selfruns=$((selfruns+1)); cmp -s $one $k.$j.new || { self=$((self+1)); echo "DIFF self $k j$j"; }; done; done
grep "DIFF" $W/cmp/log
echo "runs $runs (listed $(wc -l < $W/cmp/list)), differences from earnest-joined $refd; own j2/4/10 vs own j1: $selfruns compared, $self differ"
for j in 1 2 4 10; do
  a=$("$1" verify --no-cache --json --jobs $j --max-states 2 $S/models/budget.model.ts | python3 -c 'import json,sys; m=json.load(sys.stdin)["models"][0]; print(m["status"], m["states"])')
  b=$("$1" verify --no-cache --json --jobs $j --max-states 2050 $S/budget2/spread.model.ts | python3 -c 'import json,sys; m=json.load(sys.stdin)["models"][0]; print(m["status"], m["states"])')
  echo "budget j$j: budget.model.ts $a / spread.model.ts $b"
done
