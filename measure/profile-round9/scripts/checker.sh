#!/usr/bin/env bash
# The checker's pass: timing in turn with the floors, then the symbol-build check, then the profiles.
# Usage: checker.sh [time|sym|profile|all]   (default all). Raw output into $S/raw/.
S=${S:-/SCRATCH}
export PATH=$PATH:/root/.cargo/bin
M=$S/scripts/machine.sh
step=${1:-all}
cd $S/models || exit 1
if [ $step = time ] || [ $step = all ]; then
  { $M before; python3 $S/scripts/time_turn.py 7 $S/bin/earnest-new grid_10m:1,grid_10m:2,grid_10m:4,wide:1,wide:4,lights:1,lights:4 $S/scripts/floors.json; $M after; } > $S/raw/c1_10m_wide_lights.log 2>&1
  { $M before; python3 $S/scripts/time_turn.py 3 $S/bin/earnest-new grid_100m:1,grid_100m:2,grid_100m:4 $S/scripts/floors.json; $M after; } > $S/raw/c2_100m.log 2>&1
fi
if [ $step = sym ] || [ $step = all ]; then
  { $M before; python3 $S/scripts/time_runs.py new=$S/bin/earnest-new,sym=$S/bin/earnest-sym 5 grid_10m:1,grid_10m:4,wide:1; $M after; } > $S/raw/c3_sym_check.log 2>&1
fi
if [ $step = profile ] || [ $step = all ]; then
  bash $S/scripts/profile.sh 10m_j1 grid_10m 1 5
  bash $S/scripts/profile.sh 10m_j4 grid_10m 4 5
  bash $S/scripts/profile.sh 100m_j1 grid_100m 1 1
  bash $S/scripts/profile.sh 100m_j4 grid_100m 4 1
  bash $S/scripts/profile.sh wide_j1 wide 1 20
  for t in 10m_j1 10m_j4 100m_j1 100m_j4 wide_j1; do
    python3 $S/scripts/lines_by_earnest.py $S/prof/$t.script $S/bin/earnest-sym 25 | rustfilt > $S/raw/prof_${t}_elines.txt
    python3 $S/scripts/by_function.py $S/prof/$t.script $S/bin/earnest-sym 30 | rustfilt > $S/raw/prof_${t}_functions.txt
  done
fi
echo CHECKER-DONE $step
