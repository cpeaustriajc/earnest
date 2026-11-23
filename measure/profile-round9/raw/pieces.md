| Piece | Group | Unit | 10M 1 share | 100M 1 share | 100M 1 s | 100M 4 share | 100M 4 s | ns/unit 10M 1 | 100M 1 | 10M 4 | 100M 4 |
|---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| a part's table, `offer_unseen_word` (hashbrown in part.rs) | c | succ | 16.46% | 19.21% | 7.68 | 16.44% | 8.04 | 6.6 | 10.7 | 8.2 | 11.2 |
| index probe in `claim_level` (hashbrown in level.rs) | a | cand | 13.02% | 11.49% | 4.59 | 25.95% | 12.68 | 12.7 | 17.7 | 40.8 | 48.9 |
| index key compare and shard, `store/index.rs` | a | cand | 5.85% | 8.20% | 3.28 | 6.20% | 3.03 | 5.7 | 12.6 | 3.4 | 11.7 |
| index rehash, outlined `reserve_rehash` | a | states | 2.80% | 1.60% | 0.64 | 1.29% | 0.63 | 7.1 | 6.4 | 8.7 | 6.3 |
| frontier read, `store/rows.rs` (`word_of` in `unpack`) | b | states | 6.71% | 12.57% | 5.02 | 8.31% | 4.06 | 17.0 | 50.2 | 14.6 | 40.6 |
| rows: codec, layout, `read_state`, `outside` | b | states | 4.95% | 3.54% | 1.41 | 2.85% | 1.39 | 12.5 | 14.1 | 14.2 | 13.9 |
| `explore/part.rs` lines of its own | c | states | 8.18% | 10.30% | 4.12 | 7.83% | 3.83 | 20.7 | 41.2 | 24.2 | 38.3 |
| `store/level.rs` lines of its own | c | states | 5.71% | 3.58% | 1.43 | 3.04% | 1.48 | 14.5 | 14.3 | 14.0 | 14.8 |
| `expand.rs` | d | succ | 10.16% | 8.98% | 3.59 | 7.03% | 3.44 | 4.1 | 5.0 | 4.6 | 4.8 |
| `earnest_ir` (num, eval, cond, mod) | d | succ | 13.70% | 12.56% | 5.02 | 9.57% | 4.68 | 5.5 | 7.0 | 6.5 | 6.5 |
| hashing, `mix`/`hash_words` | e | succ | 4.27% | 3.15% | 1.26 | 2.68% | 1.31 | 1.7 | 1.7 | 1.9 | 1.8 |
| kernel page faults | f | states | 3.55% | 2.12% | 0.85 | 5.72% | 2.79 | 9.0 | 8.5 | 13.4 | 27.9 |
| kernel mmap/munmap/madvise | f | states | 0.67% | 0.32% | 0.13 | 0.80% | 0.39 | 1.7 | 1.3 | 5.5 | 3.9 |
| libc malloc/free/realloc | f | states | 1.17% | 0.15% | 0.06 | 0.54% | 0.26 | 3.0 | 0.6 | 5.4 | 2.6 |
| libc memmove/memset, caller not resolved | g | states | 1.76% | 1.81% | 0.73 | 1.24% | 0.61 | 4.5 | 7.3 | 5.1 | 6.1 |
