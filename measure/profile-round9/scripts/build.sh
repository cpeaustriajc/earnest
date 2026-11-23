#!/usr/bin/env bash
# Builds the checker at COMMIT (plain, and with symbols in its own target folder) and the floors.
# Usage: build.sh COMMIT      (from the repository; the worktree goes to $S/src/COMMIT)
set -ex
S=${S:-/SCRATCH}; c=$1
export RUSTUP_TOOLCHAIN=1.100.0
mkdir -p $S/bin $S/src
git fetch origin perf/joined
[ -d $S/src/$c ] || git worktree add $S/src/$c $c
cd $S/src/$c
cargo build --release --locked -p earnest-cli 2>&1 | tail -2
cp target/release/earnest $S/bin/earnest-new
CARGO_PROFILE_RELEASE_DEBUG=true CARGO_TARGET_DIR=$S/target-sym cargo build --release --locked -p earnest-cli 2>&1 | tail -2
cp $S/target-sym/release/earnest $S/bin/earnest-sym
mkdir -p $S/models
cp tasks/benchmark/models/grid_10m.model.ts tasks/benchmark/models/wide.model.ts $S/models/
git -C $S/src/$c show origin/measure/tools:measure/tools/models/grid_100m.model.ts > $S/models/grid_100m.model.ts
git -C $S/src/$c show origin/measure/tools:measure/tools/models/lights.model.ts > $S/models/lights.model.ts
for v in i ii iii par; do rustc +1.100.0 -O -o $S/bin/floor_$v $S/scripts/floor_$v.rs; done
git -C $S/src/$c log --oneline -1
sha256sum $S/bin/* $S/models/*
