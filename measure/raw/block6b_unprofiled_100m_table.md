# block=block6b_unprofiled_100m model=measure/grid_100m.model.ts max_states=200000000 combos=B:1 C:1 C:4 warmup_rounds=0 measured_rounds=1
# started=2026-11-21T15:49:44Z
# loadavg_before=0.63 1.01 0.99 1/112 1937
# loadavg_after=1.93 1.21 1.07 1/114 2853
# finished=2026-11-21T15:54:59Z
runs=3 measured=3 not_as_expected=0

| Build | Workers | Rounds | Status | States | Smallest wall time | Median wall time | Largest wall time | Smallest peak memory | Median peak memory |
|---|---:|---:|---|---:|---:|---:|---:|---:|---:|
| B | 1 | 1 | pass | 100000000 | 172.07 s | 172.07 s | 172.07 s | 1411 MiB | 1411 MiB |
| C | 1 | 1 | pass | 100000000 | 124.22 s | 124.22 s | 124.22 s | 1411 MiB | 1411 MiB |
| C | 4 | 1 | pass | 100000000 | 17.93 s | 17.93 s | 17.93 s | 1614 MiB | 1614 MiB |
