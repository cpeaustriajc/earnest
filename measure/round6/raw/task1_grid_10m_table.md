# block=task1_grid_10m model=tasks/benchmark/models/grid_10m.model.ts max_states=200000000 variants=a:tip:1: b:tip:4: c:tip:4:0 warmup_rounds=1 measured_rounds=7
# started=2026-11-22T01:37:57Z
# loadavg_before=0.94 0.50 0.19 2/113 3151
# loadavg_after=2.06 0.97 0.39 1/114 13683
# finished=2026-11-22T01:39:21Z
runs=24 counted=21 not_as_expected=0

| Variant | Build | Workers | CPUs | Runs | Wall min | Wall median | Wall max | CPU min | CPU median | CPU max | Median user | Median sys | Median peak memory |
|---|---|---:|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| a | tip | 1 | all | 7 | 4.59 | 5.77 | 6.99 | 4.58 | 5.76 | 6.95 | 5.67 | 0.09 | 163 MiB |
| b | tip | 4 | all | 7 | 1.07 | 1.16 | 1.32 | 3.86 | 4.22 | 4.75 | 3.87 | 0.39 | 190 MiB |
| c | tip | 4 | 0 | 7 | 3.21 | 3.52 | 3.78 | 3.19 | 3.48 | 3.76 | 3.10 | 0.38 | 196 MiB |

Every run, wall s / CPU s (round 0 and below not counted):

| Variant | 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| a | 6.68 / 6.66 | 6.99 / 6.95 | 5.05 / 5.01 | 4.59 / 4.58 | 4.68 / 4.67 | 6.36 / 6.34 | 6.61 / 6.56 | 5.77 / 5.76 |
| b | 1.27 / 4.66 | 1.16 / 4.22 | 1.14 / 4.14 | 1.07 / 3.92 | 1.08 / 3.86 | 1.26 / 4.49 | 1.32 / 4.75 | 1.28 / 4.71 |
| c | 3.33 / 3.32 | 3.21 / 3.19 | 3.31 / 3.30 | 3.26 / 3.25 | 3.63 / 3.62 | 3.78 / 3.76 | 3.52 / 3.48 | 3.72 / 3.71 |
