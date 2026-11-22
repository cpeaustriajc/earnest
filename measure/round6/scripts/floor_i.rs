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
    #[cfg(not(target_arch = "aarch64"))]
    let _ = p;
}

fn low_mask(wheels: u32) -> u64 {
    (0..wheels).fold(0u64, |m, i| m | (1u64 << (4 * i)))
}

#[inline(always)]
fn violates(s: u64, low: u64) -> bool {
    (s >> 3) & ((s >> 2) | (s >> 1)) & low != 0
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
            slots: vec![0u64; 1usize << bits],
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

fn checksum(states: &[u64], parents: &[u32]) -> (u64, u64) {
    let mut a = 0xcbf29ce484222325u64;
    let mut b = 0x84222325cbf29ce4u64;
    for (&s, &p) in states.iter().zip(parents) {
        a = (a ^ s).wrapping_mul(0x9E3779B97F4A7C15).rotate_left(29);
        b = (b ^ p as u64).wrapping_mul(0xC2B2AE3D27D4EB4F).rotate_left(31);
    }
    (a, b)
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

    let (out, par) = if variant == "serial" {
        (run_serial(wheels, use_prefetch), None)
    } else {
        let threads: usize = variant.parse().expect("threads must be a number or serial");
        let p = run_parallel(wheels, threads, shard_bits, part_size, filter);
        let ParOutcome { out, tm, successors, hits } = p;
        (out, Some((threads, tm, successors, hits)))
    };

    let (ca, cb) = checksum(&out.states, &out.parents);
    println!("variant={}", variant);
    println!("wheels={}", wheels);
    println!("states={}", out.states.len());
    println!("levels={}", out.levels);
    println!("checksum={:016x}", ca);
    println!("parent_checksum={:016x}", cb);
    println!("rule_violations={}", out.violations);
    println!("table_len={}", out.table_len);
    println!("table_slots={}", out.table_slots);
    println!("table_bytes={}", out.table_slots * 8);
    println!("time_s={:.4}", out.elapsed.as_secs_f64());
    if let Some((threads, tm, successors, hits)) = par {
        let total = tm.step1 + tm.step2 + tm.serial + tm.step3;
        let rest = out.elapsed.saturating_sub(total);
        println!("threads={}", threads);
        println!("shard_bits={} part={} filter={}", shard_bits, part_size, filter as u8);
        println!("successors={}", successors);
        println!("step1_known={}", hits);
        println!("step1_s={:.4}", tm.step1.as_secs_f64());
        println!("step2_s={:.4}", tm.step2.as_secs_f64());
        println!("serial_s={:.4}", tm.serial.as_secs_f64());
        println!("step3_s={:.4}", tm.step3.as_secs_f64());
        println!("rest_s={:.4}", rest.as_secs_f64());
    }
}
