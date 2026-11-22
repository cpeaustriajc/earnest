set -x
export RUSTUP_TOOLCHAIN=1.100.0
cd /SCRATCH/src/old && cargo build --release --locked -p earnest-cli 2>&1 | tail -2 && cp target/release/earnest /SCRATCH/bin/earnest-old
cd /SCRATCH/src/new && cargo build --release --locked -p earnest-cli 2>&1 | tail -2 && cp target/release/earnest /SCRATCH/bin/earnest-new
cd /SCRATCH/src/new && CARGO_PROFILE_RELEASE_DEBUG=true CARGO_TARGET_DIR=/SCRATCH/target-sym cargo build --release --locked -p earnest-cli 2>&1 | tail -2 && cp /SCRATCH/target-sym/release/earnest /SCRATCH/bin/earnest-sym
echo BUILD-DONE
