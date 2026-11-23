# How far the checker at `3058cf4` is from the floor, with 1 thread and with 4

Measured on 2026-11-23 on a 4-vCPU cloud machine (KVM guest), the same CPU model as `measure/profile-round7`.
The checker is commit `3058cf4` of `perf/joined`. This repeats the method of `measure/profile-round7/RESULTS.md`
(the same builds, `time_runs.py`, `profile.sh`, `attr.py` and groups (a) to (g)). It closes what that write-up
left open: the floor with several threads, in two designs, and the floor's generating of successors alone.
It then sets each piece of the checker against the floor. This branch is cut from `origin/measure/profile-round7`
and adds only this folder. No file of the project was changed: one scratch copy was changed and built outside
the repository (section 8). Every number below comes from a file in `raw/`, made by a script in `scripts/`. In
the raw files, `/SCRATCH` stands for the session's scratch folder.

Each finding is marked **measured** (it follows from the numbers) or **reading** (my reading of them, which this
machine cannot prove, mostly because it has no hardware counters).

## 1. The machine

From `raw/machine.txt` and the `## machine` lines at the start and end of every log:

- `nproc` 4, Intel(R) Xeon(R) Processor @ 2.30GHz, KVM, BogoMIPS 4600, L2 8 MiB (4 instances), L3 504 MiB (as
  the guest reports it). This is the CPU model of round 7, but not the same guest.
- Memory 16,087 MiB, no swap, kernel 6.18.44-fc-v726. Transparent huge pages: `madvise` (enabled and defrag).
- `rustc 1.100.0 (0ef1bc386 2026-11-09)`, with `RUSTUP_TOOLCHAIN=1.100.0` for every build. `perf` 6.8.12 from
  `linux-tools-generic`, and GNU `time`. Hardware counters are refused (`raw/perf_stat_hw_check.txt`), so the
  profiles sample `cpu-clock`, as before.
- Steal time: 39 s in `/proc/stat` over about 95 minutes since boot, with all the runs in that window.

The load average (1, 5 and 15 min) before and after each block. Only one block ran at a time. A load near 2 to
3 after a block is that block's own 4-thread runs.

| Block | Log | Before | After |
|---|---|---|---|
| floor tuning, 10M | `tune_floor_7.log` | 0.67 0.95 0.61 | 2.75 2.09 1.24 |
| floor tuning, 100M | `tune_floor_8.log` | 2.14 1.99 1.21 | 1.80 2.04 1.90 |
| f1: the one-thread floor, three variants | `f1_floor_serial.log` | 1.29 1.91 1.86 | 1.00 1.11 1.44 |
| f2: the floors for 2 and 4 threads, the generating alone | `f2_floor_par_gen.log` | 1.00 1.11 1.44 | 1.69 1.76 1.67 |
| c1: `grid_10m`, `wide`, `lights`, in turn with the floor | `c1_10m_wide_lights.log` | 0.94 1.57 1.60 | 1.75 1.64 1.63 |
| c2: `grid_100m`, in turn with the floor | `c2_100m.log` | 1.75 1.64 1.63 | 3.41 2.14 1.77 |
| c3: the plain build against the symbol build | `c3_sym_check.log` | 3.41 2.14 1.77 | 2.70 2.10 1.76 |
| profiles 10M 1, 10M 4, 100M 1, 100M 4, `wide` 1 | `prof_*.log` | 2.70 … 2.21 | 2.32 … 2.03 |
| p1: scratch probe, timing | `p1_probe_prefetch_rows.log` | 1.73 1.49 1.56 | 1.00 1.18 1.41 |

**Weak numbers.** The checker's `grid_100m` with 1 worker took 42.27 s, 40.12 s and 36.42 s in its three rounds
of c2. The floor in the same rounds took 20.57 s, 20.62 s and 19.71 s, so the floor was steady and the checker
was not. In block p1, 40 minutes later, the same binary took 36.30 s (median of 3, 35.11 to 39.64). So the
checker with 1 worker at 100M is somewhere between 36 and 42 s on this machine. I give the median of c2 (40.12 s)
and its smallest value (36.42 s) throughout. A ratio against the floor that uses the median may be up to 10% too
high. The 4-worker runs were steady (12.59 s to 13.16 s).

## 2. The builds

| Binary | How |
|---|---|
| `earnest-new` | `cargo build --release --locked -p earnest-cli` at `3058cf4` |
| `earnest-sym` | the same with `CARGO_PROFILE_RELEASE_DEBUG=true` and its own `CARGO_TARGET_DIR` |
| `floor_i`, `floor_ii`, `floor_iii` | `rustc -O` of `scripts/floor_{i,ii,iii}.rs`, byte for byte round 7's |
| `floor_par` | `rustc -O` of `scripts/floor_par.rs` (new, section 4) |
| `earnest-probe` | scratch only: `3058cf4` with `raw/probe_prefetch_rows.diff` (section 8) |

The sha256 of every binary is in the head of each floor log and in `raw/machine.txt`. The models are
`grid_10m` and `wide` from `tasks/benchmark/models/` at `3058cf4`, and `grid_100m` and `lights` from
`measure/tools/models/` (branch `measure/tools`).

**The symbol build runs in the same time as the plain one** (c3, 5 rounds in turn). `grid_10m`, 1 worker:
2.639 s against 2.530 s (+4%). 4 workers: 0.988 s against 0.964 s. `wide`: 0.253 s against 0.270 s.

## 3. The checker at `3058cf4`

`earnest verify --no-cache --json --jobs N --max-states 200000000 <model>.model.ts`, run from the model folder by
`scripts/time_turn.py`: one uncounted run of each per case, then 7 rounds (3 for `grid_100m`). In each round
the checker runs, then the floor for the same state space and the same number of threads, where one applies
(the best floor of section 4). CPU time is user plus system time from `wait4`. Every checker run printed `pass`
with the expected count (10,000,000, 10,000, 3,000,000 or 100,000,000) and exited 0.

Each cell gives the median, then the smallest value in brackets. Times are in seconds.

| Model | Workers | Wall | CPU | System | Minor faults | Peak MiB | Floor in the same rounds: wall, its own `time_s`, CPU |
|---|---:|---:|---:|---:|---:|---:|---|
| `grid_10m` | 1 | 2.545 (2.340) | 2.533 (2.331) | 0.100 | 62,676 | 186.3 | 1.429, 1.347, 1.427 (serial, huge pages) |
| `grid_10m` | 2 | 1.529 (1.517) | 2.951 (2.853) | 0.151 | 80,264 | 191.6 | 0.961, 0.885, 1.753 (shared table, 2) |
| `grid_10m` | 4 | 0.981 (0.952) | 3.597 (3.454) | 0.201 | 89,891 | 185.2 | 0.647, 0.562, 2.036 (shared table, 4) |
| `wide` | 1 | 0.275 (0.262) | 0.273 (0.262) | 0.004 | 449 | 8.9 | none |
| `wide` | 4 | 0.087 (0.083) | 0.308 (0.293) | 0.020 | 901 | 8.9 | none |
| `lights` | 1 | 0.770 (0.762) | 0.769 (0.758) | 0.024 | 15,123 | 57.0 | none |
| `lights` | 4 | 0.357 (0.340) | 1.259 (1.191) | 0.079 | 17,604 | 60.3 | none |
| `grid_100m` | 1 | 40.121 (36.421) | 39.968 (36.218) | 1.005 | 625,651 | 1582.9 | 20.568, 19.794, 20.483 (serial, huge pages) |
| `grid_100m` | 2 | 22.975 (22.584) | 44.837 (44.049) | 2.637 | 1,491,069 | 1519.9 | 13.519, 12.685, 25.376 (shared table, 2) |
| `grid_100m` | 4 | 13.020 (12.586) | 48.880 (47.381) | 2.893 | 1,663,386 | 1526.8 | 8.044, 7.190, 27.607 (shared table, 4) |

The floor's wall time from `wait4` includes its untimed checks after the search (0.1 s at 10M, about 0.8 s at
100M). Its own `time_s` does not.

**Measured.** Next to round 7's `10015fc` on the same CPU model (another guest, so this is only a rough
comparison): `grid_10m` at 1 worker, 2.91 s then and 2.55 s now; at 4 workers, 1.04 s then and 0.98 s now. `grid_100m` at 1
worker, 40.19 s then and 40.12 s now (36.4 s to 42.3 s, see above); at 4 workers, 13.57 s then and 13.02 s now.
Minor faults at 100M with 1 worker fell from 1.38 million to 0.63 million, and system time from 2.38 s to 1.01 s.
With 4 workers, faults went from 2.21 to 1.66 million.

## 4. The floors

### 4.1 One thread, the three variants (step 1)

`scripts/floor_block.sh`, unchanged: one uncounted run, then 5 rounds at 7 wheels (10M) and 5 at 8 (100M), with
i, ii and iii in turn in each round. Every run printed the round-7 checksums (`4c9670a40c764e17/6ff96c9c5e19c80e`,
`5a65a7777f25b9c1/b9bf724eeb0b4976`) and exited 0. Raw: `raw/f1_floor_serial.log`.

| Variant | States | `time_s` median (smallest) | All 5 | CPU (GNU time) | System | Peak MiB |
|---|---:|---:|---|---:|---:|---:|
| i, no prefetch | 10M | 2.529 (2.322) | 2.529, 2.620, 2.329, 2.767, 2.322 | 2.550 | 0.17–0.23 | 277 |
| ii, prefetch | 10M | 1.481 (1.359) | 1.359, 1.481, 1.456, 1.562, 1.550 | 1.500 | 0.15–0.21 | 277 |
| iii, plain load | 10M | 2.144 (1.833) | 2.175, 2.144, 2.820, 1.833, 1.906 | 2.150 | 0.17–0.24 | 277 |
| i, no prefetch | 100M | 49.207 (44.423) | 44.813, 51.430, 44.423, 49.207, 49.413 | 49.510 | 3.21–3.70 | 4165 |
| ii, prefetch | 100M | 27.878 (25.142) | 29.503, 27.878, 27.755, 25.142, 27.973 | 28.170 | 3.12–3.74 | 4165 |
| iii, plain load | 100M | 35.222 (31.275) | 35.222, 37.295, 34.347, 35.971, 31.275 | 35.440 | 2.82–3.74 | 4165 |

These agree with round 7 (ii: 1.564 s and 26.199 s there) within this machine's spread.

### 4.2 A faster one-thread floor: no zero pages, and huge pages

`floor_par.rs` is `floor_ii.rs` with additions (below). Its `serial` mode is the same search as `floor_ii`, with
two switches on how the tables get their memory:

- `PREFAULT=1` (on unless said): a new table writes one word of each 4 KiB page when it is made. Without this, the
  first touch of a page is a *read* (the probe), so the kernel maps the shared zero page. The first write then
  takes a second fault and copies the page.
- `HUGE=1`: `madvise(MADV_HUGEPAGE)` on the tables (the machine's THP mode is `madvise`).

Both are things the machine allows, and the floor may use anything the machine has. The checker uses neither:
it uses the system allocator and asks for no huge pages (no `madvise` anywhere in its source).

| Floor (`raw/f2_floor_par_gen.log`, 5 rounds) | 10M `time_s` | CPU | System | 100M `time_s` | CPU | System | Peak MiB at 100M |
|---|---:|---:|---:|---:|---:|---:|---:|
| `floor_ii` (section 4.1) | 1.481 (1.359) | 1.500 | 0.18 | 27.878 (25.142) | 28.170 | 3.54 | 4165 |
| `serial`, 4 KiB pages, prefaulted | 1.506 (1.440) | 1.492 | 0.14 | 23.691 (22.184) | 23.638 | 2.10 | 4165 |
| `serial`, huge pages, prefaulted | **1.435 (1.264)** | 1.425 | 0.11 | **19.695 (18.226)** | 19.615 | 1.23 | 4165 |

**Measured.** At 100M, prefaulting alone saves 4.2 s (15%) and huge pages 4.0 s more. At 10M they are within the
spread. CPU for `floor_par` here and below is the search's own, from `getrusage` before and after it, so it
leaves out the checks run after the search.

### 4.3 Several threads (step 2)

**The mode that existed** (`run_parallel`, unchanged in `floor_par.rs`). Each level is cut into parts. Step 1
generates a part's successors into one list per shard, optionally dropping those its shard's table already
holds (the filter). Step 2 settles each shard's lists into its table. Step 3 writes the winners in the serial
order, so its checksum is the serial one. As it stood, with 4 threads at 10M, it took 1.52 s with 1.69 s of
system time: the zero-page faults above, each followed by a TLB flush sent to every CPU (`perf` put 27% of the
samples in `__pv_queued_spin_lock_slowpath` under `do_anonymous_page`, and 17% in the IPIs of `flush_tlb_mm_range`
under `do_wp_page`). With the prefault on, that run takes 1.08 s with 0.24 s of system time.

**The new design** (`shared N`). One open-addressing table of 64-bit slots for all threads (key + 1, 0 = empty),
linear probing, a slot claimed by `compare_exchange`. Each thread takes chunks of `CHUNK` states of the level
from one counter, generates their successors, prefetches each successor's home slot and settles it `DIST`
successors later. A new state goes into the thread's own list. At the end of a level, each thread copies its
list into the one array of states and parents, at an offset in thread order. The table grows only between
levels: by the serial floor's rule (above 7/10 full), and also when the next level's successors (width × wheels)
could fill it past 9/10. All threads then prefault the new table and rehash into it. Like the serial floor,
it starts from 16 slots and doubles.

**What I check in place of the checksum.** The order of ids depends on which thread wins a slot, so the serial
checksum does not hold. Every run of every mode prints and is checked for: the count of states (10,000,000 or
100,000,000) and of levels (64 or 73); a checksum of the *set* of states that does not depend on their order
(the wrapping sum and the xor of a second mix of every state: `b4b67c709b34e433/9a0013722c3e2d53` at 10M and
`7ef5d63824502cfa/b7d36332c451f4a4` at 100M, the same in the serial mode); and `bad_parents=0`, meaning every
state but the first names an earlier state as its parent, one wheel one step lower. All runs passed all four
checks.

**Tuning** (`scripts/tune_floor.sh`, `scripts/tune_floor_8.sh`; `raw/tune_floor_7.log`, `raw/tune_floor_8.log`).
At 10M, two passes over: shard bits 4, 6 and 8; part size 1,024, 4,096 and 16,384; filter on and off; chunk 256,
1,024 and 4,096; distance 0, 8, 16, 32 and 48; huge pages on and off; with 2 and 4 threads. At 100M, one pass over
the settings that were near the best at 10M, plus part size 65,536 and the filter once. **The filter never
helped.** The best setting with it was 2.37 s against 1.33 s without (10M, 2 threads) and 13.38 s against 8.97 s
(100M, 4 threads, huge pages). It costs a probe of another thread's shard per successor, and in this model
almost every successor is new in its part. The part's own step, step 2 (settling into shards), is the
bottleneck: at 100M with 4 threads it is 6.2 s to 7.3 s of 8 to 9 s. The settings chosen are in
`scripts/floors_par.conf`, picked per size and per pages from the tuning.

Timing: `scripts/floor_rounds.sh scripts/floors_par.conf 5`, one uncounted run, then 5 rounds in which every
floor runs once in turn. Raw: `raw/f2_floor_par_gen.log`.

| Floor | Threads | 10M `time_s` | CPU | Sys | Peak MiB | 100M `time_s` | CPU | Sys | Peak MiB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| parts, 4 KiB pages | 2 | 1.086 (1.020) | 2.029 | 0.20 | 317 | 13.613 (13.460) | 26.128 | 2.32 | 3944 |
| parts, huge pages | 2 | 1.089 (0.962) | 2.071 | 0.14 | 317 | 14.252 (12.319) | 27.411 | 1.60 | 3943 |
| shared table, 4 KiB pages | 2 | 1.007 (0.964) | 1.889 | 0.17 | 273 | 14.872 (13.224) | 29.057 | 2.42 | 4258 |
| shared table, huge pages | 2 | **0.995 (0.894)** | 1.896 | 0.11 | 273 | **12.350 (11.596)** | 23.955 | 1.26 | 4259 |
| parts, 4 KiB pages | 4 | 0.689 (0.657) | 2.398 | 0.24 | 324 | 8.497 (7.679) | 31.940 | 2.90 | 4307 |
| parts, huge pages | 4 | 0.681 (0.659) | 2.358 | 0.24 | 324 | 7.976 (7.476) | 29.500 | 1.72 | 4052 |
| shared table, 4 KiB pages | 4 | 0.573 (0.539) | 1.968 | 0.20 | 274 | 7.139 (6.547) | 26.530 | 2.28 | 4234 |
| shared table, huge pages | 4 | **0.527 (0.496)** | 1.763 | 0.14 | 273 | **6.993 (6.824)** | 26.075 | 1.31 | 4234 |

CPU and Sys are medians; the CPU is the search's own. Wall time (GNU) is `time_s` plus 0.08 s at 10M and 0.8 s
at 100M. All five `time_s` of each row are in `raw/compare.md` and the log.

**Measured.** The floor for 4 threads is the shared table, at both sizes: 0.527 s and 6.993 s. That is 2.7 and 2.8
times faster in wall time than the best one-thread floor, for 1.24 and 1.33 times its CPU. With 2 threads it is
also the shared table (0.995 s and 12.350 s). Parts with 4 threads are 29% slower at 10M and 14% at 100M.

### 4.4 Generating successors alone (step 3)

`gen N`: a serial search (not timed) makes the states in breadth-first order. The timed pass then reads every
state once, in that order, makes each successor (`s + bit` for each wheel that may turn), hashes it with `mix`,
adds the hash into a sum and counts it, and checks the invariant once per state. There is no table. With N
threads, the states are split into chunks of 4,096 taken from one counter. It is the counterpart of the
checker's evaluation of actions and rules. Same block, 5 rounds:

| | 10M: successors | `time_s` | CPU | per successor | 100M: successors | `time_s` | CPU | per successor |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| 1 thread | 63,000,000 | 0.100 (0.096) | 0.099 | 1.6 ns | 720,000,000 | 1.135 (1.105) | 1.134 | 1.6 ns |
| 4 threads | 63,000,000 | 0.027 (0.025) | 0.097 | 1.5 ns | 720,000,000 | 0.288 (0.270) | 1.141 | 1.6 ns |

Both printed the same count and hash sum each round (`1ed5002420c8facb`, `8bb675eacdaf5626`).

**Measured.** The floor less its generating, which is its store work: 1.33 s (10M) and 18.48 s (100M) of CPU
with 1 thread (huge pages), 1.67 s and 24.93 s with 4. That is 21 ns and 26 ns per successor with 1 thread, and
26 ns and 35 ns with 4.

## 5. Profiles of `3058cf4`

`scripts/profile.sh` of round 7: `perf record -e cpu-clock -F 999 -g` around N runs of `earnest-sym`, where N is
5 for `grid_10m`, 1 for `grid_100m` and 20 for `wide`. The samples were 13,125, 15,789, 40,608, 50,391 and 5,303,
and none were lost. Every profiled run printed `pass` and the expected count. The profiled `grid_100m` runs took
40.68 s (1 worker) and 13.57 s (4 workers). **Shares** come from the profiles, and **seconds** are the share
times the median CPU of the unprofiled runs (table 3).

**`attr.py`.** I checked that it places the files that are new since `10015fc`. `crates/earnest_ir/src/code/chain.rs`
falls in (d) by its crate, like all of `earnest_ir`; it has no samples in these models (it is not used by
`grid`). The rounds of one thread live in `explore/workers.rs` (`Shape`, `parts_of_this_round`, `take_part`),
and that file is in (g). Its own lines are 0.33% at 100M. The expansion it calls is placed by the innermost
earnest frame, as before. The kept spares (`Spares`, `Kit`, `drop_spare_rows`) are in `explore/part.rs`, so
they fall in (c). No sample fell to "other earnest_check" (a file of the crate outside the groups), and one
sample in all went to "a?". **The one change to `attr.py`**: the subgroup of a sample in `earnest_ir` or
`earnest_value` now names the file (for example `eval: earnest_ir/code/num.rs`, not `eval: earnest_ir`). The
groups are unchanged.

### Shares and seconds by group (`raw/compare.md`, `raw/tables.md`)

| Group | 10M, 1 | | 10M, 4 | | 100M, 1 | | 100M, 4 | | `wide`, 1 | |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| | share | CPU s | share | CPU s | share | CPU s | share | CPU s | share | CPU s |
| (a) index of known states | 21.9% | 0.56 | 34.4% | 1.24 | 21.3% | 8.51 | 33.4% | 16.35 | 7.1% | 0.02 |
| (b) packed rows | 11.7% | 0.30 | 8.0% | 0.29 | 16.1% | 6.44 | 11.2% | 5.45 | 0.2% | 0.00 |
| (c) part tables, claim and merge | 30.4% | 0.77 | 25.0% | 0.90 | 33.1% | 13.23 | 27.3% | 13.35 | 0.0% | 0.00 |
| (d) actions and rules | 23.9% | 0.60 | 19.5% | 0.70 | 21.5% | 8.61 | 16.6% | 8.12 | 88.4% | 0.24 |
| (e) hashing | 4.3% | 0.11 | 3.4% | 0.12 | 3.1% | 1.26 | 2.7% | 1.31 | 1.3% | 0.00 |
| (f) allocation and page faults | 5.4% | 0.14 | 6.8% | 0.24 | 2.6% | 1.03 | 7.1% | 3.45 | 0.5% | 0.00 |
| (g) threads, waiting and the rest | 2.5% | 0.06 | 3.0% | 0.11 | 2.2% | 0.89 | 1.8% | 0.86 | 2.5% | 0.01 |
| total (median CPU) | 100% | 2.53 | 100% | 3.60 | 100% | 39.97 | 100% | 48.88 | 100% | 0.27 |

Against round 7 (`10015fc`, other guest): at 100M with 1 worker, (f) fell from 2.75 s to 1.03 s and (d) rose from
7.52 s to 8.61 s. The rest is within a second of what it was.

### The heaviest lines and functions (`raw/prof_*_elines.txt`, `raw/prof_*_functions.txt`, `raw/tables.md`)

| Line at `3058cf4` | What it is | 10M, 1 | 100M, 1 | 100M, 4 |
|---|---|---:|---:|---:|
| `explore/part.rs:194` | `met_words.entry(…)` in `offer_unseen_word`: the probe of a part's table | 14.50% | 17.81% | 15.20% |
| `store/level.rs:131` | `table.entry(…)` in `claim_level`: the probe of the index | 11.99% | 10.65% | 21.05% |
| `store/rows.rs:17` | `word_of` in `Rows::unpack`: reading the frontier | 5.31% | 11.28% | 7.34% |
| `store/index.rs:48` | `Key::holds`: the compare of the key the probe found (reads the row) | 5.55% | 7.24% | 5.39% |
| `store/level.rs:137` | `vacant.insert(filed)`: a new key into the index | below 1% | 0.85% | 4.83% |
| `earnest_ir/code/num.rs:97` | `int_at` | 2.36% | 2.10% | 1.70% |

The tail of the part table's probe, which round 7 found on a line-less instruction of `offer_encoded`, is now
placed on `part.rs:194`. Only 0.29% is left without a line. `store/rows.rs:17` is reached only through
`Rows::unpack` (`rows.rs:50`), at all three sizes (`raw/callers_inline.txt`), so it is the frontier read, not the
key compare.

Top functions at 100M with 1 worker: `Search::work` 24.1%, `PartSink::offer_encoded` 23.7%, the closure of
`gather` in `claim_level` 20.1%, `Rows::unpack` 11.9%, `Code::scalar` 4.7%, `Cond::leaf` 3.2%,
`Store::read_state` 2.9%, `write_level` 2.4% and `reserve_rehash` 1.7%. With 4 workers, `gather` comes first at
32.5%. In `wide`, `Search::level` takes 53.9% (97% of it (d)), `Num::rest` 35.2% and `Store::place` 7.1%.
`Seen::first_row` of round 7 is gone from the list.

## 6. The comparison (step 5)

All the checker's seconds are CPU seconds of all threads: the profile's share times the median CPU of table 3.
The floor's are the search's own CPU (medians of section 4). "Best floor" is the fastest floor by its median
`time_s` at that thread count. Made by `scripts/compare.py` and `scripts/pieces.py`; output in `raw/compare.md`
and `raw/pieces.md`.

### 6.1 The whole checker against the best floor

| Size | Threads | Best floor | Floor `time_s` | Floor CPU | Checker wall | Checker CPU | Wall / floor | CPU / floor CPU | Wall / floor with 4 KiB pages |
|---|---:|---|---:|---:|---:|---:|---:|---:|---:|
| 10M | 1 | serial, huge | 1.435 | 1.425 | 2.545 | 2.533 | 1.77 | 1.78 | 1.72 (`floor_ii`) |
| 10M | 2 | shared, huge | 0.995 | 1.896 | 1.529 | 2.951 | 1.54 | 1.56 | 1.52 |
| 10M | 4 | shared, huge | 0.527 | 1.763 | 0.981 | 3.597 | 1.86 | 2.04 | 1.71 |
| 100M | 1 | serial, huge | 19.695 | 19.615 | 40.121 | 39.968 | 2.04 | 2.04 | 1.69 (`serial`, 4 KiB) |
| 100M | 2 | shared, huge | 12.350 | 23.955 | 22.975 | 44.837 | 1.86 | 1.87 | 1.69 (parts) |
| 100M | 4 | shared, huge | 6.993 | 26.075 | 13.020 | 48.880 | 1.86 | 1.87 | 1.82 |

With the checker's smallest values (table 3) instead of its medians, the 100M ratios are 1.85 (1 worker) and 1.80
(4 workers). In the in-turn rounds of c2, the checker against the floor's `time_s` was 2.14, 2.02 and 1.93 with 1
worker, and 1.80, 1.83 and 1.75 with 4.

### 6.2 The store against the floor less its generating; the evaluation against the generating

| Run | Store (a+b+c+e) | with (f) | Floor CPU | Generating alone | Floor less generating | Store / it | with (f) / it | Same against the 4 KiB-page floor | (d) | (d) / generating |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 10M, 1 | 1.73 | 1.86 | 1.42 | 0.099 | 1.33 | 1.30 | 1.41 | 1.24, 1.34 | 0.60 | 6.1 |
| 10M, 4 | 2.55 | 2.79 | 1.76 | 0.097 | 1.67 | 1.53 | 1.67 | 1.36, 1.49 | 0.70 | 7.2 |
| 100M, 1 | 29.44 | 30.47 | 19.62 | 1.134 | 18.48 | 1.59 | 1.65 | 1.31, 1.35 | 8.61 | 7.6 |
| 100M, 4 | 36.46 | 39.91 | 26.07 | 1.141 | 24.93 | 1.46 | 1.60 | 1.44, 1.57 | 8.12 | 7.1 |

The floor's figures include its own kernel time (its faults), so "with (f)" is the fairer column. Against
`floor_ii` as round 7 had it (28.17 s of CPU at 100M, 27.04 s without its generating), the store at 100M with 1
worker would be 1.09 times the floor. **Round 7's "within 10% to 20% of the floor" was against a floor that paid
for zero-page faults. Against the floor with them removed, the store is 1.31 to 1.35 times the floor on 4 KiB
pages, and 1.59 to 1.65 times it on huge pages.**

### 6.3 Each piece above 2% of `grid_100m`, at 1 and at 4 workers

The cost per unit is the piece's seconds divided by the states (10M or 100M), the successors (63M or 720M) or
the candidates. Candidates are what a part's table lets through to the index. I did not count them in the
checker. I estimate them with `floor_par 8 cands 64`: cut each level of the floor's search into 64 parts, as
the checker does for wide levels, and count the distinct successors of each part. That gives 259.5 million at
100M and 26.0 million at 10M (32 parts: 226.5 million; `raw/cands.txt`). The floor does one probe per successor,
and its whole store work costs 25.7 ns per successor with 1 thread and 34.6 ns with 4 (section 4.4).

| Piece | Group | 100M, 1: share, s | 100M, 4: share, s | ns per unit: 10M 1 / 100M 1 / 10M 4 / 100M 4 | Mark |
|---|---|---:|---:|---|---|
| a part's table, `offer_unseen_word` | c | 19.21%, 7.68 | 16.44%, 8.04 | 6.6 / 10.7 / 8.2 / 11.2 per successor | work per successor, below the floor's cost; its growth with size is the machine |
| index probe in `claim_level` | a | 11.49%, 4.59 | 25.95%, 12.68 | 12.7 / 17.7 / 40.8 / 48.9 per candidate | the machine (grows 2.8× with threads, same code) |
| index key compare, `store/index.rs` | a | 8.20%, 3.28 | 6.20%, 3.03 | 5.7 / 12.6 / 3.4 / 11.7 per candidate | the program's own, at most 3.3 s and 3.0 s |
| frontier read, `rows.rs` (`unpack`) | b | 12.57%, 5.02 | 8.31%, 4.06 | 17.0 / 50.2 / 14.6 / 40.6 per state | the program's own, at most 3.9 s and 2.9 s (cause not found) |
| `explore/part.rs` lines of its own | c | 10.30%, 4.12 | 7.83%, 3.83 | 20.7 / 41.2 / 24.2 / 38.3 per state | `file_by_shard`: the program's own, at most 1.5 s; the rest per successor |
| `expand.rs` + `earnest_ir`, (d) | d | 21.5%, 8.61 | 16.6%, 8.12 | 9.6 / 12.0 / 11.1 / 11.3 per successor | work per successor the floor does at 1.6 ns: the program's own beyond that, at most 7.5 s and 7.0 s |
| rows: codec, layout, `read_state`, `outside` | b | 3.54%, 1.41 | 2.85%, 1.39 | 12.5 / 14.1 / 14.2 / 13.9 per state | work once per state (decoding the row into values); the floor has no values; at most 1.4 s |
| `store/level.rs` lines of its own | c | 3.58%, 1.43 | 3.04%, 1.48 | 14.5 / 14.3 / 14.0 / 14.8 per state | work once per new state, which the floor does for clearly less; at most 1.3 s |
| hashing, `mix`/`hash_words` | e | 3.15%, 1.26 | 2.68%, 1.31 | 1.7 / 1.7 / 1.9 / 1.8 per successor | work per successor at the floor's cost |
| kernel page faults | f | 2.12%, 0.85 | 5.72%, 2.79 | 9.0 / 8.5 / 13.4 / 27.9 per state | 1 worker: the machine (below the floor's own); 4 workers: the program's own, at most 1.9 s |

Below 2% at both: the rehash of the index (1.60%, 0.64 s, plus 0.78% of faults under it), libc
`memmove`/`memset` with no caller (1.81%), kernel `mmap`/`munmap`/`madvise` (0.32% and 0.80%), and libc
`malloc`/`free` (0.15% and 0.54%).

The evidence for each mark:

1. **A part's table: work once per successor, at the floor's cost or below. Measured** cost: 10.7 ns per
   successor at 100M, against the floor's 25.7 ns for all its store work per successor. **Reading:** its cost
   grows from 6.6 to 10.7 ns from 10M to 100M with the same code, which is the machine, and it does not grow
   with threads (each part's table is private). Every checker of this design probes once per successor here.
2. **The index probe: the machine. Measured:** 17.7 ns per candidate with 1 worker, 48.9 ns with 4, and the same
   ratio at 10M (12.7 and 40.8 ns), with the same instructions. **Reading:** this growth with threads is larger
   than the floor's own (25.7 → 34.6 ns per successor, 1.35×). So up to about 6.5 s of CPU at 4 workers (12.68 s
   less 4.59 s × 1.35), or about 1.6 s of wall time, is more than the floor pays for its growth. That bound is my
   reading, not a measurement. Taken together, the part's table, the index probe and the key compare cost 21.6 ns
   per successor with 1 worker (15.55 s) and 33.0 ns with 4 (23.75 s). That is 0.84 and 0.95 times the floor's
   whole store work per successor. **So the probing as a whole is at the floor's cost.** The gap between the
   store and the floor (11 s at 100M with 1 worker) is in the rows below, not in the probes.
3. **The key compare in `Key::holds`: the program's own, at most 3.3 s (1 worker) and 3.0 s (4).** The index
   holds ids, so to compare a key the probe found, the checker reads that state's row in `Rows`: a second memory
   access per look-up that finds a key. The floor keeps the key in the slot. **Measured:** 12.6 ns per
   candidate at 100M, 5.7 ns at 10M. **Reading:** a key in the slot makes the table larger (4 bytes of row
   beside a 4-byte id here, more for wider rows), so less than the bound is there to win. It is the bound if the
   compare were free.
4. **The frontier read in `Rows::unpack`: the program's own, at most 3.9 s (1 worker) and 2.9 s (4); the cause
   is not found.** **Measured:** 50 ns per state at 100M with 1 worker, 17 ns at 10M. The floor reads every
   state of its frontier in order, and generates, hashes and counts its successors, in 11 ns per state at 100M
   (1.13 s in all). So an in-order read costs at most 11 ns on this machine. The bound is 5.02 s less 1.13 s, and
   4.06 s less 1.13 s. **Measured (scratch probe, section 8):** a prefetch of the row 512 ids ahead takes
   `Rows::unpack` from 11.9% to 8.6% of the samples, and the wall time by -3.3% at both sizes (within the spread).
   **Reading:** a prefetch did not remove it, so it is not mostly a wait on the row itself. The samples land after
   the `call memcpy` of a 4-byte row (round 7), and a sample of the software clock lands on the instruction
   after a slow one. I cannot tell what is slow without counters. Round 7's probe (one load instead of
   `memcpy`) moved the samples into `Search::work` rather than removing them. This is the piece with the
   largest bound and the weakest evidence.
5. **`explore/part.rs` lines of its own: 4.12 s.** `file_by_shard` with `move_row` is 3.7% (1.48 s,
   `part.rs:473` and `:369`, `raw/callers_inline.txt`). It is a second pass over every fresh row, to put the
   rows, hashes and parents in shard order, and the floor has no such pass: **the program's own, at most 1.5 s.**
   The rest (the entry of `offer_encoded` 2.8%, `expand_ids` 1.5%, `add_fresh` 1.2%) is work once per
   successor or fresh row, like the floor's append. Its cost per state doubles from 10M to 100M, which reads as
   the machine (the part's buffers grow with the level).
6. **Evaluation, (d): 8.61 s and 8.12 s. Measured:** 12.0 ns per successor at 100M against the floor's 1.6 ns,
   which also includes reading the frontier and hashing. That is 7.6 and 7.1 times the floor. The cost per
   successor is flat with size and threads. **The program's own beyond the floor's cost: at most 7.5 s (1
   worker) and 7.0 s (4).** **Reading:** the floor's successors are one add per wheel with the model compiled
   in. A checker that reads a model (`expand.rs`, `Code::scalar`, `Cond::leaf`, `Num::int_at`) cannot reach
   that, so the bound is an outer bound. `chain.rs`, new at this commit, has no samples here.
7. **Rows: codec, layout, `read_state`, `outside`: 1.41 s, 14 ns per state, flat.** Decoding each frontier row
   into values for the evaluator. The floor works on its packed word directly. **Reading:** this is work once per
   state that the floor does not do, so at most 1.4 s. How much of it the evaluator needs is not measured here.
8. **`store/level.rs` lines of its own: 1.43 s, 14 ns per new state, flat** (`ordinal_of`, `Winners::rank`,
   `write_level`). This is work once per new state. The floor appends a state and its parent to two arrays,
   and that costs it a few ns at most. **Reading: up to about 1.3 s** of it is the program's own.
9. **Hashing: 1.26 s, 1.7 ns per successor, flat.** One `mix` per successor, as in the floor (whose
   generating, `mix` included, costs 1.6 ns). **Work per successor at the floor's cost.**
10. **Page faults, (f): 0.85 s with 1 worker, 2.79 s with 4. Measured:** 0.63 million minor faults with 1
    worker (2.4 GiB for a 1.58 GiB peak), and 1.66 million with 4 (6.3 GiB for 1.53 GiB). The floor takes
    0.31 to 0.32 million at both thread counts (huge pages) and 1.2 to 2.3 s of system time. With 1 worker,
    the checker's 1.0 s of system time is below the floor's, so this is the machine. With 4 workers, the 1.04
    million faults more than with 1 are memory given back and taken again level after level: **the program's
    own, at most 1.9 s of CPU** (2.79 s less 0.85 s), as round 7 found (1.9 s then).

### 6.4 What could still pay

**Reading**, from the marks and bounds above, at 100M with 1 worker (39.97 s CPU against a 19.62 s floor):

- the evaluation beyond the floor's generating: up to 7.5 s (an outer bound; see finding 6);
- the frontier read: up to 3.9 s, cause not found, and a prefetch did not get it;
- the key compare reading the row: up to 3.3 s;
- decoding rows into values (1.4 s), the claim's ranking and writing (1.3 s) and the second pass of
  `file_by_shard` (1.5 s): about 4 s together;
- huge pages for the big tables: the floor gains 4.0 s from them at 100M (23.7 → 19.7 s), and the checker asks
  for none. How much of that the checker would see is not measured.

With 4 workers, add up to 1.9 s of re-faulted memory and up to about 6.5 s of CPU (1.6 s of wall) in the
index's growth with threads beyond the floor's. The probes themselves (finding 2) and the hashing are at the
floor's cost. **So the answer to "can more work on the program still pay" is yes. It is not in the two
probes. It is in the frontier read, the key compare, the second pass over fresh rows, the decoding and
evaluation, and with 4 workers the faults.** The bounds overlap with each other and are upper bounds, not
predictions.

## 7. Findings, in short

1. **Measured.** With 1 thread the checker takes 1.77 (10M) and 2.04 (100M) times the best floor, in wall time.
   With 4 threads it takes 1.86 times at both sizes, and 2.04 and 1.87 times in CPU. Against floors that use
   4 KiB pages, as the checker does, the ratios are 1.69 to 1.82.
2. **Measured.** The floor for 4 threads is a shared table with slots claimed by compare-and-swap: 0.527 s and
   6.993 s. It beats the floor of parts, shards and a filter (0.681 s and 7.976 s), and the filter never helped.
3. **Measured.** Prefaulting the tables matters to every floor: 4.2 s of 27.9 s at 100M with 1 thread. With
   several threads it decides the result, because the zero-page faults send a TLB flush to every CPU (section
   4.3). Huge pages save another 4.0 s at 100M.
4. **Measured.** Generating successors alone costs the floor 1.6 ns per successor (0.10 s and 1.13 s). The
   checker's evaluation is 7.1 to 7.6 times that at 100M.
5. **Measured.** The checker's store work is 1.59 times the floor's (1.65 with faults) at 100M with 1 worker,
   and 1.46 times (1.60) with 4.
6. **Measured and reading.** The three probes together (part table, index, key compare) cost 0.84 times the
   floor's whole store work per successor with 1 worker, and 0.95 times with 4. The gap is in the frontier
   read, the rows, the claim's own lines and `file_by_shard` (section 6.3).

## 8. What failed, was skipped or was added

- **Hardware counters** are refused. "Waiting for memory" is my reading of how a cost per unit grows with size
  and threads, not a count of misses.
- **The checker's 1-worker run at 100M** varied by 16% between rounds while the floor next to it did not
  (section 1). Its ratios may be up to 10% high. The 4-worker numbers are steady.
- **The candidates per part** are estimated in the floor's order, with 64 parts a level, not counted in the
  checker (section 6.3). The costs per candidate rest on that estimate. With 32 parts they would be 15% higher.
- **The floor's tuning** at 100M is one pass, so the settings chosen are near the best rather than certainly
  the best. Several settings were within the spread.
- **The shared-table floor** grows its table between levels by a rule that knows the next level's width
  (successors at most width × wheels). The checker does not have to keep its load that way, but it is a fair
  rule for a hand-written search.
- **Added: a scratch probe**, `raw/probe_prefetch_rows.diff`: a prefetch of the row 512 ids ahead in
  `Rows::unpack`, in a worktree of `3058cf4` outside the repository, never committed. It was timed in turn
  with `earnest-new` (`raw/p1_probe_prefetch_rows.log`: 10M 2.302 s against 2.383 s, 100M 35.09 s against
  36.30 s) and profiled flat (`raw/prof_probe_100m_j1_report.txt`).
- **Added: `floor_par cands`**, a count and not a floor, built after the timed floors from the same file
  (`raw/cands.txt`). The timed `floor_par` binary is the one whose sha256 heads `raw/f2_floor_par_gen.log`.
- **Not done:** the DWARF call chains of round 7 were not recorded again. `lights` and `wide` have no floor.
  `wide` at 4 workers and `lights` were timed but not profiled, as asked.
- `perf.data` and the `perf script` dumps are not kept (up to 40 MB). What was made from them is in `raw/`.

## 9. Commands, and the second pass

```
rustup toolchain install 1.100.0 --profile minimal; rustup default 1.100.0
apt-get install -y linux-tools-generic time; cargo install rustfilt
ln -s <scratch> /SCRATCH; git worktree add /SCRATCH/src/new 3058cf4        # build.sh does this for a commit
bash scripts/build.sh 3058cf4         # earnest-new, earnest-sym, the four floors, the models
bash scripts/tune_floor.sh 7; bash scripts/tune_floor_8.sh      # raw/tune_floor_{7,8}.log -> scripts/floors_par.conf, floors.json
bash scripts/floors.sh                # f1: floor_block.sh; f2: floor_rounds.sh floors_par.conf 5
bash scripts/checker.sh all           # c1, c2: time_turn.py with floors.json; c3: time_runs.py new,sym; the five profiles
python3 scripts/compare.py /SCRATCH > raw/compare.md; python3 scripts/tables.py /SCRATCH > raw/tables.md
python3 scripts/pieces.py /SCRATCH > raw/pieces.md
floor_par 8 cands 64; floor_par 8 cands 32; floor_par 7 cands 64          # raw/cands.txt
python3 scripts/callers_inline.py X.script earnest-sym 'rows.rs:17$'       # raw/callers_inline.txt
# probe: worktree of 3058cf4 + raw/probe_prefetch_rows.diff, its own target dir
time_runs.py new=earnest-new,probe=earnest-probe 5 grid_10m:1; ... 3 grid_100m:1     # raw/p1_probe_prefetch_rows.log
```

**The second pass, for another commit of `perf/joined`, is one command per step** (`scripts/pass.sh`):

```
bash measure/profile-round9/scripts/pass.sh setup            # tools and /SCRATCH (once per machine)
bash measure/profile-round9/scripts/pass.sh build COMMIT     # about 2 minutes
bash measure/profile-round9/scripts/pass.sh checker          # about 12 minutes: timing in turn with the floors, symbol check, profiles
bash measure/profile-round9/scripts/pass.sh compare          # under a minute: compare.md, tables.md, pieces.md
bash measure/profile-round9/scripts/pass.sh floors           # optional, about 30 minutes: only to measure the floors again
```

The floors do not depend on the commit, and `checker` times each one in turn with the checker anyway. So a pass
takes about 15 minutes without `floors` and 45 minutes with it. Read `attr.py`'s placement again if the commit
adds or moves files of the store or of the evaluator (a new file in `earnest_check` falls to (g) "other" until
it is named).

Scripts that are round 7's: `attr.py` (one subgroup label changed, section 5), `lines.py`, `lines_by_earnest.py`,
`by_function.py`, `callers.py`, `machine.sh`, `time_runs.py`, `floor_{i,ii,iii}.rs`, `floor_block.sh` and
`profile.sh` (the last two read `S` from the environment). `tables.py` now reads its CPU times from the logs. New:
`floor_par.rs`, `tune_floor.sh`, `tune_floor_8.sh`, `tune_summary.py`, `floor_rounds.sh`, `floors_par.conf`,
`floor_summary.py`, `floors.sh`, `time_turn.py`, `floors.json`, `checker.sh`, `build.sh`, `compare.py`,
`pieces.py`, `callers_inline.py`, `sums.py` and `pass.sh`.
