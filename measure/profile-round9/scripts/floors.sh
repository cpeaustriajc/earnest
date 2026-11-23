#!/usr/bin/env bash
# Steps 1 to 3: the one-thread floors (floor_block.sh), then the floors for several threads and the generating
# alone (floor_rounds.sh floors_par.conf), 5 rounds each. Does not depend on the checker's commit.
S=${S:-/SCRATCH}; M=$S/scripts/machine.sh
{ $M before; bash $S/scripts/floor_block.sh; $M after; } > $S/raw/f1_floor_serial.log 2>&1
{ $M before; bash $S/scripts/floor_rounds.sh $S/scripts/floors_par.conf 5; $M after; } > $S/raw/f2_floor_par_gen.log 2>&1
echo FLOORS-DONE
