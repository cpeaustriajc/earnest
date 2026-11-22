# block=task4_grid_100m model=measure/grid_100m.model.ts max_states=200000000 variants=tip:tip:4: p16:p16:4: p8:p8:4: p32:p32:4: warmup_rounds=0 measured_rounds=3
# started=2026-11-22T02:00:18Z
# loadavg_before=2.68 1.95 1.40 1/110 29564
# loadavg_after=3.44 2.67 1.77 1/109 6460
# finished=2026-11-22T02:02:58Z
runs=12 counted=12 not_as_expected=0

| Variant | Build | Workers | CPUs | Runs | Wall min | Wall median | Wall max | CPU min | CPU median | CPU max | Median user | Median sys | Median peak memory |
|---|---|---:|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tip | tip | 4 | all | 3 | 12.69 | 13.08 | 13.82 | 47.08 | 48.74 | 51.57 | 44.30 | 4.44 | 1611 MiB |
| p16 | p16 | 4 | all | 3 | 13.03 | 13.06 | 13.30 | 48.53 | 48.55 | 49.71 | 44.20 | 4.33 | 1614 MiB |
| p8 | p8 | 4 | all | 3 | 12.98 | 13.61 | 14.55 | 48.78 | 50.64 | 54.66 | 46.53 | 4.11 | 1622 MiB |
| p32 | p32 | 4 | all | 3 | 12.40 | 13.57 | 13.93 | 46.01 | 50.40 | 51.87 | 45.65 | 4.44 | 1621 MiB |

Every run, wall s / CPU s (round 0 and below not counted):

| Variant | 1 | 2 | 3 |
|---|---:|---:|---:|
| tip | 12.69 / 47.08 | 13.82 / 51.57 | 13.08 / 48.74 |
| p16 | 13.30 / 49.71 | 13.06 / 48.55 | 13.03 / 48.53 |
| p8 | 14.55 / 54.66 | 12.98 / 48.78 | 13.61 / 50.64 |
| p32 | 13.93 / 51.87 | 12.40 / 46.01 | 13.57 / 50.40 |
