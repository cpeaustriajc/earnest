| Binary | Arguments | States | Runs | Smallest time_s | Median time_s | Largest time_s |
|---|---|---:|---:|---:|---:|---:|
| floor (no prefetch on x86-64) | `7 serial` | 10000000 | 5 | 2.8410 s | 3.0015 s | 3.2652 s |
| floor_x86 (with _mm_prefetch) | `7 serial` | 10000000 | 5 | 2.0346 s | 2.1459 s | 2.2230 s |
| floor (no prefetch on x86-64) | `8 serial` | 100000000 | 3 | 49.1853 s | 50.9091 s | 51.5801 s |
| floor_x86 (with _mm_prefetch) | `8 serial` | 100000000 | 3 | 31.8667 s | 32.7582 s | 33.2083 s |
| floor (no prefetch on x86-64) | `7 4` | 10000000 | 5 | 2.0670 s | 2.2254 s | 2.3128 s |
| floor_x86 (with _mm_prefetch) | `7 4` | 10000000 | 5 | 1.7918 s | 1.8052 s | 1.9468 s |
