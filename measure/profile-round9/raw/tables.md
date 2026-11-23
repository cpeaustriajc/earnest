| Group | 10m_j1 share | 10m_j1 CPU s | 10m_j4 share | 10m_j4 CPU s | 100m_j1 share | 100m_j1 CPU s | 100m_j4 share | 100m_j4 CPU s | wide_j1 share | wide_j1 CPU s |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| (a) index of known states | 21.9% | 0.56 | 34.4% | 1.24 | 21.3% | 8.51 | 33.4% | 16.35 | 7.1% | 0.02 |
| (b) packed rows | 11.7% | 0.30 | 8.0% | 0.29 | 16.1% | 6.44 | 11.2% | 5.45 | 0.2% | 0.00 |
| (c) part tables, claim and merge | 30.4% | 0.77 | 25.0% | 0.90 | 33.1% | 13.23 | 27.3% | 13.35 | 0.0% | 0.00 |
| (d) actions and rules | 23.9% | 0.60 | 19.5% | 0.70 | 21.5% | 8.61 | 16.6% | 8.12 | 88.4% | 0.24 |
| (e) hashing | 4.3% | 0.11 | 3.4% | 0.12 | 3.1% | 1.26 | 2.7% | 1.31 | 1.3% | 0.00 |
| (f) allocation and page faults | 5.4% | 0.14 | 6.8% | 0.24 | 2.6% | 1.03 | 7.1% | 3.45 | 0.5% | 0.00 |
| (g) threads, waiting, other | 2.5% | 0.06 | 3.0% | 0.11 | 2.2% | 0.89 | 1.8% | 0.86 | 2.5% | 0.01 |
| samples | 13125 | 2.53 (median) | 15789 | 3.60 (median) | 40608 | 39.97 (median) | 50391 | 48.88 (median) | 5303 | 0.27 (median) |

| Run | a+b+c+e share | CPU s (median CPU) | CPU s (smallest CPU) | (d) CPU s | (d) smallest |
|---|---:|---:|---:|---:|---:|
| 10m_j1 | 68.2% | 1.73 | 1.59 | 0.60 | 0.56 |
| 10m_j4 | 70.8% | 2.55 | 2.44 | 0.70 | 0.67 |
| 100m_j1 | 73.6% | 29.44 | 26.67 | 8.61 | 7.80 |
| 100m_j4 | 74.6% | 36.46 | 35.34 | 8.12 | 7.87 |
| wide_j1 | 8.6% | 0.02 | 0.02 | 0.24 | 0.23 |

### 10m_j1 top functions
| 24.88% | `<earnest_check::explore::Search>::work` | d 62%, c 17%, e 16%, b 2%, g 2%, a 1% |
| 20.52% | `earnest_check::store::level::gather::<alloc::vec::Vec<alloc::vec::Vec<u32>>, <earnest_check::store::index::Shards<earnest_check::store::index::Key<u32` | a 91%, c 9% |
| 20.45% | `<earnest_check::explore::part::PartSink>::offer_encoded` | c 100% |
| 6.04% | `<earnest_check::store::rows::Rows>::unpack` | b 100% |
| 5.04% | `<earnest_ir::code::Code>::scalar` | d 100% |
| 3.99% | `<earnest_check::store::Store>::read_state` | b 100% |
| 3.39% | `earnest_check::store::level::write_level::{closure#0}` | c 80%, b 20% |
| 3.22% | `<earnest_ir::code::cond::Cond>::leaf` | d 100% |
| 2.99% | `<hashbrown::raw::RawTable<earnest_check::store::index::Key<u32>>>::reserve_rehash::<<earnest_check::store::index::Shards<earnest_check::store::index::` | a 94%, e 6% |
| 1.14% | `<earnest_check::store::level::Winners>::of` | c 100% |

### 10m_j4 top functions
| 33.68% | `earnest_check::store::level::gather::<alloc::vec::Vec<alloc::vec::Vec<u32>>, <earnest_check::store::index::Shards<earnest_check::store::index::Key<u32` | a 94%, c 6% |
| 20.15% | `<earnest_check::explore::Search>::work` | d 62%, c 16%, e 16%, b 2%, g 2%, a 1% |
| 17.33% | `<earnest_check::explore::part::PartSink>::offer_encoded` | c 100% |
| 4.15% | `<earnest_ir::code::Code>::scalar` | d 100% |
| 3.67% | `<earnest_check::store::rows::Rows>::unpack` | b 100% |
| 3.19% | `<earnest_check::store::Store>::read_state` | b 100% |
| 2.77% | `<earnest_ir::code::cond::Cond>::leaf` | d 100% |
| 2.56% | `<hashbrown::raw::RawTable<earnest_check::store::index::Key<u32>>>::reserve_rehash::<<earnest_check::store::index::Shards<earnest_check::store::index::` | a 95%, e 5% |
| 2.00% | `earnest_check::store::level::write_level::{closure#0}` | c 81%, b 19% |
| 1.26% | `[k] exc_page_fault` | f 100% |

### 100m_j1 top functions
| 24.13% | `<earnest_check::explore::Search>::work` | d 57%, c 24%, e 13%, a 4%, b 2%, g 1% |
| 23.66% | `<earnest_check::explore::part::PartSink>::offer_encoded` | c 100% |
| 20.09% | `earnest_check::store::level::gather::<alloc::vec::Vec<alloc::vec::Vec<u32>>, <earnest_check::store::index::Shards<earnest_check::store::index::Key<u32` | a 94%, c 6% |
| 11.91% | `<earnest_check::store::rows::Rows>::unpack` | b 100% |
| 4.67% | `<earnest_ir::code::Code>::scalar` | d 100% |
| 3.22% | `<earnest_ir::code::cond::Cond>::leaf` | d 100% |
| 2.91% | `<earnest_check::store::Store>::read_state` | b 100% |
| 2.37% | `earnest_check::store::level::write_level::{closure#0}` | c 72%, b 28% |
| 1.67% | `<hashbrown::raw::RawTable<earnest_check::store::index::Key<u32>>>::reserve_rehash::<<earnest_check::store::index::Shards<earnest_check::store::index::` | a 95%, e 5% |
| 1.29% | `__memset_avx512_unaligned_erms (libc)` | g 100% |

### 100m_j4 top functions
| 32.50% | `earnest_check::store::level::gather::<alloc::vec::Vec<alloc::vec::Vec<u32>>, <earnest_check::store::index::Shards<earnest_check::store::index::Key<u32` | a 97%, c 3% |
| 19.29% | `<earnest_check::explore::Search>::work` | d 55%, c 25%, e 13%, a 4%, b 2%, g 1% |
| 19.12% | `<earnest_check::explore::part::PartSink>::offer_encoded` | c 100% |
| 7.87% | `<earnest_check::store::rows::Rows>::unpack` | b 100% |
| 3.55% | `<earnest_ir::code::Code>::scalar` | d 100% |
| 2.50% | `<earnest_ir::code::cond::Cond>::leaf` | d 100% |
| 2.32% | `<earnest_check::store::Store>::read_state` | b 100% |
| 1.95% | `earnest_check::store::level::write_level::{closure#0}` | c 78%, b 22% |
| 1.86% | `[k] exc_page_fault` | f 100% |
| 1.39% | `<hashbrown::raw::RawTable<earnest_check::store::index::Key<u32>>>::reserve_rehash::<<earnest_check::store::index::Shards<earnest_check::store::index::` | a 93%, e 7% |

### wide_j1 top functions
| 53.89% | `<earnest_check::explore::Search>::level` | d 97%, e 2%, g 0%, b 0% |
| 35.21% | `<earnest_ir::code::num::Num>::rest` | d 100% |
| 7.07% | `<earnest_check::store::Store>::place` | a 100%, b 0% |
| 1.04% | `<earnest_check::explore::here::HereSink>::place_row` | g 100% |
| 0.94% | `<earnest_ir::code::Code>::scalar` | d 100% |
| 0.23% | `__cpu_indicator_init (/usr/lib/x86_64-linux-gnu/libgcc_s.so.1)` | g 100% |
| 0.09% | `[k] _raw_spin_unlock_irqrestore` | g 60%, f 40% |
| 0.08% | `intel_check_word.constprop.0 (libc)` | g 100% |
| 0.08% | `get_common_cache_info.constprop.0 (libc)` | g 100% |
| 0.08% | `__memset_avx512_unaligned_erms (libc)` | g 100% |

### subgroups 10m_j1
| c | part table: hashbrown in explore/part.rs | 16.45% |
| a | index: hashbrown in store/level.rs | 13.02% |
| d | eval: expand.rs | 10.13% |
| c | parts/claim: explore/part.rs | 8.01% |
| b | rows: store/rows.rs | 6.71% |
| a | index: store/index.rs | 5.85% |
| c | parts/claim: store/level.rs | 5.71% |
| d | eval: earnest_ir/code/num.rs | 4.99% |
| e | hash: store/layout.rs mix/hash_words | 4.27% |
| d | eval: earnest_ir/eval.rs | 4.08% |
| f | kernel: page faults | 3.55% |
| d | eval: earnest_ir/code/cond.rs | 3.37% |
| a | index: hashbrown outlined ({closure#1}>) | 2.80% |
| g | libc memmove/memset (caller not resolved) | 1.76% |
| b | rows: store/layout.rs | 1.74% |
| b | rows: store/codec.rs | 1.71% |
| d | eval: earnest_ir/code/mod.rs | 1.26% |
| f | alloc: libc malloc/realloc/free | 1.16% |
| b | rows: store.rs read_state | 1.01% |
| f | kernel: mmap/munmap/madvise | 0.67% |
| g | loop: explore/workers.rs | 0.44% |
| b | rows: store.rs outside | 0.37% |
| a | index: hashbrown in store/index.rs | 0.18% |
| c | parts/claim (by name) | 0.18% |
kernel sites
| kernel: page faults | libc.so.6 | 1.98% |
| kernel: page faults | index: hashbrown outlined ({closure#1}>) | 1.46% |
| kernel: mmap/munmap/madvise | libc.so.6 | 0.66% |
| kernel: page faults | parts/claim: explore/part.rs | 0.08% |
| kernel: thread start/exit, exec | index: hashbrown in store/level.rs | 0.02% |
| kernel: file i/o | libc.so.6 | 0.02% |
### subgroups 10m_j4
| a | index: hashbrown in store/level.rs | 29.46% |
| c | part table: hashbrown in explore/part.rs | 13.94% |
| d | eval: expand.rs | 8.08% |
| c | parts/claim: explore/part.rs | 6.54% |
| d | eval: earnest_ir/code/num.rs | 4.38% |
| b | rows: store/rows.rs | 4.05% |
| c | parts/claim: store/level.rs | 3.90% |
| f | kernel: page faults | 3.72% |
| d | eval: earnest_ir/eval.rs | 3.48% |
| e | hash: store/layout.rs mix/hash_words | 3.39% |
| d | eval: earnest_ir/code/cond.rs | 2.51% |
| a | index: store/index.rs | 2.45% |
| a | index: hashbrown outlined ({closure#1}>) | 2.43% |
| b | rows: store/codec.rs | 1.56% |
| f | kernel: mmap/munmap/madvise | 1.52% |
| g | libc memmove/memset (caller not resolved) | 1.41% |
| b | rows: store/layout.rs | 1.39% |
| f | alloc: libc malloc/realloc/free | 1.15% |
| d | eval: earnest_ir/code/mod.rs | 1.01% |
| g | kernel: thread start/exit, exec | 0.84% |
| b | rows: store.rs read_state | 0.63% |
| c | part table: hashbrown outlined () | 0.44% |
| g | loop: explore/workers.rs | 0.41% |
| f | alloc: libc memmove inside realloc | 0.37% |
kernel sites
| kernel: page faults | libc.so.6 | 2.36% |
| kernel: mmap/munmap/madvise | libc.so.6 | 1.51% |
| kernel: page faults | index: hashbrown outlined ({closure#1}>) | 1.24% |
| kernel: thread start/exit, exec | libc.so.6 | 0.39% |
| kernel: thread start/exit, exec | index: hashbrown in store/level.rs | 0.15% |
| kernel: futex (lock wait / wake) | libc.so.6 | 0.11% |
### subgroups 100m_j1
| c | part table: hashbrown in explore/part.rs | 19.21% |
| b | rows: store/rows.rs | 12.57% |
| a | index: hashbrown in store/level.rs | 11.49% |
| c | parts/claim: explore/part.rs | 9.98% |
| d | eval: expand.rs | 8.94% |
| a | index: store/index.rs | 8.20% |
| d | eval: earnest_ir/code/num.rs | 4.22% |
| c | parts/claim: store/level.rs | 3.58% |
| d | eval: earnest_ir/eval.rs | 3.35% |
| d | eval: earnest_ir/code/cond.rs | 3.29% |
| e | hash: store/layout.rs mix/hash_words | 3.15% |
| f | kernel: page faults | 2.12% |
| g | libc memmove/memset (caller not resolved) | 1.81% |
| d | eval: earnest_ir/code/mod.rs | 1.69% |
| a | index: hashbrown outlined ({closure#1}>) | 1.60% |
| b | rows: store/codec.rs | 1.32% |
| b | rows: store/layout.rs | 1.27% |
| b | rows: store.rs read_state | 0.66% |
| g | loop: explore/workers.rs | 0.33% |
| f | kernel: mmap/munmap/madvise | 0.32% |
| c | parts/claim (by name) | 0.32% |
| b | rows: store.rs outside | 0.22% |
| f | alloc: libc malloc/realloc/free | 0.14% |
| b | rows (by name) | 0.08% |
kernel sites
| kernel: page faults | libc.so.6 | 1.11% |
| kernel: page faults | index: hashbrown outlined ({closure#1}>) | 0.78% |
| kernel: mmap/munmap/madvise | libc.so.6 | 0.32% |
| kernel: page faults | parts/claim: store/level.rs | 0.15% |
| kernel: page faults | parts/claim: explore/part.rs | 0.07% |
| kernel: thread start/exit, exec | index: store/index.rs | 0.01% |
### subgroups 100m_j4
| a | index: hashbrown in store/level.rs | 25.95% |
| c | part table: hashbrown in explore/part.rs | 16.18% |
| b | rows: store/rows.rs | 8.31% |
| c | parts/claim: explore/part.rs | 7.65% |
| d | eval: expand.rs | 7.01% |
| a | index: store/index.rs | 6.20% |
| f | kernel: page faults | 5.72% |
| d | eval: earnest_ir/code/num.rs | 3.30% |
| c | parts/claim: store/level.rs | 3.04% |
| e | hash: store/layout.rs mix/hash_words | 2.68% |
| d | eval: earnest_ir/eval.rs | 2.64% |
| d | eval: earnest_ir/code/cond.rs | 2.45% |
| a | index: hashbrown outlined ({closure#1}>) | 1.29% |
| g | libc memmove/memset (caller not resolved) | 1.24% |
| d | eval: earnest_ir/code/mod.rs | 1.17% |
| b | rows: store/codec.rs | 1.08% |
| b | rows: store/layout.rs | 1.05% |
| f | kernel: mmap/munmap/madvise | 0.80% |
| b | rows: store.rs read_state | 0.45% |
| f | alloc: libc memmove inside realloc | 0.35% |
| c | part table: hashbrown outlined () | 0.26% |
| g | loop: explore/workers.rs | 0.26% |
| f | alloc: libc malloc/realloc/free | 0.19% |
| b | rows: store.rs outside | 0.19% |
kernel sites
| kernel: page faults | libc.so.6 | 4.33% |
| kernel: mmap/munmap/madvise | libc.so.6 | 0.79% |
| kernel: page faults | index: hashbrown outlined ({closure#1}>) | 0.68% |
| kernel: page faults | parts/claim: explore/part.rs | 0.56% |
| kernel: page faults | part table: hashbrown outlined () | 0.10% |
| kernel: thread start/exit, exec | index: hashbrown in store/level.rs | 0.05% |
### subgroups wide_j1
| d | eval: expand.rs | 47.29% |
| d | eval: earnest_ir/code/chain.rs | 28.06% |
| d | eval: earnest_ir/code/num.rs | 7.15% |
| a | index: hashbrown in store/index.rs | 4.51% |
| d | eval: earnest_ir/eval.rs | 2.70% |
| d | eval: earnest_ir/code/mod.rs | 2.24% |
| a | index: store/index.rs | 1.64% |
| e | hash: store/layout.rs mix/hash_words | 1.32% |
| g | loop: explore/here.rs | 1.26% |
| d | eval (by name) | 0.98% |
| a | index: store.rs place | 0.91% |
| f | kernel: page faults | 0.25% |
| g | other dso: /usr/lib/x86_64-linux-gnu/libgcc_s.so.1 | 0.23% |
| g | kernel: thread start/exit, exec | 0.23% |
| f | kernel: mmap/munmap/madvise | 0.19% |
| g | libc memmove/memset (caller not resolved) | 0.13% |
| g | kernel: file i/o | 0.11% |
| b | rows: store/layout.rs | 0.09% |
| g | libc other: intel_check_word.constprop.0 | 0.08% |
| g | libc other: get_common_cache_info.constprop.0 | 0.08% |
| f | alloc: libc malloc/realloc/free | 0.08% |
| b | rows: store/rows.rs | 0.06% |
| g | libc other: do_lookup_x | 0.04% |
| g | libc other: cfree@GLIBC_2.2.5 | 0.04% |
kernel sites
| kernel: page faults | libc.so.6 | 0.19% |
| kernel: thread start/exit, exec | libc.so.6 | 0.13% |
| kernel: mmap/munmap/madvise | libc.so.6 | 0.09% |
| kernel: mmap/munmap/madvise | ld-linux-x86-64.so.2 | 0.09% |
| kernel: file i/o | libc.so.6 | 0.09% |
| kernel: thread start/exit, exec | eval: expand.rs | 0.08% |
