# block=task1_grid_100m model=measure/grid_100m.model.ts max_states=200000000 variants=a:tip:1: b:tip:4: c:tip:4:0 warmup_rounds=0 measured_rounds=3
# started=2026-11-22T01:39:21Z
# loadavg_before=2.06 0.97 0.39 1/114 13687
# loadavg_after=3.04 2.03 1.11 1/119 19076
# finished=2026-11-22T01:47:09Z
runs=9 counted=9 not_as_expected=0

| Variant | Build | Workers | CPUs | Runs | Wall min | Wall median | Wall max | CPU min | CPU median | CPU max | Median user | Median sys | Median peak memory |
|---|---|---:|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| a | tip | 1 | all | 3 | 89.03 | 99.65 | 100.30 | 89.00 | 99.41 | 100.14 | 98.50 | 0.91 | 1411 MiB |
| b | tip | 4 | all | 3 | 12.86 | 13.43 | 13.88 | 48.02 | 50.17 | 51.72 | 46.37 | 3.82 | 1647 MiB |
| c | tip | 4 | 0 | 3 | 45.31 | 45.82 | 47.95 | 45.28 | 45.76 | 47.86 | 41.25 | 4.51 | 1619 MiB |

Every run, wall s / CPU s (round 0 and below not counted):

| Variant | 1 | 2 | 3 |
|---|---:|---:|---:|
| a | 99.65 / 99.41 | 89.03 / 89.00 | 100.30 / 100.14 |
| b | 12.86 / 48.02 | 13.88 / 51.72 | 13.43 / 50.17 |
| c | 45.31 / 45.28 | 45.82 / 45.76 | 47.95 / 47.86 |
