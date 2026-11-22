use hashbrown::HashTable;
use hashbrown::hash_table::Entry;
use std::hint::black_box;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

const SHARD_BITS: u32 = 6;
const SHARD_COUNT: usize = 1 << SHARD_BITS;
const SHARD_SHIFT: u32 = 51;
const MOST_PARTS: usize = 64;
const BITS_PER_WORD: usize = 64;

const SEAT_SPREAD: u64 = 0x9E37_79B9_7F4A_7C15;
const MIX: u64 = 0xD6E8_FEB8_6659_FD93;

fn mix(word: u64, at: usize) -> u64 {
    let seat = (at as u64).wrapping_mul(SEAT_SPREAD);
    let mut mixed = word ^ seat;
    mixed ^= mixed >> 32;
    mixed = mixed.wrapping_mul(MIX);
    mixed ^= mixed >> 32;
    mixed = mixed.wrapping_mul(MIX);
    mixed ^ (mixed >> 32)
}

fn hash_words(row: &[u64]) -> u64 {
    row.iter().enumerate().fold(0, |hash, (at, word)| hash ^ mix(*word, at))
}

fn shard_of(hash: u64) -> usize {
    ((hash >> SHARD_SHIFT) as usize) & (SHARD_COUNT - 1)
}

#[derive(Default)]
struct Part {
    rows: Vec<u64>,
    hashes: Vec<u64>,
    froms: Vec<u32>,
    by_shard: Vec<usize>,
    shard_starts: Vec<usize>,
}

struct Level<'a> {
    parts: &'a [Part],
    words: usize,
}

impl Level<'_> {
    #[inline(always)]
    fn row_of(&self, part: usize, ordinal: usize) -> &[u64] {
        let start = ordinal * self.words;
        &self.parts[part].rows[start..start + self.words]
    }
    #[inline(always)]
    fn hash_of(&self, part: usize, ordinal: usize) -> u64 {
        self.parts[part].hashes[ordinal]
    }
    #[inline(always)]
    fn in_shard(&self, part: usize, shard: usize) -> &[usize] {
        let fresh = &self.parts[part];
        &fresh.by_shard[fresh.shard_starts[shard]..fresh.shard_starts[shard + 1]]
    }
}

type Won = Vec<Vec<usize>>;

#[derive(Default)]
struct Winners {
    bits: Vec<u64>,
    before: Vec<usize>,
    count: usize,
}

impl Winners {
    fn of(wins: &[Won], part: usize, found: usize) -> Self {
        let mut bits = vec![0_u64; found.div_ceil(BITS_PER_WORD)];
        for shard in wins {
            for ordinal in &shard[part] {
                bits[ordinal / BITS_PER_WORD] |= 1 << (ordinal % BITS_PER_WORD);
            }
        }
        let mut before = Vec::with_capacity(bits.len());
        let mut count = 0;
        for word in &bits {
            before.push(count);
            count += word.count_ones() as usize;
        }
        Self { bits, before, count }
    }
    fn rank(&self, ordinal: usize) -> usize {
        let word = ordinal / BITS_PER_WORD;
        let below = (1_u64 << (ordinal % BITS_PER_WORD)) - 1;
        self.before[word] + (self.bits[word] & below).count_ones() as usize
    }
    fn ordinals(&self) -> impl Iterator<Item = usize> + '_ {
        self.bits.iter().enumerate().flat_map(|(word, bits)| {
            let mut rest = *bits;
            std::iter::from_fn(move || {
                if rest == 0 {
                    return None;
                }
                let bit = rest.trailing_zeros() as usize;
                rest &= rest - 1;
                Some(word * BITS_PER_WORD + bit)
            })
        })
    }
}

struct Ids<'a> {
    count: usize,
    maps: &'a [Winners],
    bases: &'a [usize],
}

impl Ids<'_> {
    fn of(&self, part: usize, ordinal: usize) -> usize {
        self.count + self.bases[part] + self.maps[part].rank(ordinal)
    }
}

trait ShardIndex: Send {
    fn claim(&mut self, shard: usize, at: &Level, rows: &[u64], count: usize) -> Won;
    fn file(&mut self, won: &[Vec<usize>], at: &Level, rows: &[u64], ids: &Ids);
    fn find(&self, row: &[u64], hash: u64, rows: &[u64]) -> Option<Option<usize>>;
    fn bytes(&self) -> usize;
}

#[derive(Default)]
struct BrownKey {
    table: HashTable<u32>,
}

impl ShardIndex for BrownKey {
    fn claim(&mut self, shard: usize, at: &Level, _: &[u64], _: usize) -> Won {
        let mut won = Vec::with_capacity(at.parts.len());
        for part in 0..at.parts.len() {
            let mut mine = Vec::new();
            for ordinal in at.in_shard(part, shard) {
                let row = at.row_of(part, *ordinal);
                let key = row[0] as u32;
                let slot = self.table.entry(
                    at.hash_of(part, *ordinal),
                    |filed| *filed == key,
                    |filed| hash_words(&[u64::from(*filed)]),
                );
                if let Entry::Vacant(vacant) = slot {
                    vacant.insert(key);
                    mine.push(*ordinal);
                }
            }
            won.push(mine);
        }
        won
    }
    fn file(&mut self, _: &[Vec<usize>], _: &Level, _: &[u64], _: &Ids) {}
    fn find(&self, row: &[u64], hash: u64, _: &[u64]) -> Option<Option<usize>> {
        let key = row[0] as u32;
        self.table.find(hash, |filed| *filed == key).map(|_| None)
    }
    fn bytes(&self) -> usize {
        self.table.allocation_size()
    }
}

#[derive(Default)]
struct BrownId {
    table: HashTable<u32>,
    words: usize,
}

fn row_at(rows: &[u64], words: usize, id: usize) -> &[u64] {
    &rows[id * words..id * words + words]
}

impl ShardIndex for BrownId {
    fn claim(&mut self, shard: usize, at: &Level, rows: &[u64], _: usize) -> Won {
        let words = self.words;
        let mut seen: HashTable<(u32, u32)> = HashTable::new();
        let mut won = Vec::with_capacity(at.parts.len());
        for part in 0..at.parts.len() {
            let mut mine = Vec::new();
            for ordinal in at.in_shard(part, shard) {
                let row = at.row_of(part, *ordinal);
                let hash = at.hash_of(part, *ordinal);
                if self
                    .table
                    .find(hash, |id| row_at(rows, words, *id as usize) == row)
                    .is_some()
                {
                    continue;
                }
                if let Entry::Vacant(vacant) = seen.entry(
                    hash,
                    |(p, o)| at.row_of(*p as usize, *o as usize) == row,
                    |(p, o)| at.hash_of(*p as usize, *o as usize),
                ) {
                    vacant.insert((part as u32, *ordinal as u32));
                    mine.push(*ordinal);
                }
            }
            won.push(mine);
        }
        won
    }
    fn file(&mut self, won: &[Vec<usize>], at: &Level, rows: &[u64], ids: &Ids) {
        let words = self.words;
        let hasher = |id: &u32| hash_words(row_at(rows, words, *id as usize));
        self.table.reserve(won.iter().map(Vec::len).sum(), hasher);
        for (part, ordinals) in won.iter().enumerate() {
            for ordinal in ordinals {
                let id = ids.of(part, *ordinal) as u32;
                self.table.insert_unique(at.hash_of(part, *ordinal), id, hasher);
            }
        }
    }
    fn find(&self, row: &[u64], hash: u64, rows: &[u64]) -> Option<Option<usize>> {
        self.table
            .find(hash, |id| row_at(rows, self.words, *id as usize) == row)
            .map(|id| Some(*id as usize))
    }
    fn bytes(&self) -> usize {
        self.table.allocation_size()
    }
}

const NO_LOAD: u8 = 0;
const PLAIN_LOAD: u8 = 1;
const PREFETCH: u8 = 2;

#[cfg(feature = "prefetch")]
#[inline(always)]
fn prefetch<T>(line: &T) {
    let p = (line as *const T).cast::<i8>();
    #[cfg(target_arch = "x86_64")]
    unsafe {
        core::arch::x86_64::_mm_prefetch(p, core::arch::x86_64::_MM_HINT_T0)
    }
    #[cfg(target_arch = "aarch64")]
    unsafe {
        core::arch::asm!("prfm pldl1keep, [{p}]", p = in(reg) p, options(nostack, preserves_flags, readonly));
    }
    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    let _ = p;
}

#[cfg(not(feature = "prefetch"))]
#[inline(always)]
fn prefetch<T>(_: &T) {}

const KEY_SLOTS: usize = 16;
const EMPTY_KEY: u32 = u32::MAX;

#[repr(C, align(64))]
#[derive(Clone, Copy)]
struct KeyBucket([u32; KEY_SLOTS]);

const EMPTY_KEYS: KeyBucket = KeyBucket([EMPTY_KEY; KEY_SLOTS]);

const MOST_FILLED_NUMERATOR: usize = 3;
const MOST_FILLED_DENOMINATOR: usize = 4;

struct KeyTable {
    buckets: Vec<KeyBucket>,
    len: usize,
    holds_empty_key: bool,
}

impl KeyTable {
    fn new() -> Self {
        Self { buckets: vec![EMPTY_KEYS; 1], len: 0, holds_empty_key: false }
    }
    #[inline(always)]
    fn home(&self, hash: u64) -> usize {
        (hash as usize) & (self.buckets.len() - 1)
    }
    fn limit(&self) -> usize {
        self.buckets.len() * KEY_SLOTS * MOST_FILLED_NUMERATOR / MOST_FILLED_DENOMINATOR
    }
    fn reserve(&mut self, more: usize) {
        while self.len + more > self.limit() {
            self.grow();
        }
    }
    fn grow(&mut self) {
        let old = std::mem::replace(&mut self.buckets, vec![EMPTY_KEYS; 0]);
        self.buckets = vec![EMPTY_KEYS; old.len() * 2];
        for bucket in &old {
            for key in bucket.0 {
                if key != EMPTY_KEY {
                    let hash = hash_words(&[u64::from(key)]);
                    let _ = self.place(key, hash);
                }
            }
        }
    }
    #[inline(always)]
    fn touch<const AHEAD: u8>(&self, hash: u64) -> u32 {
        let bucket = &self.buckets[self.home(hash)];
        if AHEAD == PREFETCH {
            prefetch(bucket);
            0
        } else {
            bucket.0[0]
        }
    }
    #[inline(always)]
    fn place(&mut self, key: u32, hash: u64) -> bool {
        let mask = self.buckets.len() - 1;
        let mut at = self.home(hash);
        loop {
            let bucket = &mut self.buckets[at].0;
            let mut equal = 0_u32;
            let mut empty = 0_u32;
            for (slot, filed) in bucket.iter().enumerate() {
                equal |= u32::from(*filed == key) << slot;
                empty |= u32::from(*filed == EMPTY_KEY) << slot;
            }
            if equal != 0 {
                return false;
            }
            if empty != 0 {
                bucket[empty.trailing_zeros() as usize] = key;
                return true;
            }
            at = (at + 1) & mask;
        }
    }
    #[inline(always)]
    fn insert(&mut self, key: u32, hash: u64) -> bool {
        if key == EMPTY_KEY {
            let first = !self.holds_empty_key;
            self.holds_empty_key = true;
            return first;
        }
        let new = self.place(key, hash);
        self.len += usize::from(new);
        new
    }
    fn contains(&self, key: u32, hash: u64) -> bool {
        if key == EMPTY_KEY {
            return self.holds_empty_key;
        }
        let mask = self.buckets.len() - 1;
        let mut at = self.home(hash);
        loop {
            let bucket = &self.buckets[at].0;
            if bucket.contains(&key) {
                return true;
            }
            if bucket.contains(&EMPTY_KEY) {
                return false;
            }
            at = (at + 1) & mask;
        }
    }
}

struct OwnKey<const AHEAD: u8> {
    table: KeyTable,
    batch: usize,
}

impl<const AHEAD: u8> ShardIndex for OwnKey<AHEAD> {
    fn claim(&mut self, shard: usize, at: &Level, _: &[u64], _: usize) -> Won {
        let mut won = Vec::with_capacity(at.parts.len());
        for part in 0..at.parts.len() {
            let mut mine = Vec::new();
            for chunk in at.in_shard(part, shard).chunks(self.batch) {
                self.table.reserve(chunk.len());
                if AHEAD != NO_LOAD {
                    let mut warm = 0_u32;
                    for ordinal in chunk {
                        warm ^= self.table.touch::<AHEAD>(at.hash_of(part, *ordinal));
                    }
                    black_box(warm);
                }
                for ordinal in chunk {
                    let key = at.row_of(part, *ordinal)[0] as u32;
                    if self.table.insert(key, at.hash_of(part, *ordinal)) {
                        mine.push(*ordinal);
                    }
                }
            }
            won.push(mine);
        }
        won
    }
    fn file(&mut self, _: &[Vec<usize>], _: &Level, _: &[u64], _: &Ids) {}
    fn find(&self, row: &[u64], hash: u64, _: &[u64]) -> Option<Option<usize>> {
        self.table.contains(row[0] as u32, hash).then_some(None)
    }
    fn bytes(&self) -> usize {
        self.table.buckets.len() * size_of::<KeyBucket>()
    }
}

const ID_SLOTS: usize = 8;
const EMPTY_ID: u32 = u32::MAX;
const HIGHEST_PENDING: u32 = u32::MAX - 1;

#[repr(C, align(64))]
#[derive(Clone, Copy)]
struct IdBucket {
    ids: [u32; ID_SLOTS],
    tags: [u32; ID_SLOTS],
}

const EMPTY_IDS: IdBucket = IdBucket { ids: [EMPTY_ID; ID_SLOTS], tags: [0; ID_SLOTS] };

enum Probe {
    Known(u32),
    Vacant(usize, usize),
}

struct IdTable {
    buckets: Vec<IdBucket>,
    len: usize,
}

impl IdTable {
    fn new() -> Self {
        Self { buckets: vec![EMPTY_IDS; 1], len: 0 }
    }
    #[inline(always)]
    fn home(&self, tag: u32) -> usize {
        (tag as usize) & (self.buckets.len() - 1)
    }
    fn limit(&self) -> usize {
        self.buckets.len() * ID_SLOTS * MOST_FILLED_NUMERATOR / MOST_FILLED_DENOMINATOR
    }
    fn reserve(&mut self, more: usize) {
        while self.len + more > self.limit() {
            self.grow();
        }
    }
    fn grow(&mut self) {
        let old = std::mem::replace(&mut self.buckets, vec![EMPTY_IDS; 0]);
        self.buckets = vec![EMPTY_IDS; old.len() * 2];
        for bucket in &old {
            for (id, tag) in bucket.ids.iter().zip(bucket.tags) {
                if *id != EMPTY_ID {
                    match self.probe(tag, |_| false) {
                        Probe::Vacant(at, slot) => {
                            self.buckets[at].ids[slot] = *id;
                            self.buckets[at].tags[slot] = tag;
                        }
                        Probe::Known(_) => unreachable!(),
                    }
                }
            }
        }
    }
    #[inline(always)]
    fn probe(&self, tag: u32, holds: impl Fn(u32) -> bool) -> Probe {
        let mask = self.buckets.len() - 1;
        let mut at = self.home(tag);
        loop {
            let bucket = &self.buckets[at];
            let mut equal = 0_u32;
            let mut empty = 0_u32;
            for slot in 0..ID_SLOTS {
                equal |= u32::from(bucket.tags[slot] == tag && bucket.ids[slot] != EMPTY_ID) << slot;
                empty |= u32::from(bucket.ids[slot] == EMPTY_ID) << slot;
            }
            while equal != 0 {
                let slot = equal.trailing_zeros() as usize;
                equal &= equal - 1;
                if holds(bucket.ids[slot]) {
                    return Probe::Known(bucket.ids[slot]);
                }
            }
            if empty != 0 {
                return Probe::Vacant(at, empty.trailing_zeros() as usize);
            }
            at = (at + 1) & mask;
        }
    }
    fn set(&mut self, tag: u32, from: u32, to: u32) {
        let mask = self.buckets.len() - 1;
        let mut at = self.home(tag);
        loop {
            let bucket = &mut self.buckets[at];
            for slot in 0..ID_SLOTS {
                if bucket.ids[slot] == from && bucket.tags[slot] == tag {
                    bucket.ids[slot] = to;
                    return;
                }
            }
            at = (at + 1) & mask;
        }
    }
}

struct OwnId<const AHEAD: u8> {
    table: IdTable,
    batch: usize,
    words: usize,
    pending: Vec<(u32, u32)>,
}

impl<const AHEAD: u8> OwnId<AHEAD> {
    #[inline(always)]
    fn warm(&self, at: &Level, rows: &[u64], count: usize, part: usize, chunk: &[usize]) {
        let mut warm = 0_u64;
        for ordinal in chunk {
            let tag = at.hash_of(part, *ordinal) as u32;
            let bucket = &self.table.buckets[self.table.home(tag)];
            if AHEAD == PREFETCH {
                prefetch(bucket);
            } else {
                warm ^= u64::from(bucket.ids[0]);
            }
        }
        for ordinal in chunk {
            let tag = at.hash_of(part, *ordinal) as u32;
            let bucket = &self.table.buckets[self.table.home(tag)];
            for slot in 0..ID_SLOTS {
                let id = bucket.ids[slot] as usize;
                if bucket.tags[slot] == tag && id < count {
                    let start = id * self.words;
                    if AHEAD == PREFETCH {
                        prefetch(&rows[start]);
                    } else {
                        warm ^= rows[start];
                    }
                }
            }
        }
        black_box(warm);
    }
}

impl<const AHEAD: u8> ShardIndex for OwnId<AHEAD> {
    fn claim(&mut self, shard: usize, at: &Level, rows: &[u64], count: usize) -> Won {
        let words = self.words;
        self.pending.clear();
        let mut won = Vec::with_capacity(at.parts.len());
        for part in 0..at.parts.len() {
            let mut mine = Vec::new();
            for chunk in at.in_shard(part, shard).chunks(self.batch) {
                self.table.reserve(chunk.len());
                if AHEAD != NO_LOAD {
                    self.warm(at, rows, count, part, chunk);
                }
                for ordinal in chunk {
                    let row = at.row_of(part, *ordinal);
                    let tag = at.hash_of(part, *ordinal) as u32;
                    let pending = &self.pending;
                    let holds = |id: u32| {
                        if (id as usize) < count {
                            row_at(rows, words, id as usize) == row
                        } else {
                            let (p, o) = pending[(HIGHEST_PENDING - id) as usize];
                            at.row_of(p as usize, o as usize) == row
                        }
                    };
                    if let Probe::Vacant(bucket, slot) = self.table.probe(tag, holds) {
                        let marker = HIGHEST_PENDING - self.pending.len() as u32;
                        assert!(marker as usize >= count, "pending ids run into real ids");
                        self.table.buckets[bucket].ids[slot] = marker;
                        self.table.buckets[bucket].tags[slot] = tag;
                        self.table.len += 1;
                        self.pending.push((part as u32, *ordinal as u32));
                        mine.push(*ordinal);
                    }
                }
            }
            won.push(mine);
        }
        won
    }
    fn file(&mut self, _: &[Vec<usize>], at: &Level, _: &[u64], ids: &Ids) {
        for (local, (part, ordinal)) in self.pending.iter().enumerate() {
            let (part, ordinal) = (*part as usize, *ordinal as usize);
            let tag = at.hash_of(part, ordinal) as u32;
            self.table.set(tag, HIGHEST_PENDING - local as u32, ids.of(part, ordinal) as u32);
        }
        self.pending.clear();
    }
    fn find(&self, row: &[u64], hash: u64, rows: &[u64]) -> Option<Option<usize>> {
        match self.table.probe(hash as u32, |id| row_at(rows, self.words, id as usize) == row) {
            Probe::Known(id) => Some(Some(id as usize)),
            Probe::Vacant(..) => None,
        }
    }
    fn bytes(&self) -> usize {
        self.table.buckets.len() * size_of::<IdBucket>()
    }
}

fn per_shard<T: Send, R: Send>(
    threads: usize,
    tables: &mut [T],
    job: impl Fn(usize, &mut T) -> R + Sync,
) -> Vec<R> {
    if threads <= 1 {
        return tables.iter_mut().enumerate().map(|(shard, table)| job(shard, table)).collect();
    }
    let mut groups: Vec<Vec<(usize, &mut T)>> = (0..threads).map(|_| Vec::new()).collect();
    for (shard, table) in tables.iter_mut().enumerate() {
        groups[shard % threads].push((shard, table));
    }
    let job = &job;
    let mut done: Vec<(usize, R)> = std::thread::scope(|scope| {
        let handles: Vec<_> = groups
            .into_iter()
            .map(|group| {
                scope.spawn(move || {
                    group.into_iter().map(|(shard, table)| (shard, job(shard, table))).collect::<Vec<_>>()
                })
            })
            .collect();
        handles.into_iter().flat_map(|handle| handle.join().expect("a shard thread panicked")).collect()
    });
    done.sort_by_key(|(shard, _)| *shard);
    done.into_iter().map(|(_, result)| result).collect()
}

fn second_word(first: u64) -> u64 {
    first.wrapping_mul(SEAT_SPREAD).rotate_left(23) ^ 0x5555_AAAA_0F0F_F0F0
}

fn push_row(into: &mut Vec<u64>, first: u64, words: usize) {
    into.push(first);
    if words == 2 {
        into.push(second_word(first));
    }
}

fn file_by_shard(part: &mut Part) {
    let mut starts = vec![0; SHARD_COUNT + 1];
    for hash in &part.hashes {
        starts[shard_of(*hash) + 1] += 1;
    }
    for shard in 0..SHARD_COUNT {
        starts[shard + 1] += starts[shard];
    }
    let mut next = starts.clone();
    part.by_shard = vec![0; part.hashes.len()];
    for (ordinal, hash) in part.hashes.iter().enumerate() {
        let at = &mut next[shard_of(*hash)];
        part.by_shard[*at] = ordinal;
        *at += 1;
    }
    part.shard_starts = starts;
}

fn expand_part(rows: &[u64], words: usize, wheels: u32, ids: std::ops::Range<usize>) -> Part {
    let mut part = Part::default();
    let mut met: HashTable<u32> = HashTable::new();
    let mut row = Vec::with_capacity(words);
    for id in ids {
        let state = rows[id * words];
        for wheel in 0..wheels {
            let shift = 4 * wheel;
            if (state >> shift) & 0xF >= 9 {
                continue;
            }
            row.clear();
            push_row(&mut row, state + (1 << shift), words);
            let hash = hash_words(&row);
            let (kept, hashes) = (&part.rows, &part.hashes);
            if let Entry::Vacant(vacant) = met.entry(
                hash,
                |at| row_at(kept, words, *at as usize) == row.as_slice(),
                |at| hashes[*at as usize],
            ) {
                vacant.insert(part.hashes.len() as u32);
                part.rows.extend_from_slice(&row);
                part.hashes.push(hash);
                part.froms.push(id as u32);
            }
        }
    }
    file_by_shard(&mut part);
    part
}

fn expand(rows: &[u64], words: usize, wheels: u32, frontier: std::ops::Range<usize>, helpers: usize) -> Vec<Part> {
    let size = frontier.len().div_ceil(MOST_PARTS).max(1);
    let cuts: Vec<std::ops::Range<usize>> = (frontier.start..frontier.end)
        .step_by(size)
        .map(|start| start..(start + size).min(frontier.end))
        .collect();
    let done: Vec<Mutex<Part>> = cuts.iter().map(|_| Mutex::new(Part::default())).collect();
    let next = AtomicUsize::new(0);
    std::thread::scope(|scope| {
        for _ in 0..helpers {
            scope.spawn(|| loop {
                let at = next.fetch_add(1, Ordering::Relaxed);
                let Some(cut) = cuts.get(at) else { return };
                *done[at].lock().unwrap() = expand_part(rows, words, wheels, cut.clone());
            });
        }
    });
    done.into_iter().map(|part| part.into_inner().unwrap()).collect()
}

struct Run {
    claim_s: f64,
    lookups: usize,
    new: usize,
    levels: usize,
    digest: u64,
    bytes: usize,
}

fn digest_of(rows: &[u64], parents: &[u32]) -> u64 {
    let mut digest = 0xcbf2_9ce4_8422_2325_u64;
    for word in rows {
        digest = (digest ^ word).wrapping_mul(0x0000_0100_0000_01B3);
    }
    for parent in parents {
        digest = (digest ^ u64::from(*parent)).wrapping_mul(0x0000_0100_0000_01B3);
    }
    digest
}

fn search<I: ShardIndex>(make: impl Fn() -> I, words: usize, wheels: u32, threads: usize, helpers: usize) -> Run {
    let mut tables: Vec<I> = (0..SHARD_COUNT).map(|_| make()).collect();
    let mut rows: Vec<u64> = Vec::new();
    let mut parents: Vec<u32> = Vec::new();
    let mut start = Part::default();
    push_row(&mut start.rows, 0, words);
    start.hashes.push(hash_words(&start.rows));
    start.froms.push(u32::MAX);
    file_by_shard(&mut start);
    let mut parts = vec![start];
    let mut frontier_start = 0;
    let (mut claim_s, mut lookups, mut levels) = (0.0, 0, 0);
    loop {
        let count = parents.len();
        let at = Level { parts: &parts, words };
        lookups += parts.iter().map(|part| part.hashes.len()).sum::<usize>();
        let clock = Instant::now();
        let wins: Vec<Won> = per_shard(threads, &mut tables, |shard, table| table.claim(shard, &at, &rows, count));
        claim_s += clock.elapsed().as_secs_f64();
        let maps: Vec<Winners> =
            (0..parts.len()).map(|part| Winners::of(&wins, part, parts[part].hashes.len())).collect();
        let mut bases = Vec::with_capacity(maps.len());
        let mut total = 0;
        for map in &maps {
            bases.push(total);
            total += map.count;
        }
        for (part, map) in maps.iter().enumerate() {
            for ordinal in map.ordinals() {
                rows.extend_from_slice(at.row_of(part, ordinal));
                parents.push(parts[part].froms[ordinal]);
            }
        }
        let ids = Ids { count, maps: &maps, bases: &bases };
        let clock = Instant::now();
        if words > 1 {
            per_shard(threads, &mut tables, |shard, table| table.file(&wins[shard], &at, &rows, &ids));
        }
        claim_s += clock.elapsed().as_secs_f64();
        levels += 1;
        if total == 0 {
            break;
        }
        let frontier = frontier_start..parents.len();
        frontier_start = parents.len();
        parts = expand(&rows, words, wheels, frontier, helpers);
    }
    verify(&tables, &rows, words);
    Run {
        claim_s,
        lookups,
        new: parents.len(),
        levels,
        digest: digest_of(&rows, &parents),
        bytes: tables.iter().map(ShardIndex::bytes).sum(),
    }
}

const VERIFY_STEP: usize = 61;

fn verify<I: ShardIndex>(tables: &[I], rows: &[u64], words: usize) {
    let count = rows.len() / words;
    for id in (0..count).step_by(VERIFY_STEP).chain(std::iter::once(count - 1)) {
        let row = row_at(rows, words, id);
        let hash = hash_words(row);
        match tables[shard_of(hash)].find(row, hash, rows) {
            Some(None) => {}
            Some(Some(found)) => assert_eq!(found, id, "the index gives row {id} another id"),
            None => panic!("the index lost row {id}"),
        }
    }
    let mut absent = Vec::new();
    push_row(&mut absent, 0xA, words);
    let hash = hash_words(&absent);
    assert!(tables[shard_of(hash)].find(&absent, hash, rows).is_none(), "the index holds a row never claimed");
}

fn run_variant(name: &str, batch: usize, words: usize, wheels: u32, threads: usize, helpers: usize) -> Run {
    match (name, words) {
        ("hashbrown", 1) => search(BrownKey::default, words, wheels, threads, helpers),
        ("hashbrown", _) => search(|| BrownId { words, ..BrownId::default() }, words, wheels, threads, helpers),
        ("own", 1) => search(|| OwnKey::<NO_LOAD> { table: KeyTable::new(), batch }, words, wheels, threads, helpers),
        ("load", 1) => search(|| OwnKey::<PLAIN_LOAD> { table: KeyTable::new(), batch }, words, wheels, threads, helpers),
        ("prefetch", 1) => search(|| OwnKey::<PREFETCH> { table: KeyTable::new(), batch }, words, wheels, threads, helpers),
        ("own", _) => search(
            || OwnId::<NO_LOAD> { table: IdTable::new(), batch, words, pending: Vec::new() },
            words, wheels, threads, helpers,
        ),
        ("load", _) => search(
            || OwnId::<PLAIN_LOAD> { table: IdTable::new(), batch, words, pending: Vec::new() },
            words, wheels, threads, helpers,
        ),
        ("prefetch", _) => search(
            || OwnId::<PREFETCH> { table: IdTable::new(), batch, words, pending: Vec::new() },
            words, wheels, threads, helpers,
        ),
        _ => panic!("unknown variant {name}"),
    }
}

fn list(value: &str) -> Vec<usize> {
    value.split(',').map(|item| item.trim().parse().expect("a number")).collect()
}

fn median(values: &mut [f64]) -> f64 {
    values.sort_by(f64::total_cmp);
    let mid = values.len() / 2;
    if values.len() % 2 == 1 { values[mid] } else { (values[mid - 1] + values[mid]) / 2.0 }
}

fn main() {
    let mut wheels = 7;
    let mut word_counts = vec![1, 2];
    let mut thread_counts = vec![1, 4];
    let mut batches = vec![16];
    let mut rounds = 3;
    let mut args = std::env::args().skip(1);
    while let Some(flag) = args.next() {
        let value = args.next().unwrap_or_else(|| panic!("{flag} needs a value"));
        match flag.as_str() {
            "--states" => {
                wheels = match value.as_str() {
                    "10m" => 7,
                    "100m" => 8,
                    other => panic!("--states is 10m or 100m, not {other}"),
                }
            }
            "--words" => word_counts = list(&value),
            "--threads" => thread_counts = list(&value),
            "--batch" => batches = list(&value),
            "--rounds" => rounds = value.parse().expect("a number"),
            other => panic!("unknown flag {other}"),
        }
    }
    let helpers = std::thread::available_parallelism().map_or(4, usize::from);
    let mut variants: Vec<(String, &str, usize)> = vec![("hashbrown".into(), "hashbrown", 0)];
    for batch in &batches {
        variants.push((format!("own/{batch}"), "own", *batch));
        variants.push((format!("load/{batch}"), "load", *batch));
        if cfg!(feature = "prefetch") {
            variants.push((format!("prefetch/{batch}"), "prefetch", *batch));
        }
    }
    println!(
        "wheels={wheels} states={} rounds={rounds} prefetch_feature={} arch={} expansion_helpers={helpers}",
        10_u64.pow(wheels),
        cfg!(feature = "prefetch"),
        std::env::consts::ARCH
    );
    let mut failed = false;
    for words in &word_counts {
        let mut reference: Option<(usize, usize, u64)> = None;
        for threads in &thread_counts {
            let mut times: Vec<Vec<f64>> = vec![Vec::new(); variants.len()];
            let mut bytes = vec![0; variants.len()];
            let mut counts = (0, 0, 0);
            for round in 0..rounds {
                for (at, (label, name, batch)) in variants.iter().enumerate() {
                    let run = run_variant(name, *batch, *words, wheels, *threads, helpers);
                    println!(
                        "words={words} threads={threads} round={round} variant={label} claim_s={:.4} new={} lookups={} found={} levels={} index_bytes={} digest={:016x}",
                        run.claim_s, run.new, run.lookups, run.lookups - run.new, run.levels, run.bytes, run.digest
                    );
                    let seen = (run.new, run.lookups, run.digest);
                    match reference {
                        None => reference = Some(seen),
                        Some(expected) if expected != seen => {
                            println!("MISMATCH: {label} gave {seen:?}, expected {expected:?}");
                            failed = true;
                        }
                        Some(_) => {}
                    }
                    times[at].push(run.claim_s);
                    bytes[at] = run.bytes;
                    counts = (run.new, run.lookups, run.levels);
                }
            }
            println!(
                "SUMMARY words={words} threads={threads} new={} lookups={} found={} levels={}",
                counts.0, counts.1, counts.1 - counts.0, counts.2
            );
            let base = median(&mut times[0].clone());
            for (at, (label, _, _)) in variants.iter().enumerate() {
                let smallest = times[at].iter().copied().fold(f64::INFINITY, f64::min);
                let middle = median(&mut times[at].clone());
                println!(
                    "  {label:<14} min_s={smallest:.4} median_s={middle:.4} vs_hashbrown={:+.1}% index_bytes_per_state={:.2}",
                    (middle / base - 1.0) * 100.0,
                    bytes[at] as f64 / counts.0 as f64
                );
            }
        }
    }
    if failed {
        println!("CHECK FAILED: the variants disagree");
        std::process::exit(1);
    }
    println!("CHECK OK: every variant and thread count gave the same new rows, ids and parents");
}
