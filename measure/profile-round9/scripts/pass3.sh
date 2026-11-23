#!/usr/bin/env bash
# The third pass: three builds (3058cf4, 89de199, 0ee0be2) and the floors in turn in one block, the profiles of
# 89de199 and 0ee0be2, and the counts of instructions of wide and ledger. Raw output into $S/raw/0ee0be2/.
# Builds: $S/bin3058/, $S/bin89/, $S/bin0ee/ (each holds earnest and earnest-sym). Usage: pass3.sh [floors|time|sym|profile|callgrind|all]
S=${S:-/SCRATCH}; export PATH=$PATH:/root/.cargo/bin
R=$S/raw/0ee0be2; mkdir -p $R; M=$S/scripts/machine.sh; step=${1:-all}
B="a3058=$S/bin3058/earnest,b89de=$S/bin89/earnest,c0ee0=$S/bin0ee/earnest"
cd $S/models || exit 1
if [ $step = floors ] || [ $step = all ]; then
  { $M before; bash $S/scripts/floor_rounds.sh $S/scripts/floors_short.conf 5; $M after; } > $R/f_floors.log 2>&1
fi
if [ $step = time ] || [ $step = all ]; then
  { $M before; python3 $S/scripts/time_turn.py 7 $B grid_10m:1,grid_10m:4,grid_100m:4,grid_10m:2,wide:1,wide:4,lights:1,lights:4,ledger:1,ledger:4 $S/scripts/floors.json; $M after; } > $R/t1_7rounds.log 2>&1
  { $M before; python3 $S/scripts/time_turn.py 3 $B grid_100m:1,grid_100m:2 $S/scripts/floors.json; $M after; } > $R/t2_100m_3rounds.log 2>&1
fi
if [ $step = sym ] || [ $step = all ]; then
  { $M before; python3 $S/scripts/time_runs.py new=$S/bin0ee/earnest,sym=$S/bin0ee/earnest-sym 5 grid_10m:1,grid_10m:4,wide:1; $M after; } > $R/c3_sym_check.log 2>&1
fi
if [ $step = profile ] || [ $step = all ]; then
  for b in c0ee0:bin0ee b89de:bin89; do
    tag=${b%%:*}; sym=$S/${b##*:}/earnest-sym
    for c in "10m_j1 grid_10m 1 5" "10m_j4 grid_10m 4 5" "100m_j1 grid_100m 1 1" "100m_j4 grid_100m 4 1" "wide_j1 wide 1 20"; do
      set -- $c
      SYM=$sym bash $S/scripts/profile.sh ${tag}_$1 $2 $3 $4
      mv $S/raw/prof_${tag}_$1* $R/
      python3 $S/scripts/lines_by_earnest.py $S/prof/${tag}_$1.script $sym 25 | rustfilt > $R/prof_${tag}_$1_elines.txt
      python3 $S/scripts/by_function.py $S/prof/${tag}_$1.script $sym 30 | rustfilt > $R/prof_${tag}_$1_functions.txt
    done
  done
fi
if [ $step = callgrind ] || [ $step = all ]; then
  for b in a3058:bin3058 b89de:bin89 c0ee0:bin0ee; do
    for m in wide ledger; do for j in 1 4; do
      valgrind --tool=callgrind --callgrind-out-file=$S/prof/cg_${b%%:*}_${m}_j$j.out $S/${b##*:}/earnest verify --no-cache --json --jobs $j --max-states 200000000 $m.model.ts > /dev/null 2> $R/callgrind_${b%%:*}_${m}_j$j.txt
    done; done
  done
fi
echo PASS3-DONE $step
