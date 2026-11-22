| Group | 10m_j1 share | 10m_j1 CPU s | 10m_j4 share | 10m_j4 CPU s | 100m_j1 share | 100m_j1 CPU s | 100m_j4 share | 100m_j4 CPU s | wide_j1 share | wide_j1 CPU s |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| (a) index of known states | 20.3% | 0.58 | 33.1% | 1.25 | 20.0% | 8.00 | 30.8% | 15.58 | 3.6% | 0.01 |
| (b) packed rows | 10.9% | 0.31 | 9.2% | 0.35 | 14.8% | 5.91 | 12.7% | 6.41 | 7.6% | 0.02 |
| (c) part tables, claim and merge | 29.9% | 0.86 | 23.9% | 0.90 | 33.9% | 13.55 | 27.6% | 13.94 | 0.0% | 0.00 |
| (d) actions and rules | 22.7% | 0.65 | 19.0% | 0.72 | 18.8% | 7.52 | 15.5% | 7.82 | 76.3% | 0.25 |
| (e) hashing | 3.6% | 0.10 | 3.1% | 0.12 | 3.0% | 1.18 | 2.5% | 1.25 | 10.4% | 0.03 |
| (f) allocation and page faults | 9.5% | 0.27 | 8.5% | 0.32 | 6.9% | 2.75 | 9.1% | 4.61 | 0.5% | 0.00 |
| (g) threads, waiting, other | 3.2% | 0.09 | 3.2% | 0.12 | 2.7% | 1.06 | 1.9% | 0.98 | 1.7% | 0.01 |
| samples | 14786 | 2.88 (median) | 19725 | 3.78 (median) | 39686 | 39.98 (median) | 53080 | 50.58 (median) | 7308 | 0.32 (median) |

| Run | a+b+c+e share | CPU s (median CPU) | CPU s (smallest CPU) | (d) CPU s | (d) smallest |
|---|---:|---:|---:|---:|---:|
| 10m_j1 | 64.6% | 1.86 | 1.75 | 0.65 | 0.61 |
| 10m_j4 | 69.4% | 2.62 | 2.51 | 0.72 | 0.69 |
| 100m_j1 | 71.7% | 28.64 | 28.47 | 7.52 | 7.48 |
| 100m_j4 | 73.5% | 37.18 | 36.89 | 7.82 | 7.75 |
| wide_j1 | 21.6% | 0.07 | 0.07 | 0.25 | 0.24 |

### 10m_j1 top functions
| 24.20% | `<earnest_check::explore::Search>::work` | d 59%, c 17%, e 14%, b 8%, a 1%, g 1% |
| 19.55% | `earnest_check::store::level::gather::<alloc::vec::Vec<alloc::vec::Vec<usize>>, <earnest_check::store::index::Shards<earnest_check::store::index::Key<u` | a 88%, c 12% |
| 18.05% | `<earnest_check::explore::part::PartSink>::offer_encoded` | c 100% |
| 4.87% | `<earnest_ir::code::Code>::scalar` | d 100% |
| 4.47% | `<earnest_check::store::rows::Rows>::unpack` | b 100% |
| 4.13% | `earnest_check::store::level::write_level::{closure#0}` | c 83%, b 17% |
| 3.81% | `<earnest_check::store::Store>::read_state` | b 100%, a 0% |
| 3.34% | `<earnest_ir::code::cond::Cond>::leaf` | d 100% |
| 2.68% | `<hashbrown::raw::RawTable<earnest_check::store::index::Key<u32>>>::reserve_rehash::<<earnest_check::store::index::Shards<earnest_check::store::index::` | a 93%, e 7% |
| 1.93% | `[k] exc_page_fault` | f 100% |

### 10m_j4 top functions
| 32.66% | `earnest_check::store::level::gather::<alloc::vec::Vec<alloc::vec::Vec<usize>>, <earnest_check::store::index::Shards<earnest_check::store::index::Key<u` | a 94%, c 6% |
| 20.66% | `<earnest_check::explore::Search>::work` | d 57%, c 17%, e 14%, b 9%, a 2%, g 1% |
| 15.72% | `<earnest_check::explore::part::PartSink>::offer_encoded` | c 100% |
| 4.20% | `<earnest_ir::code::Code>::scalar` | d 100% |
| 4.05% | `<earnest_check::store::rows::Rows>::unpack` | b 100% |
| 2.93% | `<earnest_ir::code::cond::Cond>::leaf` | d 100% |
| 2.82% | `<earnest_check::store::Store>::read_state` | b 100% |
| 2.44% | `earnest_check::store::level::write_level::{closure#0}` | c 81%, b 19% |
| 2.16% | `<hashbrown::raw::RawTable<earnest_check::store::index::Key<u32>>>::reserve_rehash::<<earnest_check::store::index::Shards<earnest_check::store::index::` | a 92%, e 8% |
| 1.74% | `[k] exc_page_fault` | f 100% |

### 100m_j1 top functions
| 23.53% | `<earnest_check::explore::Search>::work` | d 49%, c 26%, e 12%, b 8%, a 4%, g 1% |
| 22.42% | `<earnest_check::explore::part::PartSink>::offer_encoded` | c 100% |
| 19.36% | `earnest_check::store::level::gather::<alloc::vec::Vec<alloc::vec::Vec<usize>>, <earnest_check::store::index::Shards<earnest_check::store::index::Key<u` | a 91%, c 9% |
| 9.58% | `<earnest_check::store::rows::Rows>::unpack` | b 100% |
| 4.42% | `<earnest_ir::code::Code>::scalar` | d 100% |
| 2.89% | `earnest_check::store::level::write_level::{closure#0}` | c 80%, b 20% |
| 2.83% | `<earnest_check::store::Store>::read_state` | b 100%, a 0% |
| 2.75% | `<earnest_ir::code::cond::Cond>::leaf` | d 100% |
| 1.76% | `__memset_avx512_unaligned_erms (libc)` | g 100% |
| 1.62% | `<hashbrown::raw::RawTable<earnest_check::store::index::Key<u32>>>::reserve_rehash::<<earnest_check::store::index::Shards<earnest_check::store::index::` | a 94%, e 6% |

### 100m_j4 top functions
| 30.07% | `earnest_check::store::level::gather::<alloc::vec::Vec<alloc::vec::Vec<usize>>, <earnest_check::store::index::Shards<earnest_check::store::index::Key<u` | a 95%, c 5% |
| 19.03% | `<earnest_check::explore::Search>::work` | d 49%, c 26%, e 12%, b 8%, a 4%, g 1% |
| 18.47% | `<earnest_check::explore::part::PartSink>::offer_encoded` | c 100% |
| 8.20% | `<earnest_check::store::rows::Rows>::unpack` | b 100% |
| 3.64% | `<earnest_ir::code::Code>::scalar` | d 100% |
| 2.55% | `<earnest_check::store::Store>::read_state` | b 100% |
| 2.52% | `<earnest_ir::code::cond::Cond>::leaf` | d 100% |
| 2.22% | `[k] exc_page_fault` | f 100% |
| 2.16% | `earnest_check::store::level::write_level::{closure#0}` | c 82%, b 18% |
| 1.38% | `<hashbrown::raw::RawTable<earnest_check::store::index::Key<u32>>>::reserve_rehash::<<earnest_check::store::index::Shards<earnest_check::store::index::` | a 95%, e 5% |

### wide_j1 top functions
| 42.38% | `<earnest_check::explore::Search>::level` | d 58%, e 25%, b 18%, g 0% |
| 30.13% | `<earnest_ir::code::num::Num>::rest` | d 100% |
| 19.79% | `<earnest_check::expand::Seen>::first_row` | d 100% |
| 3.53% | `<earnest_check::store::Store>::place` | a 100% |
| 1.53% | `<earnest_ir::code::num::Num>::eval` | d 100% |
| 0.81% | `<earnest_check::explore::here::HereSink>::place_row` | g 100% |
| 0.36% | `<earnest_ir::code::Code>::scalar` | d 100% |
| 0.10% | `get_common_cache_info.constprop.0 (libc)` | g 100% |
| 0.07% | `__cpu_indicator_init (/usr/lib/x86_64-linux-gnu/libgcc_s.so.1)` | g 100% |
| 0.05% | `<hashbrown::raw::RawTable<earnest_check::store::index::Key<u32>>>::reserve_rehash::<<earnest_check::store::index::Shards<earnest_check::store::index::` | a 100% |

### subgroups 10m_j1
| a | index: hashbrown in store/level.rs | 13.51% |
| d | eval: earnest_ir | 13.21% |
| c | part table: hashbrown in explore/part.rs | 12.11% |
| d | eval: expand.rs | 9.43% |
| c | parts/claim: explore/part.rs | 7.33% |
| c | parts/claim: store/level.rs | 7.32% |
| f | kernel: page faults | 6.17% |
| b | rows: store/rows.rs | 5.15% |
| a | index: store/index.rs | 3.97% |
| e | hash: store/layout.rs mix/hash_words | 3.56% |
| b | rows: store/layout.rs | 3.08% |
| c | parts/claim (by name) | 3.06% |
| g | libc memmove/memset (caller not resolved) | 2.67% |
| a | index: hashbrown outlined ({closure#1}>) | 2.49% |
| b | rows: store/codec.rs | 1.74% |
| f | alloc: libc malloc/realloc/free | 1.69% |
| f | kernel: mmap/munmap/madvise | 1.03% |
| b | rows: store.rs read_state | 0.81% |
| f | alloc: libc memmove inside realloc | 0.56% |
| a | index: hashbrown in store/index.rs | 0.26% |
| g | loop: explore/workers.rs | 0.22% |
| c | part table: hashbrown outlined () | 0.12% |
| b | rows (by name) | 0.10% |
| g | libc other: unlink_chunk.isra.0 | 0.09% |
kernel sites
| kernel: page faults | libc.so.6 | 4.59% |
| kernel: page faults | index: hashbrown outlined ({closure#1}>) | 1.28% |
| kernel: mmap/munmap/madvise | libc.so.6 | 1.03% |
| kernel: page faults | parts/claim: explore/part.rs | 0.30% |
| kernel: thread start/exit, exec | part table: hashbrown in explore/part.rs | 0.03% |
| kernel: thread start/exit, exec | libc.so.6 | 0.01% |
### subgroups 10m_j4
| a | index: hashbrown in store/level.rs | 27.83% |
| d | eval: earnest_ir | 11.47% |
| c | part table: hashbrown in explore/part.rs | 10.56% |
| d | eval: expand.rs | 7.48% |
| c | parts/claim: explore/part.rs | 6.08% |
| f | kernel: page faults | 5.10% |
| b | rows: store/rows.rs | 4.51% |
| c | parts/claim: store/level.rs | 4.10% |
| a | index: store/index.rs | 3.30% |
| e | hash: store/layout.rs mix/hash_words | 3.15% |
| c | parts/claim (by name) | 2.76% |
| b | rows: store/layout.rs | 2.65% |
| a | index: hashbrown outlined ({closure#1}>) | 1.98% |
| g | libc memmove/memset (caller not resolved) | 1.84% |
| f | kernel: mmap/munmap/madvise | 1.66% |
| f | alloc: libc malloc/realloc/free | 1.32% |
| b | rows: store/codec.rs | 1.26% |
| b | rows: store.rs read_state | 0.70% |
| g | kernel: thread start/exit, exec | 0.68% |
| c | part table: hashbrown outlined () | 0.40% |
| f | alloc: libc memmove inside realloc | 0.36% |
| g | loop: explore/workers.rs | 0.19% |
| g | kernel: futex (lock wait / wake) | 0.16% |
| g | libc other: unlink_chunk.isra.0 | 0.08% |
kernel sites
| kernel: page faults | libc.so.6 | 3.41% |
| kernel: mmap/munmap/madvise | libc.so.6 | 1.66% |
| kernel: page faults | index: hashbrown outlined ({closure#1}>) | 1.22% |
| kernel: page faults | parts/claim: explore/part.rs | 0.46% |
| kernel: thread start/exit, exec | libc.so.6 | 0.33% |
| kernel: futex (lock wait / wake) | libc.so.6 | 0.16% |
### subgroups 100m_j1
| c | part table: hashbrown in explore/part.rs | 12.41% |
| a | index: hashbrown in store/level.rs | 11.90% |
| d | eval: earnest_ir | 11.07% |
| b | rows: store/rows.rs | 10.16% |
| c | parts/claim: explore/part.rs | 9.23% |
| d | eval: expand.rs | 7.73% |
| c | parts/claim (by name) | 7.29% |
| a | index: store/index.rs | 6.57% |
| f | kernel: page faults | 5.21% |
| c | parts/claim: store/level.rs | 4.84% |
| e | hash: store/layout.rs mix/hash_words | 2.96% |
| b | rows: store/layout.rs | 2.62% |
| g | libc memmove/memset (caller not resolved) | 2.37% |
| a | index: hashbrown outlined ({closure#1}>) | 1.51% |
| b | rows: store/codec.rs | 1.24% |
| f | alloc: libc memmove inside realloc | 0.72% |
| f | kernel: mmap/munmap/madvise | 0.67% |
| b | rows: store.rs read_state | 0.64% |
| f | alloc: libc malloc/realloc/free | 0.28% |
| g | loop: explore/workers.rs | 0.15% |
| c | part table: hashbrown outlined () | 0.12% |
| b | rows (by name) | 0.10% |
| g | kernel: thread start/exit, exec | 0.09% |
| b | rows: store.rs outside | 0.03% |
kernel sites
| kernel: page faults | libc.so.6 | 2.35% |
| kernel: page faults | parts/claim: explore/part.rs | 1.50% |
| kernel: page faults | index: hashbrown outlined ({closure#1}>) | 0.96% |
| kernel: mmap/munmap/madvise | libc.so.6 | 0.67% |
| kernel: page faults | parts/claim: store/level.rs | 0.39% |
| kernel: thread start/exit, exec | eval: expand.rs | 0.02% |
### subgroups 100m_j4
| a | index: hashbrown in store/level.rs | 23.33% |
| c | part table: hashbrown in explore/part.rs | 9.92% |
| d | eval: earnest_ir | 9.33% |
| b | rows: store/rows.rs | 8.60% |
| c | parts/claim: explore/part.rs | 7.52% |
| f | kernel: page faults | 7.40% |
| a | index: store/index.rs | 6.17% |
| d | eval: expand.rs | 6.10% |
| c | parts/claim (by name) | 6.09% |
| c | parts/claim: store/level.rs | 3.76% |
| e | hash: store/layout.rs mix/hash_words | 2.46% |
| b | rows: store/layout.rs | 2.19% |
| g | libc memmove/memset (caller not resolved) | 1.57% |
| a | index: hashbrown outlined ({closure#1}>) | 1.31% |
| b | rows: store/codec.rs | 1.24% |
| f | kernel: mmap/munmap/madvise | 0.95% |
| b | rows: store.rs read_state | 0.56% |
| f | alloc: libc memmove inside realloc | 0.51% |
| c | part table: hashbrown outlined () | 0.27% |
| f | alloc: libc malloc/realloc/free | 0.26% |
| g | kernel: thread start/exit, exec | 0.19% |
| g | loop: explore/workers.rs | 0.12% |
| b | rows (by name) | 0.07% |
| d | eval (by name) | 0.02% |
kernel sites
| kernel: page faults | libc.so.6 | 4.85% |
| kernel: page faults | parts/claim: explore/part.rs | 1.48% |
| kernel: mmap/munmap/madvise | libc.so.6 | 0.95% |
| kernel: page faults | index: hashbrown outlined ({closure#1}>) | 0.76% |
| kernel: page faults | parts/claim: store/level.rs | 0.21% |
| kernel: page faults | part table: hashbrown outlined () | 0.10% |
### subgroups wide_j1
| d | eval: earnest_ir | 34.82% |
| d | eval: expand.rs | 23.62% |
| d | hashbrown in expand.rs | 16.49% |
| e | hash: store/layout.rs mix/hash_words | 10.41% |
| b | rows: store/layout.rs | 7.53% |
| a | index: hashbrown in store/index.rs | 1.86% |
| d | eval (by name) | 1.34% |
| a | index: store.rs place | 0.93% |
| g | loop: explore/here.rs | 0.79% |
| a | index: store/index.rs | 0.74% |
| f | kernel: mmap/munmap/madvise | 0.22% |
| f | kernel: page faults | 0.18% |
| g | kernel: thread start/exit, exec | 0.15% |
| g | libc other: get_common_cache_info.constprop.0 | 0.10% |
| g | kernel: file i/o | 0.10% |
| g | other dso: /usr/lib/x86_64-linux-gnu/libgcc_s.so.1 | 0.07% |
| a | index: hashbrown outlined ({closure#1}>) | 0.05% |
| f | alloc: libc malloc/realloc/free | 0.05% |
| g | libc memmove/memset (caller not resolved) | 0.04% |
| g | libc other: intel_check_word.constprop.0 | 0.04% |
| g | libc other: check_match | 0.03% |
| a? | hashbrown outlined, table not told: <hashbrown::raw::RawTable<(usize, u64)>>::reserve_rehash::<<earn | 0.03% |
| g | libc other: dl_main | 0.03% |
| g | libc other: do_lookup_x | 0.03% |
kernel sites
| kernel: page faults | libc.so.6 | 0.15% |
| kernel: mmap/munmap/madvise | libc.so.6 | 0.12% |
| kernel: mmap/munmap/madvise | ld-linux-x86-64.so.2 | 0.10% |
| kernel: file i/o | libc.so.6 | 0.07% |
| kernel: thread start/exit, exec | libc.so.6 | 0.07% |
| kernel: file i/o | ld-linux-x86-64.so.2 | 0.03% |
