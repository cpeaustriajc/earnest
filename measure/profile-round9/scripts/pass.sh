#!/usr/bin/env bash
# The second pass for another commit of perf/joined, one command per step (from the repository root):
#   bash measure/profile-round9/scripts/pass.sh setup              # /SCRATCH, perf, GNU time, rustfilt, scripts
#   bash measure/profile-round9/scripts/pass.sh build COMMIT       # about 2 min
#   bash measure/profile-round9/scripts/pass.sh checker            # timing in turn with the floors, symbol check, profiles: about 12 min
#   bash measure/profile-round9/scripts/pass.sh compare            # raw/compare.md, raw/tables.md, raw/pieces.md: under a minute
#   bash measure/profile-round9/scripts/pass.sh floors             # only if the floors are wanted again: about 30 min
# Results land in $S/raw (default /SCRATCH/raw); copy them into a new folder of measure/.
set -e
S=${S:-/SCRATCH}; here=$(cd "$(dirname "$0")" && pwd)
case $1 in
  setup)
    mkdir -p $S/{src,bin,models,prof,raw,scripts}
    command -v perf >/dev/null || ls /usr/lib/linux-tools/*/perf >/dev/null 2>&1 || apt-get install -y linux-tools-generic time
    command -v rustfilt >/dev/null || ls /root/.cargo/bin/rustfilt >/dev/null 2>&1 || cargo install rustfilt
    rustup toolchain install 1.100.0 --profile minimal
    cp $here/* $S/scripts/ ;;
  build) cp $here/* $S/scripts/; bash $S/scripts/build.sh $2 > $S/raw/build.log 2>&1; tail -12 $S/raw/build.log ;;
  checker) bash $S/scripts/checker.sh all ;;
  compare)
    export PATH=$PATH:/root/.cargo/bin
    python3 $S/scripts/compare.py $S > $S/raw/compare.md
    python3 $S/scripts/tables.py $S > $S/raw/tables.md
    python3 $S/scripts/pieces.py $S > $S/raw/pieces.md ;;
  floors) bash $S/scripts/floors.sh ;;
  *) echo "usage: pass.sh setup|build COMMIT|checker|compare|floors"; exit 1 ;;
esac
