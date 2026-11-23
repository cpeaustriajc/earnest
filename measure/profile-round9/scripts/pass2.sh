#!/usr/bin/env bash
# The second pass with both builds in one block: earnest-old (3058cf4) and earnest-new (89de199), the floors in
# turn beside them, the symbol check, the profiles of both builds, and wide's count of instructions.
# Raw output into $S/raw/89de199/. Usage: pass2.sh [floors|time|sym|profile|callgrind|all]
S=${S:-/SCRATCH}; export PATH=$PATH:/root/.cargo/bin
R=$S/raw/89de199; mkdir -p $R; M=$S/scripts/machine.sh; step=${1:-all}
OLD=$S/bin3058/earnest NEW=$S/bin/earnest-new
cd $S/models || exit 1
if [ $step = floors ] || [ $step = all ]; then
  { $M before; bash $S/scripts/floor_rounds.sh $S/scripts/floors_short.conf 5; $M after; } > $R/f_floors.log 2>&1
fi
if [ $step = time ] || [ $step = all ]; then
  { $M before; python3 $S/scripts/time_turn.py 7 old=$OLD,new=$NEW grid_10m:1,grid_10m:4,grid_100m:4,grid_10m:2,wide:1,wide:4,lights:1,lights:4 $S/scripts/floors.json; $M after; } > $R/t1_7rounds.log 2>&1
  { $M before; python3 $S/scripts/time_turn.py 3 old=$OLD,new=$NEW grid_100m:1,grid_100m:2 $S/scripts/floors.json; $M after; } > $R/t2_100m_3rounds.log 2>&1
fi
if [ $step = sym ] || [ $step = all ]; then
  { $M before; python3 $S/scripts/time_runs.py new=$NEW,sym=$S/bin/earnest-sym 5 grid_10m:1,grid_10m:4,wide:1; $M after; } > $R/c3_sym_check.log 2>&1
fi
if [ $step = profile ] || [ $step = all ]; then
  for b in new old; do
    sym=$S/bin/earnest-sym; [ $b = old ] && sym=$S/bin3058/earnest-sym
    for c in "10m_j1 grid_10m 1 5" "10m_j4 grid_10m 4 5" "100m_j1 grid_100m 1 1" "100m_j4 grid_100m 4 1" "wide_j1 wide 1 20"; do
      set -- $c
      SYM=$sym bash $S/scripts/profile.sh ${b}_$1 $2 $3 $4
      mv $S/raw/prof_${b}_$1* $R/
      python3 $S/scripts/lines_by_earnest.py $S/prof/${b}_$1.script $sym 25 | rustfilt > $R/prof_${b}_$1_elines.txt
      python3 $S/scripts/by_function.py $S/prof/${b}_$1.script $sym 30 | rustfilt > $R/prof_${b}_$1_functions.txt
    done
  done
fi
if [ $step = callgrind ] || [ $step = all ]; then
  for b in old new; do
    bin=$OLD; [ $b = new ] && bin=$NEW
    for j in 1 4; do
      valgrind --tool=callgrind --callgrind-out-file=$S/prof/cg_${b}_wide_j$j.out $bin verify --no-cache --json --jobs $j --max-states 200000000 wide.model.ts > /dev/null 2> $R/callgrind_${b}_wide_j$j.txt
    done
  done
fi
echo PASS2-DONE $step
