| Group | 10m_j1 share | 10m_j1 CPU s | 10m_j4 share | 10m_j4 CPU s | 100m_j1 share | 100m_j1 CPU s | 100m_j4 share | 100m_j4 CPU s |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| (a) index of known states | 20.5% | 0.81 | 35.6% | 1.77 | 22.9% | 12.57 | 30.4% | 19.82 |
| (b) packed rows | 10.3% | 0.41 | 7.6% | 0.38 | 11.0% | 6.07 | 9.4% | 6.14 |
| (c) part tables, claim and merge | 29.2% | 1.15 | 22.8% | 1.14 | 32.2% | 17.68 | 28.6% | 18.64 |
| (d) actions and rules | 24.0% | 0.95 | 19.5% | 0.97 | 19.2% | 10.55 | 17.4% | 11.33 |
| (e) hashing | 3.5% | 0.14 | 2.8% | 0.14 | 2.6% | 1.43 | 2.7% | 1.78 |
| (f) allocation and page faults | 9.2% | 0.36 | 8.1% | 0.41 | 9.7% | 5.36 | 9.1% | 5.95 |
| (g) threads, waiting, other | 3.4% | 0.13 | 3.5% | 0.17 | 2.4% | 1.33 | 2.3% | 1.51 |
| samples | 20844 | 3.95 (median) | 25061 | 4.98 (median) | 62372 | 55.00 (median) | 78358 | 65.16 (median) |

| Run | a+b+c+e share | CPU s (median CPU) | CPU s (smallest CPU) | (d) CPU s | (d) smallest |
|---|---:|---:|---:|---:|---:|
| 10m_j1 | 63.4% | 2.50 | 2.40 | 0.95 | 0.91 |
| 10m_j4 | 68.9% | 3.43 | 3.36 | 0.97 | 0.95 |
| 100m_j1 | 68.7% | 37.76 | 36.96 | 10.55 | 10.33 |
| 100m_j4 | 71.2% | 46.37 | 45.29 | 11.33 | 11.07 |

### 10m_j1 top functions
| 26.30% | `earnest_check::store::level::gather::<alloc::vec::Vec<alloc::vec::Vec<usize>>, <earnest_check::store::index::Shards<earnest_check::store::index::Key<u` | a 67%, c 33% |
| 19.26% | `<earnest_check::explore::Search>::work` | d 59%, e 17%, c 12%, b 9%, a 2%, g 1% |
| 15.49% | `<earnest_check::explore::part::PartSink>::offer_encoded` | c 100% |
| 6.88% | `<earnest_ir::code::cond::Cond>::eval` | d 100% |
| 5.11% | `<earnest_ir::code::num::Num>::eval` | d 100% |
| 4.69% | `<earnest_check::store::rows::Rows>::unpack` | b 100% |
| 3.13% | `<earnest_check::store::Store>::read_state` | b 100% |
| 2.97% | `__memmove_avx512_unaligned_erms (libc)` | g 81%, f 19% |
| 2.34% | `<hashbrown::raw::RawTable<earnest_check::store::index::Key<u32>>>::reserve_rehash::<<earnest_check::store::index::Shards<earnest_check::store::index::` | a 93%, e 7% |
| 2.12% | `earnest_check::store::level::write_level::{closure#0}` | c 66%, b 34% |

### 10m_j4 top functions
| 39.29% | `earnest_check::store::level::gather::<alloc::vec::Vec<alloc::vec::Vec<usize>>, <earnest_check::store::index::Shards<earnest_check::store::index::Key<u` | a 85%, c 15% |
| 15.39% | `<earnest_check::explore::Search>::work` | d 58%, e 18%, c 13%, b 9%, a 1%, g 1% |
| 13.58% | `<earnest_check::explore::part::PartSink>::offer_encoded` | c 100% |
| 5.88% | `<earnest_ir::code::cond::Cond>::eval` | d 100% |
| 4.24% | `<earnest_ir::code::num::Num>::eval` | d 100% |
| 3.30% | `<earnest_check::store::rows::Rows>::unpack` | b 100% |
| 2.51% | `<earnest_check::store::Store>::read_state` | b 100% |
| 2.42% | `__memmove_avx512_unaligned_erms (libc)` | g 79%, f 21% |
| 1.99% | `<hashbrown::raw::RawTable<earnest_check::store::index::Key<u32>>>::reserve_rehash::<<earnest_check::store::index::Shards<earnest_check::store::index::` | a 94%, e 6% |
| 1.43% | `[k] exc_page_fault` | f 100% |

### 100m_j1 top functions
| 28.73% | `earnest_check::store::level::gather::<alloc::vec::Vec<alloc::vec::Vec<usize>>, <earnest_check::store::index::Shards<earnest_check::store::index::Key<u` | a 73%, c 27% |
| 21.27% | `<earnest_check::explore::part::PartSink>::offer_encoded` | c 100% |
| 15.52% | `<earnest_check::explore::Search>::work` | d 57%, e 16%, c 12%, b 9%, a 4%, g 1% |
| 6.74% | `<earnest_check::store::rows::Rows>::unpack` | b 100% |
| 5.76% | `<earnest_ir::code::cond::Cond>::eval` | d 100% |
| 4.19% | `<earnest_ir::code::num::Num>::eval` | d 100% |
| 2.41% | `__memmove_avx512_unaligned_erms (libc)` | g 80%, f 20% |
| 2.23% | `[k] exc_page_fault` | f 100% |
| 2.20% | `<earnest_check::store::Store>::read_state` | b 100% |
| 1.38% | `earnest_check::store::level::write_level::{closure#0}` | c 53%, b 47% |

### 100m_j4 top functions
| 33.20% | `earnest_check::store::level::gather::<alloc::vec::Vec<alloc::vec::Vec<usize>>, <earnest_check::store::index::Shards<earnest_check::store::index::Key<u` | a 87%, c 13% |
| 21.38% | `<earnest_check::explore::part::PartSink>::offer_encoded` | c 100% |
| 14.19% | `<earnest_check::explore::Search>::work` | d 54%, e 19%, c 12%, b 10%, a 4%, g 1% |
| 5.46% | `<earnest_ir::code::cond::Cond>::eval` | d 100% |
| 5.44% | `<earnest_check::store::rows::Rows>::unpack` | b 100% |
| 3.79% | `<earnest_ir::code::num::Num>::eval` | d 100% |
| 2.18% | `__memmove_avx512_unaligned_erms (libc)` | g 79%, f 21% |
| 2.13% | `[k] exc_page_fault` | f 100% |
| 2.04% | `<earnest_check::store::Store>::read_state` | b 100% |
| 1.23% | `earnest_check::store::level::write_level::{closure#0}` | c 54%, b 46% |

### subgroups 10m_j1
| a | index: hashbrown in store/level.rs | 14.36% |
| d | eval: earnest_ir | 13.86% |
| c | part table: hashbrown in explore/part.rs | 12.53% |
| c | parts/claim: store/level.rs | 11.03% |
| d | eval: expand.rs | 9.21% |
| f | kernel: page faults | 5.96% |
| b | rows: store/rows.rs | 5.40% |
| c | parts/claim: explore/part.rs | 5.15% |
| a | index: store/index.rs | 3.72% |
| e | hash: store/layout.rs mix/hash_words | 3.48% |
| g | libc memmove/memset (caller not resolved) | 2.86% |
| b | rows: store/layout.rs | 2.70% |
| a | index: hashbrown outlined ({closure#1}>) | 2.16% |
| f | alloc: libc malloc/realloc/free | 1.58% |
| b | rows: store/codec.rs | 1.36% |
| f | kernel: mmap/munmap/madvise | 1.12% |
| d | eval (by name) | 0.91% |
| b | rows: store.rs read_state | 0.72% |
| f | alloc: libc memmove inside realloc | 0.58% |
| c | parts/claim (by name) | 0.30% |
| g | loop: explore/workers.rs | 0.22% |
| a | index: hashbrown in store/index.rs | 0.20% |
| c | part table: hashbrown outlined () | 0.16% |
| b | rows (by name) | 0.10% |
kernel sites
| kernel: page faults | libc.so.6 | 3.08% |
| kernel: page faults | parts/claim: explore/part.rs | 1.54% |
| kernel: page faults | index: hashbrown outlined ({closure#1}>) | 1.32% |
| kernel: mmap/munmap/madvise | libc.so.6 | 1.12% |
| kernel: page faults | ld-linux-x86-64.so.2 | 0.02% |
| kernel: thread start/exit, exec | hash: store/layout.rs mix/hash_words | 0.01% |
### subgroups 10m_j4
| a | index: hashbrown in store/level.rs | 30.76% |
| d | eval: earnest_ir | 11.36% |
| c | part table: hashbrown in explore/part.rs | 10.83% |
| d | eval: expand.rs | 7.29% |
| c | parts/claim: store/level.rs | 6.83% |
| f | kernel: page faults | 4.69% |
| c | parts/claim: explore/part.rs | 4.52% |
| b | rows: store/rows.rs | 3.75% |
| a | index: store/index.rs | 2.94% |
| e | hash: store/layout.rs mix/hash_words | 2.85% |
| g | libc memmove/memset (caller not resolved) | 2.27% |
| b | rows: store/layout.rs | 2.17% |
| a | index: hashbrown outlined ({closure#1}>) | 1.88% |
| f | kernel: mmap/munmap/madvise | 1.69% |
| f | alloc: libc malloc/realloc/free | 1.24% |
| b | rows: store/codec.rs | 1.14% |
| d | eval (by name) | 0.84% |
| g | kernel: thread start/exit, exec | 0.61% |
| f | alloc: libc memmove inside realloc | 0.51% |
| b | rows: store.rs read_state | 0.49% |
| c | part table: hashbrown outlined () | 0.36% |
| c | parts/claim (by name) | 0.27% |
| g | loop: explore/workers.rs | 0.19% |
| g | kernel: futex (lock wait / wake) | 0.10% |
kernel sites
| kernel: page faults | libc.so.6 | 2.33% |
| kernel: mmap/munmap/madvise | libc.so.6 | 1.67% |
| kernel: page faults | parts/claim: explore/part.rs | 1.36% |
| kernel: page faults | index: hashbrown outlined ({closure#1}>) | 1.00% |
| kernel: thread start/exit, exec | libc.so.6 | 0.35% |
| kernel: futex (lock wait / wake) | libc.so.6 | 0.10% |
### subgroups 100m_j1
| c | part table: hashbrown in explore/part.rs | 18.07% |
| a | index: hashbrown in store/level.rs | 13.15% |
| d | eval: earnest_ir | 11.26% |
| c | parts/claim: store/level.rs | 8.80% |
| a | index: store/index.rs | 8.50% |
| f | kernel: page faults | 7.72% |
| b | rows: store/rows.rs | 7.39% |
| d | eval: expand.rs | 7.25% |
| c | parts/claim: explore/part.rs | 4.83% |
| e | hash: store/layout.rs mix/hash_words | 2.61% |
| g | libc memmove/memset (caller not resolved) | 2.19% |
| b | rows: store/layout.rs | 2.09% |
| f | kernel: mmap/munmap/madvise | 1.28% |
| a | index: hashbrown outlined ({closure#1}>) | 1.18% |
| b | rows: store/codec.rs | 0.99% |
| d | eval (by name) | 0.68% |
| b | rows: store.rs read_state | 0.49% |
| f | alloc: libc memmove inside realloc | 0.49% |
| c | parts/claim (by name) | 0.37% |
| f | alloc: libc malloc/realloc/free | 0.24% |
| g | loop: explore/workers.rs | 0.15% |
| c | part table: hashbrown outlined () | 0.09% |
| b | rows (by name) | 0.06% |
| g | kernel: thread start/exit, exec | 0.04% |
kernel sites
| kernel: page faults | parts/claim: explore/part.rs | 3.83% |
| kernel: page faults | libc.so.6 | 2.54% |
| kernel: mmap/munmap/madvise | libc.so.6 | 1.27% |
| kernel: page faults | index: hashbrown outlined ({closure#1}>) | 0.77% |
| kernel: page faults | parts/claim: store/level.rs | 0.58% |
| kernel: thread start/exit, exec | part table: hashbrown in explore/part.rs | 0.01% |
### subgroups 100m_j4
| a | index: hashbrown in store/level.rs | 21.89% |
| c | part table: hashbrown in explore/part.rs | 17.95% |
| d | eval: earnest_ir | 10.40% |
| a | index: store/index.rs | 7.57% |
| f | kernel: page faults | 7.14% |
| d | eval: expand.rs | 6.32% |
| b | rows: store/rows.rs | 6.01% |
| c | parts/claim: store/level.rs | 5.19% |
| c | parts/claim: explore/part.rs | 4.74% |
| e | hash: store/layout.rs mix/hash_words | 2.72% |
| b | rows: store/layout.rs | 1.97% |
| g | libc memmove/memset (caller not resolved) | 1.96% |
| f | kernel: mmap/munmap/madvise | 1.37% |
| a | index: hashbrown outlined ({closure#1}>) | 0.96% |
| b | rows: store/codec.rs | 0.94% |
| d | eval (by name) | 0.68% |
| c | parts/claim (by name) | 0.50% |
| f | alloc: libc memmove inside realloc | 0.46% |
| b | rows: store.rs read_state | 0.43% |
| c | part table: hashbrown outlined () | 0.22% |
| g | kernel: thread start/exit, exec | 0.17% |
| f | alloc: libc malloc/realloc/free | 0.16% |
| g | loop: explore/workers.rs | 0.13% |
| b | rows (by name) | 0.06% |
kernel sites
| kernel: page faults | parts/claim: explore/part.rs | 3.42% |
| kernel: page faults | libc.so.6 | 2.74% |
| kernel: mmap/munmap/madvise | libc.so.6 | 1.37% |
| kernel: page faults | index: hashbrown outlined ({closure#1}>) | 0.59% |
| kernel: page faults | parts/claim: store/level.rs | 0.29% |
| kernel: page faults | part table: hashbrown outlined () | 0.11% |
