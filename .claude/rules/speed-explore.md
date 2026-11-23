---
paths:
  - "crates/earnest_check/src/explore.rs"
  - "crates/earnest_check/src/explore/**/*.rs"
---
- Start a thread only with `earnest_threads::start` and keep its `Joined`; `just lint-library`
  refuses the standard calls.
- Hand nothing small from one thread to another for each row or part: false sharing of such
  buffers cost 7 s of CPU time at 100 million states.
- `MOST_PARTS` (64), `PARTS_OF_A_LONE_ROUND` (32) and the cut of a level by its work each came
  from a measured curve; change one only with a new curve at 10 and 100 million states.
- The part's table forgets (`RECENT_SLOTS`) but compares whole words, so it is exact; keep both.
