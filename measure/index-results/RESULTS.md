# Index benchmark and corridor timings on a Linux cloud machine

## Machine

| | |
|---|---|
| nproc | 4 |
| processor (`/proc/cpuinfo` model name) | Intel(R) Xeon(R) Processor @ 2.10GHz |
| memory | MemTotal 16,473,744 kB (15.7 GiB), no swap |
| kernel | Linux 6.18.44 (a cloud container) |
| toolchain | `rustup toolchain install 1.100.0 --profile minimal`, rustc 1.100.0 (0ef1bc386 2026-11-09); everything built with `RUSTUP_TOOLCHAIN=1.100.0` |
| arch reported by the bench | x86_64; the prefetch is `_mm_prefetch(_MM_HINT_T0)` |

One thing ran at a time: each build before any timing, then the runs one after another. Load averages are below each group; a raised 1-minute load at the start of a group is the previous group's own run decaying, not another process.

## Part 1: the index benchmark (`measure/index-bench` at 12d977d)

Commands, in `measure/index`:

```
RUSTUP_TOOLCHAIN=1.100.0 cargo build --release --locked --features prefetch
./target/release/index-bench --states 10m --rounds 5 --batch 8,16,32
./target/release/index-bench --states 100m --rounds 3 --batch 16
```

Both runs ended with `CHECK OK`. Times are medians of `claim_s` in seconds. `own` is the own table, `load` the own table with a plain load ahead, `prefetch` the own table with a prefetch; the number after the slash is the batch. The last two columns pick the fastest median among the own table's variants and give it as a ratio of hashbrown's median (below 1 means faster than hashbrown).

### 10m states, 5 rounds, batches 8, 16 and 32

| words | threads | hashbrown | own/8 | load/8 | prefetch/8 | own/16 | load/16 | prefetch/16 | own/32 | load/32 | prefetch/32 | own table's best | best ÷ hashbrown |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 1 | 1 | 1.1467 | 1.2911 | 1.2572 | 1.1611 | 1.1894 | 1.2684 | 1.1701 | 1.1845 | 1.1584 | 1.1715 | load/32 | 1.010 |
| 1 | 4 | 0.4198 | 0.3499 | 0.4219 | 0.4056 | 0.3905 | 0.3909 | 0.3508 | 0.3299 | 0.3668 | 0.3243 | prefetch/32 | 0.773 |
| 2 | 1 | 3.1879 | 3.4341 | 3.9770 | 3.7384 | 3.6762 | 3.9920 | 3.6630 | 3.4221 | 3.8349 | 3.5318 | own/32 | 1.073 |
| 2 | 4 | 1.1023 | 1.1589 | 1.3019 | 1.2933 | 1.1817 | 1.3019 | 1.2347 | 1.2116 | 1.2459 | 1.2490 | own/8 | 1.051 |

Index bytes per state: words=1 threads=1: hashbrown 8.39, own 6.71; words=1 threads=4: hashbrown 8.39, own 6.71; words=2 threads=1: hashbrown 8.39, own 13.42; words=2 threads=4: hashbrown 8.39, own 13.42.

### 100m states, 3 rounds, batch 16

| words | threads | hashbrown | own/16 | load/16 | prefetch/16 | own table's best | best ÷ hashbrown |
|---|---|---:|---:|---:|---:|---:|---:|
| 1 | 1 | 14.8724 | 16.2092 | 17.1261 | 15.6527 | prefetch/16 | 1.052 |
| 1 | 4 | 5.0497 | 4.8607 | 5.2155 | 4.7568 | prefetch/16 | 0.942 |
| 2 | 1 | 44.9085 | 48.8117 | 47.6827 | 46.4063 | prefetch/16 | 1.033 |
| 2 | 4 | 11.5475 | 13.2469 | 13.6565 | 13.0545 | prefetch/16 | 1.131 |

Index bytes per state: words=1 threads=1: hashbrown 6.71, own 5.37; words=1 threads=4: hashbrown 6.71, own 5.37; words=2 threads=1: hashbrown 6.71, own 10.74; words=2 threads=4: hashbrown 6.71, own 10.74.

In short: the own table beats hashbrown only for rows of one word at 4 threads (best 0.773 at 10m with prefetch/32, 0.942 at 100m with prefetch/16). At 1 thread with rows of one word it is about even to 5% slower. For rows of two words it is 3–13% slower everywhere, and its index takes 10.74–13.42 bytes per state where hashbrown's takes 6.71–8.39. The prefetch is the own table's best variant in five of the eight rows.

### Load and wall time

10m:
```
loadavg before: 0.49 1.13 0.64 3/115 13043
Mem:           16087         560       12861          12        2947       15526
exit 0 wall_s 484.023598010
loadavg after: 2.74 2.09 1.29 1/114 6012
```
100m:
```
loadavg before: 2.74 2.09 1.29 1/114 6015
Mem:           16087         659       12760          12        2948       15427
exit 0 wall_s 1455.932719997
loadavg after: 3.15 2.45 1.89 1/119 31347
```

A first 10m run was stopped and thrown away: a hook of this session ran `cargo check` of the workspace (`just lint`) while it ran. The run above was started again once the load had fallen below 0.6, and nothing else ran during it. The thrown-away output is not included.

## Part 2: corridor, many narrow levels

Builds, each in its own worktree, one after another:

```
RUSTUP_TOOLCHAIN=1.100.0 cargo build --release --locked -p earnest-cli   # in 2d61e74, 0ffa4d5, 4c25766 (tip of perf/joined)
```

Build times: 2d61e74 exit 0 build_s 58; 0ffa4d5 exit 0 build_s 63; 4c25766 exit 0 build_s 62.

All three binaries accept the command line of `turn.py` (`verify --no-cache --json --jobs N --max-states 200000000`), so no build needed another one. The model `corridor.model.ts` and `turn.py` come from `measure/tools` at d6894fa and were copied into one run folder:

```
python3 -I turn.py b2d61e74=…/2d61e74/target/release/earnest,b0ffa4d5=…/0ffa4d5/target/release/earnest,b4c25766=…/4c25766/target/release/earnest 7 corridor:1,corridor:2,corridor:4
```

`turn.py` does one warm-up run with the first build, then 7 rounds of the three builds in turn. Before it, one check run of each build at 4 jobs showed that all three report status `fail` with 11,820,300 states (exit status 1, which `turn.py` prints as 256), as expected. No run took more than 5 minutes (the longest about 33 s), so every build got all 7 rounds.

Medians of wall time in seconds, with median CPU time and median peak memory:

| jobs | 2d61e74 | 0ffa4d5 | 4c25766 | 4c25766 ÷ 2d61e74 | 4c25766 ÷ 0ffa4d5 |
|---:|---:|---:|---:|---:|---:|
| 1 | 32.535 (cpu 32.2, 1965.6 MiB) | 5.546 (cpu 5.53, 212.8 MiB) | 5.461 (cpu 5.4, 212.7 MiB) | 0.168 | 0.985 |
| 2 | 21.632 (cpu 32.37, 1966.4 MiB) | 12.880 (cpu 19.83, 221.9 MiB) | 8.138 (cpu 10.19, 213.2 MiB) | 0.376 | 0.632 |
| 4 | 20.502 (cpu 41.81, 1965.2 MiB) | 20.143 (cpu 43.09, 229.2 MiB) | 11.552 (cpu 21.06, 213.8 MiB) | 0.563 | 0.573 |

Every run of every build gave exit status 256 (the expected `fail`). In short: the tip is the fastest at every worker count. 0ffa4d5 already matches it at 1 job, but it slows down as workers are added (5.5 s at 1 job, 12.9 s at 2, 20.1 s at 4). The tip also slows down with more workers on this model (5.5, 8.1 and 11.6 s). 2d61e74 takes about 1.9 GiB, where the two later builds take about 215 MiB.

Load and wall time of the whole run:
```
loadavg before: 2.76 2.74 2.16 1/110 25956
Mem:           16087         698       11689          12        4014       15389
exit 0 wall_s 1034
loadavg after: 2.67 2.33 2.02 1/119 18276
```

## Raw outputs

### Part 1, 10m

```
wheels=7 states=10000000 rounds=5 prefetch_feature=true arch=x86_64 expansion_helpers=4
words=1 threads=1 round=0 variant=hashbrown claim_s=1.1735 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=83887104 digest=2590c032de643c86
words=1 threads=1 round=0 variant=own/8 claim_s=1.3728 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=67108864 digest=2590c032de643c86
words=1 threads=1 round=0 variant=load/8 claim_s=1.3344 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=67108864 digest=2590c032de643c86
words=1 threads=1 round=0 variant=prefetch/8 claim_s=1.2224 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=67108864 digest=2590c032de643c86
words=1 threads=1 round=0 variant=own/16 claim_s=1.1721 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=67108864 digest=2590c032de643c86
words=1 threads=1 round=0 variant=load/16 claim_s=1.3342 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=67108864 digest=2590c032de643c86
words=1 threads=1 round=0 variant=prefetch/16 claim_s=1.1892 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=67108864 digest=2590c032de643c86
words=1 threads=1 round=0 variant=own/32 claim_s=1.2325 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=67108864 digest=2590c032de643c86
words=1 threads=1 round=0 variant=load/32 claim_s=1.1755 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=67108864 digest=2590c032de643c86
words=1 threads=1 round=0 variant=prefetch/32 claim_s=1.1980 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=67108864 digest=2590c032de643c86
words=1 threads=1 round=1 variant=hashbrown claim_s=1.1610 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=83887104 digest=2590c032de643c86
words=1 threads=1 round=1 variant=own/8 claim_s=1.2952 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=67108864 digest=2590c032de643c86
words=1 threads=1 round=1 variant=load/8 claim_s=1.2551 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=67108864 digest=2590c032de643c86
words=1 threads=1 round=1 variant=prefetch/8 claim_s=1.1611 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=67108864 digest=2590c032de643c86
words=1 threads=1 round=1 variant=own/16 claim_s=1.2517 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=67108864 digest=2590c032de643c86
words=1 threads=1 round=1 variant=load/16 claim_s=1.2684 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=67108864 digest=2590c032de643c86
words=1 threads=1 round=1 variant=prefetch/16 claim_s=1.1701 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=67108864 digest=2590c032de643c86
words=1 threads=1 round=1 variant=own/32 claim_s=1.1797 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=67108864 digest=2590c032de643c86
words=1 threads=1 round=1 variant=load/32 claim_s=1.1537 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=67108864 digest=2590c032de643c86
words=1 threads=1 round=1 variant=prefetch/32 claim_s=1.0491 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=67108864 digest=2590c032de643c86
words=1 threads=1 round=2 variant=hashbrown claim_s=1.1467 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=83887104 digest=2590c032de643c86
words=1 threads=1 round=2 variant=own/8 claim_s=1.2032 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=67108864 digest=2590c032de643c86
words=1 threads=1 round=2 variant=load/8 claim_s=1.2200 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=67108864 digest=2590c032de643c86
words=1 threads=1 round=2 variant=prefetch/8 claim_s=1.1605 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=67108864 digest=2590c032de643c86
words=1 threads=1 round=2 variant=own/16 claim_s=1.2032 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=67108864 digest=2590c032de643c86
words=1 threads=1 round=2 variant=load/16 claim_s=1.2239 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=67108864 digest=2590c032de643c86
words=1 threads=1 round=2 variant=prefetch/16 claim_s=1.1009 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=67108864 digest=2590c032de643c86
words=1 threads=1 round=2 variant=own/32 claim_s=1.1845 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=67108864 digest=2590c032de643c86
words=1 threads=1 round=2 variant=load/32 claim_s=1.1448 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=67108864 digest=2590c032de643c86
words=1 threads=1 round=2 variant=prefetch/32 claim_s=1.0397 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=67108864 digest=2590c032de643c86
words=1 threads=1 round=3 variant=hashbrown claim_s=1.0865 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=83887104 digest=2590c032de643c86
words=1 threads=1 round=3 variant=own/8 claim_s=1.2911 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=67108864 digest=2590c032de643c86
words=1 threads=1 round=3 variant=load/8 claim_s=1.3449 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=67108864 digest=2590c032de643c86
words=1 threads=1 round=3 variant=prefetch/8 claim_s=1.1202 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=67108864 digest=2590c032de643c86
words=1 threads=1 round=3 variant=own/16 claim_s=1.1284 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=67108864 digest=2590c032de643c86
words=1 threads=1 round=3 variant=load/16 claim_s=1.1670 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=67108864 digest=2590c032de643c86
words=1 threads=1 round=3 variant=prefetch/16 claim_s=1.1407 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=67108864 digest=2590c032de643c86
words=1 threads=1 round=3 variant=own/32 claim_s=1.1683 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=67108864 digest=2590c032de643c86
words=1 threads=1 round=3 variant=load/32 claim_s=1.1584 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=67108864 digest=2590c032de643c86
words=1 threads=1 round=3 variant=prefetch/32 claim_s=1.2638 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=67108864 digest=2590c032de643c86
words=1 threads=1 round=4 variant=hashbrown claim_s=1.0438 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=83887104 digest=2590c032de643c86
words=1 threads=1 round=4 variant=own/8 claim_s=1.2379 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=67108864 digest=2590c032de643c86
words=1 threads=1 round=4 variant=load/8 claim_s=1.2572 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=67108864 digest=2590c032de643c86
words=1 threads=1 round=4 variant=prefetch/8 claim_s=1.2285 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=67108864 digest=2590c032de643c86
words=1 threads=1 round=4 variant=own/16 claim_s=1.1894 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=67108864 digest=2590c032de643c86
words=1 threads=1 round=4 variant=load/16 claim_s=1.2701 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=67108864 digest=2590c032de643c86
words=1 threads=1 round=4 variant=prefetch/16 claim_s=1.2327 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=67108864 digest=2590c032de643c86
words=1 threads=1 round=4 variant=own/32 claim_s=1.2860 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=67108864 digest=2590c032de643c86
words=1 threads=1 round=4 variant=load/32 claim_s=1.2014 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=67108864 digest=2590c032de643c86
words=1 threads=1 round=4 variant=prefetch/32 claim_s=1.1715 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=67108864 digest=2590c032de643c86
SUMMARY words=1 threads=1 new=10000000 lookups=26138826 found=16138826 levels=65
  hashbrown      min_s=1.0438 median_s=1.1467 vs_hashbrown=+0.0% index_bytes_per_state=8.39
  own/8          min_s=1.2032 median_s=1.2911 vs_hashbrown=+12.6% index_bytes_per_state=6.71
  load/8         min_s=1.2200 median_s=1.2572 vs_hashbrown=+9.6% index_bytes_per_state=6.71
  prefetch/8     min_s=1.1202 median_s=1.1611 vs_hashbrown=+1.3% index_bytes_per_state=6.71
  own/16         min_s=1.1284 median_s=1.1894 vs_hashbrown=+3.7% index_bytes_per_state=6.71
  load/16        min_s=1.1670 median_s=1.2684 vs_hashbrown=+10.6% index_bytes_per_state=6.71
  prefetch/16    min_s=1.1009 median_s=1.1701 vs_hashbrown=+2.0% index_bytes_per_state=6.71
  own/32         min_s=1.1683 median_s=1.1845 vs_hashbrown=+3.3% index_bytes_per_state=6.71
  load/32        min_s=1.1448 median_s=1.1584 vs_hashbrown=+1.0% index_bytes_per_state=6.71
  prefetch/32    min_s=1.0397 median_s=1.1715 vs_hashbrown=+2.2% index_bytes_per_state=6.71
words=1 threads=4 round=0 variant=hashbrown claim_s=0.4198 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=83887104 digest=2590c032de643c86
words=1 threads=4 round=0 variant=own/8 claim_s=0.4401 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=67108864 digest=2590c032de643c86
words=1 threads=4 round=0 variant=load/8 claim_s=0.4315 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=67108864 digest=2590c032de643c86
words=1 threads=4 round=0 variant=prefetch/8 claim_s=0.4425 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=67108864 digest=2590c032de643c86
words=1 threads=4 round=0 variant=own/16 claim_s=0.4370 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=67108864 digest=2590c032de643c86
words=1 threads=4 round=0 variant=load/16 claim_s=0.4077 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=67108864 digest=2590c032de643c86
words=1 threads=4 round=0 variant=prefetch/16 claim_s=0.3514 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=67108864 digest=2590c032de643c86
words=1 threads=4 round=0 variant=own/32 claim_s=0.3618 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=67108864 digest=2590c032de643c86
words=1 threads=4 round=0 variant=load/32 claim_s=0.3551 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=67108864 digest=2590c032de643c86
words=1 threads=4 round=0 variant=prefetch/32 claim_s=0.3675 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=67108864 digest=2590c032de643c86
words=1 threads=4 round=1 variant=hashbrown claim_s=0.4580 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=83887104 digest=2590c032de643c86
words=1 threads=4 round=1 variant=own/8 claim_s=0.4313 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=67108864 digest=2590c032de643c86
words=1 threads=4 round=1 variant=load/8 claim_s=0.4324 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=67108864 digest=2590c032de643c86
words=1 threads=4 round=1 variant=prefetch/8 claim_s=0.4338 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=67108864 digest=2590c032de643c86
words=1 threads=4 round=1 variant=own/16 claim_s=0.3905 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=67108864 digest=2590c032de643c86
words=1 threads=4 round=1 variant=load/16 claim_s=0.4056 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=67108864 digest=2590c032de643c86
words=1 threads=4 round=1 variant=prefetch/16 claim_s=0.3508 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=67108864 digest=2590c032de643c86
words=1 threads=4 round=1 variant=own/32 claim_s=0.3234 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=67108864 digest=2590c032de643c86
words=1 threads=4 round=1 variant=load/32 claim_s=0.3540 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=67108864 digest=2590c032de643c86
words=1 threads=4 round=1 variant=prefetch/32 claim_s=0.2866 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=67108864 digest=2590c032de643c86
words=1 threads=4 round=2 variant=hashbrown claim_s=0.4083 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=83887104 digest=2590c032de643c86
words=1 threads=4 round=2 variant=own/8 claim_s=0.3423 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=67108864 digest=2590c032de643c86
words=1 threads=4 round=2 variant=load/8 claim_s=0.3991 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=67108864 digest=2590c032de643c86
words=1 threads=4 round=2 variant=prefetch/8 claim_s=0.3615 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=67108864 digest=2590c032de643c86
words=1 threads=4 round=2 variant=own/16 claim_s=0.3499 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=67108864 digest=2590c032de643c86
words=1 threads=4 round=2 variant=load/16 claim_s=0.3606 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=67108864 digest=2590c032de643c86
words=1 threads=4 round=2 variant=prefetch/16 claim_s=0.3343 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=67108864 digest=2590c032de643c86
words=1 threads=4 round=2 variant=own/32 claim_s=0.3299 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=67108864 digest=2590c032de643c86
words=1 threads=4 round=2 variant=load/32 claim_s=0.3785 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=67108864 digest=2590c032de643c86
words=1 threads=4 round=2 variant=prefetch/32 claim_s=0.3243 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=67108864 digest=2590c032de643c86
words=1 threads=4 round=3 variant=hashbrown claim_s=0.3836 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=83887104 digest=2590c032de643c86
words=1 threads=4 round=3 variant=own/8 claim_s=0.3499 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=67108864 digest=2590c032de643c86
words=1 threads=4 round=3 variant=load/8 claim_s=0.4219 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=67108864 digest=2590c032de643c86
words=1 threads=4 round=3 variant=prefetch/8 claim_s=0.3328 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=67108864 digest=2590c032de643c86
words=1 threads=4 round=3 variant=own/16 claim_s=0.3552 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=67108864 digest=2590c032de643c86
words=1 threads=4 round=3 variant=load/16 claim_s=0.3548 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=67108864 digest=2590c032de643c86
words=1 threads=4 round=3 variant=prefetch/16 claim_s=0.3260 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=67108864 digest=2590c032de643c86
words=1 threads=4 round=3 variant=own/32 claim_s=0.3276 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=67108864 digest=2590c032de643c86
words=1 threads=4 round=3 variant=load/32 claim_s=0.3668 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=67108864 digest=2590c032de643c86
words=1 threads=4 round=3 variant=prefetch/32 claim_s=0.3767 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=67108864 digest=2590c032de643c86
words=1 threads=4 round=4 variant=hashbrown claim_s=0.4389 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=83887104 digest=2590c032de643c86
words=1 threads=4 round=4 variant=own/8 claim_s=0.3426 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=67108864 digest=2590c032de643c86
words=1 threads=4 round=4 variant=load/8 claim_s=0.4017 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=67108864 digest=2590c032de643c86
words=1 threads=4 round=4 variant=prefetch/8 claim_s=0.4056 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=67108864 digest=2590c032de643c86
words=1 threads=4 round=4 variant=own/16 claim_s=0.4019 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=67108864 digest=2590c032de643c86
words=1 threads=4 round=4 variant=load/16 claim_s=0.3909 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=67108864 digest=2590c032de643c86
words=1 threads=4 round=4 variant=prefetch/16 claim_s=0.3894 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=67108864 digest=2590c032de643c86
words=1 threads=4 round=4 variant=own/32 claim_s=0.3779 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=67108864 digest=2590c032de643c86
words=1 threads=4 round=4 variant=load/32 claim_s=0.3773 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=67108864 digest=2590c032de643c86
words=1 threads=4 round=4 variant=prefetch/32 claim_s=0.3242 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=67108864 digest=2590c032de643c86
SUMMARY words=1 threads=4 new=10000000 lookups=26138826 found=16138826 levels=65
  hashbrown      min_s=0.3836 median_s=0.4198 vs_hashbrown=+0.0% index_bytes_per_state=8.39
  own/8          min_s=0.3423 median_s=0.3499 vs_hashbrown=-16.6% index_bytes_per_state=6.71
  load/8         min_s=0.3991 median_s=0.4219 vs_hashbrown=+0.5% index_bytes_per_state=6.71
  prefetch/8     min_s=0.3328 median_s=0.4056 vs_hashbrown=-3.4% index_bytes_per_state=6.71
  own/16         min_s=0.3499 median_s=0.3905 vs_hashbrown=-7.0% index_bytes_per_state=6.71
  load/16        min_s=0.3548 median_s=0.3909 vs_hashbrown=-6.9% index_bytes_per_state=6.71
  prefetch/16    min_s=0.3260 median_s=0.3508 vs_hashbrown=-16.4% index_bytes_per_state=6.71
  own/32         min_s=0.3234 median_s=0.3299 vs_hashbrown=-21.4% index_bytes_per_state=6.71
  load/32        min_s=0.3540 median_s=0.3668 vs_hashbrown=-12.6% index_bytes_per_state=6.71
  prefetch/32    min_s=0.2866 median_s=0.3243 vs_hashbrown=-22.8% index_bytes_per_state=6.71
words=2 threads=1 round=0 variant=hashbrown claim_s=3.4978 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=83887104 digest=056d833823c71e24
words=2 threads=1 round=0 variant=own/8 claim_s=3.5929 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=134217728 digest=056d833823c71e24
words=2 threads=1 round=0 variant=load/8 claim_s=3.9770 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=134217728 digest=056d833823c71e24
words=2 threads=1 round=0 variant=prefetch/8 claim_s=3.7201 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=134217728 digest=056d833823c71e24
words=2 threads=1 round=0 variant=own/16 claim_s=3.3728 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=134217728 digest=056d833823c71e24
words=2 threads=1 round=0 variant=load/16 claim_s=4.3016 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=134217728 digest=056d833823c71e24
words=2 threads=1 round=0 variant=prefetch/16 claim_s=3.6640 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=134217728 digest=056d833823c71e24
words=2 threads=1 round=0 variant=own/32 claim_s=3.4042 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=134217728 digest=056d833823c71e24
words=2 threads=1 round=0 variant=load/32 claim_s=3.7400 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=134217728 digest=056d833823c71e24
words=2 threads=1 round=0 variant=prefetch/32 claim_s=3.5318 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=134217728 digest=056d833823c71e24
words=2 threads=1 round=1 variant=hashbrown claim_s=2.9262 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=83887104 digest=056d833823c71e24
words=2 threads=1 round=1 variant=own/8 claim_s=3.2838 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=134217728 digest=056d833823c71e24
words=2 threads=1 round=1 variant=load/8 claim_s=3.8243 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=134217728 digest=056d833823c71e24
words=2 threads=1 round=1 variant=prefetch/8 claim_s=3.7384 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=134217728 digest=056d833823c71e24
words=2 threads=1 round=1 variant=own/16 claim_s=3.2719 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=134217728 digest=056d833823c71e24
words=2 threads=1 round=1 variant=load/16 claim_s=3.6206 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=134217728 digest=056d833823c71e24
words=2 threads=1 round=1 variant=prefetch/16 claim_s=3.6630 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=134217728 digest=056d833823c71e24
words=2 threads=1 round=1 variant=own/32 claim_s=3.4140 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=134217728 digest=056d833823c71e24
words=2 threads=1 round=1 variant=load/32 claim_s=3.9015 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=134217728 digest=056d833823c71e24
words=2 threads=1 round=1 variant=prefetch/32 claim_s=3.4438 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=134217728 digest=056d833823c71e24
words=2 threads=1 round=2 variant=hashbrown claim_s=3.1059 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=83887104 digest=056d833823c71e24
words=2 threads=1 round=2 variant=own/8 claim_s=3.4341 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=134217728 digest=056d833823c71e24
words=2 threads=1 round=2 variant=load/8 claim_s=3.9307 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=134217728 digest=056d833823c71e24
words=2 threads=1 round=2 variant=prefetch/8 claim_s=3.6015 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=134217728 digest=056d833823c71e24
words=2 threads=1 round=2 variant=own/16 claim_s=3.6762 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=134217728 digest=056d833823c71e24
words=2 threads=1 round=2 variant=load/16 claim_s=3.9920 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=134217728 digest=056d833823c71e24
words=2 threads=1 round=2 variant=prefetch/16 claim_s=3.4976 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=134217728 digest=056d833823c71e24
words=2 threads=1 round=2 variant=own/32 claim_s=3.4221 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=134217728 digest=056d833823c71e24
words=2 threads=1 round=2 variant=load/32 claim_s=3.4641 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=134217728 digest=056d833823c71e24
words=2 threads=1 round=2 variant=prefetch/32 claim_s=3.4787 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=134217728 digest=056d833823c71e24
words=2 threads=1 round=3 variant=hashbrown claim_s=3.1879 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=83887104 digest=056d833823c71e24
words=2 threads=1 round=3 variant=own/8 claim_s=3.3878 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=134217728 digest=056d833823c71e24
words=2 threads=1 round=3 variant=load/8 claim_s=3.9917 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=134217728 digest=056d833823c71e24
words=2 threads=1 round=3 variant=prefetch/8 claim_s=3.8277 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=134217728 digest=056d833823c71e24
words=2 threads=1 round=3 variant=own/16 claim_s=3.7366 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=134217728 digest=056d833823c71e24
words=2 threads=1 round=3 variant=load/16 claim_s=3.9872 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=134217728 digest=056d833823c71e24
words=2 threads=1 round=3 variant=prefetch/16 claim_s=3.5267 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=134217728 digest=056d833823c71e24
words=2 threads=1 round=3 variant=own/32 claim_s=3.5482 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=134217728 digest=056d833823c71e24
words=2 threads=1 round=3 variant=load/32 claim_s=3.8349 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=134217728 digest=056d833823c71e24
words=2 threads=1 round=3 variant=prefetch/32 claim_s=3.7893 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=134217728 digest=056d833823c71e24
words=2 threads=1 round=4 variant=hashbrown claim_s=3.2584 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=83887104 digest=056d833823c71e24
words=2 threads=1 round=4 variant=own/8 claim_s=3.8740 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=134217728 digest=056d833823c71e24
words=2 threads=1 round=4 variant=load/8 claim_s=4.3667 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=134217728 digest=056d833823c71e24
words=2 threads=1 round=4 variant=prefetch/8 claim_s=4.4664 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=134217728 digest=056d833823c71e24
words=2 threads=1 round=4 variant=own/16 claim_s=3.9075 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=134217728 digest=056d833823c71e24
words=2 threads=1 round=4 variant=load/16 claim_s=4.0828 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=134217728 digest=056d833823c71e24
words=2 threads=1 round=4 variant=prefetch/16 claim_s=4.0409 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=134217728 digest=056d833823c71e24
words=2 threads=1 round=4 variant=own/32 claim_s=3.5965 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=134217728 digest=056d833823c71e24
words=2 threads=1 round=4 variant=load/32 claim_s=4.0658 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=134217728 digest=056d833823c71e24
words=2 threads=1 round=4 variant=prefetch/32 claim_s=3.6168 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=134217728 digest=056d833823c71e24
SUMMARY words=2 threads=1 new=10000000 lookups=26138826 found=16138826 levels=65
  hashbrown      min_s=2.9262 median_s=3.1879 vs_hashbrown=+0.0% index_bytes_per_state=8.39
  own/8          min_s=3.2838 median_s=3.4341 vs_hashbrown=+7.7% index_bytes_per_state=13.42
  load/8         min_s=3.8243 median_s=3.9770 vs_hashbrown=+24.8% index_bytes_per_state=13.42
  prefetch/8     min_s=3.6015 median_s=3.7384 vs_hashbrown=+17.3% index_bytes_per_state=13.42
  own/16         min_s=3.2719 median_s=3.6762 vs_hashbrown=+15.3% index_bytes_per_state=13.42
  load/16        min_s=3.6206 median_s=3.9920 vs_hashbrown=+25.2% index_bytes_per_state=13.42
  prefetch/16    min_s=3.4976 median_s=3.6630 vs_hashbrown=+14.9% index_bytes_per_state=13.42
  own/32         min_s=3.4042 median_s=3.4221 vs_hashbrown=+7.3% index_bytes_per_state=13.42
  load/32        min_s=3.4641 median_s=3.8349 vs_hashbrown=+20.3% index_bytes_per_state=13.42
  prefetch/32    min_s=3.4438 median_s=3.5318 vs_hashbrown=+10.8% index_bytes_per_state=13.42
words=2 threads=4 round=0 variant=hashbrown claim_s=1.0870 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=83887104 digest=056d833823c71e24
words=2 threads=4 round=0 variant=own/8 claim_s=1.1589 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=134217728 digest=056d833823c71e24
words=2 threads=4 round=0 variant=load/8 claim_s=1.2358 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=134217728 digest=056d833823c71e24
words=2 threads=4 round=0 variant=prefetch/8 claim_s=1.0773 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=134217728 digest=056d833823c71e24
words=2 threads=4 round=0 variant=own/16 claim_s=1.2966 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=134217728 digest=056d833823c71e24
words=2 threads=4 round=0 variant=load/16 claim_s=1.3907 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=134217728 digest=056d833823c71e24
words=2 threads=4 round=0 variant=prefetch/16 claim_s=1.3664 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=134217728 digest=056d833823c71e24
words=2 threads=4 round=0 variant=own/32 claim_s=1.3271 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=134217728 digest=056d833823c71e24
words=2 threads=4 round=0 variant=load/32 claim_s=1.2373 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=134217728 digest=056d833823c71e24
words=2 threads=4 round=0 variant=prefetch/32 claim_s=1.3116 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=134217728 digest=056d833823c71e24
words=2 threads=4 round=1 variant=hashbrown claim_s=1.0620 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=83887104 digest=056d833823c71e24
words=2 threads=4 round=1 variant=own/8 claim_s=1.2277 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=134217728 digest=056d833823c71e24
words=2 threads=4 round=1 variant=load/8 claim_s=1.3019 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=134217728 digest=056d833823c71e24
words=2 threads=4 round=1 variant=prefetch/8 claim_s=1.2933 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=134217728 digest=056d833823c71e24
words=2 threads=4 round=1 variant=own/16 claim_s=1.1677 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=134217728 digest=056d833823c71e24
words=2 threads=4 round=1 variant=load/16 claim_s=1.2405 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=134217728 digest=056d833823c71e24
words=2 threads=4 round=1 variant=prefetch/16 claim_s=1.2079 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=134217728 digest=056d833823c71e24
words=2 threads=4 round=1 variant=own/32 claim_s=1.1563 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=134217728 digest=056d833823c71e24
words=2 threads=4 round=1 variant=load/32 claim_s=1.2459 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=134217728 digest=056d833823c71e24
words=2 threads=4 round=1 variant=prefetch/32 claim_s=1.2560 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=134217728 digest=056d833823c71e24
words=2 threads=4 round=2 variant=hashbrown claim_s=1.1079 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=83887104 digest=056d833823c71e24
words=2 threads=4 round=2 variant=own/8 claim_s=1.1763 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=134217728 digest=056d833823c71e24
words=2 threads=4 round=2 variant=load/8 claim_s=1.4075 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=134217728 digest=056d833823c71e24
words=2 threads=4 round=2 variant=prefetch/8 claim_s=1.3993 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=134217728 digest=056d833823c71e24
words=2 threads=4 round=2 variant=own/16 claim_s=1.1645 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=134217728 digest=056d833823c71e24
words=2 threads=4 round=2 variant=load/16 claim_s=1.3019 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=134217728 digest=056d833823c71e24
words=2 threads=4 round=2 variant=prefetch/16 claim_s=1.3123 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=134217728 digest=056d833823c71e24
words=2 threads=4 round=2 variant=own/32 claim_s=1.1927 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=134217728 digest=056d833823c71e24
words=2 threads=4 round=2 variant=load/32 claim_s=1.2936 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=134217728 digest=056d833823c71e24
words=2 threads=4 round=2 variant=prefetch/32 claim_s=1.2490 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=134217728 digest=056d833823c71e24
words=2 threads=4 round=3 variant=hashbrown claim_s=1.1219 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=83887104 digest=056d833823c71e24
words=2 threads=4 round=3 variant=own/8 claim_s=1.1429 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=134217728 digest=056d833823c71e24
words=2 threads=4 round=3 variant=load/8 claim_s=1.3059 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=134217728 digest=056d833823c71e24
words=2 threads=4 round=3 variant=prefetch/8 claim_s=1.3304 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=134217728 digest=056d833823c71e24
words=2 threads=4 round=3 variant=own/16 claim_s=1.2223 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=134217728 digest=056d833823c71e24
words=2 threads=4 round=3 variant=load/16 claim_s=1.2780 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=134217728 digest=056d833823c71e24
words=2 threads=4 round=3 variant=prefetch/16 claim_s=1.2347 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=134217728 digest=056d833823c71e24
words=2 threads=4 round=3 variant=own/32 claim_s=1.2116 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=134217728 digest=056d833823c71e24
words=2 threads=4 round=3 variant=load/32 claim_s=1.2749 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=134217728 digest=056d833823c71e24
words=2 threads=4 round=3 variant=prefetch/32 claim_s=1.1848 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=134217728 digest=056d833823c71e24
words=2 threads=4 round=4 variant=hashbrown claim_s=1.1023 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=83887104 digest=056d833823c71e24
words=2 threads=4 round=4 variant=own/8 claim_s=1.1106 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=134217728 digest=056d833823c71e24
words=2 threads=4 round=4 variant=load/8 claim_s=1.2887 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=134217728 digest=056d833823c71e24
words=2 threads=4 round=4 variant=prefetch/8 claim_s=1.2499 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=134217728 digest=056d833823c71e24
words=2 threads=4 round=4 variant=own/16 claim_s=1.1817 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=134217728 digest=056d833823c71e24
words=2 threads=4 round=4 variant=load/16 claim_s=1.3050 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=134217728 digest=056d833823c71e24
words=2 threads=4 round=4 variant=prefetch/16 claim_s=1.2058 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=134217728 digest=056d833823c71e24
words=2 threads=4 round=4 variant=own/32 claim_s=1.2477 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=134217728 digest=056d833823c71e24
words=2 threads=4 round=4 variant=load/32 claim_s=1.1514 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=134217728 digest=056d833823c71e24
words=2 threads=4 round=4 variant=prefetch/32 claim_s=1.2206 new=10000000 lookups=26138826 found=16138826 levels=65 index_bytes=134217728 digest=056d833823c71e24
SUMMARY words=2 threads=4 new=10000000 lookups=26138826 found=16138826 levels=65
  hashbrown      min_s=1.0620 median_s=1.1023 vs_hashbrown=+0.0% index_bytes_per_state=8.39
  own/8          min_s=1.1106 median_s=1.1589 vs_hashbrown=+5.1% index_bytes_per_state=13.42
  load/8         min_s=1.2358 median_s=1.3019 vs_hashbrown=+18.1% index_bytes_per_state=13.42
  prefetch/8     min_s=1.0773 median_s=1.2933 vs_hashbrown=+17.3% index_bytes_per_state=13.42
  own/16         min_s=1.1645 median_s=1.1817 vs_hashbrown=+7.2% index_bytes_per_state=13.42
  load/16        min_s=1.2405 median_s=1.3019 vs_hashbrown=+18.1% index_bytes_per_state=13.42
  prefetch/16    min_s=1.2058 median_s=1.2347 vs_hashbrown=+12.0% index_bytes_per_state=13.42
  own/32         min_s=1.1563 median_s=1.2116 vs_hashbrown=+9.9% index_bytes_per_state=13.42
  load/32        min_s=1.1514 median_s=1.2459 vs_hashbrown=+13.0% index_bytes_per_state=13.42
  prefetch/32    min_s=1.1848 median_s=1.2490 vs_hashbrown=+13.3% index_bytes_per_state=13.42
CHECK OK: every variant and thread count gave the same new rows, ids and parents
```

### Part 1, 100m

```
wheels=8 states=100000000 rounds=3 prefetch_feature=true arch=x86_64 expansion_helpers=4
words=1 threads=1 round=0 variant=hashbrown claim_s=14.4239 new=100000000 lookups=259644583 found=159644583 levels=74 index_bytes=671089664 digest=e3058aa889dcc6a5
words=1 threads=1 round=0 variant=own/16 claim_s=16.2092 new=100000000 lookups=259644583 found=159644583 levels=74 index_bytes=536870912 digest=e3058aa889dcc6a5
words=1 threads=1 round=0 variant=load/16 claim_s=17.1261 new=100000000 lookups=259644583 found=159644583 levels=74 index_bytes=536870912 digest=e3058aa889dcc6a5
words=1 threads=1 round=0 variant=prefetch/16 claim_s=15.6527 new=100000000 lookups=259644583 found=159644583 levels=74 index_bytes=536870912 digest=e3058aa889dcc6a5
words=1 threads=1 round=1 variant=hashbrown claim_s=15.0992 new=100000000 lookups=259644583 found=159644583 levels=74 index_bytes=671089664 digest=e3058aa889dcc6a5
words=1 threads=1 round=1 variant=own/16 claim_s=16.6186 new=100000000 lookups=259644583 found=159644583 levels=74 index_bytes=536870912 digest=e3058aa889dcc6a5
words=1 threads=1 round=1 variant=load/16 claim_s=17.6340 new=100000000 lookups=259644583 found=159644583 levels=74 index_bytes=536870912 digest=e3058aa889dcc6a5
words=1 threads=1 round=1 variant=prefetch/16 claim_s=15.7874 new=100000000 lookups=259644583 found=159644583 levels=74 index_bytes=536870912 digest=e3058aa889dcc6a5
words=1 threads=1 round=2 variant=hashbrown claim_s=14.8724 new=100000000 lookups=259644583 found=159644583 levels=74 index_bytes=671089664 digest=e3058aa889dcc6a5
words=1 threads=1 round=2 variant=own/16 claim_s=16.1308 new=100000000 lookups=259644583 found=159644583 levels=74 index_bytes=536870912 digest=e3058aa889dcc6a5
words=1 threads=1 round=2 variant=load/16 claim_s=15.9761 new=100000000 lookups=259644583 found=159644583 levels=74 index_bytes=536870912 digest=e3058aa889dcc6a5
words=1 threads=1 round=2 variant=prefetch/16 claim_s=14.9301 new=100000000 lookups=259644583 found=159644583 levels=74 index_bytes=536870912 digest=e3058aa889dcc6a5
SUMMARY words=1 threads=1 new=100000000 lookups=259644583 found=159644583 levels=74
  hashbrown      min_s=14.4239 median_s=14.8724 vs_hashbrown=+0.0% index_bytes_per_state=6.71
  own/16         min_s=16.1308 median_s=16.2092 vs_hashbrown=+9.0% index_bytes_per_state=5.37
  load/16        min_s=15.9761 median_s=17.1261 vs_hashbrown=+15.2% index_bytes_per_state=5.37
  prefetch/16    min_s=14.9301 median_s=15.6527 vs_hashbrown=+5.2% index_bytes_per_state=5.37
words=1 threads=4 round=0 variant=hashbrown claim_s=4.7276 new=100000000 lookups=259644583 found=159644583 levels=74 index_bytes=671089664 digest=e3058aa889dcc6a5
words=1 threads=4 round=0 variant=own/16 claim_s=4.8123 new=100000000 lookups=259644583 found=159644583 levels=74 index_bytes=536870912 digest=e3058aa889dcc6a5
words=1 threads=4 round=0 variant=load/16 claim_s=5.0176 new=100000000 lookups=259644583 found=159644583 levels=74 index_bytes=536870912 digest=e3058aa889dcc6a5
words=1 threads=4 round=0 variant=prefetch/16 claim_s=4.7568 new=100000000 lookups=259644583 found=159644583 levels=74 index_bytes=536870912 digest=e3058aa889dcc6a5
words=1 threads=4 round=1 variant=hashbrown claim_s=5.4502 new=100000000 lookups=259644583 found=159644583 levels=74 index_bytes=671089664 digest=e3058aa889dcc6a5
words=1 threads=4 round=1 variant=own/16 claim_s=5.1400 new=100000000 lookups=259644583 found=159644583 levels=74 index_bytes=536870912 digest=e3058aa889dcc6a5
words=1 threads=4 round=1 variant=load/16 claim_s=5.2940 new=100000000 lookups=259644583 found=159644583 levels=74 index_bytes=536870912 digest=e3058aa889dcc6a5
words=1 threads=4 round=1 variant=prefetch/16 claim_s=4.7103 new=100000000 lookups=259644583 found=159644583 levels=74 index_bytes=536870912 digest=e3058aa889dcc6a5
words=1 threads=4 round=2 variant=hashbrown claim_s=5.0497 new=100000000 lookups=259644583 found=159644583 levels=74 index_bytes=671089664 digest=e3058aa889dcc6a5
words=1 threads=4 round=2 variant=own/16 claim_s=4.8607 new=100000000 lookups=259644583 found=159644583 levels=74 index_bytes=536870912 digest=e3058aa889dcc6a5
words=1 threads=4 round=2 variant=load/16 claim_s=5.2155 new=100000000 lookups=259644583 found=159644583 levels=74 index_bytes=536870912 digest=e3058aa889dcc6a5
words=1 threads=4 round=2 variant=prefetch/16 claim_s=4.8030 new=100000000 lookups=259644583 found=159644583 levels=74 index_bytes=536870912 digest=e3058aa889dcc6a5
SUMMARY words=1 threads=4 new=100000000 lookups=259644583 found=159644583 levels=74
  hashbrown      min_s=4.7276 median_s=5.0497 vs_hashbrown=+0.0% index_bytes_per_state=6.71
  own/16         min_s=4.8123 median_s=4.8607 vs_hashbrown=-3.7% index_bytes_per_state=5.37
  load/16        min_s=5.0176 median_s=5.2155 vs_hashbrown=+3.3% index_bytes_per_state=5.37
  prefetch/16    min_s=4.7103 median_s=4.7568 vs_hashbrown=-5.8% index_bytes_per_state=5.37
words=2 threads=1 round=0 variant=hashbrown claim_s=43.4531 new=100000000 lookups=259644583 found=159644583 levels=74 index_bytes=671089664 digest=edc7e94be215fa40
words=2 threads=1 round=0 variant=own/16 claim_s=50.4230 new=100000000 lookups=259644583 found=159644583 levels=74 index_bytes=1073741824 digest=edc7e94be215fa40
words=2 threads=1 round=0 variant=load/16 claim_s=52.7284 new=100000000 lookups=259644583 found=159644583 levels=74 index_bytes=1073741824 digest=edc7e94be215fa40
words=2 threads=1 round=0 variant=prefetch/16 claim_s=47.2659 new=100000000 lookups=259644583 found=159644583 levels=74 index_bytes=1073741824 digest=edc7e94be215fa40
words=2 threads=1 round=1 variant=hashbrown claim_s=47.4413 new=100000000 lookups=259644583 found=159644583 levels=74 index_bytes=671089664 digest=edc7e94be215fa40
words=2 threads=1 round=1 variant=own/16 claim_s=47.0483 new=100000000 lookups=259644583 found=159644583 levels=74 index_bytes=1073741824 digest=edc7e94be215fa40
words=2 threads=1 round=1 variant=load/16 claim_s=47.6827 new=100000000 lookups=259644583 found=159644583 levels=74 index_bytes=1073741824 digest=edc7e94be215fa40
words=2 threads=1 round=1 variant=prefetch/16 claim_s=46.4063 new=100000000 lookups=259644583 found=159644583 levels=74 index_bytes=1073741824 digest=edc7e94be215fa40
words=2 threads=1 round=2 variant=hashbrown claim_s=44.9085 new=100000000 lookups=259644583 found=159644583 levels=74 index_bytes=671089664 digest=edc7e94be215fa40
words=2 threads=1 round=2 variant=own/16 claim_s=48.8117 new=100000000 lookups=259644583 found=159644583 levels=74 index_bytes=1073741824 digest=edc7e94be215fa40
words=2 threads=1 round=2 variant=load/16 claim_s=46.5106 new=100000000 lookups=259644583 found=159644583 levels=74 index_bytes=1073741824 digest=edc7e94be215fa40
words=2 threads=1 round=2 variant=prefetch/16 claim_s=44.7866 new=100000000 lookups=259644583 found=159644583 levels=74 index_bytes=1073741824 digest=edc7e94be215fa40
SUMMARY words=2 threads=1 new=100000000 lookups=259644583 found=159644583 levels=74
  hashbrown      min_s=43.4531 median_s=44.9085 vs_hashbrown=+0.0% index_bytes_per_state=6.71
  own/16         min_s=47.0483 median_s=48.8117 vs_hashbrown=+8.7% index_bytes_per_state=10.74
  load/16        min_s=46.5106 median_s=47.6827 vs_hashbrown=+6.2% index_bytes_per_state=10.74
  prefetch/16    min_s=44.7866 median_s=46.4063 vs_hashbrown=+3.3% index_bytes_per_state=10.74
words=2 threads=4 round=0 variant=hashbrown claim_s=11.8664 new=100000000 lookups=259644583 found=159644583 levels=74 index_bytes=671089664 digest=edc7e94be215fa40
words=2 threads=4 round=0 variant=own/16 claim_s=13.2469 new=100000000 lookups=259644583 found=159644583 levels=74 index_bytes=1073741824 digest=edc7e94be215fa40
words=2 threads=4 round=0 variant=load/16 claim_s=13.6937 new=100000000 lookups=259644583 found=159644583 levels=74 index_bytes=1073741824 digest=edc7e94be215fa40
words=2 threads=4 round=0 variant=prefetch/16 claim_s=13.0545 new=100000000 lookups=259644583 found=159644583 levels=74 index_bytes=1073741824 digest=edc7e94be215fa40
words=2 threads=4 round=1 variant=hashbrown claim_s=11.1293 new=100000000 lookups=259644583 found=159644583 levels=74 index_bytes=671089664 digest=edc7e94be215fa40
words=2 threads=4 round=1 variant=own/16 claim_s=13.8110 new=100000000 lookups=259644583 found=159644583 levels=74 index_bytes=1073741824 digest=edc7e94be215fa40
words=2 threads=4 round=1 variant=load/16 claim_s=13.5562 new=100000000 lookups=259644583 found=159644583 levels=74 index_bytes=1073741824 digest=edc7e94be215fa40
words=2 threads=4 round=1 variant=prefetch/16 claim_s=13.4998 new=100000000 lookups=259644583 found=159644583 levels=74 index_bytes=1073741824 digest=edc7e94be215fa40
words=2 threads=4 round=2 variant=hashbrown claim_s=11.5475 new=100000000 lookups=259644583 found=159644583 levels=74 index_bytes=671089664 digest=edc7e94be215fa40
words=2 threads=4 round=2 variant=own/16 claim_s=12.8099 new=100000000 lookups=259644583 found=159644583 levels=74 index_bytes=1073741824 digest=edc7e94be215fa40
words=2 threads=4 round=2 variant=load/16 claim_s=13.6565 new=100000000 lookups=259644583 found=159644583 levels=74 index_bytes=1073741824 digest=edc7e94be215fa40
words=2 threads=4 round=2 variant=prefetch/16 claim_s=12.2726 new=100000000 lookups=259644583 found=159644583 levels=74 index_bytes=1073741824 digest=edc7e94be215fa40
SUMMARY words=2 threads=4 new=100000000 lookups=259644583 found=159644583 levels=74
  hashbrown      min_s=11.1293 median_s=11.5475 vs_hashbrown=+0.0% index_bytes_per_state=6.71
  own/16         min_s=12.8099 median_s=13.2469 vs_hashbrown=+14.7% index_bytes_per_state=10.74
  load/16        min_s=13.5562 median_s=13.6565 vs_hashbrown=+18.3% index_bytes_per_state=10.74
  prefetch/16    min_s=12.2726 median_s=13.0545 vs_hashbrown=+13.1% index_bytes_per_state=10.74
CHECK OK: every variant and thread count gave the same new rows, ids and parents
```

### Part 2, check run at 4 jobs (status and states)

```
2d61e74: status=fail states=11820300 milliseconds=22046
0ffa4d5: status=fail states=11820300 milliseconds=19595
4c25766: status=fail states=11820300 milliseconds=11697
```

### Part 2, turn.py

```
2.76 2.74 2.16 2/110 25964
{"model": "corridor", "jobs": 1, "b2d61e74": {"real": [28.702, 29.39, 29.577, 32.535, 32.664, 32.666, 32.858], "median": 32.535, "cpu": 32.2, "mib": 1965.6, "status": [256]}, "b0ffa4d5": {"real": [5.312, 5.391, 5.529, 5.546, 5.699, 5.806, 5.928], "median": 5.546, "cpu": 5.53, "mib": 212.8, "status": [256]}, "b4c25766": {"real": [5.005, 5.129, 5.456, 5.461, 5.464, 5.498, 5.693], "median": 5.461, "cpu": 5.4, "mib": 212.7, "status": [256]}}
{"model": "corridor", "jobs": 2, "b2d61e74": {"real": [20.161, 20.948, 21.144, 21.632, 21.846, 22.275, 23.028], "median": 21.632, "cpu": 32.37, "mib": 1966.4, "status": [256]}, "b0ffa4d5": {"real": [12.151, 12.668, 12.711, 12.88, 12.886, 12.905, 12.988], "median": 12.88, "cpu": 19.83, "mib": 221.9, "status": [256]}, "b4c25766": {"real": [7.95, 8.021, 8.034, 8.138, 8.395, 8.971, 9.036], "median": 8.138, "cpu": 10.19, "mib": 213.2, "status": [256]}}
{"model": "corridor", "jobs": 4, "b2d61e74": {"real": [19.737, 20.118, 20.306, 20.502, 21.736, 21.931, 22.211], "median": 20.502, "cpu": 41.81, "mib": 1965.2, "status": [256]}, "b0ffa4d5": {"real": [19.399, 19.423, 19.966, 20.143, 20.168, 20.187, 20.643], "median": 20.143, "cpu": 43.09, "mib": 229.2, "status": [256]}, "b4c25766": {"real": [10.825, 10.994, 11.021, 11.552, 11.621, 11.668, 12.116], "median": 11.552, "cpu": 21.06, "mib": 213.8, "status": [256]}}
2.67 2.33 2.02 1/119 18274
```
