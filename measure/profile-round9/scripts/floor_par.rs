// floor_ii.rs of measure/profile-lone (serial and the parts/shards/filter mode, unchanged above `checksum`),
// with three additions: `shared N`, one table shared by all threads with slots claimed by compare-and-swap;
// `gen N`, the generating of successors alone (no table); and an order-free checksum of the set of states
// with the CPU time of the search alone, from getrusage, printed for every mode.
use std::cell::UnsafeCell;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Barrier;
use std::time::{Duration, Instant};

const RING: usize = 16;

#[inline(always)]
fn mix(mut x: u64) -> u64 {
    x = (x ^ (x >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94D049BB133111EB);
    x ^ (x >> 31)
}

#[inline(always)]
fn prefetch<T>(p: *const T) {
    #[cfg(target_arch = "aarch64")]
    unsafe {
        core::arch::asm!("prfm pldl1keep, [{x}]", x = in(reg) p, options(nostack, preserves_flags, readonly));
    }
    #[cfg(target_arch = "x86_64")]
    unsafe {
        core::arch::x86_64::_mm_prefetch(p as *const i8, core::arch::x86_64::_MM_HINT_T0)
    }
    #[cfg(not(any(target_arch = "aarch64", target_arch = "x86_64")))]
    let _ = p;
}

fn low_mask(wheels: u32) -> u64 {
    (0..wheels).fold(0u64, |m, i| m | (1u64 << (4 * i)))
}

#[inline(always)]
fn violates(s: u64, low: u64) -> bool {
    (s >> 3) & ((s >> 2) | (s >> 1)) & low != 0
}


// ---------------------------------------------------------------------------------------------------
// Memory for the tables (floor_par only). HUGE=1 asks for transparent huge pages (madvise); PREFAULT=1
// (the default) writes one word of every 4 KiB page of a serial or part table when it is made, so no page
// is first mapped to the zero page and then copied on its first write.
// ---------------------------------------------------------------------------------------------------
static HUGE: AtomicUsize = AtomicUsize::new(0);
static PREFAULT: AtomicUsize = AtomicUsize::new(1);

unsafe extern "C" {
    fn madvise(addr: *mut u8, len: usize, advice: i32) -> i32;
}
const MADV_HUGEPAGE: i32 = 14;

fn advise(p: *mut u8, bytes: usize) {
    if HUGE.load(Ordering::Relaxed) != 0 && bytes >= (2 << 20) {
        // round inward to 2 MiB; the allocator's big blocks are mmap'ed and page aligned
        let a = (p as usize + (2 << 20) - 1) & !((2 << 20) - 1);
        let e = (p as usize + bytes) & !((2 << 20) - 1);
        if e > a {
            unsafe { madvise(a as *mut u8, e - a, MADV_HUGEPAGE) };
        }
    }
}

fn big_zeroed_vec_untouched(n: usize) -> Vec<u64> {
    let v = vec![0u64; n];
    advise(v.as_ptr() as *mut u8, n * 8);
    v
}

fn big_zeroed_vec(n: usize) -> Vec<u64> {
    let mut v = big_zeroed_vec_untouched(n);
    if PREFAULT.load(Ordering::Relaxed) != 0 {
        let mut i = 0;
        while i < n {
            unsafe { std::ptr::write_volatile(v.as_mut_ptr().add(i), 0) };
            i += 512;
        }
    }
    v
}

struct Table {
    slots: Vec<u64>,
    len: usize,
    bits: u32,
    skip: u32,
    limit: usize,
}

impl Table {
    fn new(skip: u32, bits: u32) -> Table {
        Table {
            slots: big_zeroed_vec(1usize << bits),
            len: 0,
            bits,
            skip,
            limit: (1usize << bits) / 10 * 7,
        }
    }

    #[inline(always)]
    fn home(&self, h: u64) -> usize {
        ((h << self.skip) >> (64 - self.bits)) as usize
    }

    #[inline(always)]
    fn prefetch(&self, h: u64) {
        prefetch(unsafe { self.slots.as_ptr().add(self.home(h)) });
    }

    #[inline(always)]
    fn contains(&self, key: u64, h: u64) -> bool {
        let mask = self.slots.len() - 1;
        let stored = key + 1;
        let mut i = self.home(h);
        loop {
            let v = unsafe { *self.slots.get_unchecked(i) };
            if v == stored {
                return true;
            }
            if v == 0 {
                return false;
            }
            i = (i + 1) & mask;
        }
    }

    #[inline(always)]
    fn insert(&mut self, key: u64, h: u64) -> bool {
        let mask = self.slots.len() - 1;
        let stored = key + 1;
        let mut i = self.home(h);
        loop {
            let v = unsafe { *self.slots.get_unchecked(i) };
            if v == stored {
                return false;
            }
            if v == 0 {
                unsafe { *self.slots.get_unchecked_mut(i) = stored };
                self.len += 1;
                if self.len > self.limit {
                    self.grow();
                }
                return true;
            }
            i = (i + 1) & mask;
        }
    }

    fn grow(&mut self) {
        let old = std::mem::take(&mut self.slots);
        let mut t = Table::new(self.skip, self.bits + 1);
        let mask = t.slots.len() - 1;
        for &v in &old {
            if v != 0 {
                let mut i = t.home(mix(v - 1));
                while t.slots[i] != 0 {
                    i = (i + 1) & mask;
                }
                t.slots[i] = v;
            }
        }
        t.len = self.len;
        *self = t;
    }
}

#[derive(Clone, Copy, Default)]
struct Cand {
    key: u64,
    parent: u32,
    seq: u32,
}

#[derive(Clone, Copy, Default)]
struct Pend {
    c: Cand,
    h: u64,
    tag: u32,
}

struct Outcome {
    states: Vec<u64>,
    parents: Vec<u32>,
    levels: u64,
    table_slots: usize,
    table_len: usize,
    violations: u64,
    elapsed: Duration,
}

const BLK: usize = 8;
const BATCH_MAX: usize = BLK * 15;

struct Batch {
    key: [u64; BATCH_MAX],
    h: [u64; BATCH_MAX],
    parent: [u32; BATCH_MAX],
    n: usize,
}

impl Batch {
    fn new() -> Batch {
        Batch { key: [0; BATCH_MAX], h: [0; BATCH_MAX], parent: [0; BATCH_MAX], n: 0 }
    }
}

#[inline(always)]
fn fill(b: &mut Batch, states: &[u64], from: usize, to: usize, low: u64, table: &Table, use_prefetch: bool) {
    let mut n = 0usize;
    for i in from..to {
        let s = states[i];
        let mut m = !(s & (s >> 3)) & low;
        while m != 0 {
            let bit = m & m.wrapping_neg();
            m ^= bit;
            let key = s + bit;
            let h = mix(key);
            if use_prefetch {
                table.prefetch(h);
            }
            b.key[n] = key;
            b.h[n] = h;
            b.parent[n] = i as u32;
            n += 1;
        }
    }
    b.n = n;
}

#[inline(always)]
fn drain(b: &Batch, table: &mut Table, states: &mut Vec<u64>, parents: &mut Vec<u32>, violations: &mut u64, low: u64) {
    for j in 0..b.n {
        let key = b.key[j];
        if table.insert(key, b.h[j]) {
            states.push(key);
            parents.push(b.parent[j]);
            *violations += violates(key, low) as u64;
        }
    }
}

fn run_serial(wheels: u32, use_prefetch: bool) -> Outcome {
    let low = low_mask(wheels);
    let mut table = Table::new(0, 4);
    let mut states = vec![0u64];
    let mut parents = vec![u32::MAX];
    table.insert(0, mix(0));
    let mut violations = 0u64;
    let mut levels = 0u64;
    let mut lo = 0usize;
    let mut cur = Batch::new();
    let mut nxt = Batch::new();
    let start = Instant::now();
    while lo < states.len() {
        let hi = states.len();
        levels += 1;
        let mut i = lo;
        let end = (i + BLK).min(hi);
        fill(&mut cur, &states, i, end, low, &table, use_prefetch);
        i = end;
        while cur.n > 0 {
            if i < hi {
                let end = (i + BLK).min(hi);
                fill(&mut nxt, &states, i, end, low, &table, use_prefetch);
                i = end;
            } else {
                nxt.n = 0;
            }
            drain(&cur, &mut table, &mut states, &mut parents, &mut violations, low);
            std::mem::swap(&mut cur, &mut nxt);
        }
        lo = hi;
    }
    let elapsed = start.elapsed();
    Outcome {
        states,
        parents,
        levels,
        table_slots: table.slots.len(),
        table_len: table.len,
        violations,
        elapsed,
    }
}

struct Shared<T>(UnsafeCell<T>);
unsafe impl<T: Send> Sync for Shared<T> {}
impl<T> Shared<T> {
    fn new(v: T) -> Self {
        Shared(UnsafeCell::new(v))
    }
    unsafe fn get(&self) -> &T {
        unsafe { &*self.0.get() }
    }
    unsafe fn get_mut(&self) -> &mut T {
        unsafe { &mut *self.0.get() }
    }
}

#[derive(Clone, Copy)]
struct SendPtr<T>(*mut T);
unsafe impl<T> Send for SendPtr<T> {}

struct Part {
    lists: Vec<Vec<Cand>>,
    ncand: usize,
}

struct ShardData {
    table: Table,
    wins: Vec<Vec<Cand>>,
}

struct Ctl {
    base: Vec<usize>,
    next_lo: usize,
    next_hi: usize,
    states_ptr: SendPtr<u64>,
    parents_ptr: SendPtr<u32>,
}

#[derive(Default, Clone, Copy)]
struct Timing {
    step1: Duration,
    step2: Duration,
    serial: Duration,
    step3: Duration,
    levels: u64,
}

struct World {
    low: u64,
    shard_bits: u32,
    nshards: usize,
    part_size: usize,
    filter: bool,
    states: Shared<Vec<u64>>,
    parents: Shared<Vec<u32>>,
    shards: Vec<Shared<ShardData>>,
    parts: Shared<Vec<Shared<Part>>>,
    ctl: Shared<Ctl>,
    claim: [AtomicUsize; 3],
    violations: AtomicU64,
    successors: AtomicU64,
    hits: AtomicU64,
    barrier: Barrier,
}

#[inline(always)]
fn emit<const FILTER: bool>(w: &World, part: &mut Part, p: Pend, seq: &mut u32, hits: &mut u64) {
    if FILTER {
        let table = unsafe { &w.shards[p.tag as usize].get().table };
        if table.contains(p.c.key, p.h) {
            *hits += 1;
            return;
        }
    }
    let mut c = p.c;
    c.seq = *seq;
    *seq += 1;
    part.lists[p.tag as usize].push(c);
}

fn step1<const FILTER: bool>(w: &World, p: usize, lo: usize, hi: usize) {
    let part_lo = lo + p * w.part_size;
    let part_hi = (part_lo + w.part_size).min(hi);
    let states = unsafe { w.states.get() };
    let part = unsafe { w.parts.get()[p].get_mut() };
    for l in part.lists.iter_mut() {
        l.clear();
    }
    let shift = 64 - w.shard_bits;
    let mut seq = 0u32;
    let mut hits = 0u64;
    let mut succ = 0u64;
    let mut ring = [Pend::default(); RING];
    let mut head = 0usize;
    let mut n = 0usize;
    for i in part_lo..part_hi {
        let s = states[i];
        let mut m = !(s & (s >> 3)) & w.low;
        while m != 0 {
            let b = m & m.wrapping_neg();
            m ^= b;
            let key = s + b;
            let h = mix(key);
            let tag = (h >> shift) as u32;
            succ += 1;
            let pend = Pend {
                c: Cand { key, parent: i as u32, seq: 0 },
                h,
                tag,
            };
            if FILTER {
                unsafe { w.shards[tag as usize].get().table.prefetch(h) };
                if n == RING {
                    emit::<FILTER>(w, part, ring[head], &mut seq, &mut hits);
                    head = (head + 1) & (RING - 1);
                    n -= 1;
                }
                ring[(head + n) & (RING - 1)] = pend;
                n += 1;
            } else {
                emit::<FILTER>(w, part, pend, &mut seq, &mut hits);
            }
        }
    }
    while n > 0 {
        emit::<FILTER>(w, part, ring[head], &mut seq, &mut hits);
        head = (head + 1) & (RING - 1);
        n -= 1;
    }
    part.ncand = seq as usize;
    w.successors.fetch_add(succ, Ordering::Relaxed);
    w.hits.fetch_add(hits, Ordering::Relaxed);
}

#[inline(always)]
fn settle_shard(table: &mut Table, wins: &mut [Vec<Cand>], p: Pend) {
    if table.insert(p.c.key, p.h) {
        wins[p.tag as usize].push(p.c);
    }
}

fn step2(w: &World, s: usize, np: usize) {
    let sd = unsafe { w.shards[s].get_mut() };
    let parts = unsafe { w.parts.get() };
    let ShardData { table, wins } = sd;
    for win in wins.iter_mut().take(np) {
        win.clear();
    }
    let mut ring = [Pend::default(); RING];
    let mut head = 0usize;
    let mut n = 0usize;
    for (p, part) in parts.iter().enumerate().take(np) {
        let list = unsafe { &part.get().lists[s] };
        for &c in list {
            let h = mix(c.key);
            table.prefetch(h);
            if n == RING {
                settle_shard(table, wins, ring[head]);
                head = (head + 1) & (RING - 1);
                n -= 1;
            }
            ring[(head + n) & (RING - 1)] = Pend { c, h, tag: p as u32 };
            n += 1;
        }
    }
    while n > 0 {
        settle_shard(table, wins, ring[head]);
        head = (head + 1) & (RING - 1);
        n -= 1;
    }
}

#[derive(Default)]
struct Scratch {
    bits: Vec<u64>,
    prefix: Vec<u32>,
}

fn step3(w: &World, p: usize, base: usize, sc: &mut Scratch) {
    let parts = unsafe { w.parts.get() };
    let n = unsafe { parts[p].get().ncand };
    let words = n.div_ceil(64);
    sc.bits.clear();
    sc.bits.resize(words, 0);
    sc.prefix.clear();
    sc.prefix.resize(words, 0);
    for s in 0..w.nshards {
        for c in unsafe { &w.shards[s].get().wins[p] } {
            sc.bits[(c.seq >> 6) as usize] |= 1u64 << (c.seq & 63);
        }
    }
    let mut acc = 0u32;
    for j in 0..words {
        sc.prefix[j] = acc;
        acc += sc.bits[j].count_ones();
    }
    let (sp, pp) = unsafe {
        let ctl = w.ctl.get();
        (ctl.states_ptr.0, ctl.parents_ptr.0)
    };
    let mut bad = 0u64;
    for s in 0..w.nshards {
        for c in unsafe { &w.shards[s].get().wins[p] } {
            let j = (c.seq >> 6) as usize;
            let below = sc.bits[j] & ((1u64 << (c.seq & 63)) - 1);
            let idx = base + (sc.prefix[j] + below.count_ones()) as usize;
            unsafe {
                *sp.add(idx) = c.key;
                *pp.add(idx) = c.parent;
            }
            bad += violates(c.key, w.low) as u64;
        }
    }
    w.violations.fetch_add(bad, Ordering::Relaxed);
}

fn serial_section(w: &World, hi: usize, np: usize) {
    let ctl = unsafe { w.ctl.get_mut() };
    ctl.base.clear();
    let mut acc = hi;
    for p in 0..np {
        ctl.base.push(acc);
        for s in 0..w.nshards {
            acc += unsafe { w.shards[s].get().wins[p].len() };
        }
    }
    let total = acc - hi;
    let states = unsafe { w.states.get_mut() };
    let parents = unsafe { w.parents.get_mut() };
    states.resize(acc, 0);
    parents.resize(acc, 0);
    ctl.states_ptr = SendPtr(states.as_mut_ptr());
    ctl.parents_ptr = SendPtr(parents.as_mut_ptr());
    ctl.next_lo = hi;
    ctl.next_hi = acc;
    let next_np = total.div_ceil(w.part_size);
    let parts = unsafe { w.parts.get_mut() };
    while parts.len() < next_np {
        parts.push(Shared::new(Part {
            lists: (0..w.nshards).map(|_| Vec::new()).collect(),
            ncand: 0,
        }));
    }
    for s in 0..w.nshards {
        let wins = unsafe { &mut w.shards[s].get_mut().wins };
        while wins.len() < next_np {
            wins.push(Vec::new());
        }
    }
    for c in &w.claim {
        c.store(0, Ordering::Relaxed);
    }
}

fn worker(w: &World, id: usize) -> Timing {
    let mut tm = Timing::default();
    let mut sc = Scratch::default();
    let mut clock = Instant::now();
    loop {
        let (lo, hi) = unsafe {
            let c = w.ctl.get();
            (c.next_lo, c.next_hi)
        };
        if lo == hi {
            break;
        }
        let np = (hi - lo).div_ceil(w.part_size);
        if id == 0 {
            clock = Instant::now();
            tm.levels += 1;
        }
        loop {
            let p = w.claim[0].fetch_add(1, Ordering::Relaxed);
            if p >= np {
                break;
            }
            if w.filter {
                step1::<true>(w, p, lo, hi);
            } else {
                step1::<false>(w, p, lo, hi);
            }
        }
        w.barrier.wait();
        if id == 0 {
            let t = Instant::now();
            tm.step1 += t - clock;
            clock = t;
        }
        loop {
            let s = w.claim[1].fetch_add(1, Ordering::Relaxed);
            if s >= w.nshards {
                break;
            }
            step2(w, s, np);
        }
        w.barrier.wait();
        if id == 0 {
            let t = Instant::now();
            tm.step2 += t - clock;
            clock = t;
            serial_section(w, hi, np);
        }
        w.barrier.wait();
        if id == 0 {
            let t = Instant::now();
            tm.serial += t - clock;
            clock = t;
        }
        loop {
            let p = w.claim[2].fetch_add(1, Ordering::Relaxed);
            if p >= np {
                break;
            }
            let base = unsafe { w.ctl.get().base[p] };
            step3(w, p, base, &mut sc);
        }
        w.barrier.wait();
        if id == 0 {
            let t = Instant::now();
            tm.step3 += t - clock;
        }
    }
    tm
}

struct ParOutcome {
    out: Outcome,
    tm: Timing,
    successors: u64,
    hits: u64,
}

fn run_parallel(wheels: u32, threads: usize, shard_bits: u32, part_size: usize, filter: bool) -> ParOutcome {
    let nshards = 1usize << shard_bits;
    let mut shards = Vec::new();
    for _ in 0..nshards {
        shards.push(Shared::new(ShardData {
            table: Table::new(shard_bits, 4),
            wins: vec![Vec::new()],
        }));
    }
    let low = low_mask(wheels);
    let mut states = vec![0u64];
    let mut parents = vec![u32::MAX];
    let h0 = mix(0);
    unsafe { shards[(h0 >> (64 - shard_bits)) as usize].get_mut().table.insert(0, h0) };
    let ctl = Ctl {
        base: Vec::new(),
        next_lo: 0,
        next_hi: 1,
        states_ptr: SendPtr(states.as_mut_ptr()),
        parents_ptr: SendPtr(parents.as_mut_ptr()),
    };
    let world = World {
        low,
        shard_bits,
        nshards,
        part_size,
        filter,
        states: Shared::new(states),
        parents: Shared::new(parents),
        shards,
        parts: Shared::new(vec![Shared::new(Part {
            lists: (0..nshards).map(|_| Vec::new()).collect(),
            ncand: 0,
        })]),
        ctl: Shared::new(ctl),
        claim: [AtomicUsize::new(0), AtomicUsize::new(0), AtomicUsize::new(0)],
        violations: AtomicU64::new(0),
        successors: AtomicU64::new(0),
        hits: AtomicU64::new(0),
        barrier: Barrier::new(threads),
    };
    let start = Instant::now();
    let tm = std::thread::scope(|sc| {
        for id in 1..threads {
            let w = &world;
            sc.spawn(move || worker(w, id));
        }
        worker(&world, 0)
    });
    let elapsed = start.elapsed();
    let mut table_slots = 0usize;
    let mut table_len = 0usize;
    for s in &world.shards {
        let t = unsafe { &s.get().table };
        table_slots += t.slots.len();
        table_len += t.len;
    }
    let states = world.states.0.into_inner();
    let parents = world.parents.0.into_inner();
    let levels = tm.levels;
    ParOutcome {
        out: Outcome {
            states,
            parents,
            levels,
            table_slots,
            table_len,
            violations: world.violations.load(Ordering::Relaxed),
            elapsed,
        },
        tm,
        successors: world.successors.load(Ordering::Relaxed),
        hits: world.hits.load(Ordering::Relaxed),
    }
}

// ---------------------------------------------------------------------------------------------------
// `shared N`: one open-addressing table of 64-bit slots for all threads (key + 1, 0 = empty), linear
// probing, a slot claimed by compare-and-swap. Each thread takes chunks of CHUNK states of the level from
// one counter, generates their successors, prefetches each successor's home slot and settles it DIST
// successors later; a new state goes into the thread's own list. At the end of a level the lists are
// copied, each thread its own, into the one array of states and parents, in thread order. The table grows
// only between levels, by the serial floor's rule (above 7/10 full), and also when the next level's
// successors could fill it past 9/10; the rehash is done by all threads.
// ---------------------------------------------------------------------------------------------------

struct AtomTable {
    slots: Box<[AtomicU64]>,
    bits: u32,
}

impl AtomTable {
    fn new(bits: u32) -> AtomTable {
        let n = 1usize << bits;
        // Not touched here: when PREFAULT is on, the threads write one word of every page of a new table
        // before the rehash (`shared_worker`); a page first read maps the zero page and its first write
        // then flushes the TLB of every CPU, which with 4 threads cost more than the search.
        let v: Vec<AtomicU64> = unsafe { std::mem::transmute(big_zeroed_vec_untouched(n)) };
        let slots = v.into_boxed_slice();
        AtomTable { slots, bits }
    }
    #[inline(always)]
    fn home(&self, h: u64) -> usize {
        (h >> (64 - self.bits)) as usize
    }
    #[inline(always)]
    fn prefetch(&self, h: u64) {
        prefetch(unsafe { self.slots.as_ptr().add(self.home(h)) });
    }
    /// True if the key was not there and this call put it there.
    #[inline(always)]
    fn insert(&self, key: u64, h: u64) -> bool {
        let mask = self.slots.len() - 1;
        let stored = key + 1;
        let mut i = self.home(h);
        loop {
            let slot = unsafe { self.slots.get_unchecked(i) };
            let mut v = slot.load(Ordering::Relaxed);
            if v == 0 {
                match slot.compare_exchange(0, stored, Ordering::Relaxed, Ordering::Relaxed) {
                    Ok(_) => return true,
                    Err(now) => v = now,
                }
            }
            if v == stored {
                return false;
            }
            i = (i + 1) & mask;
        }
    }
}

struct SharedWorld {
    low: u64,
    threads: usize,
    chunk: usize,
    dist: usize,
    table: Shared<AtomTable>,
    old: Shared<Option<AtomTable>>,
    table_len: AtomicUsize,
    states: Shared<Vec<u64>>,
    parents: Shared<Vec<u32>>,
    lists: Vec<Shared<(Vec<u64>, Vec<u32>)>>,
    offsets: Shared<Vec<usize>>,
    range: Shared<(usize, usize)>,
    next: AtomicUsize,
    violations: AtomicU64,
    successors: AtomicU64,
    barrier: Barrier,
    levels: Shared<u64>,
    grows: Shared<u32>,
    expand_s: Shared<Duration>,
    copy_s: Shared<Duration>,
    grow_s: Shared<Duration>,
}

const GROW_CHUNK: usize = 1 << 16;

fn shared_worker(w: &SharedWorld, id: usize) {
    let mut ring_key = vec![0u64; 64];
    let mut ring_h = vec![0u64; 64];
    let mut ring_par = vec![0u32; 64];
    let rmask = 63usize;
    let dist = w.dist;
    loop {
        let (lo, hi) = unsafe { *w.range.get() };
        if lo == hi {
            break;
        }
        let t0 = Instant::now();
        // expand
        {
            let table = unsafe { w.table.get() };
            let list = unsafe { w.lists[id].get_mut() };
            list.0.clear();
            list.1.clear();
            let states = unsafe { w.states.get() };
            let mut succ = 0u64;
            let mut bad = 0u64;
            let mut inserted = 0usize;
            let mut head = 0usize;
            let mut n = 0usize;
            loop {
                let from = lo + w.next.fetch_add(w.chunk, Ordering::Relaxed);
                if from >= hi {
                    break;
                }
                let to = (from + w.chunk).min(hi);
                for i in from..to {
                    let s = unsafe { *states.get_unchecked(i) };
                    let mut m = !(s & (s >> 3)) & w.low;
                    while m != 0 {
                        let bit = m & m.wrapping_neg();
                        m ^= bit;
                        let key = s + bit;
                        let h = mix(key);
                        succ += 1;
                        if dist > 0 {
                            table.prefetch(h);
                            if n == dist {
                                let (k, hh, p) = (ring_key[head], ring_h[head], ring_par[head]);
                                head = (head + 1) & rmask;
                                n -= 1;
                                if table.insert(k, hh) {
                                    list.0.push(k);
                                    list.1.push(p);
                                    inserted += 1;
                                    bad += violates(k, w.low) as u64;
                                }
                            }
                            let t = (head + n) & rmask;
                            ring_key[t] = key;
                            ring_h[t] = h;
                            ring_par[t] = i as u32;
                            n += 1;
                        } else if table.insert(key, h) {
                            list.0.push(key);
                            list.1.push(i as u32);
                            inserted += 1;
                            bad += violates(key, w.low) as u64;
                        }
                    }
                }
            }
            while n > 0 {
                let (k, hh, p) = (ring_key[head], ring_h[head], ring_par[head]);
                head = (head + 1) & rmask;
                n -= 1;
                if table.insert(k, hh) {
                    list.0.push(k);
                    list.1.push(p);
                    inserted += 1;
                    bad += violates(k, w.low) as u64;
                }
            }
            w.successors.fetch_add(succ, Ordering::Relaxed);
            w.violations.fetch_add(bad, Ordering::Relaxed);
            w.table_len.fetch_add(inserted, Ordering::Relaxed);
        }
        w.barrier.wait();
        let t1 = Instant::now();
        if id == 0 {
            // offsets, resize the arrays, decide on growth
            let offs = unsafe { w.offsets.get_mut() };
            offs.clear();
            let mut acc = hi;
            for l in &w.lists {
                offs.push(acc);
                acc += unsafe { l.get().0.len() };
            }
            let states = unsafe { w.states.get_mut() };
            let parents = unsafe { w.parents.get_mut() };
            states.resize(acc, 0);
            parents.resize(acc, 0);
            let next_width = acc - hi;
            let len = w.table_len.load(Ordering::Relaxed);
            let table = unsafe { w.table.get_mut() };
            let cap = table.slots.len();
            let worst = len + next_width * w.low.count_ones() as usize;
            let mut bits = table.bits;
            while len > (1usize << bits) / 10 * 7 || worst > (1usize << bits) / 10 * 9 {
                bits += 1;
            }
            if bits != table.bits {
                let fresh = AtomTable::new(bits);
                let old = std::mem::replace(table, fresh);
                unsafe { *w.old.get_mut() = Some(old) };
                unsafe { *w.grows.get_mut() += 1 };
            }
            let _ = cap;
            unsafe { *w.range.get_mut() = (hi, acc) };
            w.next.store(0, Ordering::Relaxed);
            unsafe { *w.levels.get_mut() += 1 };
        }
        w.barrier.wait();
        // copy each thread's list into place
        {
            let list = unsafe { w.lists[id].get() };
            let at = unsafe { w.offsets.get()[id] };
            let sp = unsafe { w.states.get().as_ptr() as *mut u64 };
            let pp = unsafe { w.parents.get().as_ptr() as *mut u32 };
            unsafe {
                std::ptr::copy_nonoverlapping(list.0.as_ptr(), sp.add(at), list.0.len());
                std::ptr::copy_nonoverlapping(list.1.as_ptr(), pp.add(at), list.1.len());
            }
        }
        let t2 = Instant::now();
        // rehash, all threads, if the table grew; first every thread writes one word of each page of its
        // share of the new table
        if let Some(old) = unsafe { w.old.get() } {
            let table = unsafe { w.table.get() };
            if PREFAULT.load(Ordering::Relaxed) != 0 {
                let n = table.slots.len();
                let per = n.div_ceil(w.threads);
                let mut j = id * per;
                while j < ((id + 1) * per).min(n) {
                    table.slots[j].store(0, Ordering::Relaxed);
                    j += 512;
                }
                w.barrier.wait();
            }
            let n = old.slots.len();
            loop {
                let from = w.next.fetch_add(GROW_CHUNK, Ordering::Relaxed);
                if from >= n {
                    break;
                }
                for j in from..(from + GROW_CHUNK).min(n) {
                    let v = old.slots[j].load(Ordering::Relaxed);
                    if v != 0 {
                        table.insert(v - 1, mix(v - 1));
                    }
                }
            }
        }
        w.barrier.wait();
        let t3 = Instant::now();
        if id == 0 {
            unsafe {
                *w.old.get_mut() = None;
                *w.expand_s.get_mut() += t1 - t0;
                *w.copy_s.get_mut() += t2 - t1;
                *w.grow_s.get_mut() += t3 - t2;
            }
            w.next.store(0, Ordering::Relaxed);
        }
        w.barrier.wait();
    }
}

fn run_shared(wheels: u32, threads: usize, chunk: usize, dist: usize, bits0: u32) -> (Outcome, String) {
    assert!(dist < 64);
    let low = low_mask(wheels);
    let table = AtomTable::new(bits0);
    table.insert(0, mix(0));
    let w = SharedWorld {
        low,
        threads,
        chunk,
        dist,
        table: Shared::new(table),
        old: Shared::new(None),
        table_len: AtomicUsize::new(1),
        states: Shared::new(vec![0u64]),
        parents: Shared::new(vec![u32::MAX]),
        lists: (0..threads).map(|_| Shared::new((Vec::new(), Vec::new()))).collect(),
        offsets: Shared::new(Vec::new()),
        range: Shared::new((0, 1)),
        next: AtomicUsize::new(0),
        violations: AtomicU64::new(0),
        successors: AtomicU64::new(0),
        barrier: Barrier::new(threads),
        levels: Shared::new(0),
        grows: Shared::new(0),
        expand_s: Shared::new(Duration::ZERO),
        copy_s: Shared::new(Duration::ZERO),
        grow_s: Shared::new(Duration::ZERO),
    };
    let start = Instant::now();
    std::thread::scope(|sc| {
        for id in 1..threads {
            let w = &w;
            sc.spawn(move || shared_worker(w, id));
        }
        shared_worker(&w, 0)
    });
    let elapsed = start.elapsed();
    let _ = w.threads;
    let extra = unsafe {
        format!(
            "threads={}\nchunk={} dist={} bits0={}\nsuccessors={}\ngrows={}\nexpand_s={:.4}\ncopy_s={:.4}\ngrow_s={:.4}",
            threads,
            chunk,
            dist,
            bits0,
            w.successors.load(Ordering::Relaxed),
            *w.grows.get(),
            w.expand_s.get().as_secs_f64(),
            w.copy_s.get().as_secs_f64(),
            w.grow_s.get().as_secs_f64()
        )
    };
    let table = w.table.0.into_inner();
    let out = Outcome {
        states: w.states.0.into_inner(),
        parents: w.parents.0.into_inner(),
        levels: w.levels.0.into_inner(),
        table_slots: table.slots.len(),
        table_len: w.table_len.load(Ordering::Relaxed),
        violations: w.violations.load(Ordering::Relaxed),
        elapsed,
    };
    (out, extra)
}

// ---------------------------------------------------------------------------------------------------
// `gen N`: the generating of successors alone. A serial search (not timed) makes the states in breadth-
// first order; then the timed pass reads every state once, in that order, makes each successor, hashes
// it with `mix` and counts it, and checks the invariant once per state; no table. N threads split the
// states into chunks of 4,096 taken from one counter.
// ---------------------------------------------------------------------------------------------------

fn run_gen(wheels: u32, threads: usize) -> (Outcome, String) {
    let base = run_serial(wheels, true);
    let low = low_mask(wheels);
    let states = &base.states;
    let next = AtomicUsize::new(0);
    let acc = AtomicU64::new(0);
    let count = AtomicU64::new(0);
    let bad = AtomicU64::new(0);
    let r0 = rusage();
    let start = Instant::now();
    std::thread::scope(|sc| {
        let work = || {
            let (mut a, mut c, mut b) = (0u64, 0u64, 0u64);
            loop {
                let from = next.fetch_add(4096, Ordering::Relaxed);
                if from >= states.len() {
                    break;
                }
                for &s in &states[from..(from + 4096).min(states.len())] {
                    b += violates(s, low) as u64;
                    let mut m = !(s & (s >> 3)) & low;
                    while m != 0 {
                        let bit = m & m.wrapping_neg();
                        m ^= bit;
                        a = a.wrapping_add(mix(s + bit));
                        c += 1;
                    }
                }
            }
            acc.fetch_add(a, Ordering::Relaxed);
            count.fetch_add(c, Ordering::Relaxed);
            bad.fetch_add(b, Ordering::Relaxed);
        };
        for _ in 1..threads {
            sc.spawn(work);
        }
        work();
    });
    let elapsed = start.elapsed();
    let r1 = rusage();
    let extra = format!(
        "threads={}\nsuccessors={}\nhash_sum={:016x}\ngen_cpu_s={:.4}\ngen_sys_s={:.4}\nsearch_before_gen_s={:.4}",
        threads,
        count.load(Ordering::Relaxed),
        acc.load(Ordering::Relaxed),
        (r1.0 - r0.0) + (r1.1 - r0.1),
        r1.1 - r0.1,
        base.elapsed.as_secs_f64()
    );
    let out = Outcome { violations: bad.load(Ordering::Relaxed), elapsed, ..base };
    (out, extra)
}

fn rusage() -> (f64, f64) {
    #[repr(C)]
    struct Tv {
        s: i64,
        us: i64,
    }
    #[repr(C)]
    struct Ru {
        utime: Tv,
        stime: Tv,
        rest: [i64; 14],
    }
    unsafe extern "C" {
        fn getrusage(who: i32, ru: *mut Ru) -> i32;
    }
    let mut ru: Ru = unsafe { std::mem::zeroed() };
    unsafe { getrusage(0, &mut ru) };
    (
        ru.utime.s as f64 + ru.utime.us as f64 * 1e-6,
        ru.stime.s as f64 + ru.stime.us as f64 * 1e-6,
    )
}

fn checksum(states: &[u64], parents: &[u32]) -> (u64, u64) {
    let mut a = 0xcbf29ce484222325u64;
    let mut b = 0x84222325cbf29ce4u64;
    for (&s, &p) in states.iter().zip(parents) {
        a = (a ^ s).wrapping_mul(0x9E3779B97F4A7C15).rotate_left(29);
        b = (b ^ p as u64).wrapping_mul(0xC2B2AE3D27D4EB4F).rotate_left(31);
    }
    (a, b)
}

/// A checksum of the set of states that does not depend on their order: the sum and the xor of a second
/// mix of every state, and the count. Also checks, for every state but the first, that its parent is an
/// earlier state from which it is one step (one wheel one higher), and that no state is there twice
/// (the sum of squares of the mix would not tell; the table did, but this checks the arrays themselves).
fn set_checksum(states: &[u64], parents: &[u32], low: u64) -> (u64, u64, u64) {
    let mut sum = 0u64;
    let mut x = 0u64;
    let mut bad_parent = 0u64;
    for (i, (&s, &p)) in states.iter().zip(parents).enumerate() {
        let m = mix(s ^ 0x5555_5555_5555_5555);
        sum = sum.wrapping_add(m);
        x ^= m.rotate_left(17);
        if i > 0 {
            let ok = (p as usize) < i && {
                let d = s.wrapping_sub(states[p as usize]);
                d != 0 && d & (d - 1) == 0 && d & low != 0
            };
            bad_parent += !ok as u64;
        }
    }
    (sum, x, bad_parent)
}

fn env_or(name: &str, default: u64) -> u64 {
    std::env::var(name).ok().and_then(|v| v.parse().ok()).unwrap_or(default)
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let wheels: u32 = args.get(1).and_then(|a| a.parse().ok()).unwrap_or(7);
    let variant = args.get(2).map(String::as_str).unwrap_or("serial");
    let shard_bits = env_or("SHARD_BITS", 6) as u32;
    let part_size = env_or("PART", 4096) as usize;
    let filter = env_or("FILTER", 1) != 0;
    let use_prefetch = env_or("PREFETCH", 1) != 0;
    let chunk = env_or("CHUNK", 1024) as usize;
    let dist = env_or("DIST", 16) as usize;
    let bits0 = env_or("BITS0", 4) as u32;
    let nth = args.get(3).and_then(|a| a.parse().ok()).unwrap_or(1usize);
    HUGE.store(env_or("HUGE", 0) as usize, Ordering::Relaxed);
    PREFAULT.store(env_or("PREFAULT", 1) as usize, Ordering::Relaxed);

    let r0 = rusage();
    let (out, extra) = match variant {
        "serial" => (run_serial(wheels, use_prefetch), String::new()),
        "shared" => run_shared(wheels, nth, chunk, dist, bits0),
        "gen" => run_gen(wheels, nth),
        n => {
            let threads: usize = n.parse().expect("serial, shared N, gen N or a number of threads");
            let ParOutcome { out, tm, successors, hits } =
                run_parallel(wheels, threads, shard_bits, part_size, filter);
            let total = tm.step1 + tm.step2 + tm.serial + tm.step3;
            let rest = out.elapsed.saturating_sub(total);
            let e = format!(
                "threads={}\nshard_bits={} part={} filter={}\nsuccessors={}\nstep1_known={}\nstep1_s={:.4}\nstep2_s={:.4}\nserial_s={:.4}\nstep3_s={:.4}\nrest_s={:.4}",
                threads, shard_bits, part_size, filter as u8, successors, hits,
                tm.step1.as_secs_f64(), tm.step2.as_secs_f64(), tm.serial.as_secs_f64(),
                tm.step3.as_secs_f64(), rest.as_secs_f64()
            );
            (out, e)
        }
    };
    let r1 = rusage();

    let (ca, cb) = checksum(&out.states, &out.parents);
    let (ssum, sx, bad_parent) = set_checksum(&out.states, &out.parents, low_mask(wheels));
    println!("variant={}", variant);
    println!("wheels={}", wheels);
    println!("states={}", out.states.len());
    println!("levels={}", out.levels);
    println!("checksum={:016x}", ca);
    println!("parent_checksum={:016x}", cb);
    println!("set_checksum={:016x}/{:016x}", ssum, sx);
    println!("bad_parents={}", bad_parent);
    println!("rule_violations={}", out.violations);
    println!("table_len={}", out.table_len);
    println!("table_slots={}", out.table_slots);
    println!("table_bytes={}", out.table_slots * 8);
    println!("time_s={:.4}", out.elapsed.as_secs_f64());
    println!("run_cpu_s={:.4}", (r1.0 - r0.0) + (r1.1 - r0.1));
    println!("run_sys_s={:.4}", r1.1 - r0.1);
    println!("huge={} prefault={}", HUGE.load(Ordering::Relaxed), PREFAULT.load(Ordering::Relaxed));
    if !extra.is_empty() {
        println!("{}", extra);
    }
}
