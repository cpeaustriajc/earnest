# The checker at `89de199` against `3058cf4` and the floor: what is left inside the contract

The second pass of round 9, measured on 2026-11-23. It repeats `RESULTS.md` (same scripts, groups and floors)
for commit `89de199` of `perf/joined`. In one block, on one machine, it times `3058cf4` and `89de199` in turn,
with the floor between them. Both builds are profiled on this machine, so every "before" and "after" below comes
from here. Raw logs: `raw/89de199/`. Scripts: `scripts/pass2.sh` (the whole pass), `scripts/compare2.py` (every
table below; output in `raw/89de199/compare2.md`). No file of the project was changed. Each finding is marked
**measured** or **reading**, as in `RESULTS.md`.

## 1. The machine, and why the floors were run again

This is not the guest of `RESULTS.md`: Intel(R) Xeon(R) Processor @ 2.10GHz, BogoMIPS 4200, L2 8 MiB, L3 260 MiB,
4 vCPU, KVM, 16 GiB, THP `madvise` (`raw/89de199/machine.txt`). So no second of this file compares with a second
of `RESULTS.md`. The floors were run again here (`scripts/floors_short.conf`: the best of each kind from the
first pass, with its settings unchanged, and the generating alone, 5 rounds). The builds: `89de199` plain and with
symbols, as before. `3058cf4` is the same binary as in the first pass (sha256 in `raw/89de199/sha256.txt`).

Load average before and after each block, from the `## machine` lines:

| Block | Log | Before | After |
|---|---|---|---|
| floors | `f_floors.log` | 1.29 0.99 0.42 | 1.44 1.54 1.10 |
| both builds and the floor, 7 rounds | `t1_7rounds.log` | 1.44 1.54 1.10 | 2.42 2.65 1.80 |
| both builds and the floor, `grid_100m` at 1 and 2 workers, 3 rounds | `t2_100m_3rounds.log` | 2.42 2.65 1.80 | 1.96 1.74 1.62 |
| plain against symbol build | `c3_sym_check.log` | 1.96 1.74 1.62 | 2.12 1.78 1.64 |
| ten profiles | `prof_{new,old}_*.log` | 2.12 … 1.23 | 1.87 … 1.71 |

**A weak number.** In `t2`, the floor at 100M with 1 thread took 26.91 s (22.98 to 27.54) with 4.06 s of system time.
In the floor block it took 24.04 s (23.12 to 24.39) with 1.77 s. The extra system time is most likely the kernel
compacting memory for huge pages after the checker's runs (THP defrag is `madvise`); that is my reading. So the
in-turn ratio at 100M with 1 worker is too kind to the checker. I give it beside the ratio against the floor
block.

## 2. Times: both builds and the floor in turn (step 5 of the request)

`scripts/time_turn.py 7 old=…,new=… grid_10m:1,grid_10m:4,grid_100m:4,…` and then `3 … grid_100m:1,grid_100m:2`.
In each round, `3058cf4`, `89de199` and the floor ran in turn. Every checker run printed `pass` with the expected
count and exited 0. Every floor printed its set checksum. Median (smallest), in seconds.

| Model | Workers | `3058cf4` wall | `89de199` wall | change | `89de199` CPU | System | Minor faults | Peak MiB | Floor in turn, `time_s` |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `grid_10m` | 1 | 3.046 (2.989) | 2.618 (2.305) | -14.1% | 2.545 | 0.100 | 62,813 | 187.3 | 1.531 (1.399) |
| `grid_10m` | 2 | 1.821 (1.786) | 1.617 (1.574) | -11.2% | 3.083 | 0.190 | 81,941 | 187.9 | 1.134 (1.079) |
| `grid_10m` | 4 | 1.062 (1.022) | 0.973 (0.919) | -8.4% | 3.413 | 0.242 | 94,357 | 190.2 | 0.592 (0.571) |
| `grid_100m` | 1 | 43.518 (42.539) | 30.947 (30.761) | -28.9% | 30.669 | 1.261 | 656,212 | 1618.1 | 26.910 (22.978) |
| `grid_100m` | 2 | 27.399 (25.514) | 21.347 (21.072) | -22.1% | 41.028 | 3.472 | 1,644,901 | 1556.0 | 16.228 (15.802) |
| `grid_100m` | 4 | 13.810 (13.585) | 11.350 (10.955) | -17.8% | 41.794 | 3.430 | 1,807,626 | 1561.9 | 7.545 (7.487) |
| `wide` | 1 | 0.325 (0.276) | 0.284 (0.236) | (see below) | 0.283 | 0.004 | 451 | 8.8 | |
| `wide` | 4 | 0.092 (0.081) | 0.089 (0.083) | | 0.302 | 0.018 | 1,058 | 8.8 | |
| `lights` | 1 | 0.928 (0.858) | 0.864 (0.839) | -6.9% | 0.847 | 0.031 | 15,262 | 57.5 | |
| `lights` | 4 | 0.355 (0.322) | 0.328 (0.322) | -7.6% | 1.132 | 0.087 | 17,625 | 61.6 | |

At `3058cf4`: CPU 3.001, 3.467, 3.780, 43.112, 52.762 and 51.267 s, and minor faults 62,660, 78,423, 90,560, 625,640,
1,430,455 and 1,695,484, in the order of the rows above.

**`wide` with its count of instructions** (callgrind, `raw/89de199/callgrind_*.txt`): 1 worker, 4,691.7 million
instructions at `3058cf4` and 4,708.2 million at `89de199` (+0.35%); 4 workers, 4,780.0 and 4,723.5 million. With 1
worker the time moved by -12.6% (median) for the same work give or take 0.4%. That is the effect of where the
compiler places the loop, of which the request warned. **Measured:** `wide` did not get cheaper in instructions.

**Symbol build** (`c3`, 5 rounds): `grid_10m` with 1 worker 2.668 s against 2.689 s; with 4, 1.097 s against 1.065 s;
`wide` 0.293 s against 0.289 s. So I take the profiles of the symbol build as profiles of the plain one.

### The whole checker against the best floor, before and after, from one block

The best floor for 1 thread is the serial floor with huge pages, and for 2 and 4 threads the shared table with
huge pages (section 3). "In turn" divides each round's checker wall time by the floor's `time_s` in the same
round and takes the median of the rounds (`compare2.md`). "Block" divides the median wall time by the floor
block's median `time_s`.

| Model | Workers | `3058cf4`, in turn | `89de199`, in turn | `3058cf4`, block | `89de199`, block | `89de199` against the floor that keeps ids fixed |
|---|---:|---:|---:|---:|---:|---:|
| `grid_10m` | 1 | 2.04 | 1.66 | 1.98 | 1.71 | 1.71 (the serial floor) |
| `grid_10m` | 2 | 1.58 | 1.45 | | | |
| `grid_10m` | 4 | 1.81 | 1.68 | 1.72 | 1.58 | 1.22 (parts, 0.796 s) |
| `grid_100m` | 1 | 1.67 | 1.18 | 1.81 | 1.29 | 1.29 (the serial floor) |
| `grid_100m` | 2 | 1.69 | 1.33 | | | |
| `grid_100m` | 4 | 1.81 | 1.48 | 1.73 | 1.42 | 1.23 (parts, 9.194 s) |

Per round, at 100M with 4 workers: `3058cf4` 1.85, 1.77, 1.80, 1.84, 1.81, 1.84, 1.77; `89de199` 1.48, 1.48, 1.57, 1.45,
1.57, 1.51, 1.44. With 1 worker: 1.85, 1.58, 1.67 and 1.35, 1.12, 1.18. Against the floor on 4 KiB pages (27.78 s at
100M and 1.707 s at 10M), `89de199` with 1 worker is 1.11 and 1.53 times.

## 3. The floors on this machine (`raw/89de199/f_floors.log`, 5 rounds)

| Floor | 10M `time_s` | CPU | 100M `time_s` | CPU | Sys | Ids fixed whatever the threads? |
|---|---:|---:|---:|---:|---:|---|
| serial, huge pages | 1.535 (1.458) | 1.530 | 24.038 (23.116) | 23.814 | 1.77 | yes (one thread) |
| serial, 4 KiB pages | 1.707 (1.643) | 1.702 | 27.778 (26.320) | 27.547 | 2.52 | yes |
| shared table, 4 threads, huge pages | 0.617 (0.601) | 2.059 | 7.966 (7.844) | 29.178 | 1.65 | **no**: a slot goes to the thread that comes first |
| shared table, 4 threads, 4 KiB pages | 0.678 (0.668) | 2.319 | 8.981 (8.355) | 33.286 | 2.88 | no |
| parts, 4 threads, huge pages | 0.796 (0.778) | 2.782 | 9.194 (8.838) | 33.132 | 2.20 | **yes**: its checksum is the serial one |
| generating alone, 1 thread | 0.095 (0.094) | 0.095 | 1.333 (1.275) | 1.314 | | |
| generating alone, 4 threads | 0.029 (0.028) | 0.114 | 0.313 (0.284) | 1.212 | | |

Every run had the expected counts of states and levels and the set checksums of `RESULTS.md`.

**Measured.** The parts floor writes its states in the serial order (it prints the serial checksum
`5a65a7777f25b9c1` at 100M), as the checker's ids are the same whatever the number of workers. It ranks each part's
winners in step 3, as `Winners::rank` does in the checker. The shared table does not. **So the parts floor is the
floor inside the contract for several threads, and the difference between the two is the price of fixed ids at
the floor's level:** at 100M with 4 threads, 1.23 s of wall time and 3.95 s of CPU; at 10M, 0.18 s and 0.72 s.

## 4. Profiles and groups, both builds on this machine

`profile.sh` (now taking the binary from `SYM`) for each build, with the same five cases as before. Samples:
15,348 → 13,919, 20,360 → 18,283, 47,351 → 34,568, 52,715 → 44,752 and 5,888 → 6,323 (`3058cf4` → `89de199`). None were
lost. Seconds are the share times the median CPU of the same build in `t1`/`t2`.

**`attr.py` at `89de199`.** A part's table for rows of one word is now `RecentWords` in `explore/part.rs`, an array
and not `hashbrown`. Without a rule it would fall to the part's own lines. **The change:** a frame in
`explore/part.rs` whose function names `RecentWords` is placed in the subgroup "part table: RecentWords in
explore/part.rs", group (c), the same piece as "hashbrown in explore/part.rs" at `3058cf4`. `write_directly` is in
`expand.rs` and falls in (d) by its file, so it needed nothing. As in round 7, 2.67% (1 worker) and 2.39% (4) of the
samples sit on one instruction of `PartSink::offer_encoded` with no line, `0x205a27`. It is the instruction after
`cmp %r14,(%r9,%rdx,8)`, the compare of the word with the `RecentWords` slot, followed by the `xor %eax,%eax` of
"already met". So it is the tail of that table's probe, and I count it with the part table (at `3058cf4` the same
by-name subgroup held 0.31%).

| Group, CPU s, `3058cf4` → `89de199` | 10M, 1 | 10M, 4 | 100M, 1 | 100M, 4 | `wide`, 1 |
|---|---:|---:|---:|---:|---:|
| (a) index of known states | 0.72 → 0.69 | 1.24 → 1.31 | 8.33 → 8.90 | 14.97 → 15.95 | 0.03 → 0.02 |
| (b) packed rows | 0.34 → 0.21 | 0.33 → 0.20 | 7.04 → 2.19 | 6.06 → 2.37 | 0.00 → 0.00 |
| (c) part tables, claim and merge | 0.89 → 0.73 | 0.89 → 0.79 | 14.93 → 9.00 | 15.41 → 10.08 | 0.00 → 0.00 |
| (d) actions and rules | 0.71 → 0.56 | 0.83 → 0.59 | 9.00 → 6.68 | 8.74 → 6.85 | 0.28 → 0.25 |
| (e) hashing | 0.12 → 0.17 | 0.14 → 0.17 | 1.38 → 1.69 | 1.45 → 1.78 | 0.00 → 0.00 |
| (f) allocation and page faults | 0.15 → 0.14 | 0.24 → 0.27 | 1.46 → 1.54 | 3.60 → 4.15 | 0.00 → 0.00 |
| (g) threads, waiting and the rest | 0.08 → 0.04 | 0.11 → 0.08 | 0.96 → 0.68 | 1.04 → 0.60 | 0.01 → 0.01 |
| total | 3.00 → 2.54 | 3.78 → 3.41 | 43.11 → 30.67 | 51.27 → 41.79 | 0.32 → 0.28 |

### Store and evaluation against the floor of this machine

| Run | Build | Store a+b+c+e | with (f) | Floor less its generating | Store / it | with (f) / it | (d) | (d) / generating |
|---|---|---:|---:|---:|---:|---:|---:|---:|
| 10M, 1 | `3058cf4` | 2.06 | 2.21 | 1.44 | 1.44 | 1.54 | 0.71 | 7.5 |
| 10M, 1 | `89de199` | 1.80 | 1.94 | 1.44 | 1.26 | 1.35 | 0.56 | 5.9 |
| 10M, 4 | `3058cf4` | 2.60 | 2.84 | 1.95 | 1.34 | 1.46 | 0.83 | 7.3 |
| 10M, 4 | `89de199` | 2.47 | 2.74 | 1.95 | 1.27 | 1.41 | 0.59 | 5.2 |
| 100M, 1 | `3058cf4` | 31.68 | 33.15 | 22.50 | 1.41 | 1.47 | 9.00 | 6.9 |
| 100M, 1 | `89de199` | 21.78 | 23.31 | 22.50 | **0.97** | **1.04** | 6.68 | 5.1 |
| 100M, 4 | `3058cf4` | 37.89 | 41.49 | 27.97 | 1.35 | 1.48 | 8.74 | 7.2 |
| 100M, 4 | `89de199` | 30.19 | 34.34 | 27.97 | 1.08 | 1.23 | 6.85 | 5.7 |

The 4-thread floor here is the shared table. Against the parts floor (33.13 s of CPU less 1.21 s = 31.92 s), the store
of `89de199` at 100M with 4 workers is 0.95 times, or 1.08 times with (f).

## 5. The pieces above 2% of `grid_100m`, at both commits (request 1)

Seconds of CPU of all threads. The units are those of `RESULTS.md`: 100M states, 720M successors, and an estimated
259.5M candidates at `3058cf4`. At `89de199` about 16% more rows reach the claim (the request's figure), so
about 301M candidates, and its cost per candidate is the column's figure divided by about 1.16.

| Piece | Group | 100M, 1: `3058cf4` | `89de199` | change | 100M, 4: `3058cf4` | `89de199` | change | ns per unit at `89de199`: 10M 1 / 100M 1 / 10M 4 / 100M 4 |
|---|---|---:|---:|---:|---:|---:|---:|---|
| a part's table (`hashbrown`, then `RecentWords`, with its tail) | c | 8.97 | 3.48 | **-5.49** | 9.67 | 4.26 | **-5.41** | 5.3 / 4.8 / 6.3 / 5.9 per successor |
| index probe in `claim_level` (`level.rs:131`) | a | 4.51 | 4.77 | +0.26 | 11.15 | 11.82 | +0.67 | 16.2 / 18.4 / 42.8 / 45.5 per candidate (÷1.16) |
| key load from the bucket, `Key::holds` (`index.rs:48`) | a | 3.19 | 3.51 | +0.32 | 3.17 | 3.48 | +0.30 | 7.1 / 13.5 / 4.2 / 13.4 per candidate (÷1.16) |
| frontier read, `store/rows.rs` | b | 5.59 | 0.63 | **-4.96** | 4.48 | 0.69 | **-3.78** | 5.8 / 6.3 / 5.6 / 6.9 per state |
| rows: codec, layout, `read_state`, `outside` | b | 1.45 | 1.56 | +0.11 | 1.59 | 1.68 | +0.09 | 14.9 / 15.6 / 14.8 / 16.8 per state |
| `explore/part.rs` lines of its own | c | 4.39 | 4.05 | -0.34 | 4.18 | 4.27 | +0.09 | 23.3 / 40.5 / 24.7 / 42.7 per state |
| `store/level.rs` lines of its own | c | 1.57 | 1.47 | -0.09 | 1.56 | 1.55 | -0.01 | 16.5 / 14.7 / 14.3 / 15.5 per state |
| `expand.rs` | d | 3.90 | 3.70 | -0.20 | 3.64 | 3.66 | +0.02 | 4.9 / 5.1 / 5.2 / 5.1 per successor |
| `earnest_ir` | d | 5.10 | 2.97 | **-2.13** | 5.10 | 3.19 | **-1.91** | 4.0 / 4.1 / 4.2 / 4.4 per successor |
| hashing, `mix`/`hash_words` | e | 1.38 | 1.69 | +0.30 | 1.45 | 1.78 | +0.33 | 2.7 / 2.3 / 2.7 / 2.5 per successor |
| kernel page faults | f | 1.26 | 1.36 | +0.10 | 2.99 | 3.42 | +0.43 | 9.0 / 13.6 / 14.4 / 34.2 per state |
| index rehash, outlined `reserve_rehash` | a | 0.61 | 0.61 | 0.00 | 0.65 | 0.66 | +0.01 | (2.0% and 1.6%) |
| libc `memmove`/`memset`, no caller | g | 0.79 | 0.62 | -0.16 | 0.78 | 0.46 | -0.32 | (2.0% and 1.1%) |

Of `explore/part.rs`'s own lines, `file_by_shard` with `move_row` is 6.7% (2.06 s) with 1 worker and 5.5% (2.32 s) with 4,
against 3.9% (1.70 s) and 3.4% (1.72 s) at `3058cf4`. The rest is the entry of `offer_encoded`, `fill_part` and
`add_fresh`.

**What moved. Measured:**
- The part's table, -5.5 s with 1 worker and -5.4 s with 4 (`85500f6`, `7bce9b9`, `89de199`). Its cost per successor at
  100M went from 12.5 ns to 4.8 ns, and it no longer grows from 10M to 100M (5.3 → 4.8 ns): it fits in the cache.
- The frontier read, -5.0 s and -3.8 s (`e94680b`). It is now 6.3 ns per state, flat with size. **This settles
  finding 4 of `RESULTS.md`: the cause was the program's (a copy of run-time length into a stack word, read back
  wider), not the machine.** My reading there, "not a wait on the row itself", was right. Its bound of 3.9 s was
  short of the 5.0 s the fix gave.
- `earnest_ir`, -2.1 s and -1.9 s (`f2bbf32`, `c066516`, `f19f2f5`). `write_directly`, new in `expand.rs`, has 3.4% and
  2.7%, while `expand.rs` as a whole stayed (-0.2 s and 0.0 s).

**What grew, a little. Measured:** the index probe (+0.26 s and +0.67 s), the key load (+0.32 s and +0.30 s) and the
hashing (+0.30 s and +0.33 s). **Reading:** that is the 16% more rows that the forgetting table lets through to the
claim. The index pieces grew by 6% to 10%, less than the rows, and the hashing in the claim grew with them.

**What did not move:** the decoding of rows, the claim's own lines in `level.rs`, `expand.rs` outside
`write_directly`, the rehash, and the page faults with 1 worker. With 4 workers the faults rose by 0.43 s (1.81
million against 1.70 million).

## 6. Each piece marked, with the correction (requests 2 and 3)

**The correction.** `RESULTS.md` marked the key compare in `Key::holds` as the program's own because "the index
holds ids, so the checker reads the state's row". That was wrong for `grid`. Rows of 4 bytes with no graph are
indexed by `Key<u32>`, which holds the row itself in the slot (`Filed::HOLDS_ROW = true` for `Key<W>` in
`store/index.rs`; `holds` is `self.0.into() == row[0]`). No row is read from the store. The samples on
`index.rs:48` are the load of the key from the table's bucket: in `hashbrown` the control bytes and the buckets
are separate arrays, so a probe that finds a candidate tag reads a second cache line. **Mark: work once per look-up
that the floor also does, at a cost the floor does not pay in that form.** The floor keeps the key itself in the
one slot it probes, so its compare is in the line it already loaded. The second line costs 3.51 s with 1 worker
and 3.48 s with 4 at `89de199`: 13.5 ns per candidate at 100M, against 7.1 ns at 10M, so it grows with size, which is
the machine. Together, the probe and this load cost 27.5 ns per candidate (8.28 s for about 301M), and the floor's
whole store work costs 31 ns per successor. The note in `RESULTS.md` is corrected to point here.

The contract, as the request gives it: the model is read at run time (no code is generated for it), and the answer,
the ids of states and the paths are the same whatever the number of workers.

**The distance at 100M, `89de199`, measured:**
- 1 worker: 30.67 s of CPU (30.95 s wall) against the serial floor's 23.81 s (24.04 s): 6.86 s of CPU, 1.29 times in
  wall. The store is already at the floor (0.97, 1.04 with (f)). The gap is (d) beyond the floor's generating
  (5.37 s), the store with faults (+0.81 s) and (g) (0.68 s).
- 4 workers: 41.79 s of CPU (11.35 s wall) against the shared table's 29.18 s (7.97 s), and the parts floor's 33.13 s
  (9.19 s).

### The contract's price: what only a compiled model or a first-come table would remove

| Piece | 100M, 1 worker | 100M, 4 workers | Evidence |
|---|---:|---:|---|
| Evaluation beyond the floor's generating: (d) less the generating alone | ≤ 5.37 s CPU | ≤ 5.64 s CPU (≈ 1.4 s wall) | **Measured:** 9.3 ns against 1.8 ns per successor. **Reading:** an outer bound. This round took 2.3 s out of (d) inside the contract, so not all of what is left belongs here. How it splits I cannot measure. |
| Ids fixed whatever the workers: the parts floor less the shared-table floor | 0 (one thread) | 3.95 s CPU, 1.23 s wall | **Measured** at the floor's level (section 3). The checker's own counterparts are the claim in shard order and `Winners::rank`/`write_level` (1.55 s). They are inside this price, not on top of it. |

### Inside the contract: what a change of the checker could still take

Each is an upper bound against the floor's way of doing the same thing. They overlap, and their sum is larger than the
distance, because the checker already does some of the store work for less than the floor (its part's table: 4.8 ns
against the floor's 31 ns per successor for all of its store work).

| Piece | 100M, 1 worker | 100M, 4 workers | Mark and evidence |
|---|---:|---:|---|
| The index's second cache line (`Key::holds`, the corrected piece) | ≤ 3.51 s | ≤ 3.48 s | **The machine, in a table layout** (section 6, the correction). A table with the key beside its tag in one line would remove most of it. Hand-made tables lost to `hashbrown` before (from the other sessions), so the bound is weak. |
| The second pass over fresh rows, `file_by_shard` with `move_row` | ≤ 2.06 s | ≤ 2.32 s | **The program's own.** The floor has no such pass. One pass was tried this round and not kept (the request). |
| Decoding each frontier row into values (codec, `read_state`, `outside`) | ≤ 1.56 s | ≤ 1.68 s | **The program's own.** 15.6 ns per state, flat. The floor works on its packed word, and `write_directly` shows that part of the evaluation can do so too. |
| The claim's own lines in `level.rs` (`Winners`, `rank`, `write_level`) | ≤ 1.47 s | (in the price above) | With 1 worker the serial floor gets fixed ids without ranking, so this is **the program's own** there (the cost of the parts design at one thread). With 4 workers it is part of the price of fixed ids. |
| Page faults with 4 workers beyond those with 1 | 0 | ≤ 2.06 s | **The program's own.** 1.81 million faults against 0.66 million; memory given back and taken again level after level. Buffers kept by several workers were tried this round and not kept. |
| The index probe's growth with threads beyond the in-contract floor's | 0 | ≤ about 5.0 s CPU (≈ 1.3 s wall) | **Reading.** 18.4 → 45.5 ns per candidate (2.5 times). The parts floor goes from 31.2 to 44.3 ns per successor (1.42 times), so 11.82 s less 4.77 s × 1.42. Some of it may be the shard claim, which is the contract's price. |
| Huge pages for the big tables | unknown, the floor gains 3.74 s | unknown | **The program's own (a choice of the allocator).** The floor goes from 27.78 to 24.04 s at 100M with them. The checker asks for none. Not measured on the checker. |

**Reading: how much of the distance is left inside the contract.** With 1 worker at 100M the distance is 6.9 s of CPU.
The compiled model accounts for at most 5.4 s of it, so at least 1.5 s is inside the contract. The pieces above say the
checker could go below the floor in its store, by up to the bounds given. With 4 workers the distance to the shared
table is 12.6 s of CPU (3.4 s wall). The contract's price is at most 5.6 s (compiled model) plus 3.95 s (fixed ids),
which leaves at least 3.0 s of CPU inside the contract against the parts floor. The faults beyond 1 worker (2.1 s) are
the largest measured part of that.

## 7. What failed, was skipped or is weak

- **Another machine** (2.10 GHz, L3 260 MiB) than `RESULTS.md`. That is why the floors were run again and both builds
  were profiled again here. The floors' settings were tuned on the other machine and not tuned again.
- **The floor at 100M with 1 thread** was slower in turn (26.9 s, 4 s of system time) than in its own block (24.0 s).
  Both ratios are given (section 2).
- **Candidates** are still estimated in the floor (259.5M at 64 parts a level), and at `89de199` taken as 16% more, by
  the request's figure. Not counted in the checker.
- **Hardware counters** are refused, so the second cache line of the index and the fits-in-the-cache of the part's
  table are readings of how costs grow with size, not counts of misses.
- **`wide`** moved in time but not in instructions (section 2). It is not used for any conclusion.
- **Not done:** the floor was not tuned again, huge pages were not tried in the checker, and the split of (d) between
  "inside" and "price" is not measured.

## 8. Commands

```
git fetch origin perf/joined; bash scripts/build.sh 89de199    # or the two cargo builds, as here
# 3058cf4's plain and symbol builds kept from the first pass, under /SCRATCH/bin3058/
bash scripts/pass2.sh all       # floors (floors_short.conf, 5 rounds), t1 and t2 in turn, c3, ten profiles, callgrind of wide
python3 scripts/compare2.py /SCRATCH > raw/89de199/compare2.md
```

A pass of `pass2.sh all` took 37 minutes here (05:40 to 06:17 UTC): floors 13, timing in turn 18, symbol check 1,
profiles of both builds 3.5, callgrind 1.5. Without the floors and with one build it is about 15 minutes, as `pass.sh`.
