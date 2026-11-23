---
paths:
  - "crates/earnest_check/src/store.rs"
  - "crates/earnest_check/src/store/**/*.rs"
---
- What several threads write for every row gets a pair of cache lines of its own, as `Shard` has
  with `#[repr(align(128))]`: shards side by side made the claim with 4 workers twice as dear.
- Read and write a short row as fixed pieces (`word_of`, `pack` in `rows.rs`), never by a copy
  whose length is known only at run time: that copy cost 3% to 11%.
- The index stays on `hashbrown` with 64 shards; tables of our own and more or growing shards
  lost (`tasks/benchmark/TRIED.md`).
- Do not keep buffers across levels to save page faults: the kept buffer comes back cold.
