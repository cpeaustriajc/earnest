# earnest on the cloud machine: three builds, measured

Measured on 2026-11-21 on a quiet 4-vCPU, 16 GB cloud machine, on the branch `measure/cloud-results` (cut from `measure/rounds-4-5`, tip `87964c7`). During a block I started nothing but light shell commands (a `diff`, a file copy, a polling loop). No product code was changed. Every number below comes from a file in `measure/raw/`; the scripts that ran the blocks and made the tables are in `measure/scripts/`.

| Build | Commit | What it is |
|---|---|---|
| A | `cb8f468` | the search code of the draft pull request |
| B | `34bc027` | round 4 on top of A |
| C | `34c5d04` | round 5 on top of B |

## 1. The machine and the toolchain

```
$ nproc
4

$ grep -m1 'model name' /proc/cpuinfo
model name	: Intel(R) Xeon(R) Processor @ 2.10GHz

$ free -m
               total        used        free      shared  buff/cache   available
Mem:           16087         559       12771          12        3035       15528
Swap:              0           0           0

$ uname -r
6.18.44-fc-v726

$ rustc +1.97.0 --version
rustc 1.97.0 (2d8144b78 2026-07-07)

$ cargo +1.97.0 --version
cargo 1.97.0 (c980f4866 2026-06-30)

$ sha256sum /tmp/earnest-A /tmp/earnest-B /tmp/earnest-C
7e3f9a3613bf8717485ad5a50a8fcdd517d4b5ea68bb57ef7dc94ed10240711e  /tmp/earnest-A
da6a1678c6d73b23a9ea2e0f6fce31294edb5910305e988bc1c01d9433e2589b  /tmp/earnest-B
2fefa91b7ca6cde8ef58759c8f27ffdb5e7f4e6a43585f051226311459534a43  /tmp/earnest-C

$ lscpu (flags and caches)
CPU(s):                                  4
Model name:                              Intel(R) Xeon(R) Processor @ 2.10GHz
Thread(s) per core:                      1
Core(s) per socket:                      4
Socket(s):                               1
BogoMIPS:                                4200.00
Hypervisor vendor:                       KVM
Virtualization type:                     full
L1d cache:                               192 KiB (4 instances)
L2 cache:                                8 MiB (4 instances)
L3 cache:                                260 MiB (1 instance)
```

- Rust: `rustc 1.97.0 (2d8144b78 2026-07-07)`, the version CI pins (installed with `rustup toolchain install 1.97.0 --profile minimal`). The builds were made with `cargo +1.97.0 build --release --locked -p earnest-cli`; `just benchmark` ran with `RUSTUP_TOOLCHAIN=1.97.0`.
- The machine is a KVM guest. `/usr/bin/time` 1.9, `just` 1.21.0 and `perf` 6.8.12 were installed with apt; the only `perf` the package sources offer is the 6.8 build, and the kernel is 6.18.44, so `perf` and kernel do not match.
- Wall time in the tables is the "Elapsed (wall clock) time" of `/usr/bin/time -v` for the whole process (resolution 0.01 s); peak memory is its "Maximum resident set size". The per-run values, with the `milliseconds` that `earnest` prints itself, are in the `.csv` next to each `.log`.
- `cat /proc/loadavg` before and after each block (the first number is the 1-minute average; one single-threaded run adds about 1 to it, so about 1 before a block is the tail of the block before):

| Block | Before | After |
|---|---|---|
| Block 1 `grid_10m` | 0.91 0.99 0.45 1/115 3620 | 1.77 1.37 0.77 1/109 19896 |
| Block 2 state limit | 1.63 1.35 0.77 1/117 19914 | 1.27 1.29 0.78 1/112 23290 |
| Block 3 `grid_1m` | 1.27 1.29 0.78 1/112 23295 | 1.49 1.34 0.80 1/112 31621 |
| Block 3 `wide` | 1.49 1.34 0.80 1/111 31625 | 1.69 1.38 0.82 1/115 1240 |
| Block 4 `just benchmark` | 1.32 1.31 0.80 1/111 1265 | 1.97 1.57 0.95 1/111 622 |
| Block 5 `perf stat` | 1.54 1.49 0.93 1/113 657 | 1.32 1.44 0.93 1/112 679 |
| Block 6 profile B, 1 worker | 1.03 1.37 0.92 1/112 710 | 1.00 1.20 0.93 1/115 781 |
| Block 6 profile C, 1 worker | 1.00 1.20 0.93 1/115 784 | 1.03 1.14 0.95 1/111 821 |
| Block 6 profile C, 4 workers | 1.03 1.14 0.95 1/111 824 | 1.87 1.33 1.02 1/115 1630 |
| Block 6b not profiled | 0.63 1.01 0.99 1/112 1937 | 1.93 1.21 1.07 1/114 2853 |
| Block 7 floor | 1.10 1.19 0.98 1/117 1735 | 1.72 1.24 1.06 1/111 1872 |

## 2. Block 1: `grid_10m` wall times

`earnest verify --no-cache --json --jobs J --max-states 20000000 tasks/benchmark/models/grid_10m.model.ts`; one unmeasured round, then 7 measured rounds; the six combinations go round-robin inside each round. Every one of the 48 runs printed status `pass` and 10,000,000 states. Raw: `block1_grid_10m.log`, `.csv`.

| Build | Workers | Rounds | Status | States | Smallest wall time | Median wall time | Largest wall time | Smallest peak memory | Median peak memory |
|---|---:|---:|---|---:|---:|---:|---:|---:|---:|
| A | 1 | 7 | pass | 10000000 | 10.71 s | 11.49 s | 12.89 s | 162 MiB | 163 MiB |
| B | 1 | 7 | pass | 10000000 | 9.96 s | 11.07 s | 11.65 s | 162 MiB | 163 MiB |
| C | 1 | 7 | pass | 10000000 | 7.68 s | 7.97 s | 10.35 s | 163 MiB | 163 MiB |
| A | 4 | 7 | pass | 10000000 | 2.07 s | 2.17 s | 2.68 s | 188 MiB | 189 MiB |
| B | 4 | 7 | pass | 10000000 | 1.89 s | 2.13 s | 2.55 s | 188 MiB | 189 MiB |
| C | 4 | 7 | pass | 10000000 | 1.50 s | 1.72 s | 2.04 s | 188 MiB | 189 MiB |

Wall time of every round, in seconds (round 0 is the unmeasured one):

| Build | Workers | 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| A | 1 | 12.45 | 12.89 | 12.84 | 11.97 | 11.30 | 11.49 | 10.71 | 10.81 |
| B | 1 | 11.77 | 11.17 | 11.65 | 11.10 | 10.59 | 9.96 | 11.07 | 10.98 |
| C | 1 | 9.46 | 10.35 | 9.22 | 7.74 | 8.43 | 7.95 | 7.68 | 7.97 |
| A | 4 | 2.54 | 2.68 | 2.67 | 2.55 | 2.17 | 2.15 | 2.13 | 2.07 |
| B | 4 | 2.19 | 2.34 | 2.46 | 2.55 | 2.13 | 1.93 | 2.08 | 1.89 |
| C | 4 | 1.88 | 2.04 | 1.70 | 1.88 | 1.72 | 1.72 | 1.80 | 1.50 |

- The first rounds were slower than the last ones for most combinations (build A with 1 worker took 12.89 s in round 1 and 10.71 s in round 6; build C with 1 worker 10.35 s and 7.68 s), but the pattern is not clean (build C with 1 worker took 7.74 s in round 3); the cause is not known, so the largest values stay in the table.
- The runs with 4 workers are more than 4 times faster than the runs with 1 worker on 4 vCPUs: by the medians 5.3 times for A, 5.2 for B and 4.6 for C (by the smallest values 5.2, 5.3 and 5.1); the numbers are kept as measured and the cause was not investigated.
- Build to build, by the medians: A to B 1.04 times faster with 1 worker and 1.02 with 4; B to C 1.39 times faster with 1 worker and 1.24 with 4.

## 3. Block 2: the default state limit

The same command with `--max-states 5000000`, builds B and C, 5 measured rounds, no unmeasured round. Every one of the 20 runs printed status `unfinished` and 5,000,000 states. Raw: `block2_limit_5m.log`, `.csv`.

| Build | Workers | Rounds | Status | States | Smallest wall time | Median wall time | Largest wall time | Smallest peak memory | Median peak memory |
|---|---:|---:|---|---:|---:|---:|---:|---:|---:|
| B | 1 | 5 | unfinished | 5000000 | 4.37 s | 4.58 s | 5.54 s | 84 MiB | 85 MiB |
| C | 1 | 5 | unfinished | 5000000 | 3.18 s | 3.27 s | 3.60 s | 84 MiB | 85 MiB |
| B | 4 | 5 | unfinished | 5000000 | 1.04 s | 1.12 s | 1.17 s | 135 MiB | 136 MiB |
| C | 4 | 5 | unfinished | 5000000 | 0.74 s | 0.81 s | 0.96 s | 137 MiB | 137 MiB |

Build B with 1 worker took 5.54 s in one round against 4.37 s to 4.90 s in the other four. `earnest verify` exits with status 1 when the result is `unfinished`; all 20 runs did (the `exit` column of the `.csv`), and the wall times are those of the whole process.

## 4. Block 3: `grid_1m` and `wide`, 4 workers

`--jobs 4 --max-states 20000000`, builds A, B and C, 5 measured rounds, no unmeasured round. Raw: `block3_grid_1m.log`, `block3_wide.log`, `.csv`.

`grid_1m` (all 15 runs: status `pass`, 1,000,000 states):

| Build | Workers | Rounds | Status | States | Smallest wall time | Median wall time | Largest wall time | Smallest peak memory | Median peak memory |
|---|---:|---:|---|---:|---:|---:|---:|---:|---:|
| A | 4 | 5 | pass | 1000000 | 0.26 s | 0.27 s | 0.30 s | 27 MiB | 27 MiB |
| B | 4 | 5 | pass | 1000000 | 0.24 s | 0.25 s | 0.26 s | 27 MiB | 27 MiB |
| C | 4 | 5 | pass | 1000000 | 0.18 s | 0.21 s | 0.22 s | 27 MiB | 28 MiB |

`wide` (all 15 runs: status `pass`, 10,000 states; the model has 10,000 states, and a run takes 0.21 s to 0.35 s):

| Build | Workers | Rounds | Status | States | Smallest wall time | Median wall time | Largest wall time | Smallest peak memory | Median peak memory |
|---|---:|---:|---|---:|---:|---:|---:|---:|---:|
| A | 4 | 5 | pass | 10000 | 0.26 s | 0.33 s | 0.35 s | 7 MiB | 7 MiB |
| B | 4 | 5 | pass | 10000 | 0.25 s | 0.29 s | 0.34 s | 7 MiB | 7 MiB |
| C | 4 | 5 | pass | 10000 | 0.21 s | 0.25 s | 0.28 s | 7 MiB | 7 MiB |

## 5. Block 4: `just benchmark` on build C, whole output

Checkout `34c5d04`, `just benchmark` once. Raw: `block4_just_benchmark.txt` (its first lines are the load averages; the one home-directory path in the compile lines is written `~`).

The line that judges the target:

> `- grid_10m with 4 workers: 1.546 s, peak memory 191.1 MiB, target under 5 s on 4 cores and 16 GB (this machine has 4 cores, at least the 4 of the target)`

Its own line "grid_10m: 1 worker 7.061 s, 4 workers (all cores) 1.588 s, speed-up 4.45x" is above 4 on 4 cores as well; kept as printed.

```
# just benchmark on build C (34c5d04), toolchain 1.97.0
# loadavg_before=1.32 1.31 0.80 1/111 1265
# loadavg_after=1.97 1.57 0.95 1/111 622
cargo build --release --locked -p earnest-cli
    Finished `release` profile [optimized] target(s) in 0.08s
cargo run --release --locked -p earnest_benchmark -- --runs 5
 Downloading crates ...
  Downloaded getrandom v0.4.3
  Downloaded errno v0.3.15
  Downloaded once_cell v1.21.4
  Downloaded tempfile v3.27.0
  Downloaded rustix v1.1.6
  Downloaded libc v0.2.191
  Downloaded linux-raw-sys v0.12.1
   Compiling libc v0.2.191
   Compiling getrandom v0.4.3
   Compiling serde_core v1.0.229
   Compiling rustix v1.1.6
   Compiling linux-raw-sys v0.12.1
   Compiling bitflags v2.13.2
   Compiling memchr v2.8.3
   Compiling once_cell v1.21.4
   Compiling fastrand v2.5.0
   Compiling serde_json v1.0.154
   Compiling tempfile v3.27.0
   Compiling earnest_benchmark v0.1.0 (~/earnest/tasks/benchmark)
    Finished `release` profile [optimized] target(s) in 10.80s
     Running `target/release/benchmark --runs 5`
measuring grid_1k.model.ts with all worker(s)
measuring grid_1k.rs with all worker(s)
measuring ledger.model.ts with all worker(s)
measuring ledger.rs with all worker(s)
measuring wide.model.ts with all worker(s)
measuring wide.rs with all worker(s)
measuring grid_100k.model.ts with all worker(s)
measuring grid_100k.rs with all worker(s)
measuring grid_1m.model.ts with all worker(s)
measuring grid_1m.rs with all worker(s)
measuring grid_10m.model.ts with all worker(s)
measuring grid_10m.rs with all worker(s)
measuring counter100.model.ts traffic.model.ts plane.model.ts basket.model.ts mailbox4.model.ts trail3.model.ts shelf.model.ts seats.model.ts votes.model.ts pairs.model.ts ledger.model.ts wide.model.ts with all worker(s)
measuring counter100.model.ts with all worker(s)
measuring traffic.model.ts with all worker(s)
measuring plane.model.ts with all worker(s)
measuring basket.model.ts with all worker(s)
measuring mailbox4.model.ts with all worker(s)
measuring trail3.model.ts with all worker(s)
measuring shelf.model.ts with all worker(s)
measuring seats.model.ts with all worker(s)
measuring votes.model.ts with all worker(s)
measuring pairs.model.ts with all worker(s)
measuring grid_10m.model.ts with 1 worker(s)
measuring grid_10m.model.ts with 4 worker(s)
measuring counter100.model.ts traffic.model.ts plane.model.ts basket.model.ts mailbox4.model.ts trail3.model.ts shelf.model.ts seats.model.ts votes.model.ts pairs.model.ts ledger.model.ts wide.model.ts grid_1m.model.ts with all worker(s)
Cores: 4 (Intel(R) Xeon(R) Processor @ 2.10GHz). Runs per model: 5 after one unmeasured run. Wall time includes starting the process and reading the files; peak memory is the largest resident set of any run.

## Scaled models

| Model | Language | States | Median wall time (s) | States per second | Peak memory (MiB) |
|---|---|---:|---:|---:|---:|
| Grid1k | TypeScript | 1,000 | 0.004 | 268,744 | 5.9 |
| Grid1k | Rust | 1,000 | 0.004 | 273,822 | 6.4 |
| Ledger | TypeScript | 36,400 | 0.055 | 664,779 | 7.5 |
| Ledger | Rust | 36,400 | 0.052 | 693,676 | 8.1 |
| Wide | TypeScript | 10,000 | 0.217 | 46,084 | 7.8 |
| Wide | Rust | 10,000 | 0.240 | 41,635 | 8.2 |
| Grid100k | TypeScript | 100,000 | 0.057 | 1,747,762 | 8.7 |
| Grid100k | Rust | 100,000 | 0.057 | 1,765,256 | 9.3 |
| Grid1m | TypeScript | 1,000,000 | 0.183 | 5,455,596 | 27.9 |
| Grid1m | Rust | 1,000,000 | 0.187 | 5,348,394 | 28.9 |
| Grid10m | TypeScript | 10,000,000 | 1.588 | 6,296,348 | 191.4 |
| Grid10m | Rust | 10,000,000 | 1.544 | 6,476,759 | 191.7 |

## The 12 mixed models, each alone

| Model | Language | States | Median wall time (s) | States per second | Peak memory (MiB) |
|---|---|---:|---:|---:|---:|
| counter100 | TypeScript | 100 | 0.004 | 27,233 | 5.7 |
| traffic | TypeScript | 30 | 0.004 | 8,154 | 5.8 |
| plane | TypeScript | 400 | 0.004 | 107,933 | 5.8 |
| basket | TypeScript | 16 | 0.004 | 4,428 | 5.7 |
| mailbox4 | TypeScript | 31 | 0.004 | 8,396 | 5.9 |
| trail3 | TypeScript | 40 | 0.004 | 11,204 | 5.7 |
| shelf | TypeScript | 64 | 0.004 | 17,025 | 5.8 |
| seats | TypeScript | 18 | 0.004 | 4,829 | 5.8 |
| votes | TypeScript | 21 | 0.004 | 5,720 | 5.7 |
| pairs | TypeScript | 120 | 0.004 | 32,635 | 5.8 |
| ledger | TypeScript | 36,400 | 0.055 | 664,779 | 7.5 |
| wide | TypeScript | 10,000 | 0.217 | 46,084 | 7.8 |

## Spec targets

- 12 models together: 0.242 s, target under 5 s (a stand-in: these benchmark models, not the first app's 12)
- slowest model alone: 0.217 s, target under 10 s (a stand-in: these benchmark models, not the first app's)
- the 12 models in that one run: 47,240 states, peak memory 8.6 MiB
- grid_10m with 4 workers: 1.546 s, peak memory 191.1 MiB, target under 5 s on 4 cores and 16 GB (this machine has 4 cores, at least the 4 of the target)

## One large model on one worker and on all cores

- grid_10m: 1 worker 7.061 s, 4 workers (all cores) 1.588 s, speed-up 4.45x

## R12: many models and one large model in one run

- the 13 models in one run: 0.418 s; the large model (grid_1m) alone: 0.183 s; the slowest of the 12 alone (wide): 0.217 s
- R12 asks for about as long as the largest model alone: the one run took 1.93x the longest of them alone (wide)
- peak memory 31.9 MiB
exit=0
```

## 6. Block 6: the profiles of `measure/grid_100m.model.ts`

`perf record -e cpu-clock -F 999 -o /tmp/<tag>.data /tmp/earnest-X verify --no-cache --json --jobs J --max-states 200000000 measure/grid_100m.model.ts`, then `perf report -i /tmp/<tag>.data --no-children --sort symbol --stdio --percent-limit 0.3`. The hardware event is not available (see section 7), so the profiles are `cpu-clock` samples. Each run printed status `pass` and 100,000,000 states. Raw: `block6_profile_B-j1.txt`, `block6_profile_C-j1.txt`, `block6_profile_C-j4.txt` (the whole `perf record` and `/usr/bin/time -v` output and the whole report).

The profiled runs, and one unprofiled run of each (an addition to the brief, to see what sampling costs; raw: `block6b_unprofiled_100m.log`, `.csv`):

| Build | Workers | Wall time, profiled | Wall time, not profiled | Peak memory, profiled | Peak memory, not profiled | Samples |
|---|---:|---:|---:|---:|---:|---:|
| B | 1 | 172.38 s | 172.07 s | 1411 MiB | 1411 MiB | 171365 |
| C | 1 | 126.31 s | 124.22 s | 1411 MiB | 1411 MiB | 124895 |
| C | 4 | 20.19 s | 17.93 s | 1657 MiB | 1614 MiB | 71920 |

The 4-worker run of C is 6.9 times faster than its 1-worker run on 4 vCPUs (17.93 s against 124.22 s, not profiled; 6.3 times profiled), which is above 4; kept as measured. The brief expected about 2 GB; the peak resident memory is 1.4 GiB with 1 worker and 1.6 GiB with 4 workers.

The lines of `perf report` down to 0.3%, as printed. Cut, to keep a line short: the padding after the symbol and the two last columns (IPC and IPC coverage, which are `-` on every line because the machine has no counters). `[.]` is user code, `[k]` is the kernel. The files in `measure/raw/` hold the full lines.

### Build B, 1 worker

```
# Total Lost Samples: 0
# Samples: 171K of event 'cpu-clock'
# Event count (approx.): 171536536365
    67.74%  [.] _RNvMNtCsbNELigJprFy_13earnest_check5storeNtB2_5Store5place
    15.06%  [.] _RNvMNtCsbNELigJprFy_13earnest_check7exploreNtB2_6Search5level
     7.63%  [.] _RNvMNtNtCsbNELigJprFy_13earnest_check5store4rowsNtB2_4Rows6unpack
     3.44%  [.] _RNvMs_NtNtCs35bxwrR96G3_10earnest_ir4code4condNtB4_4Cond4eval.llvm.8401547323063095347
     1.71%  [.] _RNvMs2_NtNtCs35bxwrR96G3_10earnest_ir4code3numNtB5_3Num4eval.llvm.8401547323063095347
     1.15%  [.] _RNvMNtCsbNELigJprFy_13earnest_check5storeNtB2_5Store10read_state
     1.05%  [.] _RNvMNtNtCsbNELigJprFy_13earnest_check7explore4hereNtB2_8HereSink9place_row
     0.74%  [.] __memmove_avx512_unaligned_erms
     0.51%  [.] _RINvMs6_NtCs6RpvpIXDhib_9hashbrown3rawINtB6_8RawTableNtNtNtCsbNELigJprFy_13earnest_check5store5index9NarrowKeyE14reserve_rehashNCNCNvMs_NtBT_5levelINtBR_6ShardsBP_E11claim_level0s_0EBV_
```

### Build C, 1 worker

```
# Total Lost Samples: 0
# Samples: 124K of event 'cpu-clock'
# Event count (approx.): 125020019895
    69.83%  [.] _RNvMNtCsbNELigJprFy_13earnest_check5storeNtB2_5Store5place
     8.79%  [.] _RNvMNtNtCsbNELigJprFy_13earnest_check5store4rowsNtB2_4Rows6unpack
     8.16%  [.] _RNvMNtCsbNELigJprFy_13earnest_check7exploreNtB2_6Search5level
     3.00%  [.] _RNvMs_NtNtCs35bxwrR96G3_10earnest_ir4code4condNtB4_4Cond4eval.llvm.586355361143759734
     2.47%  [.] __memmove_avx512_unaligned_erms
     2.37%  [.] _RNvMs2_NtNtCs35bxwrR96G3_10earnest_ir4code3numNtB5_3Num4eval.llvm.586355361143759734
     1.71%  [.] _RNvMNtCsbNELigJprFy_13earnest_check5storeNtB2_5Store10read_state
     1.40%  [.] _RNvMNtNtCsbNELigJprFy_13earnest_check7explore4hereNtB2_8HereSink9place_row
     0.74%  [.] _RINvMs6_NtCs6RpvpIXDhib_9hashbrown3rawINtB6_8RawTableNtNtNtCsbNELigJprFy_13earnest_check5store5index9NarrowKeyE14reserve_rehashNCNCNvMs_NtBT_5levelINtBR_6ShardsBP_E11claim_level0s_0EBV_
```

### Build C, 4 workers

```
# Total Lost Samples: 0
# Samples: 71K of event 'cpu-clock'
# Event count (approx.): 71991991920
    28.58%  [.] _RNCINvNtNtCsbNELigJprFy_13earnest_check5store5level6gatherINtNtCs40k4W9msRzi_5alloc3vec3VecIBV_jEENCNvMs_B4_INtNtB6_5index6ShardsNtB1L_9NarrowKeyE11claim_level0Es_0B8_
    20.51%  [.] _RNvMs1_NtNtCsbNELigJprFy_13earnest_check7explore4partNtB5_8PartSink13offer_encoded
    17.54%  [.] _RNvMs_NtNtCsbNELigJprFy_13earnest_check7explore7workersNtB6_6Search4work
     5.89%  [.] _RNvMs_NtNtCs35bxwrR96G3_10earnest_ir4code4condNtB4_4Cond4eval.llvm.586355361143759734
     5.15%  [.] _RNvMNtNtCsbNELigJprFy_13earnest_check5store4rowsNtB2_4Rows6unpack
     4.28%  [.] _RNvMs2_NtNtCs35bxwrR96G3_10earnest_ir4code3numNtB5_3Num4eval.llvm.586355361143759734
     2.57%  [.] _RNvMNtCsbNELigJprFy_13earnest_check5storeNtB2_5Store10read_state
     2.41%  [k] exc_page_fault
     2.38%  [.] __memmove_avx512_unaligned_erms
     1.33%  [.] _RNCNvNtNtCsbNELigJprFy_13earnest_check5store5level11write_level0B7_
     1.21%  [.] _RINvMs6_NtCs6RpvpIXDhib_9hashbrown3rawINtB6_8RawTableNtNtNtCsbNELigJprFy_13earnest_check5store5index9NarrowKeyE14reserve_rehashNCNCNvMs_NtBT_5levelINtBR_6ShardsBP_E11claim_level0s_0EBV_
     0.85%  [k] _raw_spin_lock
     0.82%  [k] clear_page_erms
     0.77%  [k] memcg1_commit_charge
     0.69%  [k] _raw_spin_unlock_irqrestore
     0.51%  [.] _RINvNtCscI6d9CVNmLh_4core3ptr9drop_glueNtNtCseZDVTF6dyRJ_13earnest_value5value5ValueECsbNELigJprFy_13earnest_check
     0.33%  [.] realloc
```

`perf` 6.8 does not demangle Rust's v0 symbol names (a demangler could not be installed here, see section 9), so the names above are as printed. My reading of them, from the symbols and from the doc and code of `crates/earnest_check`; the readings are mine, not `perf` output:

| Symbol in the report | Reading |
|---|---|
| `…earnest_check5storeNtB2_5Store5place` | `earnest_check::store::Store::place`: claim a row in the table of seen states and, if new, push the row and its parent (the 1-worker search) |
| `…earnest_check7exploreNtB2_6Search5level` | `earnest_check::explore::Search::level`: the loop over one level of the search (1 worker) |
| `…5store4rowsNtB2_4Rows6unpack` | `earnest_check::store::rows::Rows::unpack` |
| `…earnest_ir4code4condNtB4_4Cond4eval…` and `…4code3numNtB5_3Num4eval…` | `earnest_ir::code::cond::Cond::eval` and `earnest_ir::code::num::Num::eval`: evaluating the model's conditions and numbers |
| `…5store10read_state`, `…7explore4hereNtB2_8HereSink9place_row` | `Store::read_state`; `explore::here::HereSink::place_row` |
| `…5store5level6gather…claim_level0Es_0` | a closure of `earnest_check::store::level::gather`, called from `Shards::claim_level`: the shards of the table claim the rows of one level (4 workers) |
| `…7explore4partNtB5_8PartSink13offer_encoded` | `earnest_check::explore::part::PartSink::offer_encoded`: a worker looks a new row up in the store and offers it (4 workers) |
| `…7explore7workersNtB6_6Search4work` | `earnest_check::explore::workers::Search::work`: the worker loop (4 workers) |
| `…hashbrown3raw…RawTable…14reserve_rehash…claim_level…` | `hashbrown`'s table growing, inside `claim_level` |
| `__memmove_avx512_unaligned_erms`, `realloc`, `exc_page_fault`, `clear_page_erms`, `_raw_spin_lock`, `memcg1_commit_charge`, `_raw_spin_unlock_irqrestore` | libc and kernel (page faults while the tables grow) |

The biggest lines of the 1-worker profiles and of the 4-worker profile are different functions (the smaller lines, such as `Cond::eval`, `Rows::unpack` and `memmove`, appear in both): with 1 worker, `Store::place` takes 67.74% (B) and 69.83% (C) of the samples and no other line is above 15.1%; with 4 workers the three biggest are the `claim_level` closure (28.58%), `offer_encoded` (20.51%) and `Search::work` (17.54%), and the five kernel lines (`exc_page_fault`, `clear_page_erms`, `_raw_spin_lock`, `memcg1_commit_charge`, `_raw_spin_unlock_irqrestore`) add up to 5.54%.

## 7. Block 5: hardware counters

This machine has none: it is a KVM guest, and `perf stat -e instructions,cycles,cache-misses,LLC-load-misses,dTLB-load-misses` prints `<not supported>` for all five. Only software events work (`cpu-clock`, `task-clock`, `page-faults`). So instructions, cycles, cache misses and TLB misses are not measured. The `perf stat` runs on `grid_10m` with 1 worker (one run each, status `pass`, 10,000,000 states; `task-clock` and `page-faults` added to the event list) are kept as printed. Raw: `block5_perf_stat.txt`.

```
# perf version 6.8.12 (kernel is 6.18.44-fc-v726; the perf of the distribution's 6.8 tools package is the one that is installable)
# loadavg_before=1.54 1.49 0.93 1/113 657
### build=A
      "status": "pass",
      "states": 10000000,

 Performance counter stats for '/tmp/earnest-A verify --no-cache --json --jobs 1 --max-states 20000000 tasks/benchmark/models/grid_10m.model.ts':

   <not supported>      instructions
   <not supported>      cycles
   <not supported>      cache-misses
   <not supported>      LLC-load-misses
   <not supported>      dTLB-load-misses
          10772.83 msec task-clock                       #    1.000 CPUs utilized
             56284      page-faults                      #    5.225 K/sec

      10.774756520 seconds time elapsed

      10.468627000 seconds user
       0.122795000 seconds sys


### build=B
      "status": "pass",
      "states": 10000000,

 Performance counter stats for '/tmp/earnest-B verify --no-cache --json --jobs 1 --max-states 20000000 tasks/benchmark/models/grid_10m.model.ts':

   <not supported>      instructions
   <not supported>      cycles
   <not supported>      cache-misses
   <not supported>      LLC-load-misses
   <not supported>      dTLB-load-misses
           9404.53 msec task-clock                       #    1.000 CPUs utilized
             56285      page-faults                      #    5.985 K/sec

       9.405966410 seconds time elapsed

       9.249177000 seconds user
       0.099727000 seconds sys


### build=C
      "status": "pass",
      "states": 10000000,

 Performance counter stats for '/tmp/earnest-C verify --no-cache --json --jobs 1 --max-states 20000000 tasks/benchmark/models/grid_10m.model.ts':

   <not supported>      instructions
   <not supported>      cycles
   <not supported>      cache-misses
   <not supported>      LLC-load-misses
   <not supported>      dTLB-load-misses
           6898.02 msec task-clock                       #    1.000 CPUs utilized
             56281      page-faults                      #    8.159 K/sec

       6.899522985 seconds time elapsed

       6.586164000 seconds user
       0.102779000 seconds sys


# loadavg_after=1.32 1.44 0.93 1/112 679
```

These are single runs. Build C took 6.90 s here, 0.78 s below the smallest value of block 1 for C with 1 worker (7.68 s), and B took 9.41 s against 9.96 s; kept as measured, not repeated.

## 8. Block 7: the floor

`measure/floor.rs` has its prefetch written for ARM only; on x86-64 its prefetch does nothing (`floor`). `floor_x86` is a copy outside the repository with `_mm_prefetch(…, _MM_HINT_T0)` added for x86-64 (the diff is `block7_floor_x86.diff`). Both were built with `rustc -O` (1.97.0). The runs alternate between the two binaries. Every run printed the number of states in the table, and the 10,000,000-state runs agree on `checksum=4c9670a40c764e17` and `parent_checksum=6ff96c9c5e19c80e`. Raw: `block7_floor.log` (whole output of every run, with the step times of the 4-thread runs), `.csv`.

| Binary | Arguments | States | Runs | Smallest time_s | Median time_s | Largest time_s |
|---|---|---:|---:|---:|---:|---:|
| floor (no prefetch on x86-64) | `7 serial` | 10000000 | 5 | 2.8410 s | 3.0015 s | 3.2652 s |
| floor_x86 (with _mm_prefetch) | `7 serial` | 10000000 | 5 | 2.0346 s | 2.1459 s | 2.2230 s |
| floor (no prefetch on x86-64) | `8 serial` | 100000000 | 3 | 49.1853 s | 50.9091 s | 51.5801 s |
| floor_x86 (with _mm_prefetch) | `8 serial` | 100000000 | 3 | 31.8667 s | 32.7582 s | 33.2083 s |
| floor (no prefetch on x86-64) | `7 4` | 10000000 | 5 | 2.0670 s | 2.2254 s | 2.3128 s |
| floor_x86 (with _mm_prefetch) | `7 4` | 10000000 | 5 | 1.7918 s | 1.8052 s | 1.9468 s |

- With the x86-64 prefetch the serial floor takes 2.03 s to 2.22 s for 10,000,000 states and 31.9 s to 33.2 s for 100,000,000; without it, 2.84 s to 3.27 s and 49.2 s to 51.6 s.
- The 4-thread floor (`floor_x86 7 4`, smallest 1.79 s, median 1.81 s) is slower than build C with 4 workers on the same grid (smallest 1.50 s, median 1.72 s), and only 1.1 to 1.2 times faster than its own serial run; it is not a lower bound for 4 cores on this machine.
- Build C with 1 worker is 3.7 times the x86-64 serial floor at 10,000,000 states (medians 7.97 s and 2.15 s) and 3.8 times at 100,000,000 states (124.22 s, not profiled, against the 32.76 s median of the floor); build B with 1 worker is 5.3 times the floor at 100,000,000 states (172.07 s).
- The floor is a hand-written search of this one grid, not a general one, so these ratios are what the general search costs over it; they are not a count of avoidable work.

## 9. What was not measured, and why

- **Hardware counters** (instructions, cycles, cache-misses, LLC-load-misses, dTLB-load-misses): the KVM guest exposes none; `perf stat` prints `<not supported>`. Whether the 1-worker time goes to cache misses or to instructions is therefore not measured; no profile here can tell it.
- **Profiles with the hardware event:** the hardware event is not available, so the profiles are `-e cpu-clock` samples (software clock, 999 Hz). They are flat (`--sort symbol`); no call graph was recorded, so the callers of a function are not measured.
- **Demangled symbol names:** `perf` 6.8 prints Rust v0 names mangled. I tried to install a demangler (`cargo install rustfilt` into a temporary directory); the environment refused the action, so I did not retry. The readings in section 6 are by hand.
- **A `perf` that matches the kernel:** only `perf` 6.8.12 could be installed for kernel 6.18.44. It sampled and printed without error; whether a matching `perf` would print anything different is not known.
- **A profile of build A**, and profiles of any build with 4 workers other than C: not asked for.
- **Unmeasured warm-up rounds** were run only in block 1, as asked; blocks 2 and 3 have 5 measured rounds and no warm-up, and the floor runs and the `perf stat` runs have none.
- **Other machines:** none was measured. The numbers hold for this guest: Intel Xeon at 2.10 GHz, 4 vCPUs, 16 GB, kernel 6.18.44. Whether the target machine has the same processor is not known to me.
- **Cut blocks:** none. No block took more than 20 minutes. Every `verify` run printed the status and the number of states named for its block (`pass` and 10,000,000; `unfinished` and 5,000,000; `pass` and 1,000,000 for `grid_1m`; `pass` and 10,000 for `wide`; `pass` and 100,000,000 for the profiled and the unprofiled runs of `grid_100m`).

## 10. What the numbers say

1. **Does `grid_10m` run in under 5 s with 4 workers on this machine?** Yes. `just benchmark` prints 1.546 s. In block 1, the smallest wall time with 4 workers is 2.07 s (A), 1.89 s (B) and 1.50 s (C); the medians are 2.17 s, 2.13 s and 1.72 s, and the largest single run of the 21 runs with 4 workers is 2.68 s (A). Peak memory is 189 MiB. With 1 worker every build is above 5 s (smallest 7.68 s for C).
2. **Where the time goes on a large model** (100,000,000 states): with 1 worker, about 70% in `Store::place` for both B and C, with `Rows::unpack` (7.6% and 8.8%) and `Search::level` (15.1% and 8.2%) next; with 4 workers, `claim_level`'s closure, `offer_encoded` and `Search::work` take 28.6%, 20.5% and 17.5%. Round 5 (B to C) cut the 100,000,000-state run with 1 worker from 172.07 s to 124.22 s; the share of `Store::place` did not fall (67.74% to 69.83%).
3. **Machine or code?** The hand-written serial floor does the 10,000,000 states in 2.03 s to 2.22 s on one core of this machine, and the 100,000,000 states in 31.9 s to 33.2 s. Build C with 1 worker needs 3.7 to 3.8 times as long. With 4 workers build C (1.50 s smallest) is faster than the 4-thread floor (1.79 s smallest).
