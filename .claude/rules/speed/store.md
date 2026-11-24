---
paths:
  - "crates/earnest_check/src/store/**"
  - "crates/earnest_check/src/store.rs"
---

- What two workers write for every row gets cache lines of its own, as `Shard` in
  `store/index.rs` has. Two tables in one cache line made every claim with 4 workers cost twice
  as much. A test in `store/level.rs` fails when two tables share a pair of lines.
- Read and write the bytes of a row in pieces of a fixed size (`word_of` and `put_word` in
  `store/rows.rs`), never by a copy whose length is known only when the checker runs.
- `read_state` hands out only the row where no field's codes grow. Decode a row into values
  only where something reads the values.
- The index stays on `hashbrown` with 64 shards. A table of our own, more shards and shards
  that grow were each measured and lost (`tasks/benchmark/TRIED.md`).
