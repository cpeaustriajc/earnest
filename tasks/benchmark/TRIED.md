# Tried and not kept

The speed work of pull request 15 (issue 14 holds the full account) tried each idea below in the
search and took it back. Read this before you try one of them again.

How to judge a speed change here:

1. Time the old and the new build in turn, run for run on one machine (`turn.py` beside this
   file), never one after the other and never across machines: the same code differs by up to 1.9
   times between runners and by 5% to 10% between runs on one.
2. When the clock and the count of instructions disagree, the count (`just counts`) decides: a
   change that executes the same instructions and reads slower moved code, not cost.
3. Fix the rule for keeping it before you measure: which model, which number of workers, what gain
   over how many rounds, what peak memory. A change that misses its rule is taken back.
4. `wide` with 1 worker is not judged by the clock: its time moves by up to 15% with where the
   compiler places one loop, at an equal count.
5. Keep nothing that changes an answer: `just same-answers <commit>` must find 0 differences.

| Tried | What it gained or cost | Why it went |
|---|---|---|
| Index shards whose number grows with the index (a full table splits by the next bit of the hash) | No split size from 2^18 to 2^23 bytes gained 3% at one number of workers without losing at the other; 230 MiB to 330 MiB more at 100 million states with 4 workers | Tables below glibc's moving threshold come from the workers' arenas and the old ones stay resident |
| More, smaller index shards (256, 1,024, 4,096 in place of 64) | 256: -3.5% at 100 million states on a machine with 2 MiB of L2 a core, +5% to +7% at 10 million, nothing on one with 8 MiB; 1,024 and 4,096 slower everywhere | Every shard has a fixed cost; one list of winners for each shard cut that cost and gained nothing at 64 |
| An index table of our own | Won 1 case of 8 by more than the spread and lost 6; 10.7 to 13.4 bytes a state against 6.7 to 8.4 | Slower, larger, and it needs `unsafe` |
| A table of our own for a part (linear probing with the word in the slot; the same without a branch; 32-bit words) | 9.9 ns to 13.0 ns against 7.4 ns for `hashbrown`; 12 ns to 13 ns against 8.4 ns to 8.8 ns; 9.05 ns against 8.67 ns | The cost is one branch a probe that no table removes |
| Buffers (parts and kits) kept from level to level by several workers | At best -2.6% at 100 million states with 4 workers, with `grid_10m` with 4 workers 1.14 s to 1.30 s, or +5% to +19% peak memory | A kept buffer is cold when it is used again, so the kernel time saved comes back as user time; kept arena buffers stop glibc from trimming |
| Spare rows kept within a level | More page faults | Slower |
| A fresh row filed where the claim reads it, in one pass | 2% to 23% slower at 10 million states, nothing at 100 million, page faults 1.82 million to 2.42 million | Small blocks make the claim jump between them, and safe Rust fills each block with zeros first |
| Room made for a shard's rows once before its claim | 3.142 s against 3.137 s, 9 MiB more | The same doublings happen at the reserve |
| A split in place of a full index table, moving half its rows | Slower and larger than doubling | Nothing cheaper than the doubling was found |
| Help for the claim: touching the table ahead of it, a small table for each shard of the rows claimed this level, a round's rows sorted by bucket, claiming in found order without filing, filing by an indirect order | Each slower | Slower |
| Huge pages from the allocator (`GLIBC_TUNABLES=glibc.malloc.hugetlb=1`) | Inside the spread at 10 and 100 million states | The checker allocates in many pieces; only a table allocated once gains |
| glibc told to keep freed memory (`MALLOC_MMAP_THRESHOLD_`, `MALLOC_TRIM_THRESHOLD_`, `MALLOC_TOP_PAD_`) | No gain at 10 million states for 100 MiB more; -5% to -8% at 100 million with 4 workers for 350 MiB more | Memory for time is the owner's to choose, and setting it from the program is an `unsafe` call or another allocator |
| Rounds of fewer than 32 parts for one thread | 14 parts: +10.8% at 10 million states and +6.1% at 100 million; 24 parts: +2.5% at 10 million | Fewer look-ups reach each shard's table in a round and miss the cache; 32 is the smallest that costs no time |
| Several threads taking a level in rounds | Rounds of 1, 2 and 4 parts a thread: +68%, +46%, +20% at `grid_10m` with 4 workers | Slower, and more memory |
| Fixed-size copies for rows of 4 and 8 bytes | `grid_10m` -1.3% and -0.3% | Below its rule; the read of two overlapping fixed pieces was kept later (-3% to -5%) |
| Fields read from the packed row, as first written in rounds 4 and 5 | Every model slower | The packed path of round 10, prepared once for each model, was kept in its place |
| Evaluator forms of round 9: the expansion built in place, the chain run inline, two more forms of a comparison | -1% to -4% with one run +3%; `wide` -4.1% then -0.3%; `grid_10m` -2.1% and `wide` +2.1% | Below their rule |
| `Code::scalar` forced inline in the walk over input combinations | Gave back a fifth of the 4.7% that `wide` with 4 workers lost, cost 1 worker 0.6% | The walk was made one function that is neither generic nor inlined in its place |
| Round 9's direct path on values removed outright | `wide` +1.0% in instructions, by placement alone | `FieldOrder` is tested first in its place, which leaves +0.58% |
| Slice 3's walk over inputs picked from a set in each state (`every_pick` in `expand.rs`), reached any way but its pointer, `Picking::walk`, from the last branch of `general` | Calling `try_input` or `known_or_scalar` from it: they stop being inlined into `every_input_told_apart`, `wide` +3.7% to +5.8%. A new `FromRow` variant, never built: `ledger` +0.6%. A direct call from `general` or from `every_input_told_apart`, or a test at the head of `every_input_told_apart`: `ledger` +1.3% in `Search::level`, `wide` +1.7% | No model of `tasks/benchmark/models` picks from such a set, so each cost came from how the compiler laid out the code around it; through the pointer every count stays within 0.3% |
