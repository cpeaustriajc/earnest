# Where the checker's time goes at `10015fc`, against `9c5413e` and the floor

Measured on 2026-11-22 on a 4-vCPU cloud machine (KVM guest). The checker is commit `10015fc`, the tip of
`perf/joined`. It is timed in turn with `9c5413e`, the commit of the earlier write-up in
`measure/profile-lone/RESULTS.md`, whose method this repeats: the same builds, the same scripts, `perf` on
the software clock, samples placed by source line through `addr2line -i`, the groups (a) to (g), and the
floor program in three variants. This branch is cut from `origin/measure/profile-lone` and adds only this
folder. No source file of the project was changed on a branch (one scratch copy was changed and built
outside the repository, section 8). Every number below comes from a file in `raw/`, made by a script in
`scripts/`. In the raw files, `/SCRATCH` stands for the scratch folder of the session.

**Nothing here compares in seconds with the earlier write-up.** This is another CPU (2.30 GHz, L3 504 MiB,
against 2.10 GHz and L3 260 MiB there). Every "before and after" below comes from this machine: `9c5413e` and
`10015fc` ran in turn, round by round.

## 1. The machine

From `raw/machine.txt` and the `## machine` lines at the start and end of every log:

- `nproc` 4. Intel(R) Xeon(R) Processor @ 2.30GHz, 1 thread per core, KVM. BogoMIPS 4600. L1d 192 KiB
  (4 instances), L2 8 MiB (4 instances), L3 504 MiB (1 instance). This is not the CPU of
  `measure/profile-lone` (BogoMIPS 4200, L3 260 MiB).
- Memory 16,087 MiB in total, no swap. Kernel 6.18.44-fc-v726.
- `rustc 1.100.0 (0ef1bc386 2026-11-09)`, `cargo 1.100.0`, with `RUSTUP_TOOLCHAIN=1.100.0` for every build.
- `perf` 6.8.12 (`linux-tools-generic`, installed with apt) and GNU `time` (installed with apt).
  `perf_event_paranoid` is 2. Hardware counters are refused (`cycles`, `instructions` and `cache-misses`
  print `<not supported>`, `raw/perf_stat_hw_check.txt`), so the profiles sample the software clock
  (`cpu-clock`), as before.

The load average (1, 5 and 15 min) before and after each group. Only one build or run went at any time: the
builds ran first, one after another, and then the groups, one after another.

| Group | Before | After |
|---|---|---|
| 1: `grid_10m`, `lights`, `wide`, timing | 1.81 2.94 1.70 | 3.76 3.27 1.96 |
| 1b: the 4-job cases of group 1 again, and `grid_10m` with 1 job | 1.14 1.27 1.48 | 1.41 1.39 1.51 |
| 2: `grid_100m`, timing | 3.76 3.27 1.96 | 3.68 2.51 1.96 |
| 3: the floor | 3.68 2.51 1.96 | 1.03 1.22 1.50 |
| 4: `10015fc` against its symbol build | 1.03 1.22 1.50 | 1.43 1.28 1.51 |
| 5: profile `grid_10m`, 1 job | 1.43 1.28 1.51 | 1.34 1.27 1.50 |
| 6: profile `grid_10m`, 4 jobs | 1.34 1.27 1.50 | 1.55 1.31 1.51 |
| 7: profile `grid_100m`, 1 job | 1.55 1.31 1.51 | 1.28 1.27 1.49 |
| 8: profile `grid_100m`, 4 jobs | 1.26 1.27 1.48 | 1.87 1.40 1.52 |
| 9: profile `wide`, 1 job | 1.87 1.40 1.52 | 1.73 1.39 1.52 |
| 10: call chains (DWARF) of `grid_100m`, 1 job | 0.21 0.91 1.31 | 0.63 0.92 1.29 |
| 11: scratch probe of `word_of`, timing | 1.65 1.01 1.24 | 1.01 1.00 1.16 |
| 12: scratch probe of `word_of`, profile | 0.85 0.97 1.15 | 0.94 0.97 1.14 |

The load average counts the runs of the group before. The 1.81 before group 1 is the builds and the
`rustfilt` install that ran just before it.

**Group 1's 4-job runs were disturbed, and group 1b replaces them.** In group 1, `grid_10m` with 4 jobs took
2.15 s (`10015fc`) and 2.55 s (`9c5413e`) of wall time for 3.5 s and 4.1 s of CPU, so fewer than 2 of the 4
threads ran at a time. The same binary took 1.00 s in group 4 and 1.04 s in group 1b. Nothing else ran in the
guest. So the cause was most likely outside it, on the host: by group 1b the guest had 20.3 s of steal in
`/proc/stat` since boot, and the 5-minute load average was 2.94 at the start of group 1. Group 1b ran the 4-job cases again 13 minutes later. Its `/proc/stat` line
before and after shows 0.87 s of steal over 74 s of runs (`raw/g1b_rerun_j4.log`). The 1-job cases of group 1
agree with group 1b (`grid_10m`, 1 job: 2.909 s against 2.833 s for `10015fc`, 3.426 s against 3.633 s for
`9c5413e`), so I keep group 1 for 1 job and group 1b for 4 jobs. Both are in `raw/`.

## 2. The builds

| Binary | How | sha256 (`raw/machine.txt`) |
|---|---|---|
| `earnest-old` | `cargo build --release --locked -p earnest-cli` at `9c5413e` | `raw/machine.txt` |
| `earnest-new` | the same at `10015fc` | `raw/machine.txt` |
| `earnest-sym` | `10015fc` with `CARGO_PROFILE_RELEASE_DEBUG=true` and its own `CARGO_TARGET_DIR` | `raw/machine.txt` |
| `floor_i`, `floor_ii`, `floor_iii` | `rustc -O` (1.100.0) of `scripts/floor_{i,ii,iii}.rs` | `raw/machine.txt` |
| `earnest-probe` | scratch only: `10015fc` with `raw/probe_word_of.diff` (section 8) | not kept |

`floor_i.rs` is `measure/round6/scripts/floor_i.rs` of `measure/round6-results`, and I checked that it is byte
for byte `measure/floor.rs` of `measure/cloud-results`. `floor_ii.rs` and `floor_iii.rs` are the ones in
`measure/profile-lone/scripts/`, and `floor_ii.rs` is byte for byte the round-6 file. The four models were
copied into one folder: `grid_10m` and `wide` from `tasks/benchmark/models/` (the same bytes at both commits),
and `lights` and `grid_100m` from `measure/tools/models/` (branch `measure/tools`).

**The symbol build runs in the same time as the plain one** (group 4, `raw/g4_sym_check.log`, 5 rounds in
turn). `grid_10m`, 1 job: 2.886 s against 2.889 s of wall time. 4 jobs: 0.996 s against 0.995 s. `wide`,
1 job: 0.329 s against 0.364 s (+11%; at a third of a second this is mostly the loading of a 76 MB binary).
So I take the profiles of `earnest-sym` as profiles of `earnest-new`.

## 3. Wall time, CPU time and peak memory: `9c5413e` against `10015fc`

`earnest verify --no-cache --json --jobs N --max-states 200000000 <model>.model.ts`, run from the model
folder, by `scripts/time_runs.py` of the earlier write-up, unchanged. For each case there is one uncounted
run, then 7 rounds (3 for `grid_100m`), and in each round the two builds run in turn. CPU time is user plus
system time from `wait4`. Peak memory is `ru_maxrss`. Every run printed `pass` with the expected number of
states (10,000,000, 3,000,000, 10,000 or 100,000,000) and exited 0.

Each cell is the median, then the smallest value in brackets. Times are in seconds.

| Model | Jobs | Build | Wall | CPU | System | Peak MiB | Wall change |
|---|---:|---|---:|---:|---:|---:|---:|
| `grid_10m` | 1 | `9c5413e` | 3.426 (3.252) | 3.396 (3.217) | 0.228 | 206.7 | |
| `grid_10m` | 1 | `10015fc` | 2.909 (2.708) | 2.880 (2.702) | 0.214 | 191.4 | -15.1% |
| `grid_10m` | 4 | `9c5413e` | 1.285 (1.198) | 4.641 (4.305) | 0.362 | 198.4 | |
| `grid_10m` | 4 | `10015fc` | 1.043 (0.997) | 3.782 (3.617) | 0.279 | 190.4 | -18.8% |
| `lights` | 1 | `9c5413e` | 0.923 (0.892) | 0.916 (0.890) | 0.036 | 65.7 | |
| `lights` | 1 | `10015fc` | 0.782 (0.757) | 0.775 (0.756) | 0.028 | 62.3 | -15.3% |
| `lights` | 4 | `9c5413e` | 0.379 (0.367) | 1.331 (1.283) | 0.086 | 66.5 | |
| `lights` | 4 | `10015fc` | 0.344 (0.331) | 1.201 (1.150) | 0.084 | 63.4 | -9.2% |
| `wide` | 1 | `9c5413e` | 0.560 (0.548) | 0.556 (0.545) | 0.000 | 9.6 | |
| `wide` | 1 | `10015fc` | 0.326 (0.316) | 0.323 (0.315) | 0.004 | 9.6 | -41.8% |
| `wide` | 4 | `9c5413e` | 0.161 (0.158) | 0.597 (0.568) | 0.016 | 9.5 | |
| `wide` | 4 | `10015fc` | 0.107 (0.101) | 0.383 (0.365) | 0.019 | 9.5 | -33.5% |
| `grid_100m` | 1 | `9c5413e` | 50.353 (48.549) | 50.260 (48.274) | 4.674 | 1621.1 | |
| `grid_100m` | 1 | `10015fc` | 40.188 (39.954) | 39.975 (39.740) | 2.375 | 1726.7 | -20.2% |
| `grid_100m` | 4 | `9c5413e` | 15.978 (15.357) | 59.486 (57.039) | 5.481 | 1637.3 | |
| `grid_100m` | 4 | `10015fc` | 13.570 (13.451) | 50.583 (50.186) | 4.151 | 1612.3 | -15.1% |

The 1-job rows come from group 1 (`raw/g1_10m_lights_wide.log`) and the 4-job rows of the three small models
from group 1b (`raw/g1b_rerun_j4.log`). The `grid_100m` rows come from group 2 (`raw/g2_100m.log`). Every
change is larger than this machine's spread of 5% to 10%, except `lights` with 4 jobs (-9.2%).

Minor page faults (medians, `minflt`): `grid_100m` with 1 job 2,620,482 → 1,377,882, so 10.0 GiB → 5.3 GiB
faulted in for a peak of 1.6 to 1.7 GiB. With 4 jobs 2,803,405 → 2,209,856 (8.4 GiB). `grid_10m` with 1 job
135,187 → 124,701. With 1 job at 100 million states, `10015fc` takes 6.5% more peak memory (1,727 MiB
against 1,621 MiB).

## 4. The floor

`scripts/floor_block.sh`: one uncounted run (`floor_i 7 serial`), then 5 rounds at 7 wheels (10 million states)
and 5 at 8 wheels (100 million). In each round the variants run in turn (i, ii, iii). Every run printed the
expected state count and the same checksums as in the earlier write-up (`4c9670a40c764e17/6ff96c9c5e19c80e`
and `5a65a7777f25b9c1/b9bf724eeb0b4976`) and exited 0. `time_s` is the floor's own clock. CPU is user plus
system time from GNU `time`. Raw: `raw/g3_floor.log`.

| Variant | States | `time_s` median (smallest) | All 5 `time_s` | CPU median (smallest) | System | Peak MiB |
|---|---:|---:|---|---:|---:|---:|
| i, no prefetch | 10M | 2.788 (2.657) | 2.788, 2.900, 2.895, 2.657, 2.783 | 2.81 (2.69) | 0.22–0.26 | 277 |
| ii, prefetch | 10M | 1.564 (1.429) | 1.564, 1.676, 1.429, 1.545, 1.602 | 1.59 (1.46) | 0.19–0.28 | 277 |
| iii, plain load | 10M | 2.078 (1.990) | 2.003, 2.377, 1.990, 2.261, 2.078 | 2.10 (2.00) | 0.21–0.27 | 277 |
| i, no prefetch | 100M | 48.599 (46.032) | 48.599, 46.033, 48.259, 48.716, 49.089 | 48.76 (46.20) | 3.08–3.57 | 4165 |
| ii, prefetch | 100M | 26.199 (25.326) | 26.683, 25.326, 26.199, 25.422, 27.628 | 26.52 (25.64) | 3.02–3.53 | 4165 |
| iii, plain load | 100M | 35.766 (34.632) | 35.766, 34.632, 35.290, 36.071, 36.914 | 35.95 (34.80) | 3.08–3.47 | 4165 |

On this machine the prefetch saves 1.22 s (44%) at 10 million states and 22.40 s (46%) at 100 million
(medians). The plain load saves 0.71 s (25%) and 12.83 s (26%).

## 5. Profiles of `10015fc`

### How they were taken

`scripts/profile.sh` of the earlier write-up, with only the script paths changed: `perf record -e cpu-clock
-F 999 -g` around N runs of `earnest-sym`, from the model folder. N is 5 for `grid_10m` (14,786 and 19,725
samples), 1 for `grid_100m` (39,686 and 53,080 samples) and 20 for `wide` (7,308 samples). No samples were
lost. Every profiled run printed `pass` and the expected state count. The profiled `grid_100m` runs took
39.77 s (1 job) and 14.30 s (4 jobs) of wall time, close to the unprofiled medians (40.19 s, 13.57 s). As
before, the **shares** come from the profiles and the **seconds** are the share times the median CPU time of
the unprofiled `10015fc` runs in table 3.

Samples are placed by `scripts/attr.py` of the earlier write-up, unchanged. The groups are decided by file,
and none of the files of the groups was renamed or split between the two commits. Its rules:

- The first frame in an earnest crate decides the group by its file: `store/index.rs` (a); `store/rows.rs`,
  `store/codec.rs`, `store/layout.rs` and `Store::read_state` in `store.rs` (b); `explore/part.rs`,
  `explore/merge.rs`, `store/level.rs` (c); `expand.rs`, `explore/encode.rs`, `earnest_ir`, `earnest_value`
  (d); `mix`, `hash_words` and `hash_each` in `store/layout.rs` (e); `explore/workers.rs`, `explore.rs`,
  `explore/here.rs`, `helpers.rs` (g).
- hashbrown inlined into `store/level.rs`, `store/index.rs` or `store.rs` is a probe of the index (a).
  hashbrown inlined into `explore/part.rs` is a probe of a part's table (c). The outlined
  `RawTable<Key<u32>>::reserve_rehash` is (a).
- Kernel page faults, `mmap`, `munmap` and `madvise`, and libc `malloc`, `realloc`, `free` (and a `memmove` in
  `realloc`) are (f). Futex, threads, scheduling, other `memmove`/`memset` and everything else are (g).
- A sample with no earnest line is placed by the name of its function.

**One new placement by name, which matters.** At `10015fc`, 2.5% to 7.0% of the samples fall on one
instruction of `PartSink::offer_encoded` that has no earnest line (`vec/mod.rs:?`): `0x1f79e1`, the shared
`return false` of the function. The jumps to it come from `Store::find` (the graph path, not taken here) and
from `cmp %rbx,-0x8(%rbp)` / `je`: the key compare of the part table's probe in `offer_unseen_word`
(`part.rs:125`), when the word is already there. A sample of the software clock lands on the instruction after
a slow one, so these samples are the tail of the part table's probe, a load from the table that missed. The
name rule puts them in (c), the group of the probe. Below I count them with the probe.
`raw/prof_*_elines.txt` lists them, and section 8 has the disassembly.

### Shares and seconds by group

| Group | 10M, 1 job | | 10M, 4 jobs | | 100M, 1 job | | 100M, 4 jobs | | `wide`, 1 job | |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| | share | CPU s | share | CPU s | share | CPU s | share | CPU s | share | CPU s |
| (a) index of known states | 20.3% | 0.58 | 33.1% | 1.25 | 20.0% | 8.00 | 30.8% | 15.58 | 3.6% | 0.012 |
| (b) packed rows | 10.9% | 0.31 | 9.2% | 0.35 | 14.8% | 5.91 | 12.7% | 6.41 | 7.6% | 0.024 |
| (c) part tables, claim and merge | 29.9% | 0.86 | 23.9% | 0.90 | 33.9% | 13.55 | 27.6% | 13.94 | 0.0% | 0.000 |
| (d) actions and rules | 22.7% | 0.65 | 19.0% | 0.72 | 18.8% | 7.52 | 15.5% | 7.82 | 76.3% | 0.246 |
| (e) hashing | 3.6% | 0.10 | 3.1% | 0.12 | 3.0% | 1.18 | 2.5% | 1.25 | 10.4% | 0.034 |
| (f) allocation and page faults | 9.5% | 0.27 | 8.5% | 0.32 | 6.9% | 2.75 | 9.1% | 4.61 | 0.5% | 0.001 |
| (g) threads, waiting and the rest | 3.2% | 0.09 | 3.2% | 0.12 | 2.7% | 1.06 | 1.9% | 0.98 | 1.7% | 0.005 |
| total (median CPU of `10015fc`) | 100% | 2.88 | 100% | 3.78 | 100% | 39.98 | 100% | 50.58 | 100% | 0.323 |

By dso: the kernel has 7.3%, 7.6%, 6.0%, 8.6% and 0.7% of the samples (same column order), libc 5.1%, 3.7%,
3.4%, 2.4% and 0.2%, and the rest is the earnest binary (`raw/prof_*_dso.txt`).

`wide` takes another path: 10,000 states in levels too narrow for parts, so one thread expands them in
`Search::level` (`explore/here.rs`) and puts each new state straight into the index (`Store::place`). Group (c)
has no work there. Each state tries 1,000 input combinations, so almost all of the time is (d).

### The largest subgroups

Share of all samples. The full lists are in `raw/tables.md` and `raw/prof_*_groups.json`.

| Subgroup | Group | 10M, 1 job | 10M, 4 jobs | 100M, 1 job | 100M, 4 jobs |
|---|---|---:|---:|---:|---:|
| index probe and insert in `claim_level` (hashbrown in `store/level.rs`) | a | 13.51% | 27.83% | 11.90% | 23.33% |
| `store/index.rs` (the key compare `Key::holds`, `shard_of`, `Key::new`) | a | 3.97% | 3.30% | 6.57% | 6.17% |
| `RawTable<Key<u32>>::reserve_rehash` (shard tables growing) | a | 2.49% | 1.98% | 1.51% | 1.31% |
| a part's table, `offer_unseen_word` (hashbrown in `explore/part.rs`) | c | 12.11% | 10.56% | 12.41% | 9.92% |
| the tail of that probe, placed by name in `offer_encoded` | c | 2.73% | 2.54% | 7.01% | 5.90% |
| `explore/part.rs` lines of its own (`file_by_shard`, `move_row`, `add_fresh`, `expand_ids`) | c | 7.33% | 6.08% | 9.23% | 7.52% |
| `store/level.rs` lines of its own (`ordinal_of`, `rank`, `Winners`, `write_level`) | c | 7.32% | 4.10% | 4.84% | 3.76% |
| `earnest_ir` (`Code::scalar`, `Cond::leaf`, `Num::int_at`, `eval.rs`) | d | 13.21% | 11.47% | 11.07% | 9.33% |
| `expand.rs` (`Expand::state`, `try_input`, the odometer) | d | 9.43% | 7.48% | 7.73% | 6.10% |
| `store/rows.rs` (`word_of` in `Rows::unpack`) | b | 5.15% | 4.51% | 10.16% | 8.60% |
| `store/layout.rs`, not hashing (`write`, reading fields) | b | 3.08% | 2.65% | 2.62% | 2.19% |
| `mix` and `hash_words` | e | 3.56% | 3.15% | 2.96% | 2.46% |
| kernel page faults | f | 6.17% | 5.10% | 5.21% | 7.40% |
| kernel `mmap`, `munmap`, `madvise` | f | 1.03% | 1.66% | 0.67% | 0.95% |
| libc `memmove`/`memset`, caller not resolved | g | 2.67% | 1.84% | 2.37% | 1.57% |
| kernel futex | g | below 0.1% | 0.16% | below 0.03% | 0.01% |
| kernel thread start and exit | g | below 0.1% | 0.68% | 0.09% | 0.19% |

`wide`, 1 job: `earnest_ir` 34.82%, `expand.rs` 23.62%, hashbrown inlined into `expand.rs` (the table of
`Seen::first_row`, which keeps one state's successors apart) 16.49%, hashing 10.41%, `store/layout.rs` 7.53%,
the index 3.5% in all.

Where the page faults come from, by the first user frame under the kernel chain (`raw/prof_*_groups.json`,
`kernel_sites`). At 100M with 1 job: libc (`memset`, `memmove`, `realloc` into fresh memory) 2.35%, a part's
buffers in `explore/part.rs` 1.50%, the outlined rehash of the index 0.96%, `store/level.rs` 0.39%. The DWARF
recording (section 8) names the callers: `claim_level` 3.0% of its samples, `write_level` 1.6%,
`move_row` 0.3%.

### The heaviest source lines

All the samples whose first earnest frame is that line, inlined library lines included
(`raw/prof_*_elines.txt`). The old write-up's line is given where the code moved.

| Line at `10015fc` | At `9c5413e` | What it is | 10M, 1 job | 10M, 4 jobs | 100M, 1 job | 100M, 4 jobs |
|---|---|---|---:|---:|---:|---:|
| `store/level.rs:118` | `:113` | `table.entry(…)` in `claim_level`: the probe of the index | 12.90% | 23.89% | 11.49% | 19.74% |
| `explore/part.rs:125` | `:124` | `met_words.entry(…)` in `offer_unseen_word`: the probe of a part's table | 10.69% | 9.41% | 11.25% | 8.98% |
| (no line) in `offer_encoded` | (none) | the tail of that probe (section 5, placement) | 2.73% | 2.54% | 7.01% | 5.90% |
| `store/rows.rs:17` | `:17` | `word_of`, the copy for a row shorter than a word (4 bytes here) | 3.77% | 3.32% | 8.86% | 7.71% |
| `store/index.rs:48` | `:48` | `Key::holds`: compares the key the probe found | 3.07% | 2.57% | 4.77% | 4.92% |
| `store/level.rs:124` | `:119` | `vacant.insert(filed)`: a new key into the index | 0.61% | 3.97% | 0.41% | 3.64% |
| `earnest_ir/code/num.rs:95` | (new) | `int_at`: reading an integer slot for `Code::scalar` | 2.36% | 2.41% | 1.83% | 1.84% |
| `explore/part.rs:289` | (new) | `move_row`, the copy of a fresh row into shard order | 0.47% | (below 0.6%) | 1.26% | 0.91% |
| `store/level.rs:43` | `:43` | `hash_of` read in shard order by the claim (6.49% and 3.10% before) | below 0.6% | below 0.6% | below 0.6% | below 0.6% |

### The ten heaviest functions

The functions as `perf` sees them, after inlining (`raw/tables.md`). The last column is how the samples of the
function split over the groups.

**`grid_10m`, 1 job**

| # | Share | Function | Groups |
|---:|---:|---|---|
| 1 | 24.20% | `explore::Search::work` (with the expansion inlined) | d 59%, c 17%, e 14%, b 8%, a 1% |
| 2 | 19.55% | the closure of `store::level::gather` passed by `Shards<Key<u32>>::claim_level` | a 88%, c 12% |
| 3 | 18.05% | `explore::part::PartSink::offer_encoded` | c |
| 4 | 4.87% | `earnest_ir::code::Code::scalar` (new) | d |
| 5 | 4.47% | `store::rows::Rows::unpack` | b |
| 6 | 4.13% | the closure of `store::level::write_level` | c 83%, b 17% |
| 7 | 3.81% | `store::Store::read_state` | b |
| 8 | 3.34% | `earnest_ir::code::cond::Cond::leaf` (new) | d |
| 9 | 2.68% | `hashbrown::raw::RawTable<Key<u32>>::reserve_rehash` | a 93%, e 7% |
| 10 | 1.93% | kernel `exc_page_fault` | f |

**`grid_10m`, 4 jobs**

| # | Share | Function | Groups |
|---:|---:|---|---|
| 1 | 32.66% | the closure of `gather` in `claim_level` | a 94%, c 6% |
| 2 | 20.66% | `Search::work` | d 57%, c 17%, e 14%, b 9%, a 2% |
| 3 | 15.72% | `PartSink::offer_encoded` | c |
| 4 | 4.20% | `Code::scalar` | d |
| 5 | 4.05% | `Rows::unpack` | b |
| 6 | 2.93% | `Cond::leaf` | d |
| 7 | 2.82% | `Store::read_state` | b |
| 8 | 2.44% | the closure of `write_level` | c 81%, b 19% |
| 9 | 2.16% | `RawTable<Key<u32>>::reserve_rehash` | a 92%, e 8% |
| 10 | 1.74% | kernel `exc_page_fault` | f |

**`grid_100m`, 1 job**

| # | Share | Function | Groups |
|---:|---:|---|---|
| 1 | 23.53% | `Search::work` | d 49%, c 26%, e 12%, b 8%, a 4% |
| 2 | 22.42% | `PartSink::offer_encoded` | c |
| 3 | 19.36% | the closure of `gather` in `claim_level` | a 91%, c 9% |
| 4 | 9.58% | `Rows::unpack` | b |
| 5 | 4.42% | `Code::scalar` | d |
| 6 | 2.89% | the closure of `write_level` | c 80%, b 20% |
| 7 | 2.83% | `Store::read_state` | b |
| 8 | 2.75% | `Cond::leaf` | d |
| 9 | 1.76% | libc `__memset_avx512_unaligned_erms` | g |
| 10 | 1.62% | `RawTable<Key<u32>>::reserve_rehash` | a 94%, e 6% |

**`grid_100m`, 4 jobs**

| # | Share | Function | Groups |
|---:|---:|---|---|
| 1 | 30.07% | the closure of `gather` in `claim_level` | a 95%, c 5% |
| 2 | 19.03% | `Search::work` | d 49%, c 26%, e 12%, b 8%, a 4% |
| 3 | 18.47% | `PartSink::offer_encoded` | c |
| 4 | 8.20% | `Rows::unpack` | b |
| 5 | 3.64% | `Code::scalar` | d |
| 6 | 2.55% | `Store::read_state` | b |
| 7 | 2.52% | `Cond::leaf` | d |
| 8 | 2.22% | kernel `exc_page_fault` | f |
| 9 | 2.16% | the closure of `write_level` | c 82%, b 18% |
| 10 | 1.38% | `RawTable<Key<u32>>::reserve_rehash` | a 95%, e 5% |

**`wide`, 1 job**

| # | Share | Function | Groups |
|---:|---:|---|---|
| 1 | 42.38% | `explore::Search::level` (the one-thread path, expansion inlined) | d 58%, e 25%, b 18% |
| 2 | 30.13% | `earnest_ir::code::num::Num::rest` | d |
| 3 | 19.79% | `expand::Seen::first_row` | d |
| 4 | 3.53% | `store::Store::place` | a |
| 5 | 1.53% | `Num::eval` | d |
| 6 | 0.81% | `explore::here::HereSink::place_row` | g |
| 7 | 0.36% | `Code::scalar` | d |
| 8 | 0.10% | libc `get_common_cache_info` (start-up) | g |
| 9 | 0.07% | libgcc `__cpu_indicator_init` (start-up) | g |
| 10 | 0.05% | `RawTable<Key<u32>>::reserve_rehash` | a |

### Where the functions of the old write-up went

- **The closure of `gather` in `claim_level`** is the same function. The index probe moved from
  `level.rs:113` to `:118` and the insert from `:119` to `:124`. It no longer looks a fresh row up through
  `by_shard` (`hash_of(part, ordinal)` at `level.rs:43`, 6.49% of the samples before). The part now hands its
  rows, hashes and parents already in shard order, with an `ordinals` array beside them
  (`Level::ordinal_of`, `level.rs:51`). Its share at 10M with 1 job is 26.30% → 19.55%.
- **`PartSink::offer_encoded` and `offer_unseen_word`** are the same. The probe moved from `part.rs:124` to
  `:125`. The tail of the probe now shows as a line-less instruction of `offer_encoded` (section 5).
- **`PartSink::file_by_shard`** used to fill only `by_shard`. Now it also copies the rows, hashes and parents
  into shard order (`part.rs:214`–`255`, `move_row` at `:280`). `Part::into_ordinal_order` is new and does not
  show here. Together, `file_by_shard` and `move_row` are 2.47% at 10M and 4.51% at 100M with 1 job
  (`raw/prof_*_functions.txt`).
- **`Cond::eval` (6.88%) and `Num::eval` (5.11%)** at 10M with 1 job are now mostly `Code::scalar` (4.87%,
  new: an update written into the packed row without a `Value`) and `Cond::leaf` (3.34%, new: the inlined
  compare of a field with a constant), with `Num::int_at` inlined into them. `Cond::eval` keeps 1.03% and
  `Num::eval` 0.74% of the lines.
- **`Search::work`, `Rows::unpack`, `Store::read_state`, `write_level`, `reserve_rehash`** are the same
  functions in the same files. The odometer and the kept answers of guards and updates are new code in
  `expand.rs` (`turn`, `Known::forget_turned`, `known_or_scalar`, `Move::kept`), inlined into `Search::work`.
- **libc `memmove`** (2.97% before, the eighth function) is out of the ten. libc `memset` is ninth at 100M
  with 1 job (1.76%).

## 6. The checker's store work against the floor, and the evaluation next to it

As in the earlier write-up: the floor does in one table and one loop what groups (a), (b), (c) and (e) do in
the checker, and it does no rule evaluation, so group (d) has no counterpart. "Before" is the old write-up's
shares times this machine's median CPU time of `9c5413e` (table 3). "After" is this profile's shares times
the median CPU time of `10015fc`. The floor columns are the floor's `time_s` medians, all on one thread. The
ratios use medians. Made by `scripts/compare.py`, output in `raw/compare.md`.

| Run | (a)+(b)+(c)+(e) before | after | (a)+(b)+(c)+(e)+(f) after | after / ii | with (f) / ii | Floor i | Floor ii | Floor iii | (d) before | (d) after | Whole checker after |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 10M, 1 job | 2.15 | 1.86 | 2.13 | 1.19 | 1.36 | 2.79 | 1.56 | 2.08 | 0.81 | 0.65 | 2.88 |
| 10M, 4 jobs (CPU of all threads) | 3.20 | 2.62 | 2.94 | 1.68 | 1.88 | 2.79 | 1.56 | 2.08 | 0.90 | 0.72 | 3.78 |
| 100M, 1 job | 34.51 | 28.64 | 31.39 | 1.09 | 1.20 | 48.60 | 26.20 | 35.77 | 9.64 | 7.52 | 39.98 |
| 100M, 4 jobs (CPU of all threads) | 42.33 | 37.18 | 41.79 | 1.42 | 1.60 | 48.60 | 26.20 | 35.77 | 10.35 | 7.82 | 50.58 |

The floor's `time_s` includes its own kernel time: 0.19 s to 0.28 s at 10 million states and 3.0 s to 3.6 s at
100 million, mostly the faults of its 2 GiB table and its arrays. So the column with (f) is the fairer
comparison. The checker's system time is 0.21 s and 2.38 s with 1 job (table 3).

## 7. Findings

Each finding is marked **measured** (it follows from the numbers above) or **reading** (my reading of them,
which this machine cannot prove, mostly because it has no hardware counters).

### The groups before and after

1. **Measured.** CPU seconds by group. "Before" is the old write-up's share times this machine's median CPU
   time of `9c5413e`, which is how the brief allowed the column to be made. "After" is measured at `10015fc`.

   | Group | 10M, 1 job | 10M, 4 jobs | 100M, 1 job | 100M, 4 jobs |
   |---|---:|---:|---:|---:|
   | (a) index | 0.69 → 0.58 | 1.65 → 1.25 | 11.49 → 8.00 | 18.09 → 15.58 |
   | (b) packed rows | 0.35 → 0.31 | 0.36 → 0.35 | 5.55 → 5.91 | 5.60 → 6.41 |
   | (c) part tables, claim, merge | 0.99 → 0.86 | 1.06 → 0.90 | 16.16 → 13.55 | 17.01 → 13.94 |
   | (d) actions and rules | 0.81 → 0.65 | 0.90 → 0.72 | 9.64 → 7.52 | 10.35 → 7.82 |
   | (e) hashing | 0.12 → 0.10 | 0.13 → 0.12 | 1.31 → 1.18 | 1.62 → 1.25 |
   | (f) allocation, page faults | 0.31 → 0.27 | 0.38 → 0.32 | 4.90 → 2.75 | 5.43 → 4.61 |
   | (g) threads and the rest | 0.11 → 0.09 | 0.16 → 0.12 | 1.21 → 1.06 | 1.37 → 0.98 |
   | total | 3.40 → 2.88 | 4.64 → 3.78 | 50.26 → 39.98 | 59.49 → 50.58 |

   At 100M with 1 job, the 10.3 s saved is (a) 3.5 s, (c) 2.6 s, (f) 2.2 s and (d) 2.1 s. Group (b) is the only
   group that grew (+0.4 s and +0.8 s), and it is within the error of the "before" column, which multiplies
   shares taken on another CPU. **Reading:** (a) and (c) gained because the claim now reads each part in
   shard order instead of jumping through `by_shard` (the old `level.rs:43` was 6.5% at 10M), and (f) because
   half as many pages are faulted in (2.62 → 1.38 million). (d) gained from the odometer, the kept answers and
   `Code::scalar`.

### The whole checker against the floor

2. **Measured.** With 1 job, the whole checker takes 2.88 s of CPU at 10 million states, against 2.79 s (floor i,
   no prefetch), 1.56 s (ii, prefetch) and 2.08 s (iii, plain load). At 100 million it takes 39.98 s against
   48.60 s, 26.20 s and 35.77 s. That is 1.84 and 1.53 times the floor with a prefetch, and below the floor
   without one at 100 million (0.82 times).
3. **Measured.** With 4 jobs, the checker takes 1.04 s of wall time at 10 million states and 13.57 s at 100
   million. That is below every one-thread floor (1.56 s and 26.20 s for ii), for 3.78 s and 50.58 s of CPU
   (2.4 and 1.9 times floor ii). The 4-thread floor was not run (section 8).
4. **Measured.** The store work alone, (a)+(b)+(c)+(e) with 1 job, is 1.19 times floor ii at 10 million
   states (1.86 s against 1.56 s) and 1.09 times at 100 million (28.64 s against 26.20 s), and 1.36 and 1.20
   times with (f) added. Before, on this machine, it was 2.15 s and 34.51 s (1.37 and 1.32 times). With 4 jobs
   it is 1.68 and 1.42 times the one-thread floor (CPU of all threads).
5. **Reading.** So, with one worker, the checker's store is now within 10% to 20% of a hand-written table with
   a prefetch, which the machine allows. What is left between the whole checker and the floor is mostly (d),
   7.5 s at 100 million, which the floor does not do. With 4 workers, (a) grows by 7.6 s at 100 million (8.00 →
   15.58 s), with no futex time (3 samples of 53,080). The probes of the index get slower when four threads
   share the L3 and the memory, so that is the machine.

### Each piece above 3%: waiting for memory, work once per state or successor, or avoidable

The pieces are named from the 100M, 1-job profile. Seconds are at that run unless said.

6. **The index probe in `claim_level`: 20.0% (8.00 s), 30.8% with 4 jobs.** `level.rs:118` 11.49%,
   `index.rs:48` 4.77%, the rehash 1.51% plus 0.96% of page faults under it. **Reading: work once per
   candidate, and memory.** Every row that survives its part's table is looked up once in the index. That
   is the design's one look-up per new candidate, and no checker of this design does it fewer times. The
   rest of the share is waiting: the share per state grows from 10M to 100M and with 4 threads, while the
   instructions stay the same. Given from other sessions: a table for the index written by hand lost to
   `hashbrown` in six of eight cases, and making room once for a level's rows removes the resizes inside the
   claim but not the rehash. So the rehash, about 2.5% with its faults, is the only part I would call
   avoidable, and it is not avoided by reserving room.
7. **A part's table, `offer_unseen_word`: 12.41% at `part.rs:125` plus 7.01% for the tail of the probe =
   19.4% (7.77 s); 14.8% (0.43 s) at 10M.** **Reading: work once per successor, and memory.** Every successor
   of a part is offered once, and the table is what keeps repeats away from the index. Given from other
   sessions: a probe costs about 5 ns for each offer, about 3.5 ns of it one branch the processor cannot
   predict (found or not, which the data decides), and three exact tables written by hand were slower than
   `hashbrown`. The share per state nearly doubles from 10 to 100 million states (43 ns to 78 ns of this
   piece per state). The part's table grows with the level, and the tail of the probe (the load of the
   compare) is the part that grows: 2.7% → 7.0%. So the growth is the machine waiting for memory. The
   per-offer part is inherent to the design.
8. **Reading the frontier, `Rows::unpack` through `word_of`: 8.86% at `rows.rs:17` (3.54 s), 3.77% (0.11 s) at
   10M.** Called only from `Store::read_state` in `Part::expand_ids` (DWARF chains, section 8), once per
   state, in id order. Almost every sample lands on the instruction after `call memcpy`: the 4-byte row is
   copied by a libc call into a stack word and read back as 8 bytes. **Measured (scratch probe, section
   8):** reading the 4 bytes with one load instead (`raw/probe_word_of.diff`) takes `Rows::unpack` from
   9.58% to 2.14% of the samples. But `Search::work` rises from 23.53% to 30.28%, and the wall time changes by
   -2.3% at 10M and -6.4% at 100M (medians, 5 and 3 rounds), within this machine's spread: one 100M round
   is a tie, 38.18 s against 38.19 s. **Reading: memory.** The samples moved to the first use of the row.
   Removing the call did not remove the wait. The cost per state is 11 ns at 10M and 35 ns at 100M, so it
   depends on size, not on the code. Once per state, any checker must read the state it expands. Why an
   in-order read waits this long I cannot tell without counters.
9. **`explore/part.rs` lines of its own: 9.23% (3.69 s).** `file_by_shard` with `move_row` is 4.51% (1.80 s),
   and `add_fresh`, `expand_ids` and `offer_encoded` are the rest. **Reading: work once per fresh row, and
   partly avoidable.** Copying the fresh rows, hashes and parents into shard order is new work at `10015fc`.
   It replaced the claim's jumping reads, and `store/level.rs`'s own lines fell from 4.42 s to 1.93 s (8.80% →
   4.84%), so it paid for itself. It is still a second pass over every fresh row. Writing each fresh row into
   its shard's run when it is found would save most of the 1.8 s, minus the cost of 64 open runs. That is up
   to about 4% at 100M and 2.5% at 10M.
10. **Evaluating actions and rules, (d): 18.8% (7.52 s); 22.7% (0.65 s) at 10M; 76% of `wide`.** `earnest_ir`
    11.07% (`Code::scalar`, `Cond::leaf`, `Num::int_at`, `eval.rs`), `expand.rs` 7.73%. **Reading: work once
    per try and per successor.** It is compute on values already read. It fell by 22% (9.64 → 7.52 s) and
    `wide` by 42% in wall time. In `wide`, `Seen::first_row`, the table that keeps one state's successors
    apart (1,000 tries, at most 190 distinct rows), is 18.7% (16.5% of it hashbrown). `eval.rs`
    `arithmetic` (the `m.rem`) is 9.1%, and `Num::rest` is 30% as a function. Some of that is avoidable for
    a model like `wide`: the update of `y` reads only `b + c`, which has 19 values for 100 combinations, so the
    same row is built and offered again and again. The odometer keeps answers, but `Seen` still meets every
    repeat. This is my reading of the shares, not a measurement of a change.
11. **Kernel page faults and allocation, (f): 6.9% (2.75 s) with 1 job, 9.1% (4.61 s) with 4.** **Measured:**
    1.38 million minor faults with 1 job (5.3 GiB for a 1.7 GiB peak, 3.2 times) and 2.21 million with 4 (5.4
    times). The DWARF chains put the faults under `claim_level` (the index growing), `write_level` and libc.
    **Reading: the machine, plus some avoidable re-faulting with 4 jobs.** The cost per page is the kernel's,
    and the floor pays 3.0 to 3.6 s of system time for its own 4.1 GiB once. With 1 job the checker is now
    below that (2.38 s). With 4 jobs, memory is given back and taken anew level after level: 0.8 million more
    faults and 1.9 s more of (f) than with 1 job. Keeping a thread's buffers across levels could reach that
    1.9 s, about 3.7% of the 4-job CPU.
12. **Hashing, (e): 3.0% (1.18 s).** **Reading: work once per successor.** It is `mix` over one word for each
    successor, and it is needed for both probes.
13. **`store/level.rs` lines of its own: 4.84% (1.93 s).** `ordinal_of`, `Winners::rank` and the copy in
    `write_level`. **Reading: work once per new state.** Every winner is ranked and written once into the
    store.

### What I would look at next

14. **Reading.** With 1 worker, the store is within 1.09 to 1.19 times floor ii, and its two largest pieces
    (the two probes, 39% of the time together) are work the design needs once per candidate or per
    successor, plus memory. Hand-made tables already lost to `hashbrown`. So the machine is close to being
    the limit there. What the program still does that it need not, in order of the share it could reach:
    - re-faulting freed memory with 4 jobs: up to about 1.9 s of 50.6 s (3.7%) at 100M;
    - the second pass of `file_by_shard`/`move_row`: up to 1.8 s of 40.0 s (4.5%) at 100M with 1 job, less once
      the cost of filing rows into 64 runs at once is paid;
    - for models whose inputs collapse to few rows (`wide`): building and checking rows already met in
      `Seen::first_row`, a share of up to about 19% of such a model;
    - the rehash of the index shards: about 2.5% with its faults, which room made once does not remove.

    The frontier read (8.9%) is not on this list: taking out its `memcpy` moved the samples instead of
    removing them (finding 8). A prefetch some states ahead of `read_state` would be the experiment to try.
    The floor's prefetch saves 46% of its own time, but for an in-order read I would not expect much.

## 8. What failed, was skipped or was added

- **Group 1's 4-job runs** were slowed by something outside this guest, and group 1b replaces them
  (section 1). Both logs are kept.
- **Hardware counters** are refused on this guest. "Waiting for memory" is my reading of lines and of how a
  share grows with size and threads, not a count of misses.
- **The "before" column of the groups** multiplies the old write-up's shares (taken on another CPU) by this
  machine's time of `9c5413e`. `9c5413e` was not profiled here. `wide` was not in the old write-up, so it
  has no "before" by group, only its times in table 3.
- **Rounds:** 7 for the small models, 3 for `grid_100m` (the old write-up had 5), 5 for the floor at each size,
  one uncounted run per timing case and one before the floor block, and none before the profiles.
- **The 4-thread floor** (`floor 7 4`) was not run, as before.
- **Added: a DWARF call-chain recording** of `grid_100m` with 1 job (`perf record --call-graph dwarf,8192
  -F 97`, 4,136 samples, `raw/prof_dwarf_100m_j1_*`), to find the callers of `Rows::unpack`, `memset` and
  `memmove` and of the page faults. It found that `unpack` is called only by `Store::read_state` in
  `expand_ids`, and that 1.47% of the 1.57% of `memset`/`memmove` is called from inlined code under
  `explore::explore`, which this recording could not split further.
- **Added: a scratch probe** of `word_of` (`raw/probe_word_of.diff`, three lines in a worktree of `10015fc`
  outside the repository, never committed). It was timed in turn with `10015fc` (`raw/g5_probe_word_of.log`)
  and profiled without debug lines (`raw/prof_probe_100m_j1_report.txt`).
- **The disassembly behind the placement of section 5:** at `0x1f792c` there is `cmp %rbx,-0x8(%rbp)` and then
  `je 1f79e1`. `0x1f79e1` (`xor %eax,%eax`, return false) holds 2,764 of the 2,781 line-less samples at 100M
  with 1 job. The only other jump to it is after `call Store::find` (the graph path).
- **Line attribution and skid:** a sample of the software clock lands on the instruction after the one that
  waited. That is how the probe's tail shows in `offer_encoded`, and how `word_of`'s line shows after the
  `memcpy` call.
- `perf.data` files and the `perf script` dumps are not kept (up to 40 MB). The reports, groups, line counts and
  function counts made from them are in `raw/`.

## 9. Commands

```
export RUSTUP_TOOLCHAIN=1.100.0
git worktree add /SCRATCH/src/old 9c5413e; git worktree add /SCRATCH/src/new 10015fc
bash scripts/build.sh            # earnest-old, earnest-new, earnest-sym, one after another
rustc +1.100.0 -O -o floor_i floor_i.rs; rustc +1.100.0 -O -o floor_ii floor_ii.rs; rustc +1.100.0 -O -o floor_iii floor_iii.rs
# models: grid_10m, wide from tasks/benchmark/models (10015fc); lights, grid_100m from measure/tools; one folder
bash scripts/chain.sh            # groups 1 to 9 in one chain, below
#   time_runs.py old=earnest-old,new=earnest-new 7 grid_10m:1,grid_10m:4,lights:1,lights:4,wide:1,wide:4  > raw/g1_10m_lights_wide.log
#   time_runs.py old=earnest-old,new=earnest-new 3 grid_100m:1,grid_100m:4                                  > raw/g2_100m.log
#   floor_block.sh                                                                                         > raw/g3_floor.log
#   time_runs.py new=earnest-new,sym=earnest-sym 5 grid_10m:1,grid_10m:4,wide:1                             > raw/g4_sym_check.log
#   profile.sh 10m_j1 grid_10m 1 5; profile.sh 10m_j4 grid_10m 4 5; profile.sh 100m_j1 grid_100m 1 1
#   profile.sh 100m_j4 grid_100m 4 1; profile.sh wide_j1 wide 1 20
time_runs.py old=earnest-old,new=earnest-new 7 grid_10m:4,lights:4,wide:4,grid_10m:1                         # raw/g1b_rerun_j4.log
python3 tables.py > raw/tables.md; python3 compare.py /SCRATCH measure/profile-lone/raw > raw/compare.md
python3 lines.py X.script earnest-sym 40 > raw/prof_X_lines.txt
python3 lines_by_earnest.py X.script earnest-sym 25 | rustfilt > raw/prof_X_elines.txt
python3 by_function.py X.script earnest-sym 30 | rustfilt > raw/prof_X_functions.txt
perf record -e cpu-clock -F 97 --call-graph dwarf,8192 -o dwarf_100m_j1.data -- earnest-sym verify ... grid_100m.model.ts
python3 callers.py dwarf.script 'Rows.*unpack' ...                                                          # raw/prof_dwarf_100m_j1_callers.txt
# probe: worktree of 10015fc + raw/probe_word_of.diff, built into its own target dir
time_runs.py new=earnest-new,probe=earnest-probe 5 grid_10m:1; ... 3 grid_100m:1                             # raw/g5_probe_word_of.log
perf record -e cpu-clock -F 999 -o probe_100m_j1.data -- earnest-probe verify ... grid_100m.model.ts          # raw/prof_probe_100m_j1_*
```

Every group printed `scripts/machine.sh` before and after: `nproc`, the processor model, `free -m` and
`/proc/loadavg`. `attr.py`, `lines.py`, `floor_*.rs`, `floor_block.sh`, `machine.sh` and `time_runs.py` are
the earlier write-up's, with only the scratch path changed. `profile.sh` changes only the paths of
`machine.sh` and `attr.py`, and `tables.py` only its table of CPU times. `compare.py`, `lines_by_earnest.py`,
`by_function.py`, `callers.py`, `chain.sh` and `build.sh` are new.

## 10. Raw outputs (trimmed)

- `raw/machine.txt`: `lscpu`, `free -m`, the kernel, the toolchain, `perf --version`, `perf_event_paranoid`,
  THP, and the sha256 of every binary.
- `raw/g1_10m_lights_wide.log`, `raw/g1b_rerun_j4.log`, `raw/g2_100m.log`, `raw/g4_sym_check.log`,
  `raw/g5_probe_word_of.log`: one JSON line for every run (wall, user, sys, peak memory, exit code, status,
  states, `milliseconds`, context switches, minor faults), the summaries, and the machine lines.
- `raw/g3_floor.log`: the whole output of every floor run, with GNU `time`.
- `raw/prof_<run>.log`: the `perf record` lines and GNU `time` of every profiled run.
  `raw/prof_<run>_report.txt`: the flat symbol report down to 0.3%. `raw/prof_<run>_dso.txt`: the split by
  dso. `raw/prof_<run>_groups.json`: groups, subgroups, the sites of kernel samples and the 40 heaviest
  symbols. `raw/prof_<run>_lines.txt`: samples by innermost line. `raw/prof_<run>_elines.txt`: samples by
  first earnest line, and the samples without one. `raw/prof_<run>_functions.txt`: samples by earnest
  function.
- `raw/tables.md`, `raw/compare.md`: the tables of sections 5 and 6 as the scripts printed them.
- `raw/prof_dwarf_100m_j1.log`, `raw/prof_dwarf_100m_j1_report.txt`, `raw/prof_dwarf_100m_j1_callers.txt`: the
  DWARF recording.
- `raw/probe_word_of.diff`, `raw/prof_probe_100m_j1.log`, `raw/prof_probe_100m_j1_report.txt`: the scratch
  probe.
- `raw/perf_stat_hw_check.txt`: the refused hardware counters.

Trimmed excerpts. `grid_100m` summaries (`raw/g2_100m.log`):

```
{"event": "summary", "build": "old", "model": "grid_100m", "jobs": 1, "n": 3, "wall": [50.353, 48.549, 52.785], "cpu": [50.26, 48.274, 52.513], ... "mib": [1621.1, ...], "statuses": [["pass", 100000000, 0]]}
{"event": "summary", "build": "new", "model": "grid_100m", "jobs": 1, "n": 3, "wall": [40.188, 39.954, 41.227], "cpu": [39.975, 39.74, 41.034], ... "mib": [1726.7, ...], "statuses": [["pass", 100000000, 0]]}
{"event": "summary", "build": "old", "model": "grid_100m", "jobs": 4, "n": 3, "wall": [15.978, 15.357, 18.397], "cpu": [59.486, 57.039, 68.002], ... "statuses": [["pass", 100000000, 0]]}
{"event": "summary", "build": "new", "model": "grid_100m", "jobs": 4, "n": 3, "wall": [13.57, 13.451, 14.437], "cpu": [50.583, 50.186, 53.453], ... "statuses": [["pass", 100000000, 0]]}
```

The probe's effect on the flat profile, 100M with 1 job (`raw/prof_100m_j1_report.txt` against
`raw/prof_probe_100m_j1_report.txt`):

```
                       10015fc   probe
Search::work            23.53%   30.28%
PartSink::offer_encoded 22.42%   23.97%
gather (claim_level)    19.36%   18.69%
Rows::unpack             9.58%    2.14%
Store::read_state        2.83%    3.14%
```
