use super::encode::{encode_next, with_updates};
use super::{STATES_BETWEEN_CLOCK_READS, Search};
use crate::expand::{Asked, Base, Clock, Mark, Scratch, Successors, same_words};
use crate::store::{SHARD_COUNT, Store, hash_words, shard_of};
use earnest_value::Value;
use hashbrown::HashTable;
use rustc_hash::FxHashSet;
use std::ops::Range;
use std::sync::{Mutex, PoisonError};
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug)]
pub(super) enum Target {
    Known(usize),
    Fresh(usize),
    Raw { fresh_before: usize },
}

#[derive(Clone, Copy, Debug)]
pub(super) struct Entry {
    pub(super) from: usize,
    pub(super) action: usize,
    pub(super) target: Target,
}

#[derive(Default)]
pub(super) struct Part {
    pub(super) first: usize,
    pub(super) expanded: usize,
    pub(super) marks: Vec<(usize, Mark)>,
    pub(super) entries: Vec<Entry>,
    pub(super) fresh: usize,
    pub(super) rows: Vec<u64>,
    pub(super) hashes: Vec<u64>,
    pub(super) froms: Vec<u32>,
    pub(super) ordinals: Vec<u32>,
    pub(super) shard_starts: Vec<usize>,
    pub(super) raws: Vec<Box<[Value]>>,
    pub(super) stopped: bool,
    pub(super) crowded: bool,
    pub(super) took: Duration,
    next: Vec<usize>,
}

#[derive(Default)]
struct FreshRows {
    rows: Vec<u64>,
    hashes: Vec<u64>,
    froms: Vec<u32>,
}

#[derive(Default)]
pub(super) struct Spares {
    parts: Mutex<Vec<Part>>,
    kits: Mutex<Vec<Kit>>,
    tables: Mutex<Vec<RecentWords>>,
}

impl Spares {
    pub(super) fn part(&self) -> Part {
        self.parts
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .pop()
            .unwrap_or_default()
    }

    pub(super) fn kit(&self) -> Kit {
        self.kits
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .pop()
            .unwrap_or_default()
    }

    pub(super) fn keep_kit(&self, kit: Kit) {
        self.kits
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(kit);
    }

    fn table(&self) -> RecentWords {
        self.tables
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .pop()
            .unwrap_or_default()
    }

    pub(super) fn keep_table_of(&self, kit: Kit) {
        let table = kit.rows.recent_words;
        if !table.is_clean() {
            return;
        }
        let mut kept = self.tables.lock().unwrap_or_else(PoisonError::into_inner);
        if kept.len() < MOST_SPARE_TABLES {
            kept.push(table);
        }
    }

    pub(super) fn keep(&self, parts: Vec<Part>, most: usize) {
        let mut kept = self.parts.lock().unwrap_or_else(PoisonError::into_inner);
        for mut part in parts {
            if kept.len() >= most {
                return;
            }
            part.empty();
            kept.push(part);
        }
    }
}

#[derive(Default)]
pub(super) struct Kit {
    rows: RowKit,
    spare_rows: FreshRows,
    fresh_per_state: (usize, usize),
    scratch: Scratch,
    state: Vec<Value>,
    base: Vec<u64>,
}

impl Kit {
    pub(super) fn drop_spare_rows(&mut self) {
        self.spare_rows = FreshRows::default();
    }
}

#[derive(Default)]
struct RowKit {
    row: Vec<u64>,
    met: HashTable<usize>,
    recent_words: RecentWords,
    met_raw: FxHashSet<Box<[Value]>>,
}

const RECENT_SLOTS: usize = 1 << 16;
const FEWEST_RECENT_SLOTS: usize = 1 << 10;
const RECENT_GROWTH: usize = 4;
const MOST_SPARE_TABLES: usize = 64;
const TRIES_FOR_A_MARK: u64 = 64;

#[derive(Default)]
struct RecentWords {
    words: Vec<u64>,
    marks: Option<[u64; 2]>,
    written: usize,
}

fn slot_of(hash: u64, slots: usize) -> usize {
    let mask = u64::try_from(slots.wrapping_sub(1)).unwrap_or(u64::MAX);
    usize::try_from(hash & mask).unwrap_or(0)
}

fn low_bit(hash: u64) -> usize {
    usize::from(hash & 1 == 1)
}

fn marks() -> Option<[u64; 2]> {
    let mut found = [None, None];
    for word in 0..TRIES_FOR_A_MARK {
        let side = found.get_mut(low_bit(hash_words(&[word])))?;
        side.get_or_insert(word);
        if let [Some(even), Some(odd)] = found {
            return Some([even, odd]);
        }
    }
    None
}

impl RecentWords {
    fn is_clean(&self) -> bool {
        self.written == 0
    }

    fn mark_of(marks: [u64; 2], at: usize) -> u64 {
        let [even, odd] = marks;
        if at & 1 == 1 { even } else { odd }
    }

    fn emptied(marks: [u64; 2], count: usize) -> Vec<u64> {
        (0..count).map(|at| Self::mark_of(marks, at)).collect()
    }

    fn ready(&mut self) {
        if self.words.is_empty() {
            self.marks = marks();
            if let Some(marks) = self.marks {
                self.words = Self::emptied(marks, FEWEST_RECENT_SLOTS);
            }
        }
    }

    #[inline]
    fn met(&mut self, word: u64, hash: u64) -> bool {
        let at = slot_of(hash, self.words.len());
        let Some(slot) = self.words.get_mut(at) else {
            return false;
        };
        if *slot == word {
            return true;
        }
        *slot = word;
        self.written += 1;
        let count = self.words.len();
        if self.written > count / 2 && count < RECENT_SLOTS {
            self.grow();
        }
        false
    }

    #[cold]
    fn grow(&mut self) {
        let Some(marks) = self.marks else {
            return;
        };
        let count = self.words.len().saturating_mul(RECENT_GROWTH);
        let mut grown = Self::emptied(marks, count);
        for (at, word) in self.words.iter().enumerate() {
            if *word != Self::mark_of(marks, at)
                && let Some(slot) = grown.get_mut(slot_of(hash_words(&[*word]), count))
            {
                *slot = *word;
            }
        }
        self.words = grown;
    }

    fn forget(&mut self, part: &Part) {
        let Some(marks) = self.marks.filter(|_| self.written > 0) else {
            return;
        };
        let count = self.words.len();
        if part.crowded || part.hashes.len() >= count / 4 {
            for (at, slot) in self.words.iter_mut().enumerate() {
                *slot = Self::mark_of(marks, at);
            }
        } else {
            for hash in &part.hashes {
                let at = slot_of(*hash, count);
                if let Some(slot) = self.words.get_mut(at) {
                    *slot = Self::mark_of(marks, at);
                }
            }
        }
        self.written = 0;
    }
}

struct PartSink<'a> {
    store: &'a Store,
    graph: bool,
    from: usize,
    from_offset: u32,
    counted: usize,
    kit: &'a mut RowKit,
    part: Part,
}

impl PartSink<'_> {
    fn push(&mut self, action: usize, target: Target) {
        self.part.entries.push(Entry {
            from: self.from,
            action,
            target,
        });
    }

    #[inline]
    fn add_fresh(&mut self, action: usize, row: &[u64], hash: u64) {
        let at = self.part.fresh;
        let Some(fresh) = at
            .checked_add(1)
            .filter(|fresh| u32::try_from(*fresh).is_ok())
        else {
            self.part.crowded = true;
            return;
        };
        self.part.fresh = fresh;
        append_row(&mut self.part.rows, row);
        self.part.hashes.push(hash);
        if self.graph {
            self.push(action, Target::Fresh(at));
        } else {
            self.part.froms.push(self.from_offset);
        }
    }

    fn offer_fresh(&mut self, action: usize, row: &[u64], hash: u64) -> bool {
        let words = self.store.layout().words();
        let (rows, hashes) = (&self.part.rows, &self.part.hashes);
        let met = self.kit.met.entry(
            hash,
            |at| same_words(row_at(rows, words, *at), row),
            |at| hashes.get(*at).copied().unwrap_or(0),
        );
        match met {
            hashbrown::hash_table::Entry::Occupied(earlier) => {
                if self.graph {
                    let at = *earlier.get();
                    self.push(action, Target::Fresh(at));
                }
                false
            }
            hashbrown::hash_table::Entry::Vacant(vacant) => {
                if !self.graph && self.store.find(row, hash).is_some() {
                    return false;
                }
                vacant.insert(self.part.fresh);
                self.add_fresh(action, row, hash);
                true
            }
        }
    }

    #[inline]
    fn offer_unseen(&mut self, action: usize, row: &[u64], hash: u64) -> bool {
        if let [word] = row {
            return self.offer_unseen_word(action, *word, hash);
        }
        self.offer_fresh(action, row, hash)
    }

    #[inline]
    fn offer_unseen_word(&mut self, action: usize, word: u64, hash: u64) -> bool {
        if self.kit.recent_words.met(word, hash) {
            return false;
        }
        self.add_fresh(action, &[word], hash);
        true
    }

    fn offer_unless_met(
        &mut self,
        action: usize,
        base: Base<'_>,
        updates: &[(usize, Value)],
    ) -> bool {
        if !encode_next(
            self.store,
            base.row,
            &mut self.kit.row,
            base.values,
            updates,
        ) {
            let next = with_updates(base.values, updates);
            if let Some(to) = self.store.find_values(&next) {
                if self.graph {
                    self.push(action, Target::Known(to));
                }
                return false;
            }
            let first = self.kit.met_raw.insert(next.clone());
            if !self.graph && !first {
                return false;
            }
            self.part.raws.push(next);
            let fresh_before = self.part.fresh;
            self.push(action, Target::Raw { fresh_before });
            return first;
        }
        let row = std::mem::take(&mut self.kit.row);
        let hash = hash_words(&row);
        let counted = self.offer_encoded(action, &row, hash);
        self.kit.row = row;
        counted
    }

    #[inline]
    fn offer_encoded(&mut self, action: usize, row: &[u64], hash: u64) -> bool {
        if !self.graph {
            return self.offer_unseen(action, row, hash);
        }
        if let Some(found) = self.store.find(row, hash) {
            if let Some(to) = found.id {
                self.push(action, Target::Known(to));
            }
            return false;
        }
        self.offer_fresh(action, row, hash)
    }
}

impl Successors for PartSink<'_> {
    fn offer(
        &mut self,
        action: usize,
        _: usize,
        base: Base<'_>,
        updates: &[(usize, Value)],
    ) -> bool {
        let counted = self.offer_unless_met(action, base, updates);
        self.counted += usize::from(counted);
        counted
    }

    fn offer_row(&mut self, action: usize, _: usize, row: &[u64], hash: u64) -> bool {
        let counted = self.offer_encoded(action, row, hash);
        self.counted += usize::from(counted);
        counted
    }
}

impl Part {
    pub(super) fn kept_rows(&self) -> usize {
        self.fresh.saturating_add(self.raws.len())
    }

    pub(super) fn hash_at(&self, at: usize) -> u64 {
        self.hashes.get(at).copied().unwrap_or(0)
    }

    pub(super) fn parent_at(&self, at: usize) -> usize {
        let offset = self.froms.get(at).copied().unwrap_or(0);
        self.first
            .saturating_add(usize::try_from(offset).unwrap_or(usize::MAX))
    }

    pub(super) fn by_ordinal(&self) -> Option<Vec<usize>> {
        if self.shard_starts.is_empty() {
            return None;
        }
        let mut positions = vec![0; self.fresh];
        for (position, ordinal) in self.ordinals.iter().enumerate() {
            if let Some(slot) = usize::try_from(*ordinal)
                .ok()
                .and_then(|ordinal| positions.get_mut(ordinal))
            {
                *slot = position;
            }
        }
        Some(positions)
    }

    fn empty(&mut self) {
        self.first = 0;
        self.expanded = 0;
        self.marks.clear();
        self.entries.clear();
        self.fresh = 0;
        self.rows.clear();
        self.hashes.clear();
        self.froms.clear();
        self.ordinals.clear();
        self.shard_starts.clear();
        self.raws.clear();
        self.stopped = false;
        self.crowded = false;
        self.took = Duration::ZERO;
    }

    fn file_by_shard(&mut self, words: usize, filed: &mut FreshRows) {
        let starts = &mut self.shard_starts;
        starts.clear();
        starts.resize(SHARD_COUNT + 1, 0);
        for hash in &self.hashes {
            if let Some(count) = starts.get_mut(shard_of(*hash) + 1) {
                *count += 1;
            }
        }
        for shard in 0..SHARD_COUNT {
            let before = starts.get(shard).copied().unwrap_or(0);
            if let Some(start) = starts.get_mut(shard + 1) {
                *start += before;
            }
        }
        let count = self.fresh;
        self.next.clone_from(starts);
        filed.rows.clear();
        filed.rows.resize(self.rows.len(), 0);
        filed.hashes.clear();
        filed.hashes.resize(self.hashes.len(), 0);
        filed.froms.clear();
        filed.froms.resize(count, 0);
        self.ordinals.clear();
        self.ordinals.resize(count, 0);
        for ((at, hash), ordinal) in self.hashes.iter().enumerate().zip(0_u32..) {
            let hash = *hash;
            let Some(next) = self.next.get_mut(shard_of(hash)) else {
                continue;
            };
            let position = *next;
            *next += 1;
            if let (Some(to_ordinal), Some(to_from), Some(from)) = (
                self.ordinals.get_mut(position),
                filed.froms.get_mut(position),
                self.froms.get(at),
            ) {
                *to_ordinal = ordinal;
                *to_from = *from;
            }
            if let Some(to_hash) = filed.hashes.get_mut(position) {
                *to_hash = hash;
            }
            move_row(&self.rows, at, &mut filed.rows, position, words);
        }
        std::mem::swap(&mut self.rows, &mut filed.rows);
        std::mem::swap(&mut self.hashes, &mut filed.hashes);
        std::mem::swap(&mut self.froms, &mut filed.froms);
    }
}

fn move_row(from: &[u64], at: usize, into: &mut [u64], place: usize, words: usize) {
    if words == 1 {
        if let (Some(word), Some(slot)) = (from.get(at), into.get_mut(place)) {
            *slot = *word;
        }
        return;
    }
    let start = place.saturating_mul(words);
    if let Some(slots) = into.get_mut(start..start.saturating_add(words)) {
        for (slot, word) in slots.iter_mut().zip(row_at(from, words, at)) {
            *slot = *word;
        }
    }
}

fn append_row(rows: &mut Vec<u64>, row: &[u64]) {
    if let [word] = row {
        rows.push(*word);
    } else {
        rows.extend_from_slice(row);
    }
}

pub(super) fn row_at(rows: &[u64], words: usize, at: usize) -> &[u64] {
    let start = at.saturating_mul(words);
    rows.get(start..start.saturating_add(words)).unwrap_or(&[])
}

impl Search<'_> {
    pub(super) fn expand_ids(
        &self,
        ids: Range<usize>,
        deadline: Option<Instant>,
        asked: Asked,
        kit: &mut Kit,
        part: Part,
    ) -> Part {
        if kit.rows.recent_words.words.is_empty() {
            kit.rows.recent_words = self.spares.table();
            kit.rows.recent_words.ready();
        }
        let part = self.fill_part(ids, deadline, asked, kit, part);
        kit.rows.recent_words.forget(&part);
        part
    }

    fn fill_part(
        &self,
        ids: Range<usize>,
        deadline: Option<Instant>,
        asked: Asked,
        kit: &mut Kit,
        mut part: Part,
    ) -> Part {
        let expand = self.expand(deadline, asked);
        kit.rows.met.clear();
        kit.rows.met_raw.clear();
        let (fresh_before, states_before) = kit.fresh_per_state;
        let expected = (ids.len() * fresh_before)
            .checked_div(states_before)
            .unwrap_or(0);
        let words = self.store.layout().words();
        part.first = ids.start;
        if !self.limits.keep_graph {
            part.rows.reserve(expected * words);
            part.hashes.reserve(expected);
            part.froms.reserve(expected);
        }
        let mut sink = PartSink {
            store: &self.store,
            graph: self.limits.keep_graph,
            from: 0,
            from_offset: 0,
            counted: 0,
            kit: &mut kit.rows,
            part,
        };
        let stops_at = deadline.filter(|_| asked == Asked::Everything);
        for (position, id) in ids.enumerate() {
            let clock_due = position.is_multiple_of(STATES_BETWEEN_CLOCK_READS);
            if clock_due && stops_at.is_some_and(|limit| Instant::now() >= limit) {
                sink.part.stopped = true;
                return sink.part;
            }
            let is_row = self.store.read_state(id, &mut kit.base, &mut kit.state);
            let outside = self.store.outside(id, &kit.base, &self.model.fields);
            let Ok(from_offset) = u32::try_from(id.saturating_sub(sink.part.first)) else {
                sink.part.crowded = true;
                return sink.part;
            };
            sink.from = id;
            sink.from_offset = from_offset;
            sink.counted = 0;
            let base = Base {
                values: &kit.state,
                row: is_row.then_some(kit.base.as_slice()),
            };
            let mark = expand.state(base, outside, &mut sink, &mut kit.scratch);
            if mark.clock == Clock::Stopped {
                sink.part.stopped = true;
                return sink.part;
            }
            if sink.part.crowded || sink.counted > self.limits.max_states {
                sink.part.crowded = true;
                return sink.part;
            }
            if !mark.is_plain() {
                sink.part.marks.push((id, mark));
            }
            sink.part.expanded = sink.part.expanded.saturating_add(1);
        }
        if !self.limits.keep_graph {
            sink.part.file_by_shard(words, &mut kit.spare_rows);
        }
        kit.fresh_per_state = (sink.part.fresh, sink.part.expanded);
        sink.part
    }
}

#[cfg(test)]
mod tests {
    use super::{
        FEWEST_RECENT_SLOTS, FreshRows, Kit, MOST_SPARE_TABLES, Part, PartSink, RECENT_SLOTS,
        RecentWords, RowKit, Spares, slot_of,
    };
    use crate::expand::Successors;
    use crate::store::{Fresh, Lookup, SHARD_COUNT, Spread, Store, hash_words};
    use earnest_ir::Field;
    use earnest_value::{Domain, Symbols, Value};
    use std::collections::{HashMap, HashSet};

    struct OneThread;

    impl Spread for OneThread {
        fn run(&self, tasks: usize, job: &(dyn Fn(usize) + Sync)) {
            for task in 0..tasks {
                job(task);
            }
        }
    }

    fn fields_of(names: &[&str], hi: i64) -> Vec<Field> {
        let mut symbols = Symbols::default();
        names
            .iter()
            .map(|name| Field {
                name: (*name).to_owned(),
                sym: symbols.intern(name),
                domain: Domain::Int { lo: 0, hi },
                starts: vec![Value::Int(0)],
            })
            .collect()
    }

    fn row_of(store: &Store, values: &[i64]) -> Vec<u64> {
        let values: Vec<Value> = values.iter().map(|value| Value::Int(*value)).collect();
        let mut row = Vec::new();
        assert!(
            store.encode(&values, &mut row),
            "every value is in its domain"
        );
        row
    }

    struct Offer {
        from: usize,
        values: Vec<i64>,
    }

    fn offers_of(
        parts: usize,
        per_part: usize,
        first: usize,
        spread: i64,
        arity: usize,
    ) -> Vec<Vec<Offer>> {
        let mut seed: u64 = 0x2545_F491_4F6C_DD1D;
        (0..parts)
            .map(|part| {
                (0..per_part)
                    .map(|at| {
                        let values = (0..arity)
                            .map(|_| {
                                seed = seed
                                    .wrapping_mul(6_364_136_223_846_793_005)
                                    .wrapping_add(1_442_695_040_888_963_407);
                                i64::try_from(seed >> 33).unwrap_or(0) % spread
                            })
                            .collect();
                        Offer {
                            from: first + part * 4 + at % 4,
                            values,
                        }
                    })
                    .collect()
            })
            .collect()
    }

    fn part_of(store: &Store, kit: &mut RowKit, first: usize, offers: &[Offer]) -> (Part, usize) {
        let mut sink = PartSink {
            store,
            graph: false,
            from: 0,
            from_offset: 0,
            counted: 0,
            kit,
            part: Part {
                first,
                ..Part::default()
            },
        };
        if sink.kit.recent_words.words.is_empty() {
            sink.kit.recent_words.ready();
        }
        let mut counted = 0;
        for offer in offers {
            sink.from = offer.from;
            sink.from_offset = u32::try_from(offer.from - first).expect("a small offset");
            let row = row_of(store, &offer.values);
            counted += usize::from(sink.offer_row(0, 0, &row, hash_words(&row)));
        }
        let mut part = sink.part;
        part.expanded = 4;
        part.file_by_shard(store.layout().words(), &mut FreshRows::default());
        kit.recent_words.forget(&part);
        (part, counted)
    }

    struct Reference {
        ids: HashMap<Vec<u64>, usize>,
        parents: Vec<Option<usize>>,
    }

    impl Reference {
        fn place(&mut self, row: Vec<u64>, parent: Option<usize>) {
            let next = self.parents.len();
            if let std::collections::hash_map::Entry::Vacant(vacant) = self.ids.entry(row) {
                vacant.insert(next);
                self.parents.push(parent);
            }
        }
    }

    fn a_level_in_parts_is_the_level_row_by_row(names: &[&str], hi: i64, spread: i64) {
        let fields = fields_of(names, hi);
        let mut store = Store::new(&fields, 10_000, Lookup::Presence);
        let mut by_rows = Store::new(&fields, 10_000, Lookup::Presence);
        let mut reference = Reference {
            ids: HashMap::new(),
            parents: Vec::new(),
        };
        let arity = names.len();
        let known: Vec<Vec<i64>> = (0..16).map(|at| vec![at % spread; arity]).collect();
        for values in &known {
            let row = row_of(&store, values);
            let hash = hash_words(&row);
            store.place(&row, hash, None, usize::MAX);
            by_rows.place(&row, hash, None, usize::MAX);
            reference.place(row, None);
        }
        let first = 0;
        let offers = offers_of(3, 400, first, spread, arity);
        let mut kit = RowKit::default();
        let mut parts = Vec::new();
        let mut all_counted = 0;
        for offered in &offers {
            let (part, counted) = part_of(&store, &mut kit, first, offered);
            assert!(!part.crowded, "a small part is not crowded");
            all_counted += counted;
            parts.push(part);
        }
        for offered in &offers {
            for offer in offered {
                let row = row_of(&store, &offer.values);
                by_rows.place(&row, hash_words(&row), Some(offer.from), usize::MAX);
                reference.place(row, Some(offer.from));
            }
        }
        let fresh: Vec<Fresh<'_>> = parts
            .iter()
            .map(|part| Fresh {
                count: part.fresh,
                rows: &part.rows,
                hashes: &part.hashes,
                first: part.first,
                froms: &part.froms,
                ordinals: &part.ordinals,
                shard_starts: &part.shard_starts,
            })
            .collect();
        let before = store.len();
        let new = reference.parents.len() - before;
        assert!(new > 100, "the offers find many new rows");
        assert!(
            all_counted >= new,
            "every new row was counted by the part that found it first"
        );
        assert_eq!(
            store.place_level(&fresh, &OneThread, before + new - 1),
            None,
            "one state short of the limit, nothing is placed"
        );
        assert_eq!(store.len(), before, "the level is taken back whole");
        assert_eq!(
            store.place_level(&fresh, &OneThread, before + new),
            Some(new),
            "every row new to the level, once"
        );
        assert_eq!(store.len(), reference.parents.len(), "the counts agree");
        for (id, parent) in reference.parents.iter().enumerate() {
            assert_eq!(store.parent(id), *parent, "the parent of {id}");
            assert_eq!(store.state(id), by_rows.state(id), "the state of {id}");
        }
        let distinct: HashSet<Vec<i64>> = offers
            .iter()
            .flatten()
            .map(|offer| offer.values.clone())
            .collect();
        assert!(
            distinct.len() < offers.iter().map(Vec::len).sum(),
            "some offers repeat"
        );
    }

    #[test]
    fn a_level_of_one_word_rows_taken_in_parts_gives_the_ids_and_parents_of_the_level_row_by_row() {
        a_level_in_parts_is_the_level_row_by_row(&["a", "b", "c"], 999, 9);
    }

    #[test]
    fn a_level_of_rows_of_several_words_taken_in_parts_gives_the_ids_and_parents_of_the_level_row_by_row()
     {
        a_level_in_parts_is_the_level_row_by_row(&["a", "b"], 1 << 40, 23);
    }

    fn ready_table() -> RecentWords {
        let mut recent = RecentWords::default();
        recent.ready();
        assert!(
            !recent.words.is_empty(),
            "the hash gives marks of both parities"
        );
        recent
    }

    fn word_hashes(words: std::ops::Range<u64>) -> Vec<(u64, u64)> {
        words.map(|word| (word, hash_words(&[word]))).collect()
    }

    #[test]
    fn a_slot_keeps_the_low_bits_of_a_hash_of_any_width() {
        for hash in [0, 1, 0xFFFF, 1 << 32, (1 << 40) | 5, u64::MAX, u64::MAX - 1] {
            for slots in [FEWEST_RECENT_SLOTS, RECENT_SLOTS] {
                let wanted = hash % u64::try_from(slots).expect("a small count");
                assert_eq!(
                    u64::try_from(slot_of(hash, slots)).ok(),
                    Some(wanted),
                    "hash {hash:#x}, {slots} slots"
                );
            }
        }
    }

    #[test]
    fn recent_words_meet_a_row_let_in_and_never_one_that_was_not() {
        let mut recent = ready_table();
        let rows = word_hashes(0..1 << 18);
        let fresh = rows
            .iter()
            .filter(|(word, hash)| !recent.met(*word, *hash))
            .count();
        assert_eq!(fresh, rows.len(), "nothing is met before it was let in");
        let (word, hash) = rows.last().copied().unwrap_or((0, 0));
        assert!(recent.met(word, hash), "the row let in last is met again");
    }

    #[test]
    fn recent_words_tell_apart_two_words_that_share_a_hash() {
        let mut recent = ready_table();
        for hash in [0, 1, 7, 1 << 40, u64::MAX] {
            assert!(!recent.met(10, hash), "a new row");
            assert!(
                !recent.met(11, hash),
                "another row with the same hash is new too"
            );
            assert!(recent.met(11, hash), "the row kept in that slot is met");
        }
    }

    #[test]
    fn recent_words_meet_no_mark_of_an_empty_slot() {
        let recent = ready_table();
        let [even, odd] = recent.marks.unwrap_or([0, 0]);
        assert_ne!(
            hash_words(&[even]) & 1,
            hash_words(&[odd]) & 1,
            "the two marks land in slots of either parity"
        );
        let mut fresh = ready_table();
        for (word, hash) in [even, odd].map(|word| (word, hash_words(&[word]))) {
            assert!(!fresh.met(word, hash), "a mark, offered as a row, is new");
        }
    }

    #[test]
    fn recent_words_grow_to_their_most_slots_and_still_meet_what_they_kept() {
        let mut recent = ready_table();
        let rows = word_hashes(0..1 << 12);
        for (word, hash) in &rows {
            assert!(!recent.met(*word, *hash), "a new row");
        }
        let met = rows
            .iter()
            .filter(|(word, hash)| recent.met(*word, *hash))
            .count();
        assert!(
            met * 2 > rows.len(),
            "more than half the rows are met again after growing: {met}"
        );
        for (word, hash) in word_hashes(1 << 12..1 << 20) {
            recent.met(word, hash);
        }
        assert_eq!(recent.words.len(), RECENT_SLOTS, "it grows no further");
    }

    #[test]
    fn recent_words_forget_every_row_of_a_part_and_everything_after_a_crowded_one() {
        for crowded in [false, true] {
            let mut recent = ready_table();
            let rows = word_hashes(0..100);
            for (word, hash) in &rows {
                assert!(!recent.met(*word, *hash), "a new row");
            }
            let part = Part {
                hashes: if crowded {
                    Vec::new()
                } else {
                    rows.iter().map(|(_, hash)| *hash).collect()
                },
                crowded,
                ..Part::default()
            };
            recent.forget(&part);
            assert!(recent.is_clean(), "nothing is left written");
            for (word, hash) in &rows {
                assert!(!recent.met(*word, *hash), "forgotten, so new again");
            }
        }
    }

    #[test]
    fn spares_keep_only_clean_tables_and_no_more_than_their_most() {
        let spares = Spares::default();
        let mut dirty = Kit::default();
        dirty.rows.recent_words = ready_table();
        dirty.rows.recent_words.met(3, hash_words(&[3]));
        spares.keep_table_of(dirty);
        assert!(
            spares.table().words.is_empty(),
            "a table with rows of a part in it is not kept"
        );
        for _ in 0..=MOST_SPARE_TABLES {
            let mut kit = Kit::default();
            kit.rows.recent_words = ready_table();
            spares.keep_table_of(kit);
        }
        let kept = std::iter::from_fn(|| Some(spares.table()))
            .take_while(|table| !table.words.is_empty())
            .count();
        assert_eq!(kept, MOST_SPARE_TABLES, "the rest are dropped");
    }

    #[test]
    fn a_part_counts_its_fresh_rows_in_32_bits_and_is_crowded_past_them() {
        let mut symbols = Symbols::default();
        let fields = [Field {
            name: "a".to_owned(),
            sym: symbols.intern("a"),
            domain: Domain::Int { lo: 0, hi: 9 },
            starts: vec![Value::Int(0)],
        }];
        let store = Store::new(&fields, 100, Lookup::Presence);
        let mut kit = RowKit::default();
        let most = usize::try_from(u32::MAX).expect("a usize holds every u32");
        let mut sink = PartSink {
            store: &store,
            graph: false,
            from: 0,
            from_offset: 0,
            counted: 0,
            kit: &mut kit,
            part: Part {
                fresh: most - 1,
                ..Part::default()
            },
        };
        sink.add_fresh(0, &[1], 1);
        assert_eq!(
            (sink.part.fresh, sink.part.crowded),
            (most, false),
            "the last count a u32 holds, so every place of a row still fits one"
        );
        sink.add_fresh(0, &[2], 2);
        assert_eq!(
            (sink.part.fresh, sink.part.crowded),
            (most, true),
            "one row more would make the end of the part's last shard 2^32, which no u32 holds"
        );
    }

    fn part_that_found_two_rows(first: usize) -> Part {
        Part {
            first,
            expanded: 3,
            fresh: 2,
            rows: vec![1, 2],
            hashes: vec![5, 6],
            froms: vec![0, 1],
            ordinals: vec![1, 0],
            shard_starts: vec![0; SHARD_COUNT + 1],
            stopped: true,
            crowded: true,
            ..Part::default()
        }
    }

    #[test]
    fn spares_keep_no_more_parts_than_asked_and_hand_each_back_empty_with_its_room() {
        let spares = Spares::default();
        spares.keep(
            vec![
                part_that_found_two_rows(10),
                part_that_found_two_rows(20),
                part_that_found_two_rows(30),
            ],
            2,
        );
        for _ in 0..2 {
            let part = spares.part();
            assert_eq!(
                (part.first, part.expanded, part.fresh),
                (0, 0, 0),
                "nothing found"
            );
            assert!(
                part.rows.is_empty()
                    && part.hashes.is_empty()
                    && part.froms.is_empty()
                    && part.ordinals.is_empty()
                    && part.shard_starts.is_empty(),
                "every buffer emptied"
            );
            assert!(
                !part.stopped && !part.crowded,
                "neither stopped nor crowded"
            );
            assert!(part.rows.capacity() >= 2, "the room its rows grew to stays");
        }
        assert_eq!(
            spares.part().rows.capacity(),
            0,
            "a third part was not kept, so a new one comes"
        );
    }
}
