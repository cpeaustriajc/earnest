# block=task4_grid_10m model=tasks/benchmark/models/grid_10m.model.ts max_states=200000000 variants=tip:tip:4: p16:p16:4: p8:p8:4: p32:p32:4: warmup_rounds=1 measured_rounds=7
# started=2026-11-22T01:59:43Z
# loadavg_before=1.86 1.73 1.31 1/114 8596
# loadavg_after=2.68 1.95 1.40 1/110 29560
# finished=2026-11-22T02:00:18Z
runs=32 counted=28 not_as_expected=0

| Variant | Build | Workers | CPUs | Runs | Wall min | Wall median | Wall max | CPU min | CPU median | CPU max | Median user | Median sys | Median peak memory |
|---|---|---:|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tip | tip | 4 | all | 7 | 1.05 | 1.08 | 1.26 | 3.86 | 3.91 | 4.59 | 3.56 | 0.35 | 190 MiB |
| p16 | p16 | 4 | all | 7 | 1.03 | 1.07 | 1.22 | 3.75 | 3.88 | 4.33 | 3.47 | 0.41 | 189 MiB |
| p8 | p8 | 4 | all | 7 | 1.03 | 1.09 | 1.18 | 3.75 | 3.95 | 4.33 | 3.59 | 0.37 | 190 MiB |
| p32 | p32 | 4 | all | 7 | 0.99 | 1.05 | 1.16 | 3.61 | 3.73 | 4.27 | 3.38 | 0.34 | 190 MiB |

Every run, wall s / CPU s (round 0 and below not counted):

| Variant | 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| tip | 1.12 / 4.13 | 1.26 / 4.59 | 1.05 / 3.87 | 1.21 / 4.41 | 1.07 / 3.87 | 1.05 / 3.86 | 1.08 / 3.91 | 1.13 / 4.03 |
| p16 | 1.10 / 3.95 | 1.22 / 4.33 | 1.16 / 4.25 | 1.13 / 4.06 | 1.06 / 3.75 | 1.07 / 3.88 | 1.07 / 3.84 | 1.03 / 3.77 |
| p8 | 1.10 / 4.01 | 1.18 / 4.33 | 1.07 / 3.91 | 1.07 / 3.92 | 1.03 / 3.75 | 1.09 / 3.95 | 1.11 / 4.00 | 1.14 / 4.10 |
| p32 | 1.10 / 3.92 | 1.05 / 3.73 | 1.01 / 3.68 | 1.16 / 4.27 | 1.02 / 3.70 | 0.99 / 3.61 | 1.05 / 3.78 | 1.05 / 3.80 |
