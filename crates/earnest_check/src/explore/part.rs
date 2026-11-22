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
    met_words: HashTable<u64>,
    met_raw: FxHashSet<Box<[Value]>>,
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
        let met = self.kit.met_words.entry(
            hash,
            |earlier| *earlier == word,
            |earlier| hash_words(&[*earlier]),
        );
        let hashbrown::hash_table::Entry::Vacant(vacant) = met else {
            return false;
        };
        vacant.insert(word);
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
        mut part: Part,
    ) -> Part {
        let expand = self.expand(deadline, asked);
        kit.rows.met.clear();
        kit.rows.met_words.clear();
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
    use super::{Part, PartSink, RowKit, Spares};
    use crate::store::{Lookup, SHARD_COUNT, Store};
    use earnest_ir::Field;
    use earnest_value::{Domain, Symbols, Value};

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
