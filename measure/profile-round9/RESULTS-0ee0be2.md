# The checker at `0ee0be2` against `89de199`, `3058cf4` and the floor

The third pass of round 9, measured on 2026-11-23 between 09:13 and 09:57 UTC. Three builds of `perf/joined`
(`3058cf4`, `89de199` and the tip `0ee0be2`) and the floor ran in turn in one block on one machine, so every ratio
below comes from that block. Both later builds were profiled in the same session. Raw logs: `raw/0ee0be2/`. Script
of the pass: `scripts/pass3.sh`. Tables: `scripts/compare3.py`, output in `raw/0ee0be2/compare3.md`. No file of the
project was changed. Findings are marked **measured** or **reading**, as before.

## 1. The machine, and how far its numbers can be trusted

The same kind of guest as `RESULTS-89de199.md` (Xeon @ 2.10GHz, 4 vCPU, L3 260 MiB, THP `madvise`), after a fresh
boot. **This boot was slower than the last one, and unevenly so.** `89de199` took 40.58 s at 100M with 1 worker,
against 30.95 s in the second pass. `3058cf4` took 57.82 s against 43.52 s. The serial floor took 27.00 s against
24.04 s. So no second here compares with a second of the earlier files. Only the ratios within this block do.

Within the block, two of the five rounds of the 4-thread floors at 100M were disturbed: the shared table took 12.55 s
and 15.04 s in rounds 1 and 2, then 8.98, 7.98 and 8.40 s, and the parts floor ran alike. So the block medians of the
4-thread floors at 100M are too high, by about 0.5 s to 1 s. For 4 workers I therefore use the floor run in the same
round as the checker (`t1`, 7 rounds), which was steady (8.09 s to 8.97 s). The block medians are given beside it.

Load average (1, 5, 15 min): floors 1.71 1.20 0.51 → 1.35 2.02 1.86; 7-round block → 2.13 2.91 2.49; 100M 3-round
block → 1.89 1.72 1.81; symbol check → 1.63 1.67 1.79.

## 2. Times: three builds and the floor in turn

Median (smallest), in seconds. 7 rounds, but 3 for `grid_100m` at 1 and 2 workers. Every checker run printed `pass`
with the expected count (`ledger`: 36,400 states) and exited 0. Every floor run printed its set checksum.

| Model | Workers | `3058cf4` | `89de199` | `0ee0be2` | `0ee0be2` CPU | System | Minor faults | Peak MiB | Floor in turn, `time_s` |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `grid_10m` | 1 | 4.069 (3.281) | 3.488 (2.927) | 2.824 (2.211) | 2.732 | 0.154 | 63,264 | 188.6 | 1.895 (1.647) |
| `grid_10m` | 2 | 2.332 (2.130) | 1.977 (1.852) | 1.433 (1.280) | 2.677 | 0.204 | 82,007 | 192.0 | 1.193 (1.069) |
| `grid_10m` | 4 | 1.448 (1.351) | 1.273 (1.110) | 0.890 (0.823) | 3.085 | 0.283 | 90,310 | 191.1 | 0.684 (0.666) |
| `grid_100m` | 1 | 57.824 (50.490) | 40.575 (36.501) | 33.177 (29.923) | 32.822 | 1.300 | 668,345 | 1617.3 | 26.771 (24.960) |
| `grid_100m` | 2 | 33.247 (31.321) | 26.108 (24.542) | 18.584 (17.323) | 35.456 | 3.252 | 1,636,348 | 1572.5 | 15.779 (15.620) |
| `grid_100m` | 4 | 18.261 (16.895) | 14.123 (13.504) | 10.376 (9.701) | 37.416 | 4.031 | 1,837,240 | 1560.9 | 8.509 (8.088) |
| `lights` | 1 | 1.236 (1.162) | 1.098 (0.984) | 0.744 (0.685) | 0.704 | 0.036 | 15,116 | 56.7 | |
| `lights` | 4 | 0.436 (0.417) | 0.440 (0.393) | 0.293 (0.278) | 0.964 | 0.089 | 17,588 | 61.9 | |
| `ledger` | 1 | 0.093 (0.072) | 0.080 (0.067) | 0.022 (0.014) | 0.017 | 0.000 | 618 | 9.0 | |
| `ledger` | 4 | 0.041 (0.036) | 0.040 (0.036) | 0.018 (0.015) | 0.023 | 0.006 | 915 | 9.0 | |
| `wide` | 1 | 0.372 (0.337) | 0.387 (0.328) | 0.371 (0.334) | 0.356 | 0.008 | 445 | 9.0 | |
| `wide` | 4 | 0.139 (0.131) | 0.134 (0.123) | 0.149 (0.133) | 0.532 | 0.012 | 1,034 | 9.0 | |

**Counts of instructions** (callgrind, `raw/0ee0be2/callgrind_*.txt`, millions, `3058cf4` / `89de199` / `0ee0be2`):

| Model | Workers | Instructions | Time (median, from the table) |
|---|---:|---|---|
| `wide` | 1 | 4,691.7 / 4,708.2 / 4,691.1 | 0.372 / 0.387 / 0.371 |
| `wide` | 4 | 4,780.0 / 4,723.2 / 4,943.8 | 0.139 / 0.134 / 0.149 |
| `ledger` | 1 | 509.6 / 510.4 / 122.0 | 0.093 / 0.080 / 0.022 |
| `ledger` | 4 | 557.1 / 555.5 / 160.9 | 0.041 / 0.040 / 0.018 |

**Measured.** `wide` with 1 worker runs the same work at all three commits (±0.4%), in the same time. With 4 workers,
`0ee0be2` executes 4.7% more instructions than `89de199`, and its time is 11% higher, within this block's spread for
a 0.14 s run. `ledger` executes 76% fewer instructions at `0ee0be2`, and `lights` takes 32% less wall time with 1
worker and 33% less with 4: the packed path.

**Symbol build** (`c3`, 5 rounds): `grid_10m` 2.961 s against 2.876 s with 1 worker, and 0.924 s against 0.961 s with
4; `wide` 0.466 s against 0.381 s, which is the binary's load, as in the first pass. So the profiles of `grid` stand
for the plain build.

### The whole checker against the floors

"In turn" is each round's checker wall time divided by the floor's `time_s` in that round, median of the rounds.
"Block" uses the floor block's median (too high at 100M with 4 threads, section 1).

| Model | Workers | `3058cf4` in turn | `89de199` in turn | `0ee0be2` in turn | `0ee0be2` per round | `0ee0be2` block | `0ee0be2` against the parts floor (fixed ids) |
|---|---:|---:|---:|---:|---|---:|---:|
| `grid_10m` | 1 | 2.16 | 1.86 | **1.48** | 1.62, 1.48, 1.71, 1.41, 1.18, 1.53, 1.42 | 1.53 | (the serial floor) |
| `grid_10m` | 2 | 2.04 | 1.78 | 1.19 | 1.07 … 1.42 | | |
| `grid_10m` | 4 | 2.09 | 1.82 | **1.28** | 1.24, 1.37, 1.12, 1.30, 1.28, 1.36, 1.22 | 1.25 | **0.96** (0.925 s) |
| `grid_100m` | 1 | 2.16 | 1.49 | **1.24** | 1.20, 1.24, 1.43 | 1.23 | (the serial floor) |
| `grid_100m` | 2 | 2.13 | 1.59 | 1.14 | 1.19, 1.14, 1.10 | | |
| `grid_100m` | 4 | 2.16 | 1.67 | **1.21** | 1.15, 1.19, 1.21, 1.25, 1.21, 1.19, 1.28 | 1.16 | **1.01** (10.298 s) |

The parts floor of the block at 100M carries the disturbed rounds too. Its last three rounds (10.30, 9.16, 10.07 s)
give 1.01 to 1.13.

### The floors of the block (`raw/0ee0be2/f_floors.log`)

| Floor | 10M `time_s` | CPU | 100M `time_s` | CPU |
|---|---:|---:|---:|---:|
| serial, huge pages | 1.843 (1.659) | 1.842 | 27.004 (25.988) | 26.797 |
| serial, 4 KiB pages | 2.062 (1.891) | 2.022 | 32.221 (29.917) | 32.070 |
| shared table, 4 threads, huge pages | 0.714 (0.691) | 2.172 | 8.977 (7.978) | 30.533 |
| parts, 4 threads, huge pages (ids fixed) | 0.925 (0.812) | 3.034 | 10.298 (9.156) | 35.014 |
| generating alone, 1 thread | 0.161 (0.111) | 0.161 | 1.509 (1.452) | 1.500 |
| generating alone, 4 threads | 0.033 (0.031) | 0.127 | 0.463 (0.377) | 1.652 |

## 3. Profiles, `89de199` against `0ee0be2`

The five profiles of `RESULTS.md` for each build, recorded in this session. Samples, `89de199` → `0ee0be2`: 17,591 →
14,239; 20,406 → 13,483; 42,104 → 32,838; 51,871 → 39,327; 7,946 → 8,893. None were lost. Seconds are the share times
the median CPU of the same build in this block.

**`attr.py`.**
- `crates/earnest_ir/src/code/packed.rs` falls in (d) by its crate, as all of `earnest_ir` does. Its subgroup names the
  file: 6.47% at 100M with 1 worker (`holds`, `code` and a closure: the packed tests and writes). `try_packed` and the
  rest of `FromRow` are in `expand.rs`, also (d).
- **The row hand-out** is `Store::read_state` in `store.rs` (with `outside`) and `word_of` in `store/rows.rs`, group
  (b). At `0ee0be2`, `decode`, `decoder` and `read_values` in `store.rs` were caught only by the rule's fall-through to
  (b), so they are now named in it. That changes no group.
- `codec.rs` no longer shows: a state is no longer decoded into values.
- The line-less tail of the part table's probe in `offer_encoded` (2.86% and 3.06%) is counted with the part table, as
  in `RESULTS-89de199.md`.

| Group, CPU s, `89de199` → `0ee0be2` | 10M, 1 | 10M, 4 | 100M, 1 | 100M, 4 | `wide`, 1 |
|---|---:|---:|---:|---:|---:|
| (a) index of known states | 0.83 → 0.80 | 1.47 → **0.79** | 9.88 → 10.12 | 15.47 → **9.82** | 0.03 → 0.03 |
| (b) packed rows | 0.30 → 0.13 | 0.31 → 0.12 | 3.44 → 1.23 | 3.67 → 1.37 | 0.00 → 0.00 |
| (c) part tables, claim and merge | 1.03 → 0.96 | 1.09 → 1.04 | 12.89 → 12.54 | 14.90 → 13.81 | 0.00 → 0.00 |
| (d) actions and rules | 0.84 → 0.51 | 0.91 → 0.58 | 9.22 → 5.66 | 10.01 → 6.44 | 0.33 → 0.31 |
| (e) hashing | 0.23 → 0.12 | 0.27 → 0.13 | 2.48 → 1.11 | 2.63 → 1.33 | 0.01 → 0.01 |
| (f) allocation and page faults | 0.17 → 0.17 | 0.34 → 0.33 | 1.38 → 1.57 | 4.64 → 4.05 | 0.00 → 0.00 |
| (g) threads, waiting and the rest | 0.06 → 0.04 | 0.10 → 0.09 | 0.71 → 0.60 | 0.66 → 0.60 | 0.01 → 0.01 |
| total | 3.46 → 2.73 | 4.48 → 3.08 | 39.99 → 32.82 | 51.98 → 37.42 | 0.38 → 0.36 |

### The pieces, `grid_100m` and `grid_10m`, at 1 and 4 workers

CPU seconds `89de199` → `0ee0be2`, and the cost per unit at `0ee0be2`. Candidates are taken as 1.16 × 259.5M at 100M
and 1.16 × 26.0M at 10M. The per-candidate column below uses the unscaled count, so divide it by 1.16.

| Piece | Group | 100M, 1 | 100M, 4 | 10M, 1 | 10M, 4 | ns per unit at `0ee0be2`, 100M 1 / 4 |
|---|---|---|---|---|---|---|
| a part's table (`RecentWords`, with its tail) | c | 4.57 → 4.07 | 5.78 → 4.70 | 0.42 → 0.36 | 0.47 → 0.44 | 5.6 / 6.5 per successor |
| index probe in the claim (`level.rs`) | a | 5.77 → 5.93 | 11.28 → **5.89** | 0.53 → 0.52 | 1.24 → **0.53** | 22.8 / 22.7 per candidate |
| key load from the bucket (`index.rs:48`) | a | 3.31 → 3.38 | 3.34 → 3.10 | 0.19 → 0.18 | 0.13 → 0.14 | 13.0 / 11.9 per candidate |
| index rehash | a | 0.79 → 0.80 | 0.85 → 0.82 | 0.10 → 0.10 | 0.11 → 0.12 | 8.0 / 8.2 per state |
| frontier read (`rows.rs`) | b | 0.76 → 0.79 | 0.80 → 0.87 | 0.07 → 0.08 | 0.07 → 0.07 | 7.9 / 8.7 per state |
| row hand-out and decoding (`read_state`, `outside`, codec, layout) | b | 2.68 → **0.44** | 2.87 → **0.50** | 0.22 → 0.05 | 0.24 → 0.05 | 4.4 / 5.0 per state |
| `explore/part.rs` lines of its own | c | 6.37 → 6.45 | 7.14 → 7.09 | 0.40 → 0.39 | 0.43 → 0.42 | 64.5 / 70.9 per state |
| `store/level.rs` lines of its own | c | 1.95 → 2.03 | 1.99 → 2.03 | 0.21 → 0.21 | 0.19 → 0.18 | 20.3 / 20.3 per state |
| `expand.rs` | d | 4.79 → 3.53 | 5.14 → 3.81 | 0.45 → 0.32 | 0.46 → 0.36 | 4.9 / 5.3 per successor |
| `earnest_ir` (now with `packed.rs`) | d | 4.42 → 2.12 | 4.87 → 2.63 | 0.39 → 0.19 | 0.44 → 0.23 | 2.9 / 3.6 per successor |
| hashing | e | 2.48 → 1.11 | 2.63 → 1.33 | 0.23 → 0.12 | 0.27 → 0.13 | 1.5 / 1.9 per successor |
| kernel page faults | f | 1.14 → 1.34 | 3.79 → 3.23 | 0.12 → 0.11 | 0.20 → 0.20 | 13.4 / 32.3 per state |

Of the part's own lines at `0ee0be2`, `file_by_shard` with `move_row` is 9.4% (3.09 s) with 1 worker and 9.2% (3.45 s)
with 4 at 100M. The rest is `offer_encoded`, `fill_part`, `add_fresh` and `offer_row`.

**What moved. Measured:**
- The claim with 4 workers (`f3ce61d`): -5.4 s at 100M, -0.7 s at 10M.
- The row hand-out (decoding no more): -2.2 s and -2.4 s.
- `earnest_ir` (-2.3 s and -2.2 s) and `expand.rs` (-1.3 s and -1.3 s): the packed path.
- The hashing: -1.4 s and -1.3 s. **Reading:** the general path hashed each successor's row twice (once in
  `offer_row_once`, once for the part), and the packed path once. Its cost per successor fell from 3.4 ns to 1.5 ns at
  100M with 1 worker.

**What did not move:** the part's table, the probe and key load with 1 worker, the part's and the claim's own lines,
the rehash, the frontier read, and the faults.

### The claim with 4 workers against 1, from the shares (request 4)

Group (a), seconds per row reaching the claim (`compare3.md`):

| Build | Size | 1 worker | 4 workers | 4 / 1 | of it, the probe 1 / 4 | the key load 1 / 4 |
|---|---|---:|---:|---:|---|---|
| `89de199` | 10M | 27.5 ns | 48.8 ns | 1.78 | 17.5 / 41.0 | 6.3 / 4.2 |
| `89de199` | 100M | 32.8 ns | 51.4 ns | 1.57 | 19.2 / 37.5 | 11.0 / 11.1 |
| `0ee0be2` | 10M | 26.7 ns | 26.1 ns | **0.98** | 17.3 / 17.5 | 5.8 / 4.7 |
| `0ee0be2` | 100M | 33.6 ns | 32.6 ns | **0.97** | 19.7 / 19.6 | 11.2 / 10.3 |

**Measured, from outside the clocks:** at `0ee0be2` a row of the claim costs the same with 4 workers as with 1, at
both sizes. The growth was all in the probe line (`level.rs:131`, the line of the insert that wrote the table's count).
The key load did not grow. This agrees with the other session's clocks (33 ns to 37 ns with 4 workers against 36 ns
to 38 ns with 1). So my reading in `RESULTS-89de199.md` ("the index probe's growth with threads beyond the parts floor's,
up to about 5 s, may be partly the contract") was wrong. It was the program's own, it was inside the contract, and it
was about 5.4 s.

### Store and evaluation against the floors

| Run | Build | Store a+b+c+e | with (f) | Floor (CPU less generating) | Store / it | with (f) / it | (d) | **(d) / generating** |
|---|---|---:|---:|---|---:|---:|---:|---:|
| 10M, 1 | `89de199` | 2.39 | 2.56 | serial 1.68 | 1.42 | 1.53 | 0.84 | 5.2 |
| 10M, 1 | `0ee0be2` | 2.01 | 2.18 | serial 1.68 | 1.20 | 1.29 | 0.51 | **3.2** |
| 10M, 4 | `0ee0be2` | 2.08 | 2.41 | shared 2.05 / parts 2.91 | 1.02 / 0.71 | 1.18 / 0.83 | 0.58 | **4.6** |
| 100M, 1 | `89de199` | 28.68 | 30.06 | serial 25.30 | 1.13 | 1.19 | 9.22 | 6.1 |
| 100M, 1 | `0ee0be2` | 25.00 | 26.57 | serial 25.30 | 0.99 | 1.05 | 5.66 | **3.8** |
| 100M, 4 | `89de199` | 36.67 | 41.31 | shared 28.88 / parts 33.36 | 1.27 / 1.10 | 1.43 / 1.24 | 10.01 | 6.1 |
| 100M, 4 | `0ee0be2` | 26.33 | 30.38 | shared 28.88 / parts 33.36 | 0.91 / 0.79 | 1.05 / 0.91 | 6.44 | **3.9** |

**Measured.** Evaluation against the floor's generating at 100M: 6.9 times at `3058cf4` (second pass), 5.1 at `89de199`
there and 6.1 in this block, and **3.8 times with 1 worker and 3.9 with 4 at `0ee0be2`**. It is 7.9 ns of (d) per
successor, against the floor's 2.1 ns for generating, hashing and reading the frontier together.

## 4. The two lists, at `0ee0be2`

The contract: the model is read at run time (no code generated for it), and the answer, the ids and the paths are the
same whatever the number of workers.

### The contract's price

| Piece | 10M, 1 | 100M, 1 | 10M, 4 | 100M, 4 | Evidence |
|---|---:|---:|---:|---:|---|
| (A) evaluation beyond the floor's generating, (d) less generating | ≤ 0.35 s | ≤ 4.16 s | ≤ 0.45 s | ≤ 4.79 s | **Measured** difference. **Reading:** an outer bound. Two rounds took 3.3 s out of (d) at 100M inside the contract (9.0 → 5.7 s), so some of it is still inside. |
| (B) ids fixed whatever the workers: parts floor less shared floor | 0 | 0 | 0.86 s CPU, 0.21 s wall | 4.48 s CPU, 1.32 s wall | **Measured** at the floor's level. 100M is weak (disturbed rounds in both floors). |

### Inside the contract: the bounds of each piece (overlapping, against the floor's way of doing it)

| Piece | 100M, 1 | 100M, 4 | Mark |
|---|---:|---:|---|
| the index's second cache line (`Key::holds`) | ≤ 3.38 s | ≤ 3.10 s | The machine, through `hashbrown`'s layout; a table with the key beside its tag would remove most of it. Weak: hand tables lost before. |
| the second pass over fresh rows (`file_by_shard`, `move_row`) | ≤ 3.09 s | ≤ 3.45 s | The program's own. The parts floor writes each candidate into its shard's list when it is made, and keeps ids fixed. One pass was tried in this round and not kept. |
| the claim's own lines in `level.rs` (rank, write) | ≤ 2.03 s | (in (B)) | The program's own with 1 worker, where a serial table gives fixed ids without a rank. With 4 workers it is part of (B). |
| page faults with 4 workers beyond those with 1 | 0 | ≤ 1.89 s | The program's own. Its price is memory: keeping the buffers costs 19% to 25% more for 4% to 8% of wall (the other session), and it is not taken. |
| huge pages | ? | ? | The floor gains 5.2 s at 100M with them (32.22 → 27.00 s). Not tried in the checker. |

No longer on the list, by measurement:
- The claim's growth with threads (section 3).
- The frontier read and the row hand-out: 12 ns a state together at 100M, at or below the floor's own reading of its
  frontier.
- The part's table: 5.6 ns a successor, below the floor's 35 ns of store work a successor.

## 5. Take-away

Each case at `0ee0be2`, against the best floor of the same block (wall time in turn), with the rest split by the bounds
above. CPU seconds; the floor's CPU is the block median.

- **10 million states, 1 worker: 1.48 times the floor** (2.73 s of CPU against 1.84 s). Of the 0.89 s left, up to 0.35 s
  is the contract's price (a model read at run time). At least 0.54 s is inside the contract. It is the store and the
  faults above the floor's (0.50 s: the part's own lines, the claim's rank and write, the rehash) and 0.04 s of (g).
- **100 million states, 1 worker: 1.24 times the floor** (32.82 s against 26.80 s). Of the 6.0 s left, up to 4.2 s is the
  contract's price (evaluation), and at least 1.9 s is inside the contract: the store and faults 1.3 s above the floor,
  and (g) 0.6 s. The pieces behind it are the second pass over fresh rows (≤ 3.1 s), the index's second cache line
  (≤ 3.4 s) and the claim's rank and write (≤ 2.0 s). Each is an upper bound against the floor's way, and they overlap.
  Huge pages may add a few seconds more (the floor gains 5.2 s from them).
- **10 million states, 4 workers: 1.28 times the shared-table floor, 0.96 times the parts floor that keeps ids fixed.**
  The 0.91 s of CPU above the shared table is less than the contract's price by the bounds: 0.45 s of evaluation plus
  0.86 s for fixed ids. **Nothing measurable is left inside the contract here.**
- **100 million states, 4 workers: 1.21 times the shared-table floor, 1.01 times the parts floor.** Of the 6.9 s of CPU
  above the shared table, the contract's price by the bounds is up to 4.8 s (evaluation) plus 4.5 s (fixed ids, weak).
  That is more than the gap. What is left inside the contract is at most the faults beyond 1 worker (1.9 s of CPU),
  whose cure costs memory, and the second pass over fresh rows (≤ 3.5 s, tried once).

**Reading.** With 4 workers the checker is now at the floor that keeps ids fixed, and its distance to the fastest floor
is the contract's price. With 1 worker it is within 1.24 to 1.48 times the floor. The larger part of what is left at
100 million states is the price of reading the model at run time: (d) at 3.8 times the floor's generating. The rest is
2 s or less of store work that the bounds say is there to take, but no attempt so far has taken it.

## 6. What failed, was skipped or is weak

- **This boot was slower** than the second pass's, and unevenly: 31% for `89de199` at 100M with 1 worker, 12% for the
  serial floor. The ratios of this file come from this block only.
- **Two disturbed rounds** in the 4-thread floors at 100M (section 1). The in-turn floor is used for the 4-worker
  ratios. The contract's price (B) at 100M rests on the disturbed block medians and is weak.
- **Candidates** are still estimated (1.16 × the floor's count of distinct successors per part), not counted in the
  checker. The 4-against-1 ratios of the claim do not depend on that count.
- **Hardware counters** are refused. Cache lines and memory are readings of how costs grow.
- **Not done:** the floors were not tuned again, huge pages were not tried in the checker, and `3058cf4` was not
  profiled in this pass (its pieces are in `RESULTS-89de199.md`, from the other boot).

## 7. Commands

```
git fetch origin perf/joined; git worktree add /SCRATCH/src/0ee0be2 0ee0be2; two cargo builds into /SCRATCH/bin0ee/
# 3058cf4 in /SCRATCH/bin3058/, 89de199 in /SCRATCH/bin89/ (each earnest and earnest-sym)
bash scripts/pass3.sh all      # floors 15 min, timing 27 min, symbol check 1, profiles and callgrind about 5
python3 scripts/compare3.py /SCRATCH > raw/0ee0be2/compare3.md
```
