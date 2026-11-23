---
paths:
  - "crates/earnest_check/src/**/*.rs"
  - "crates/earnest_ir/src/**/*.rs"
  - "crates/earnest_threads/src/**/*.rs"
---
- Before a speed change, read `tasks/benchmark/TRIED.md` and fix the rule for keeping it (model,
  workers, gain, rounds, peak memory); take back what misses it.
- Time builds in turn on one machine (`tasks/benchmark/turn.py`); when the clock and
  `just counts` disagree, the count decides.
- `just benchmark` and `just counts` must pass; their limits and `instructions.txt` change only on
  purpose, with the reason in the commit (`just record-counts`).
- After any change to the search, `just same-answers <the commit before>` must find 0 differences.
