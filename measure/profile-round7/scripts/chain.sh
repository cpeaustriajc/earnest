#!/usr/bin/env bash
S=/SCRATCH
export PATH=$PATH:/root/.cargo/bin
cd $S/models
M=$S/scripts/machine.sh
chmod +x $M
{ $M before; python3 $S/scripts/time_runs.py old=$S/bin/earnest-old,new=$S/bin/earnest-new 7 grid_10m:1,grid_10m:4,lights:1,lights:4,wide:1,wide:4; $M after; } > $S/raw/g1_10m_lights_wide.log 2>&1
{ $M before; python3 $S/scripts/time_runs.py old=$S/bin/earnest-old,new=$S/bin/earnest-new 3 grid_100m:1,grid_100m:4; $M after; } > $S/raw/g2_100m.log 2>&1
{ $M before; bash $S/scripts/floor_block.sh; $M after; } > $S/raw/g3_floor.log 2>&1
{ $M before; python3 $S/scripts/time_runs.py new=$S/bin/earnest-new,sym=$S/bin/earnest-sym 5 grid_10m:1,grid_10m:4,wide:1; $M after; } > $S/raw/g4_sym_check.log 2>&1
bash $S/scripts/profile.sh 10m_j1 grid_10m 1 5
bash $S/scripts/profile.sh 10m_j4 grid_10m 4 5
bash $S/scripts/profile.sh 100m_j1 grid_100m 1 1
bash $S/scripts/profile.sh 100m_j4 grid_100m 4 1
bash $S/scripts/profile.sh wide_j1 wide 1 20
echo CHAIN-DONE
