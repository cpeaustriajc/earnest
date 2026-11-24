---
paths:
  - "crates/earnest_check/src/explore/**"
  - "crates/earnest_check/src/explore.rs"
---

- Start a thread only with `earnest_threads::start`, whose handle joins it. `just lint-library`
  refuses the standard calls: a detached worker with a large stack can crash the process.
- The numbers that shape a level were each taken from a measured curve: at most 64 parts, a
  part for every 512 microseconds of work, rounds of 32 parts for one thread, a part's table of
  at most 2^16 words. Change one only with the curve for the new number at 10 million and at
  100 million states, with 1 worker and with 4.
- A part's table for rows of one word may forget and let a repeat through, because the claim
  that follows is exact. It must never call a new row seen.
- Buffers kept by several workers and a fresh row filed in one pass were each built more than
  once and taken back (`tasks/benchmark/TRIED.md`).
