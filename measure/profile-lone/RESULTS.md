# Where the checker's time goes at `9c5413e`, measured against the floor

Measured on 2026-11-22 on a 4-vCPU cloud machine (KVM guest). The checker is commit `9c5413e`, the tip of
`perf/lone-by-width`. This branch is cut from `origin/measure/tools` and holds only this folder. No source
file of the project was changed. Every number below comes from a file in `raw/`, and the scripts that made
them are in `scripts/`. In the raw files, `/SCRATCH` stands for the scratch folder of the session.

## 1. The machine

From `raw/machine.txt` and the `## machine` lines at the start and end of every log:

- `nproc` 4. Intel(R) Xeon(R) Processor @ 2.10GHz, 1 thread per core, KVM. BogoMIPS 4200. L1d 192 KiB
  (4 instances), L2 8 MiB (4 instances), L3 260 MiB (1 instance). This is the same CPU as in
  `measure/cloud-results` (BogoMIPS 4200, L3 260 MiB). It is not the CPU of `measure/round6-results`
  (BogoMIPS 5400, L3 480 MiB).
- Memory 16,087 MiB in total, no swap. Kernel 6.18.44-fc-v726. Transparent huge pages: see `machine.txt`.
- `rustc 1.100.0 (0ef1bc386 2026-11-09)`, `cargo 1.100.0 (3acc79c0c 2026-11-05)`, with `RUSTUP_TOOLCHAIN=1.100.0`.
- `perf` 6.8.12 (from `linux-tools-generic`, installed with apt), and GNU `time`. `perf_event_paranoid` is 2.
  Hardware counters are refused (`cycles`, `instructions` and `cache-misses` print `<not supported>`,
  `raw/perf_stat_hw_check.txt`). So the profiles sample the software clock (`cpu-clock`).

The load average (1, 5 and 15 min) before and after each group:

| Group | Before | After |
|---|---|---|
| 1: `grid_10m`, `lights`, timing | 1.74 1.53 0.74 | 2.15 1.65 0.86 |
| 2: `grid_100m`, timing | 1.67 1.57 0.85 | 3.87 3.05 2.18 |
| 3: the floor | 3.56 3.00 2.17 | 1.00 1.32 1.71 |
| 4: profile `grid_10m`, 1 job | 1.00 1.32 1.71 | 1.00 1.30 1.69 |
| 5: profile `grid_10m`, 4 jobs | 1.00 1.30 1.69 | 1.14 1.32 1.69 |
| 6: profile `grid_100m`, 1 job | 1.14 1.32 1.69 | 1.05 1.26 1.65 |
| 7: profile `grid_100m`, 4 jobs | 1.05 1.26 1.65 | 2.05 1.48 1.71 |

The load average counts the runs of the group before. Only one build or run went at any time: the groups ran
one after another in one shell chain.

## 2. The builds

| Binary | How | sha256 |
|---|---|---|
| `earnest` | `cargo build --release --locked -p earnest-cli` at `9c5413e` | `f9f9c742…e7fcd6` |
| `earnest-sym` | the same with `CARGO_PROFILE_RELEASE_DEBUG=true` and its own `CARGO_TARGET_DIR` | `4a325a44…d0e6c6` |
| `floor_i` | `measure/floor.rs` (`measure/cloud-results`) as it is. Its prefetch is for ARM only, so on x86-64 it has none | `8649ff57…c5ddf2` |
| `floor_ii` | the same with `measure/raw/block7_floor_x86.diff`: `_mm_prefetch(…, _MM_HINT_T0)` | `3faf1290…020779` |
| `floor_iii` | the same with a plain load ahead, `black_box(table.slots[table.home(h)])`, in place of the prefetch | `780b329c…dd94b` |

The floor programs are `measure/round6/scripts/floor_{i,ii,iii}.rs` from `measure/round6-results`. I checked
that `floor_i.rs` is byte for byte `measure/floor.rs`. They were built with `rustc -O` (1.100.0). The full hashes
are in `raw/machine.txt`.

**The two checker builds run in the same time.** Every case was timed with both builds in turn (table 3). The
medians differ by -2.4% to +4.8% in wall time and by -2.1% to +2.6% in CPU time. That is inside the 5% to 10%
spread of this machine. So I take the profiles of `earnest-sym` as profiles of `earnest`.

## 3. Wall time, CPU time and peak memory of the checker

`earnest verify --no-cache --json --jobs N --max-states 200000000 <model>.model.ts`, run from the model's folder.
`scripts/time_runs.py` is `measure/tools/scripts/linux/turn.py` with two additions: it keeps every run, and it
reads the status and the state count from the JSON. For each case there is one uncounted run, then 7 rounds
(`grid_10m`, `lights`) or 5 rounds (`grid_100m`). In each round the two builds run in turn. CPU time is user
plus system time from `wait4`. Peak memory is `ru_maxrss`. Every run printed `pass` with the expected number of
states (10,000,000, 3,000,000 or 100,000,000) and exited 0. Raw: `raw/g1_10m_lights.log`, `raw/g2_100m.log`.

Each cell is the median, then the smallest value in brackets. Times are in seconds.

| Model | Jobs | Build | Runs | Wall | CPU | User | System | Peak memory MiB |
|---|---:|---|---:|---:|---:|---:|---:|---:|
| `grid_10m` | 1 | earnest | 7 | 4.012 (3.825) | 3.947 (3.783) | 3.685 | 0.238 | 206.7 |
| `grid_10m` | 1 | earnest-sym | 7 | 3.916 (3.567) | 3.864 (3.523) | 3.563 | 0.256 | 206.9 |
| `grid_10m` | 4 | earnest | 7 | 1.417 (1.353) | 4.978 (4.881) | 4.705 | 0.338 | 197.5 |
| `grid_10m` | 4 | earnest-sym | 7 | 1.395 (1.327) | 4.975 (4.635) | 4.569 | 0.376 | 196.6 |
| `lights` | 1 | earnest | 7 | 1.089 (0.983) | 1.073 (0.973) | 1.054 | 0.047 | 65.5 |
| `lights` | 1 | earnest-sym | 7 | 1.105 (0.909) | 1.100 (0.897) | 1.044 | 0.040 | 65.7 |
| `lights` | 4 | earnest | 7 | 0.415 (0.404) | 1.416 (1.387) | 1.325 | 0.105 | 66.4 |
| `lights` | 4 | earnest-sym | 7 | 0.435 (0.396) | 1.464 (1.374) | 1.375 | 0.088 | 66.7 |
| `grid_100m` | 1 | earnest | 5 | 55.868 (54.272) | 54.996 (53.836) | 49.533 | 5.152 | 1621.0 |
| `grid_100m` | 1 | earnest-sym | 5 | 56.211 (55.488) | 55.743 (54.680) | 50.542 | 5.201 | 1621.2 |
| `grid_100m` | 4 | earnest | 5 | 17.716 (17.469) | 65.160 (63.639) | 59.171 | 6.016 | 1634.2 |
| `grid_100m` | 4 | earnest-sym | 5 | 18.055 (17.530) | 66.292 (64.765) | 60.283 | 6.034 | 1640.0 |

Minor page faults, from round 1 of `earnest` (`minflt` in the logs): 135,194 for `grid_10m` with 1 job and
2,620,480 for `grid_100m` with 1 job. At 4 KiB a page, that is 528 MiB faulted in for a peak of 207 MiB (2.6
times), and 10.0 GiB for a peak of 1,621 MiB (6.3 times).

## 4. The floor

`scripts/floor_block.sh`: one uncounted run (`floor_i 7 serial`), then 5 rounds at 7 wheels (10 million states)
and 5 at 8 wheels (100 million). In each round the variants run in turn (i, ii, iii). Every run printed the
expected state count and checksums (`4c9670a40c764e17/6ff96c9c5e19c80e` at 10 million and
`5a65a7777f25b9c1/b9bf724eeb0b4976` at 100 million) and exited 0. `time_s` is the floor's own clock, and CPU
is user plus system time from GNU `time`. Raw: `raw/g3_floor.log`.

| Variant | States | `time_s` median (smallest) | All 5 `time_s` | CPU median (smallest) | Peak memory MiB | Table |
|---|---:|---:|---|---:|---:|---|
| i, no prefetch | 10M | 2.998 (2.782) | 2.998, 3.046, 2.782, 2.904, 3.229 | 3.00 (2.81) | 277 | 2^24 slots, 128 MiB |
| ii, prefetch | 10M | 1.943 (1.833) | 1.868, 2.030, 1.943, 1.833, 1.986 | 1.94 (1.85) | 277 | 2^24 slots, 128 MiB |
| iii, plain load | 10M | 2.545 (2.352) | 2.431, 2.545, 2.352, 2.692, 2.699 | 2.49 (2.37) | 277 | 2^24 slots, 128 MiB |
| i, no prefetch | 100M | 53.426 (52.249) | 53.788, 58.396, 52.249, 52.408, 53.426 | 53.52 (52.50) | 4165 | 2^28 slots, 2 GiB |
| ii, prefetch | 100M | 31.622 (31.039) | 31.380, 33.270, 31.039, 31.622, 32.517 | 31.87 (31.29) | 4165 | 2^28 slots, 2 GiB |
| iii, plain load | 100M | 41.003 (39.802) | 43.565, 41.003, 39.802, 40.713, 43.615 | 41.08 (40.03) | 4165 | 2^28 slots, 2 GiB |

The prefetch saves 1.06 s (35%) at 10 million states and 21.80 s (41%) at 100 million (medians). The plain
load saves 0.45 s (15%) and 12.42 s (23%).

## 5. Profiles of the checker

### How they were taken

`scripts/profile.sh`: `perf record -e cpu-clock -F 999 -g` around a shell loop of N runs of `earnest-sym`,
from the model folder. N is 5 for `grid_10m` (20,844 and 25,061 samples) and 1 for `grid_100m` (62,372 and
78,358 samples). No samples were lost. Every profiled run printed `pass` and the expected state count. `-g`
takes frame-pointer call chains. Those chains are good for the kernel side only (the Rust code has no frame
pointers), and I use them only to see what led to a kernel sample and to find `memmove` calls from inside
`realloc`.

Sampling slowed the runs down. The profiled `grid_100m` run took 62.57 s of wall time with 1 job (55.87 s
unprofiled) and 21.35 s with 4 jobs (17.72 s). At 10 million states the profiled runs took 3.89 to 4.39 s with
1 job and 1.44 to 1.51 s with 4 jobs (`raw/prof_*.log`). So I take the **shares** from the profiles and turn
them into **seconds** with the median CPU time of the unprofiled `earnest` runs in table 3.

### How a sample is placed in a group

`scripts/attr.py` reads `perf script -F tid,ip,sym,dso,dsoff`. It turns the file offset of each sampled
address into a virtual address (through the `LOAD` segments) and resolves it with
`addr2line -i -f -C -e earnest-sym`, which gives the whole chain of inlined frames. Almost everything is
inlined into a few large functions (for example, `Search::work` holds the expansion and `Cond::eval` calls),
so the function name alone does not say what the code is doing. The source line does. The rules:

- Frames of `core`, `alloc` and `std` are skipped. The first frame in an earnest crate decides the group by its
  file: `store/index.rs` (a); `store/rows.rs`, `store/codec.rs`, `store/layout.rs` and `Store::read_state` in
  `store.rs` (b); `explore/part.rs`, `explore/merge.rs`, `store/level.rs` (c); `expand.rs`,
  `explore/encode.rs`, `earnest_ir`, `earnest_value` (d); `mix`, `hash_words` and `hash_each` in
  `store/layout.rs` (e); `explore/workers.rs`, `explore.rs`, `explore/here.rs`, `helpers.rs` (g).
- A hashbrown frame that is inlined into `store/level.rs` (the `table.entry` and `insert` of `claim_level`),
  into `store/index.rs` or into `store.rs` is a probe of the index (a). A hashbrown frame inlined into
  `explore/part.rs` is a probe of a part's own table (c). An outlined hashbrown function is placed by its type:
  `RawTable<Key<u32>>::reserve_rehash` (the index's shards growing) is (a).
- Kernel samples are placed by their kernel call chain. Page faults, `mmap`, `munmap` and `madvise` go to (f).
  Futex, thread start and exit, scheduling and everything else go to (g). libc `malloc`, `realloc`, `free` and
  their helpers go to (f), and so does a `memmove` whose caller is `realloc`. Other `memmove` and `memset`
  calls have no reliable caller, so they go to (g) as "caller not resolved".

`raw/prof_*_groups.json` holds every subgroup with its count. `raw/prof_10m_j1_lines.txt` and
`raw/prof_100m_j4_lines.txt` count samples by source line. `raw/prof_*_report.txt` is
`perf report --no-children --sort symbol --percent-limit 0.3`, demangled with `rustfilt`.

### Shares and seconds by group

The seconds are the share times the median CPU time of `earnest` from table 3.

| Group | 10M, 1 job | | 10M, 4 jobs | | 100M, 1 job | | 100M, 4 jobs | |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| | share | CPU s | share | CPU s | share | CPU s | share | CPU s |
| (a) index of known states | 20.5% | 0.81 | 35.6% | 1.77 | 22.9% | 12.57 | 30.4% | 19.82 |
| (b) packed rows | 10.3% | 0.41 | 7.6% | 0.38 | 11.0% | 6.07 | 9.4% | 6.14 |
| (c) part tables, claim and merge | 29.2% | 1.15 | 22.8% | 1.14 | 32.2% | 17.68 | 28.6% | 18.64 |
| (d) actions and rules | 24.0% | 0.95 | 19.5% | 0.97 | 19.2% | 10.55 | 17.4% | 11.33 |
| (e) hashing | 3.5% | 0.14 | 2.8% | 0.14 | 2.6% | 1.43 | 2.7% | 1.78 |
| (f) allocation and page faults | 9.2% | 0.36 | 8.1% | 0.41 | 9.7% | 5.36 | 9.1% | 5.95 |
| (g) threads, waiting and the rest | 3.4% | 0.13 | 3.5% | 0.17 | 2.4% | 1.33 | 2.3% | 1.51 |
| total (median CPU of `earnest`) | 100% | 3.95 | 100% | 4.98 | 100% | 55.00 | 100% | 65.16 |

By dso (`perf report --sort dso`): the kernel has 7.2%, 7.1%, 9.0% and 8.7% of the samples (same column
order), libc has 5.1%, 4.2%, 3.0% and 2.6%, and the rest is the earnest binary.

The largest subgroups (share of all samples; the full lists are in `raw/tables.md`):

| Subgroup | Group | 10M, 1 job | 10M, 4 jobs | 100M, 1 job | 100M, 4 jobs |
|---|---|---:|---:|---:|---:|
| index probe and insert in `claim_level` (hashbrown in `store/level.rs`) | a | 14.36% | 30.76% | 13.15% | 21.89% |
| the key compare `Key::holds` (`store/index.rs:48`) and the rest of `index.rs` | a | 3.72% | 2.94% | 8.50% | 7.57% |
| `RawTable<Key<u32>>::reserve_rehash` (shard tables growing) | a | 2.16% | 1.88% | 1.18% | 0.96% |
| a part's table, `offer_unseen_word` (hashbrown in `explore/part.rs:124`) | c | 12.53% | 10.83% | 18.07% | 17.95% |
| `store/level.rs` lines of its own (`hash_of` and `row_of` of the claim, `Winners`, `write_level`) | c | 11.03% | 6.83% | 8.80% | 5.19% |
| `explore/part.rs` lines of its own (`add_fresh`, `file_by_shard`) | c | 5.15% | 4.52% | 4.83% | 4.74% |
| `earnest_ir` (`Cond::eval`, `Num::eval`, `eval.rs`) | d | 13.86% | 11.36% | 11.26% | 10.40% |
| `expand.rs` (`Expand::state`, `try_input`) | d | 9.21% | 7.29% | 7.25% | 6.32% |
| `Rows::unpack` and the rest of `rows.rs` | b | 5.40% | 3.75% | 7.39% | 6.01% |
| kernel page faults | f | 5.96% | 4.69% | 7.72% | 7.14% |
| kernel `mmap`, `munmap`, `madvise` | f | 1.12% | 1.69% | 1.28% | 1.37% |
| libc `memmove`/`memset`, caller not resolved | g | 2.86% | 2.27% | 2.19% | 1.96% |
| kernel futex | g | below 0.1% | 0.10% | below 0.05% | below 0.05% |
| kernel thread start and exit | g | below 0.05% | 0.61% | below 0.05% | 0.17% |

Where the page faults come from, by the first user frame under the kernel chain (100M, 1 job): writes into
a part's fresh buffers in `explore/part.rs` 3.83%, libc (`memset`, `memmove`, `realloc` into fresh memory)
2.54%, the outlined rehash of the index 0.77%, `store/level.rs` (`write_level`) 0.58%. At 10M, 1 job: libc
3.08%, `part.rs` 1.54%, the index rehash 1.32%.

The heaviest source lines (`raw/prof_10m_j1_lines.txt`, `raw/prof_100m_j4_lines.txt`):

| Line | What it is | 10M, 1 job | 100M, 4 jobs |
|---|---|---:|---:|
| `store/level.rs:113` | `table.entry(…)` in `claim_level`: the probe of the index (all inlined hashbrown lines under it) | 11.6% | 17.5% |
| `explore/part.rs:124` | `met_words.entry(…)` in `offer_unseen_word`: the probe of a part's table | 9.9% | 16.2% |
| `store/level.rs:43` | `hash_of(part, ordinal)`, read in shard order by the claim | 6.49% | 3.10% |
| `store/index.rs:48` | `Key::holds`: compares the key that the probe found | 2.35% | 6.49% |
| `store/rows.rs:17` | `word_of`, the copy for a row shorter than a word (the rows are 4 bytes) | 3.80% | 4.96% |
| `store/level.rs:119` | `vacant.insert(filed)`: a new key into the index | below 0.6% | 3.02% |

### The ten heaviest functions

These are the functions as `perf` sees them, after inlining. The last column says how the samples of each
function split over the groups.

**`grid_10m`, 1 job**

| # | Share | Function | Groups |
|---:|---:|---|---|
| 1 | 26.30% | the closure of `store::level::gather` passed by `Shards<Key<u32>>::claim_level` | a 67%, c 33% |
| 2 | 19.26% | `explore::Search::work` (with the expansion inlined) | d 59%, e 17%, c 12%, b 9%, a 2% |
| 3 | 15.49% | `explore::part::PartSink::offer_encoded` | c 100% |
| 4 | 6.88% | `earnest_ir::code::cond::Cond::eval` | d |
| 5 | 5.11% | `earnest_ir::code::num::Num::eval` | d |
| 6 | 4.69% | `store::rows::Rows::unpack` | b |
| 7 | 3.13% | `store::Store::read_state` | b |
| 8 | 2.97% | libc `__memmove_avx512_unaligned_erms` | g 81% (caller not resolved), f 19% (in `realloc`) |
| 9 | 2.34% | `hashbrown::raw::RawTable<Key<u32>>::reserve_rehash` | a 93%, e 7% |
| 10 | 2.12% | the closure of `store::level::write_level` | c 66%, b 34% |

**`grid_10m`, 4 jobs**

| # | Share | Function | Groups |
|---:|---:|---|---|
| 1 | 39.29% | the closure of `gather` in `claim_level` | a 85%, c 15% |
| 2 | 15.39% | `Search::work` | d 58%, e 18%, c 13%, b 9% |
| 3 | 13.58% | `PartSink::offer_encoded` | c |
| 4 | 5.88% | `Cond::eval` | d |
| 5 | 4.24% | `Num::eval` | d |
| 6 | 3.30% | `Rows::unpack` | b |
| 7 | 2.51% | `Store::read_state` | b |
| 8 | 2.42% | libc `memmove` | g 79%, f 21% |
| 9 | 1.99% | `RawTable<Key<u32>>::reserve_rehash` | a 94%, e 6% |
| 10 | 1.43% | kernel `exc_page_fault` | f |

**`grid_100m`, 1 job**

| # | Share | Function | Groups |
|---:|---:|---|---|
| 1 | 28.73% | the closure of `gather` in `claim_level` | a 73%, c 27% |
| 2 | 21.27% | `PartSink::offer_encoded` | c |
| 3 | 15.52% | `Search::work` | d 57%, e 16%, c 12%, b 9%, a 4% |
| 4 | 6.74% | `Rows::unpack` | b |
| 5 | 5.76% | `Cond::eval` | d |
| 6 | 4.19% | `Num::eval` | d |
| 7 | 2.41% | libc `memmove` | g 80%, f 20% |
| 8 | 2.23% | kernel `exc_page_fault` | f |
| 9 | 2.20% | `Store::read_state` | b |
| 10 | 1.38% | the closure of `write_level` | c 53%, b 47% |

**`grid_100m`, 4 jobs**

| # | Share | Function | Groups |
|---:|---:|---|---|
| 1 | 33.20% | the closure of `gather` in `claim_level` | a 87%, c 13% |
| 2 | 21.38% | `PartSink::offer_encoded` | c |
| 3 | 14.19% | `Search::work` | d 54%, e 19%, c 12%, b 10%, a 4% |
| 4 | 5.46% | `Cond::eval` | d |
| 5 | 5.44% | `Rows::unpack` | b |
| 6 | 3.79% | `Num::eval` | d |
| 7 | 2.18% | libc `memmove` | g 79%, f 21% |
| 8 | 2.13% | kernel `exc_page_fault` | f |
| 9 | 2.04% | `Store::read_state` | b |
| 10 | 1.23% | the closure of `write_level` | c 54%, b 46% |

## 6. The checker's store work against the floor

The floor does in one table and one loop what groups (a), (b), (c) and (e) do in the checker: it finds the
successors of a state, hashes them, looks them up, and stores the new ones with their parents. Its successor
step is a few bit operations in `fill`, with no rule evaluation. So group (d) has no counterpart in the floor
in this form. "Smallest" below is the share times the smallest CPU time of the case: a rough lower value, not
a measured run.

| Run | (a)+(b)+(c)+(e), CPU s, median (smallest) | Ratio to ii | (a)+(b)+(c)+(e)+(f), CPU s | Ratio to ii | Floor i, no prefetch | Floor ii, prefetch | Floor iii, plain load | (d), CPU s, median (smallest) |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| 10M, 1 job | 2.50 (2.40) | 1.29 | 2.86 | 1.47 | 3.00 (2.78) | 1.94 (1.83) | 2.55 (2.35) | 0.95 (0.91) |
| 10M, 4 jobs (CPU of all threads) | 3.43 (3.36) | 1.77 | 3.84 | 1.98 | 3.00 (2.78) | 1.94 (1.83) | 2.55 (2.35) | 0.97 (0.95) |
| 100M, 1 job | 37.76 (36.96) | 1.19 | 43.12 | 1.36 | 53.43 (52.25) | 31.62 (31.04) | 41.00 (39.80) | 10.55 (10.33) |
| 100M, 4 jobs (CPU of all threads) | 46.37 (45.29) | 1.47 | 52.32 | 1.65 | 53.43 (52.25) | 31.62 (31.04) | 41.00 (39.80) | 11.33 (11.07) |

The floor columns are the floor's `time_s` (median, then smallest), all on one thread. The ratios use the
medians. The floor's `time_s` includes its own kernel time. From GNU `time`, its system time is 0.21 s to
0.30 s at 10 million states and 4.64 s to 9.12 s at 100 million (median 5.8 s for ii), mostly the faults of
its 2 GiB table and its state arrays. So the column with (f) is the fairer comparison of the two. The column
without (f) is the one the brief asked for.

## 7. Findings

Each finding is marked **measured** (it follows from the numbers above) or **reading** (my reading of
them, which this machine cannot prove, mostly because it has no hardware counters).

**What is within reach of the floor and what is not**

1. **Measured.** With one job, the store work of the checker, groups (a)+(b)+(c)+(e), is 1.29 times the
   floor with a prefetch at 10 million states (2.50 s against 1.94 s) and 1.19 times at 100 million (37.76 s
   against 31.62 s). It is faster than the floor without a prefetch at both sizes (2.50 s against 3.00 s;
   37.76 s against 53.43 s), and faster than the floor with a plain load (2.55 s and 41.00 s). With the
   kernel and allocation time (f) added on the checker's side, as the floor's time includes its own, the
   ratios are 1.47 and 1.36.
2. **Measured.** The whole checker with one job at 100 million states (55.00 s of CPU, 55.87 s of wall time)
   takes about as long as the floor without a prefetch (53.43 s). At 10 million it takes 3.95 s, against
   3.00 s.
3. **Measured.** The floor is not a hard lower bound here. Its single table has 2^28 slots (2 GiB) at 100
   million states, and its process peaks at 4,165 MiB. The checker peaks at 1,621 MiB, and its index is 64
   shard tables of 4-byte keys, claimed one shard at a time. **Reading:** a shard of the checker at 100
   million states holds about 1.6 million keys: 2^21 buckets of 4 bytes plus a control byte each, about
   10 MiB. That fits
   in the 260 MiB L3, while the floor's probes go to a 2 GiB table. That is how the checker's store work can
   beat a floor that has no prefetch.
4. **Measured.** With 4 jobs, only group (a) grows. It goes from 0.81 s to 1.77 s of CPU at 10 million states
   (2.2 times) and from 12.57 s to 19.82 s at 100 million (1.58 times, +7.25 s). At 100 million, (b) gains
   0.07 s, (c) 0.96 s, (d) 0.78 s and (e) 0.35 s. At 10 million, none of them changes by more than 0.03 s. Kernel futex time is below 0.1%
   of the samples. **Reading:** the extra time is the index probes of `claim_level` (`level.rs:113`, 14.4% →
   30.8% of the samples at 10 million) on threads that share the L3 and the memory bus, not waiting on a
   lock. The shard tables are behind a `Mutex`, but each shard is taken by one task, and no lock wait shows
   in the kernel. A wait in user space would have to be a spin, and no spin function shows up in the profile.
5. **Measured.** Group (d), evaluating actions and rules, has 0.95 s to 0.97 s at 10 million states and
   10.55 s to 11.33 s at 100 million: 17% to 24% of the CPU time. The floor has no such work. Of the group,
   `earnest_ir` (`Cond::eval`, `Num::eval`, `eval.rs`) takes about 60% and `expand.rs` 40%.
6. **Reading.** Within reach of the floor: (a)+(b)+(c)+(e) with one job, already 1.2 to 1.3 times the floor
   with a prefetch, and 1.36 to 1.47 times with (f) added. The (f) time itself, 0.36 s and 5.36 s with one
   job, is about what the floor pays in the kernel (about 0.25 s and 5.8 s). Not within reach of the floor
   in this form: (d), which the floor does not do (0.95 s and 10.55 s). Also not within reach: the extra
   CPU of (a) with 4 jobs (point 4). The 4-job store work is 1.77 and 1.47 times the one-thread floor.

**The three largest sources of time that are not the machine waiting for memory (reading, by kind of code)**

I sort the code into "waits for memory" and "does not" by what each line does. Without counters, that sort
is my reading. The lines I count as memory waits are the probes into large tables: `level.rs:113` and `:119`,
`index.rs:48` and `part.rs:124`.

7. **Rule and action evaluation, group (d): 10.55 s at 100 million states with 1 job and 0.95 s at 10
   million.** The functions are `earnest_ir::code::cond::Cond::eval` (5.8% of the samples at 100M, 1 job),
   `earnest_ir::code::num::Num::eval` (4.2%), and the inlined `Expand::state` and `Expansion::try_input` of
   `expand.rs` inside `Search::work` (7.3%). The hot lines are `cond.rs:152`, `cond.rs:164`, `num.rs:180`,
   `num.rs:95`, `eval.rs:478`, `expand.rs:410` and `expand.rs:520`. They work on the state already read into
   values, so they are compute.
8. **Kernel page faults and allocation, group (f): 5.36 s at 100 million states with 1 job and 0.36 s at 10
   million.** The page faults alone are 7.7% of the samples. **Measured:** the run takes 2,620,480 minor faults
   at 100 million states, 6.3 times its peak memory in pages, and system time is 5.15 s of the 55.00 s. The
   faults come from writes into a part's fresh buffers (`PartSink::add_fresh` in `explore/part.rs`), from libc
   (`memset`, `memmove` and `realloc` into new memory), from `RawTable<Key<u32>>::reserve_rehash` and from
   `write_level`. **Reading:** memory is given back and taken anew level after level, so the same bytes are
   faulted in more than once. The floor pays about the same in the kernel at 100 million states (system time
   median about 5.8 s, for a 4 GiB peak faulted once). So this is a cost of the kernel per page, not a gap to
   the floor in seconds. It is still 10% of the checker's time.
9. **Reading and unpacking rows, group (b): 6.07 s at 100 million states with 1 job and 0.41 s at 10
   million.** The functions are `Rows::unpack` (6.7% at 100M, 1 job) and `Store::read_state` (2.2%). Their
   hottest line is `rows.rs:17`, the `copy_from_slice` path of `word_of` for a row shorter than 8 bytes (this
   grid's rows are 4 bytes). **Reading:** the frontier is read in order, so this is copying and turning a row
   into values, not memory waits.

   Next in size, and only partly memory, is the claim's own reading of the parts in `store/level.rs` (8.8%
   at 100M, 1 job, 4.84 s): `hash_of` (`level.rs:43`) and `row_of` read a part's hashes and rows in shard
   order, so each of the 64 shards walks the whole part.

**What surprised me**

10. **Measured.** `PartSink::offer_encoded` is 13.6% to 21.4% of the samples (0.61 s at 10 million states
    and 11.70 s at 100 million with 1 job), and almost all of it is the probe of the part's own table in `offer_unseen_word`
    (`part.rs:124`): 12.5% of the samples at 10 million states and 18.1% at 100 million with 1 job, which is
    9.94 s. That is about 80% of the time the index probes take (`level.rs:113` and `index.rs`, 21.65% at
    100M, 1 job) for a table that holds only one part. **Reading:** with no graph, `offer_unseen` does not ask
    the index, so a part's table takes every successor of the part, about 7 offers for each new row in this
    grid. The table grows with the width of the level divided by 64 parts, and at 100 million states it has
    outgrown the L2.
11. **Measured.** Page faults are a tenth of the time: the kernel has 7.1% to 9.0% of the samples and system
    time is 5.15 s to 6.02 s at 100 million states, with 6.3 times as many faults as the peak memory has pages
    (point 8). The floor, faulting a peak twice as large once, has about the same system time.
12. **Measured.** `RawTable<Key<u32>>::reserve_rehash`, the shard tables of the index growing, is 1.0% to
    2.3% of the samples, and 0.77% to 1.32% more goes to the page faults it takes.
13. **Measured.** Nothing waits on a lock in the kernel: futex is at most 0.10% of the samples. Thread start
    and exit is 0.61% at 10 million states with 4 jobs and 0.17% at 100 million.
14. **Measured.** One job at 10 million states now takes 4.01 s of wall time (3.95 s of CPU). The 4-job run
    takes 1.42 s with 4.98 s of CPU, so 4 threads are 2.8 times faster in wall time for 26% more CPU.

## 8. What failed or was skipped

- **Hardware counters:** refused on this guest. Cache misses, instructions and cycles are not measured, so
  "waiting for memory" is my reading of source lines, not a measurement.
- **The first run of the attribution script** placed every sample in the wrong file. It took perf's file
  offset for a virtual address (the text segment is mapped 0x1000 higher). The fixed script is the one in
  `scripts/`, and every number above comes from it. A second fix: the script first read the sample headers
  of `perf script` as call-chain lines.
- **The callers of `memmove` and `memset`:** 2.0% to 2.9% of the samples. Only the part called from
  `realloc` is resolved (0.5% to 0.6%). The rest is in (g) as "caller not resolved". A DWARF call-graph
  recording would place them, but I did not make one.
- **`lights` was timed but not profiled.** The brief asked for profiles of `grid_10m` and `grid_100m` only.
- **The 4-thread floor** (`floor 7 4`) was not run. The brief asked for the floor's variants, and the earlier
  results ran them on one thread.
- **Uncounted runs:** one per timing case and one before the floor block. There was none before the
  profiles. The profiled runs are slower than unprofiled ones (section 5), which is why the seconds come from
  the unprofiled medians.
- **Line attribution and skid:** a sample of the software clock lands on the instruction where the timer
  interrupt was taken. That may be the instruction after a slow load, so a share given to a line can belong
  to the line just before it.
- `perf.data` and the `perf script` dumps are not kept (20 to 25 MB each). The reports, the groups and the
  line counts made from them are in `raw/`.

## 9. Commands

```
rustup toolchain install 1.100.0 --profile minimal
export RUSTUP_TOOLCHAIN=1.100.0
git worktree add /SCRATCH/src/tip 9c5413e
cd /SCRATCH/src/tip && cargo build --release --locked -p earnest-cli
CARGO_PROFILE_RELEASE_DEBUG=true CARGO_TARGET_DIR=/SCRATCH/target-sym cargo build --release --locked -p earnest-cli
rustc -O -o floor_i floor_i.rs; rustc -O -o floor_ii floor_ii.rs; rustc -O -o floor_iii floor_iii.rs
# models: tasks/benchmark/models/grid_10m.model.ts (checkout), grid_100m and lights from measure/tools, in one folder
cd /SCRATCH/models
python3 time_runs.py plain=earnest,sym=earnest-sym 7 grid_10m:1,grid_10m:4,lights:1,lights:4   # raw/g1_10m_lights.log
python3 time_runs.py plain=earnest,sym=earnest-sym 5 grid_100m:1,grid_100m:4                   # raw/g2_100m.log
bash floor_block.sh                                                                             # raw/g3_floor.log
bash profile.sh 10m_j1 grid_10m 1 5; bash profile.sh 10m_j4 grid_10m 4 5
bash profile.sh 100m_j1 grid_100m 1 1; bash profile.sh 100m_j4 grid_100m 4 1
python3 attr.py X.script earnest-sym > raw/prof_X_groups.json; python3 tables.py > raw/tables.md
python3 lines.py X.script earnest-sym 40 > raw/prof_X_lines.txt
```

Every group also printed `scripts/machine.sh` before and after: `nproc`, the processor model, `free -m` and
`/proc/loadavg`.

## 10. Raw outputs

- `raw/machine.txt`: `lscpu`, `free -m`, the kernel, the toolchain, `perf --version`, `perf_event_paranoid`,
  THP, and the sha256 of every binary.
- `raw/g1_10m_lights.log`, `raw/g2_100m.log`: one JSON line for every run (wall, user, sys, peak memory, exit
  code, status, states, `milliseconds`, context switches, minor faults), the summaries, and the machine lines.
- `raw/g3_floor.log`: the whole output of every floor run, with GNU `time`.
- `raw/prof_<run>.log`: the `perf record` lines and GNU `time` for every profiled run.
  `raw/prof_<run>_report.txt`: the flat symbol report down to 0.3%. `raw/prof_<run>_dso.txt`: the split by
  dso. `raw/prof_<run>_groups.json`: groups, subgroups, the sites of kernel samples, and the 40 heaviest
  symbols. `raw/prof_10m_j1_lines.txt`, `raw/prof_100m_j4_lines.txt`: samples by source line.
  `raw/tables.md`: the tables of section 5 as the script printed them.
- `raw/perf_stat_hw_check.txt`: the refused hardware counters.
