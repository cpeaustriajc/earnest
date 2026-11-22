# Round 6: where the search still loses time, measured on the target machine

Measured on 2026-11-22 on a 4-vCPU, 16 GB cloud machine (KVM guest, Linux x86-64). The tip is `perf/joined` at `0ffa4d5`. The session started on another branch, at `2d61e74` (`main`). Before building anything I checked out `origin/perf/joined` at `0ffa4d5`. Every binary of the tip and every binary of the prototype comes from `0ffa4d5`.

## Report

1. **The parts path on one CPU (c) against 1 worker (a).** At 10 million states, c uses 2.28 s less CPU time than a (medians 3.48 s against 5.76 s, 40% less), and 2.25 s less wall time (3.52 s against 5.77 s). At 100 million states, c uses 53.65 s less CPU time (45.76 s against 99.41 s, 54% less), and 53.83 s less wall time (45.82 s against 99.65 s). Variant c is a stand-in for "1 worker expands a large level in parts". It also pays for thread switches and locks, so the change itself would not pay for those. With all 4 CPUs (b), the 4 workers use more CPU time than c: 4.22 s at 10 million states and 50.17 s at 100 million.
2. **The largest functions at 10 million states with 4 workers.** The closure of `claim_level` (the index) takes 43.0% of the samples. `Search::work` takes 13.4% and `PartSink::offer_encoded` 11.8%. Next come `Cond::eval` (4.2%), `Num::eval` (3.7%) and `Rows::unpack` (3.2%). User code has 91% of the samples and the kernel 9%. In the old 100-million profile (build C, on a slower CPU), the `claim_level` closure was 28.6%, `offer_encoded` 20.5% and `Search::work` 17.5%. I also profiled the tip at 100 million states on this machine (an extra). There, the shares are 37.2%, 18.0% and 13.5%. So at 10 million states the index takes a larger share and `offer_encoded` a smaller one than at 100 million. The kernel share is about the same (8.8% at 100 million).
3. **The prefetch and the plain load in the floor.** At 10 million states, the prefetch (ii) cuts the median from 2.70 s to 1.65 s, a gain of 1.05 s or 39%. The plain load (iii) cuts it to 1.90 s, a gain of 0.80 s or 30%. At 100 million states, the prefetch cuts the median from 43.36 s to 23.44 s, a gain of 19.92 s or 46%. The plain load cuts it to 30.72 s, a gain of 12.64 s or 29%. By the medians, the plain load gets 76% of the prefetch's gain at 10 million states and 63% at 100 million. By the smallest values it gets 38% and 40%.
4. **The two-pass claim, `--jobs 4`.** I tried batches of 16, 8 and 32. At 10 million states, the median wall times are 1.08 s (tip), 1.07 s (16), 1.09 s (8) and 1.05 s (32). The median CPU times are 3.91 s, 3.88 s, 3.95 s and 3.73 s. At 100 million states, the median wall times are 13.08 s, 13.06 s, 13.57 s (32) and 13.61 s (8). The median CPU times are 48.74 s, 48.55 s, 50.40 s (32) and 50.64 s (8). Every difference is smaller than the spread between runs of one build: the tip alone ran from 1.05 s to 1.26 s at 10 million states, and from 12.69 s to 13.82 s at 100 million. The tests pass with the prototype, and the JSON without `milliseconds` is the same for the tip and for all three batch sizes.

### Odd numbers, as measured

- Variant c, 4 workers on one CPU, needs less CPU time than 1 worker: 40% less at 10 million states and 54% less at 100 million. It also needs less CPU time than 4 workers on 4 CPUs (b): 3.48 s against 4.22 s, and 45.76 s against 50.17 s.
- Variant a at 10 million states falls into two groups. Three runs took 4.59 s to 5.05 s and four took 5.77 s to 6.99 s (the uncounted run took 6.68 s). At 100 million states one run took 89.03 s and two took 99.65 s and 100.30 s. I did not look for the cause.
- The floor without a prefetch varies a lot at 100 million states: from 35.54 s to 48.21 s across 5 runs. At 10 million states it took either about 2.2 s (2 runs) or about 2.7 s (3 runs). The prefetch build has two groups too: 1.37 s to 1.39 s (2 runs) and 1.65 s (3 runs). The plain-load build took 1.86 s to 1.90 s (3 runs) or 2.43 s to 2.46 s (2 runs).
- At 10 million states the tip with 1 worker spends 66.4% of the samples in `Store::place`, about the same share as at 100 million states in the old profile (69.8%, build C).
- This CPU is not the CPU of the last session. lscpu gives a BogoMIPS of 5400 here, against 4200 there, and an L3 of 480 MiB, against 260 MiB. So the numbers here cannot be set beside the old numbers as if both came from one machine. For the tip with 4 workers at 10 million states, the median wall time is 1.16 s (task 1) and 1.08 s (task 4). The old session's medians were 1.72 s for build C, on the other CPU.
- In the c runs, the involuntary context switches have a median of 868 at 10 million states and 11,305 at 100 million. For b the medians are 373 and 1,345, and for a 14 and 211 (from `/usr/bin/time -v`).
- The load average was 1.86 to 2.68 before each task 4 block, and 0.94 to 2.23 before the other blocks. The block before, or my own JSON runs and builds, ran just before. Each block was the only heavy work while it ran.

### What I did not measure

- **Hardware counters.** `perf stat -e cycles,instructions` prints `<not supported>` on this guest (`raw/perf_stat_hw_check.txt`). Cache misses and instruction counts are not measured. So no number here says whether a gain comes from fewer memory waits or from fewer instructions.
- **Call graphs.** The profiles are flat (`--sort symbol`, cpu-clock, 999 Hz), so the callers of a function are not measured.
- **A matching `perf`.** The only `perf` that apt offers is 6.8.12, and the kernel is 6.18.44. `perf` sampled and printed without errors.
- **Demangling.** `perf` 6.8 prints Rust v0 names mangled. I read them by hand. I did not try to install a demangler.
- **The machine code of floor variant iii.** I did not check whether the compiler keeps the load where it stands in `fill`. It ran slower than ii and faster than i, so it does something.
- **The order of the rows a part wins.** I made sure of it only by construction: pass 2 runs in the same order as today's loop, and pass 1 never inserts anything. The JSON of `grid_10m` holds no state order, so the equal JSON does not prove the order. `cargo test -p earnest_check` passes.
- **The two-pass claim with 1 worker.** Not asked for. With 1 worker, `Store::place` is used and `claim_level` is not.
- **An uncounted run before the 100-million blocks.** Each task had one uncounted round before its 10-million block. The 100-million blocks came right after, with no uncounted run. Task 3 had one uncounted floor run (`floor_i 7 serial`).
- **The 4-thread floor.** Not asked for this round.

## 1. The machine

From `raw/machine.txt`:

- Intel Xeon Processor (family 6, model 173, stepping 1), 4 vCPUs, 1 thread per core, KVM guest. BogoMIPS 5400. L1d 192 KiB (4 instances), L2 8 MiB (4 instances), L3 480 MiB (1 instance).
- Memory 15 GiB total in `free -g` (`free -m` is in the file). No swap.
- Kernel 6.18.44-fc-v726.
- `rustc 1.97.0 (2d8144b78 2026-07-07)` and `cargo 1.97.0 (c980f4866 2026-06-30)`. The tip was built with `rustup run 1.97.0 cargo build --release --locked -p earnest-cli`. The sha256 of the binary is `41cd0147…39fe4`.
- `perf` 6.8.12 and `/usr/bin/time` (GNU time) were installed with apt. `perf_event_paranoid` is 2.
- The model of 100 million states is `measure/grid_100m.model.ts` from the branch `measure/cloud-results`. I copied it to the same path for the runs. It is not part of this branch.

Every `verify` run was `/usr/bin/time -v [taskset -c 0] earnest verify --no-cache --json --jobs N --max-states 200000000 MODEL`. Wall time is the "Elapsed (wall clock) time" of `/usr/bin/time -v`, and CPU time is its user time plus its system time. The `.csv` files also hold the `milliseconds` that `earnest` prints, the peak memory and the context switches. A parser checks status `pass`, the state count and exit 0 for every run, including the uncounted ones. Every run passed: 24 + 9 + 32 + 12 runs.

## 2. Task 1: three ways to run the tip

Runs go round-robin a, b, c inside each round. Round 0 is not counted. Raw: `raw/task1_grid_10m.*`, `raw/task1_grid_100m.*`.

`grid_10m`, 7 counted rounds:

| Variant | Workers | CPUs | Wall min | Wall median | Wall max | CPU min | CPU median | CPU max | Median user | Median sys | Median peak memory |
|---|---:|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| a | 1 | all | 4.59 | 5.77 | 6.99 | 4.58 | 5.76 | 6.95 | 5.67 | 0.09 | 163 MiB |
| b | 4 | all | 1.07 | 1.16 | 1.32 | 3.86 | 4.22 | 4.75 | 3.87 | 0.39 | 190 MiB |
| c | 4 | `taskset -c 0` | 3.21 | 3.52 | 3.78 | 3.19 | 3.48 | 3.76 | 3.10 | 0.38 | 196 MiB |

Every run, wall s / CPU s:

| Variant | 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| a | 6.68 / 6.66 | 6.99 / 6.95 | 5.05 / 5.01 | 4.59 / 4.58 | 4.68 / 4.67 | 6.36 / 6.34 | 6.61 / 6.56 | 5.77 / 5.76 |
| b | 1.27 / 4.66 | 1.16 / 4.22 | 1.14 / 4.14 | 1.07 / 3.92 | 1.08 / 3.86 | 1.26 / 4.49 | 1.32 / 4.75 | 1.28 / 4.71 |
| c | 3.33 / 3.32 | 3.21 / 3.19 | 3.31 / 3.30 | 3.26 / 3.25 | 3.63 / 3.62 | 3.78 / 3.76 | 3.52 / 3.48 | 3.72 / 3.71 |

`grid_100m`, 3 rounds:

| Variant | Workers | CPUs | Wall min | Wall median | Wall max | CPU min | CPU median | CPU max | Median user | Median sys | Median peak memory |
|---|---:|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| a | 1 | all | 89.03 | 99.65 | 100.30 | 89.00 | 99.41 | 100.14 | 98.50 | 0.91 | 1411 MiB |
| b | 4 | all | 12.86 | 13.43 | 13.88 | 48.02 | 50.17 | 51.72 | 46.37 | 3.82 | 1647 MiB |
| c | 4 | `taskset -c 0` | 45.31 | 45.82 | 47.95 | 45.28 | 45.76 | 47.86 | 41.25 | 4.51 | 1619 MiB |

| Variant | 1 | 2 | 3 |
|---|---:|---:|---:|
| a | 99.65 / 99.41 | 89.03 / 89.00 | 100.30 / 100.14 |
| b | 12.86 / 48.02 | 13.88 / 51.72 | 13.43 / 50.17 |
| c | 45.31 / 45.28 | 45.82 / 45.76 | 47.95 / 47.86 |

Variant c is a stand-in for a change under discussion: 1 worker expands a large level in parts, as the workers do. The stand-in also pays for thread switches and for locks between 4 threads on one CPU, which that change would not pay for.

## 3. Task 2: profiles at 10 million states

The command is `perf record -e cpu-clock -F 999 -o … -- sh -c 'for i in $(seq N); do earnest-tip verify … grid_10m …; done'`. One recording holds N runs, so that a profile reaches 20,000 samples: N = 5 with 4 workers (22,795 samples) and N = 4 with 1 worker (24,580 samples). There were 0 lost samples. Every run in a recording printed `pass` and 10,000,000 states. One uncounted run came first (`raw/task2_warmup.json`). The table shows the 12 largest lines of `perf report --no-children --sort symbol --percent-limit 0`. Raw: `raw/task2_profile_10m_j4.txt`, `raw/task2_profile_10m_j1.txt` (whole reports, with `--sort dso`), and `raw/*_top12.txt`.

The extra profile (not asked for) is the tip at 100 million states with 4 workers: one run, 54,859 samples, `raw/task2x_profile_100m_j4.txt`. I added it because the only other 100-million profile was of build C on another CPU.

User and kernel split. "Lines" is the sum of the `[.]` and `[k]` lines of the symbol report. "dso" is `perf report --sort dso`.

| Profile | User, lines | Kernel, lines | dso: earnest | dso: kernel | dso: libc |
|---|---:|---:|---:|---:|---:|
| 10M, 4 workers | 90.88% | 8.72% | 85.64% | 9.06% | 5.29% |
| 10M, 1 worker | 98.25% | 1.62% | 96.47% | 1.74% | 1.77% |
| 100M, 4 workers (extra) | 91.16% | 8.69% | 88.46% | 8.82% | 2.72% |

**10 million states, 4 workers**

| # | Share | Mode | Symbol as printed (shortened) | My reading |
|---:|---:|---|---|---|
| 1 | 43.02% | user | `…store5level6gather…Shards…NarrowKey…11claim_level0Es_0` | the closure of `store::level::gather` that `Shards::claim_level` passes: the shards claim the rows of a level |
| 2 | 13.40% | user | `…explore7workers…6Search4work` | `explore::workers::Search::work`, the worker loop |
| 3 | 11.76% | user | `…explore4part…8PartSink13offer_encoded` | `explore::part::PartSink::offer_encoded`, a worker's own small table |
| 4 | 4.19% | user | `…earnest_ir4code4cond…4Cond4eval` | `earnest_ir::code::cond::Cond::eval` |
| 5 | 3.70% | user | `…earnest_ir4code3num…3Num4eval` | `earnest_ir::code::num::Num::eval` |
| 6 | 3.19% | user | `…store4rows…4Rows6unpack` | `store::rows::Rows::unpack` |
| 7 | 2.50% | user | `…store…5Store10read_state` | `store::Store::read_state` |
| 8 | 1.85% | user | `realloc` | libc |
| 9 | 1.78% | user | `__memmove_avx512_unaligned_erms` | libc |
| 10 | 1.74% | user | `…hashbrown3raw…RawTable…NarrowKey…14reserve_rehash…claim_level…` | `hashbrown` growing a shard table inside `claim_level` |
| 11 | 1.69% | kernel | `exc_page_fault` | page faults |
| 12 | 1.40% | kernel | `_raw_spin_unlock_irqrestore` | kernel lock |

**10 million states, 1 worker**

| # | Share | Mode | Symbol (shortened) | My reading |
|---:|---:|---|---|---|
| 1 | 66.37% | user | `…store…5Store5place` | `store::Store::place`, which looks one row up at a time |
| 2 | 9.37% | user | `…explore…6Search5level` | `explore::Search::level`, the loop over one level |
| 3 | 9.33% | user | `…store4rows…4Rows6unpack` | `Rows::unpack` |
| 4 | 3.29% | user | `…4Cond4eval` | `Cond::eval` |
| 5 | 2.68% | user | `…3Num4eval` | `Num::eval` |
| 6 | 1.87% | user | `…explore4here…8HereSink9place_row` | `explore::here::HereSink::place_row` |
| 7 | 1.77% | user | `…5Store10read_state` | `Store::read_state` |
| 8 | 1.59% | user | `__memmove_avx512_unaligned_erms` | libc |
| 9 | 1.43% | user | `…hashbrown…14reserve_rehash…claim_level…` | `hashbrown` growing a table. The symbol names `claim_level`'s closure as the hasher, but the 1-worker search also grows tables; I did not check from where |
| 10 | 0.37% | user | `…drop_glue…earnest_value5value5Value…` | dropping `earnest_value::Value` |
| 11 | 0.35% | kernel | `exc_page_fault` | page faults |
| 12 | 0.28% | kernel | `_raw_spin_lock` | kernel lock |

**100 million states, 4 workers (extra), to compare with 10 million**

| # | Share | Mode | Reading |
|---:|---:|---|---|
| 1 | 37.22% | user | `claim_level` closure |
| 2 | 18.02% | user | `PartSink::offer_encoded` |
| 3 | 13.51% | user | `Search::work` |
| 4 | 5.69% | user | `Rows::unpack` |
| 5 | 4.19% | user | `Cond::eval` |
| 6 | 4.09% | user | `Num::eval` |
| 7 | 2.33% | kernel | `exc_page_fault` |
| 8 | 2.19% | user | `Store::read_state` |
| 9 | 1.94% | user | `__memmove_avx512_unaligned_erms` |
| 10 | 1.40% | user | closure of `store::level::write_level` |
| 11 | 1.27% | kernel | `_raw_spin_lock` |
| 12 | 1.24% | user | `hashbrown` `reserve_rehash` inside `claim_level` |

From 10 million to 100 million states with 4 workers, both on the tip on this machine: `claim_level` drops from 43.0% to 37.2%, and `offer_encoded` rises from 11.8% to 18.0%. `Search::work` stays at 13.4% and 13.5%, and the kernel at 8.7%. `realloc` is 1.85% at 10 million states and below the 12th line at 100 million. The old profile (build C, other CPU, 100 million states) had 28.6% / 20.5% / 17.5% for `claim_level` / `offer_encoded` / `Search::work`. The readings of the names are mine, made from the symbols and from the source of `crates/earnest_check` and `crates/earnest_ir`. They are not `perf` output.

## 4. Task 3: the floor

`measure/floor.rs` from `measure/cloud-results`, one thread (`serial`), built with `rustup run 1.97.0 rustc -O`. The three variants are in `scripts/`:

- i: `floor_i.rs`, the file unchanged. Its prefetch is written for ARM only, so on x86-64 it does nothing.
- ii: `floor_ii.rs`, with `measure/raw/block7_floor_x86.diff` applied (`_mm_prefetch`, `_MM_HINT_T0`).
- iii: `floor_iii.rs`. In `fill`, `table.prefetch(h)` is replaced by `std::hint::black_box(table.slots[table.home(h)]);`, a plain, bounds-checked load with no `unsafe` (`scripts/floor_iii.diff`).

The variants go round-robin i, ii, iii inside each round. Every run printed the expected checksums: `4c9670a40c764e17/6ff96c9c5e19c80e` at 10 million states and `5a65a7777f25b9c1/b9bf724eeb0b4976` at 100 million. Raw: `raw/task3_floor.log`, `.csv`, `_table.md`.

| Variant | Arguments | States | Runs | Smallest | Median | Largest | Every counted run (s) |
|---|---|---:|---:|---:|---:|---:|---|
| i, no prefetch | `7 serial` | 10000000 | 5 | 2.1629 | 2.6965 | 2.7423 | 2.1629, 2.7032, 2.7423, 2.6965, 2.2759 |
| ii, prefetch | `7 serial` | 10000000 | 5 | 1.3662 | 1.6455 | 1.6489 | 1.6473, 1.6489, 1.3920, 1.3662, 1.6455 |
| iii, plain load | `7 serial` | 10000000 | 5 | 1.8561 | 1.8998 | 2.4561 | 1.8998, 1.8817, 2.4278, 2.4561, 1.8561 |
| i, no prefetch | `8 serial` | 100000000 | 5 | 35.5421 | 43.3565 | 48.2082 | 35.5421, 43.3565, 48.2082, 36.7482, 45.4151 |
| ii, prefetch | `8 serial` | 100000000 | 5 | 21.9146 | 23.4353 | 28.4147 | 28.4147, 26.2281, 21.9146, 23.3623, 23.4353 |
| iii, plain load | `8 serial` | 100000000 | 5 | 30.1345 | 30.7184 | 33.0150 | 30.1345, 33.0150, 32.6549, 30.4419, 30.7184 |

Gains against i:

| States | ii, by median | iii, by median | iii / ii | ii, by smallest | iii, by smallest | iii / ii |
|---:|---:|---:|---:|---:|---:|---:|
| 10 million | 1.05 s (39%) | 0.80 s (30%) | 76% | 0.80 s (37%) | 0.31 s (14%) | 38% |
| 100 million | 19.92 s (46%) | 12.64 s (29%) | 63% | 13.63 s (38%) | 5.41 s (15%) | 40% |

At 10 million states, the tip with 1 worker took 5.77 s (median, task 1). That is 2.1 times floor i, 3.5 times floor ii and 3.0 times floor iii. At 100 million states it took 99.65 s: 2.3, 4.3 and 3.2 times. The floor is a hand-written search of this one grid, not a general search.

## 5. Task 4: the two-pass claim

The prototype is `claim_two_passes.patch`. It applies to `0ffa4d5` and changes only `crates/earnest_check/src/store/level.rs`. In `claim_level`, the rows of a part in one shard are walked in batches of `CLAIM_BATCH` (16 in the patch). Pass 1 calls `table.find(hash, |filed| filed.holds(rows, row))` for every row of the batch and notes which rows it found. Pass 2 runs over the same rows in the same order, skips the rows pass 1 found, and does `table.entry(…)` and the insert as today. Pass 1 inserts nothing, and pass 2 keeps today's order, so a part wins the same rows in the same order. Batches of 8 and 32 differ from the patch only in that constant. The prototype is not committed into the source tree. It was built in a separate worktree.

- `rustup run 1.97.0 cargo test --locked -p earnest_check` with batch 16: 26 + 109 + 0 tests, all pass (`raw/task4_cargo_test_batch16.txt`). I did not run the tests with 8 and 32.
- The JSON of `grid_10m` with `--jobs 4`, without the `milliseconds` lines, is byte for byte the same for the tip and for batches of 16, 8 and 32 (`raw/task4_json_compare.txt`, `raw/task4_json_*.json`).
- Timing: `--jobs 4`. The builds go round-robin tip, 16, 8, 32 inside each round. Raw: `raw/task4_grid_10m.*`, `raw/task4_grid_100m.*`. The binary hashes are in the last section.

`grid_10m`, 7 counted rounds (round 0 not counted):

| Build | Wall min | Wall median | Wall max | CPU min | CPU median | CPU max | Median peak memory |
|---|---:|---:|---:|---:|---:|---:|---:|
| tip | 1.05 | 1.08 | 1.26 | 3.86 | 3.91 | 4.59 | 190 MiB |
| batch 16 | 1.03 | 1.07 | 1.22 | 3.75 | 3.88 | 4.33 | 189 MiB |
| batch 8 | 1.03 | 1.09 | 1.18 | 3.75 | 3.95 | 4.33 | 190 MiB |
| batch 32 | 0.99 | 1.05 | 1.16 | 3.61 | 3.73 | 4.27 | 190 MiB |

| Build | 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| tip | 1.12 / 4.13 | 1.26 / 4.59 | 1.05 / 3.87 | 1.21 / 4.41 | 1.07 / 3.87 | 1.05 / 3.86 | 1.08 / 3.91 | 1.13 / 4.03 |
| 16 | 1.10 / 3.95 | 1.22 / 4.33 | 1.16 / 4.25 | 1.13 / 4.06 | 1.06 / 3.75 | 1.07 / 3.88 | 1.07 / 3.84 | 1.03 / 3.77 |
| 8 | 1.10 / 4.01 | 1.18 / 4.33 | 1.07 / 3.91 | 1.07 / 3.92 | 1.03 / 3.75 | 1.09 / 3.95 | 1.11 / 4.00 | 1.14 / 4.10 |
| 32 | 1.10 / 3.92 | 1.05 / 3.73 | 1.01 / 3.68 | 1.16 / 4.27 | 1.02 / 3.70 | 0.99 / 3.61 | 1.05 / 3.78 | 1.05 / 3.80 |

`grid_100m`, 3 rounds:

| Build | Wall min | Wall median | Wall max | CPU min | CPU median | CPU max | Median peak memory |
|---|---:|---:|---:|---:|---:|---:|---:|
| tip | 12.69 | 13.08 | 13.82 | 47.08 | 48.74 | 51.57 | 1611 MiB |
| batch 16 | 13.03 | 13.06 | 13.30 | 48.53 | 48.55 | 49.71 | 1614 MiB |
| batch 8 | 12.98 | 13.61 | 14.55 | 48.78 | 50.64 | 54.66 | 1622 MiB |
| batch 32 | 12.40 | 13.57 | 13.93 | 46.01 | 50.40 | 51.87 | 1621 MiB |

| Build | 1 | 2 | 3 |
|---|---:|---:|---:|
| tip | 12.69 / 47.08 | 13.82 / 51.57 | 13.08 / 48.74 |
| 16 | 13.30 / 49.71 | 13.06 / 48.55 | 13.03 / 48.53 |
| 8 | 14.55 / 54.66 | 12.98 / 48.78 | 13.61 / 50.64 |
| 32 | 13.93 / 51.87 | 12.40 / 46.01 | 13.57 / 50.40 |

## 6. Files

- `scripts/run_block.sh`: runs `verify` round-robin over variants. `scripts/parse_block.py`: makes the `.csv` and the tables from a block log.
- `scripts/task1.sh`, `scripts/task2.sh`, `scripts/task2x_100m.sh`, `scripts/task3.sh`, `scripts/task4_json.sh`, `scripts/task4_time.sh`: the tasks as I ran them, from the repository root. The tasks ran one after another, and nothing else heavy ran during a block. Task 2 started with one uncounted run, and task 3 with one uncounted floor run.
- `scripts/parse_floor.py`, `scripts/top_profile.py`: the floor table, and the top 12 lines and user/kernel split of a profile.
- `scripts/floor_i.rs`, `floor_ii.rs`, `floor_iii.rs`, `floor_ii.diff`, `floor_iii.diff`: the three floor programs.
- `claim_two_passes.patch`: the prototype (batch 16).
- `raw/`: every output. The `perf.data` files are not kept. The reports made from them are in `raw/`.

Binary hashes (sha256):

```
41cd01472e0ff1b82d5fb44a96487b8894eee90114095c24c5865722d5439fe4  earnest-tip   (0ffa4d5)
4d9a4bdb0de24c5da7a8996ed3c79e866d747d12eed18009373ce03bd9a0c6b2  earnest-p16   (0ffa4d5 + patch, batch 16)
d30d989a99e8a730d04b50bbc84e5d0b261d47237ca870b39725fcb5f8df6dae  earnest-p8    (batch 8)
6026635f711e8ff4e7f60fcabf2184b1efa6c6c71f96797da1fbd8b5ee68b445  earnest-p32   (batch 32)
9c696066f9e48b311a3d47df931f36b0a3e2532d03bb787a3110ff61dd7e3a91  floor_i
ba8562af94c0b63529548b2185f76ba7a3acabe6b492123356cadf85cf088413  floor_ii
dfc9361df09965f12be89dfdceb461f40644cefca255d489f29ba389208689e8  floor_iii
```
