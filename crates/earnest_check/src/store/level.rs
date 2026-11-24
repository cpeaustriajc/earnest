use super::index::{Filed, SHARD_COUNT, Shards};
use super::rows::{Parents, ParentsTail, Rows, pack};
use hashbrown::HashTable;
use hashbrown::hash_table::Entry as Slot;
use std::ops::Range;
use std::sync::{Mutex, PoisonError};

const BITS_PER_WORD: usize = 64;

pub(crate) trait Spread: Sync {
    fn run(&self, tasks: usize, job: &(dyn Fn(usize) + Sync));
}

pub(crate) struct Fresh<'a> {
    pub(crate) count: usize,
    pub(crate) rows: &'a [u64],
    pub(crate) hashes: &'a [u64],
    pub(crate) first: usize,
    pub(crate) froms: &'a [u32],
    pub(crate) ordinals: &'a [u32],
    pub(crate) shard_starts: &'a [usize],
}

#[derive(Clone, Copy)]
pub(super) struct Level<'a> {
    pub(super) parts: &'a [Fresh<'a>],
    pub(super) spread: &'a dyn Spread,
    pub(super) words: usize,
    pub(super) count: usize,
    pub(super) most: usize,
}

impl Level<'_> {
    fn row_of(&self, part: usize, position: u32) -> &[u64] {
        let start = wide(position).saturating_mul(self.words);
        self.parts
            .get(part)
            .and_then(|fresh| fresh.rows.get(start..start.saturating_add(self.words)))
            .unwrap_or(&[])
    }

    fn hash_of(&self, part: usize, position: u32) -> u64 {
        self.parts
            .get(part)
            .and_then(|fresh| fresh.hashes.get(wide(position)))
            .copied()
            .unwrap_or(0)
    }

    fn parent_of(&self, part: usize, position: u32) -> usize {
        self.parts.get(part).map_or(0, |fresh| {
            let offset = fresh.froms.get(wide(position)).copied().unwrap_or(0);
            fresh.first.saturating_add(wide(offset))
        })
    }

    fn ordinal_of(&self, part: usize, position: u32) -> usize {
        self.parts
            .get(part)
            .and_then(|fresh| fresh.ordinals.get(wide(position)))
            .map_or(0, |ordinal| wide(*ordinal))
    }

    fn in_shard(&self, part: usize, shard: usize) -> Range<u32> {
        self.parts
            .get(part)
            .and_then(|fresh| {
                let start = *fresh.shard_starts.get(shard)?;
                let end = *fresh.shard_starts.get(shard + 1)?;
                let end = end.min(fresh.count).max(start);
                Some(u32::try_from(start).ok()?..u32::try_from(end).ok()?)
            })
            .unwrap_or(0..0)
    }
}

fn wide(position: u32) -> usize {
    usize::try_from(position).unwrap_or(usize::MAX)
}

type Won = Vec<Vec<u32>>;

impl<F: Filed> Shards<F> {
    pub(super) fn place_level(
        &mut self,
        rows: &mut Rows,
        parents: &mut Parents,
        at: Level<'_>,
    ) -> Option<usize> {
        let wins = if F::HOLDS_ROW {
            self.claim_level(rows, &at)
        } else {
            first_in_level(&at)
        };
        let maps: Vec<Winners> = gather(at.spread, at.parts.len(), |part| {
            Winners::of(&wins, &at, part)
        });
        let mut bases = Vec::with_capacity(maps.len());
        let mut total = 0;
        for map in &maps {
            bases.push(total);
            total += map.count;
        }
        if at.count + total > at.most {
            if F::HOLDS_ROW {
                self.unclaim_level(rows, &at, &wins);
            }
            return None;
        }
        write_level(rows, parents, &at, &wins, &maps, total);
        if !F::HOLDS_ROW {
            self.file_level(rows, &at, &wins, &maps, &bases);
        }
        Some(total)
    }

    fn claim_level(&mut self, rows: &Rows, at: &Level<'_>) -> Vec<Won> {
        let tables: Vec<Mutex<&mut HashTable<F>>> = self
            .tables
            .iter_mut()
            .map(|shard| Mutex::new(&mut shard.table))
            .collect();
        gather(at.spread, SHARD_COUNT, |shard| {
            let mut won = Vec::with_capacity(at.parts.len());
            let Some(table) = tables.get(shard) else {
                return won;
            };
            let mut table = table.lock().unwrap_or_else(PoisonError::into_inner);
            for part in 0..at.parts.len() {
                let filed = at.in_shard(part, shard);
                let mut mine = Vec::with_capacity(filed.len());
                for position in filed {
                    let row = at.row_of(part, position);
                    let slot = table.entry(
                        at.hash_of(part, position),
                        |filed| filed.holds(rows, row),
                        |filed| filed.hash(rows),
                    );
                    if let (Slot::Vacant(vacant), Some(filed)) = (slot, F::new(row, 0)) {
                        vacant.insert(filed);
                        mine.push(position);
                    }
                }
                won.push(mine);
            }
            won
        })
    }

    fn unclaim_level(&mut self, rows: &Rows, at: &Level<'_>, wins: &[Won]) {
        let tables: Vec<Mutex<&mut HashTable<F>>> = self
            .tables
            .iter_mut()
            .map(|shard| Mutex::new(&mut shard.table))
            .collect();
        at.spread.run(tables.len(), &|shard| {
            let (Some(table), Some(won)) = (tables.get(shard), wins.get(shard)) else {
                return;
            };
            let mut table = table.lock().unwrap_or_else(PoisonError::into_inner);
            for (part, positions) in won.iter().enumerate() {
                for position in positions {
                    let row = at.row_of(part, *position);
                    let hash = at.hash_of(part, *position);
                    if let Ok(found) = table.find_entry(hash, |filed| filed.holds(rows, row)) {
                        found.remove();
                    }
                }
            }
        });
    }

    fn file_level(
        &mut self,
        rows: &Rows,
        at: &Level<'_>,
        wins: &[Won],
        maps: &[Winners],
        bases: &[usize],
    ) {
        let tables: Vec<Mutex<&mut HashTable<F>>> = self
            .tables
            .iter_mut()
            .map(|shard| Mutex::new(&mut shard.table))
            .collect();
        at.spread.run(tables.len(), &|shard| {
            let (Some(table), Some(won)) = (tables.get(shard), wins.get(shard)) else {
                return;
            };
            let mut table = table.lock().unwrap_or_else(PoisonError::into_inner);
            let hasher = |filed: &F| filed.hash(rows);
            table.reserve(won.iter().map(Vec::len).sum(), hasher);
            for (part, positions) in won.iter().enumerate() {
                let (Some(map), Some(base)) = (maps.get(part), bases.get(part)) else {
                    continue;
                };
                for position in positions {
                    let id = at.count + base + map.rank(at.ordinal_of(part, *position));
                    let row = at.row_of(part, *position);
                    if let Some(filed) = F::new(row, id) {
                        table.insert_unique(at.hash_of(part, *position), filed, hasher);
                    }
                }
            }
        });
    }
}

fn first_in_level(at: &Level<'_>) -> Vec<Won> {
    gather(at.spread, SHARD_COUNT, |shard| {
        let in_shard = (0..at.parts.len()).map(|part| at.in_shard(part, shard).len());
        let mut seen: HashTable<(usize, u32)> = HashTable::with_capacity(in_shard.sum());
        let mut won = Vec::with_capacity(at.parts.len());
        for part in 0..at.parts.len() {
            let filed = at.in_shard(part, shard);
            let mut mine = Vec::with_capacity(filed.len());
            for position in filed {
                let row = at.row_of(part, position);
                if let Slot::Vacant(vacant) = seen.entry(
                    at.hash_of(part, position),
                    |(other, earlier)| at.row_of(*other, *earlier) == row,
                    |(other, earlier)| at.hash_of(*other, *earlier),
                ) {
                    vacant.insert((part, position));
                    mine.push(position);
                }
            }
            won.push(mine);
        }
        won
    })
}

fn write_level(
    rows: &mut Rows,
    parents: &mut Parents,
    at: &Level<'_>,
    wins: &[Won],
    maps: &[Winners],
    total: usize,
) {
    let stride = rows.stride();
    let mut row_rest = rows.grow(total);
    let mut parent_rest = parents.grow(total);
    let mut slices = Vec::with_capacity(maps.len());
    for map in maps {
        let row_count = (map.count * stride).min(row_rest.len());
        let (row_part, row_after) = std::mem::take(&mut row_rest).split_at_mut(row_count);
        row_rest = row_after;
        let (parent_part, parent_after) = parent_rest.split_at(map.count);
        parent_rest = parent_after;
        slices.push(Mutex::new((row_part, parent_part)));
    }
    at.spread.run(maps.len(), &|part| {
        let (Some(map), Some(slice)) = (maps.get(part), slices.get(part)) else {
            return;
        };
        let mut slice = slice.lock().unwrap_or_else(PoisonError::into_inner);
        let (row_part, parent_part): &mut (&mut [u8], ParentsTail<'_>) = &mut slice;
        for won in wins.iter().filter_map(|shard| shard.get(part)) {
            for position in won {
                let place = map.rank(at.ordinal_of(part, *position));
                let start = place * stride;
                if let Some(into) = row_part.get_mut(start..start + stride) {
                    pack(at.row_of(part, *position), into);
                }
                parent_part.set(place, at.parent_of(part, *position));
            }
        }
    });
}

#[derive(Default)]
struct Winners {
    bits: Vec<u64>,
    before: Vec<usize>,
    count: usize,
}

impl Winners {
    fn of(wins: &[Won], at: &Level<'_>, part: usize) -> Self {
        let found = at.parts.get(part).map_or(0, |fresh| fresh.count);
        let mut bits = vec![0_u64; found.div_ceil(BITS_PER_WORD)];
        for shard in wins {
            for position in shard.get(part).map_or(&[][..], Vec::as_slice) {
                let ordinal = at.ordinal_of(part, *position);
                if let Some(word) = bits.get_mut(ordinal / BITS_PER_WORD) {
                    *word |= 1 << (ordinal % BITS_PER_WORD);
                }
            }
        }
        let mut before = Vec::with_capacity(bits.len());
        let mut count = 0;
        for word in &bits {
            before.push(count);
            count += usize::try_from(word.count_ones()).unwrap_or(0);
        }
        Self {
            bits,
            before,
            count,
        }
    }

    fn rank(&self, ordinal: usize) -> usize {
        let word = ordinal / BITS_PER_WORD;
        let below = (1_u64 << (ordinal % BITS_PER_WORD)) - 1;
        let within = self.bits.get(word).map_or(0, |bits| {
            usize::try_from((bits & below).count_ones()).unwrap_or(0)
        });
        self.before.get(word).copied().unwrap_or(0) + within
    }
}

fn gather<R: Send + Default>(
    spread: &dyn Spread,
    tasks: usize,
    job: impl Fn(usize) -> R + Sync,
) -> Vec<R> {
    let results: Vec<Mutex<R>> = (0..tasks).map(|_| Mutex::new(R::default())).collect();
    spread.run(tasks, &|task| {
        if let Some(slot) = results.get(task) {
            *slot.lock().unwrap_or_else(PoisonError::into_inner) = job(task);
        }
    });
    results
        .into_iter()
        .map(|slot| slot.into_inner().unwrap_or_else(PoisonError::into_inner))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::super::index::{Key, NarrowId, Shard, WideId};
    use super::{Fresh, Spread};
    use crate::store::{Lookup, SHARD_COUNT, Store, hash_words, shard_of};
    use earnest_ir::Field;
    use earnest_value::{Domain, Symbols, Value};
    use std::sync::atomic::{AtomicUsize, Ordering};

    const CACHE_LINE_PAIR: usize = 128;

    struct InOrder;

    impl Spread for InOrder {
        fn run(&self, tasks: usize, job: &(dyn Fn(usize) + Sync)) {
            (0..tasks).for_each(job);
        }
    }

    struct Backwards;

    impl Spread for Backwards {
        fn run(&self, tasks: usize, job: &(dyn Fn(usize) + Sync)) {
            (0..tasks).rev().for_each(job);
        }
    }

    struct Crowd {
        threads: usize,
        each_task_to_a_fixed_thread: bool,
    }

    impl Spread for Crowd {
        fn run(&self, tasks: usize, job: &(dyn Fn(usize) + Sync)) {
            let next = AtomicUsize::new(0);
            let threads = self.threads;
            std::thread::scope(|scope| {
                for thread in 0..threads {
                    let next = &next;
                    let fixed = self.each_task_to_a_fixed_thread;
                    scope.spawn(move || {
                        if fixed {
                            (thread..tasks).step_by(threads).for_each(job);
                            return;
                        }
                        loop {
                            let task = next.fetch_add(1, Ordering::Relaxed);
                            if task >= tasks {
                                return;
                            }
                            job(task);
                        }
                    });
                }
            });
        }
    }

    fn spreads() -> Vec<(&'static str, Box<dyn Spread>)> {
        vec![
            ("in order", Box::new(InOrder)),
            ("backwards", Box::new(Backwards)),
            (
                "4 threads from a counter",
                Box::new(Crowd {
                    threads: 4,
                    each_task_to_a_fixed_thread: false,
                }),
            ),
            (
                "3 threads, each task to a fixed thread",
                Box::new(Crowd {
                    threads: 3,
                    each_task_to_a_fixed_thread: true,
                }),
            ),
            (
                "10 threads from a counter",
                Box::new(Crowd {
                    threads: 10,
                    each_task_to_a_fixed_thread: false,
                }),
            ),
        ]
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

    struct FiledPart {
        rows: Vec<u64>,
        hashes: Vec<u64>,
        froms: Vec<u32>,
        ordinals: Vec<u32>,
        shard_starts: Vec<usize>,
        first: usize,
    }

    impl FiledPart {
        fn of(found: &[(Vec<u64>, u32)], first: usize, words: usize) -> Self {
            let mut order: Vec<usize> = (0..found.len()).collect();
            order.sort_by_key(|at| shard_of(hash_words(&found[*at].0)));
            let mut shard_starts = vec![0; SHARD_COUNT + 1];
            for (row, _) in found {
                shard_starts[shard_of(hash_words(row)) + 1] += 1;
            }
            for shard in 0..SHARD_COUNT {
                shard_starts[shard + 1] += shard_starts[shard];
            }
            let mut filed = Self {
                rows: Vec::with_capacity(found.len() * words),
                hashes: Vec::with_capacity(found.len()),
                froms: Vec::with_capacity(found.len()),
                ordinals: Vec::with_capacity(found.len()),
                shard_starts,
                first,
            };
            for at in order {
                let (row, from) = &found[at];
                filed.rows.extend_from_slice(row);
                filed.hashes.push(hash_words(row));
                filed.froms.push(*from);
                filed
                    .ordinals
                    .push(u32::try_from(at).expect("a small part"));
            }
            filed
        }

        fn fresh(&self) -> Fresh<'_> {
            Fresh {
                count: self.hashes.len(),
                rows: &self.rows,
                hashes: &self.hashes,
                first: self.first,
                froms: &self.froms,
                ordinals: &self.ordinals,
                shard_starts: &self.shard_starts,
            }
        }
    }

    const PARTS: usize = 7;
    const ROWS_A_PART_FINDS: usize = 600;
    const STATES_A_PART_EXPANDS: usize = 5;
    const KNOWN: i64 = 40;

    fn found_in_parts(store: &Store, arity: usize, spread: i64) -> Vec<Vec<(Vec<u64>, u32)>> {
        let mut seed: u64 = 0x9E37_79B9_7F4A_7C15;
        (0..PARTS)
            .map(|_| {
                (0..ROWS_A_PART_FINDS)
                    .filter_map(|at| {
                        let values: Vec<i64> = (0..arity)
                            .map(|_| {
                                seed = seed
                                    .wrapping_mul(6_364_136_223_846_793_005)
                                    .wrapping_add(1_442_695_040_888_963_407);
                                i64::try_from(seed >> 33).unwrap_or(0) % spread
                            })
                            .collect();
                        let from = u32::try_from(at % STATES_A_PART_EXPANDS).expect("small");
                        let row = row_of(store, &values);
                        store
                            .find(&row, hash_words(&row))
                            .is_none()
                            .then_some((row, from))
                    })
                    .collect()
            })
            .collect()
    }

    fn store_with_known_rows(fields: &[Field], lookup: Lookup, arity: usize) -> Store {
        let mut store = Store::new(fields, 1_000_000, lookup);
        for known in 0..KNOWN {
            let row = row_of(&store, &vec![known; arity]);
            store.place(&row, hash_words(&row), None, usize::MAX);
        }
        store
    }

    fn any_assignment_of_shards_to_threads_is_the_level_row_by_row(
        names: &[&str],
        hi: i64,
        spread: i64,
        lookup: Lookup,
    ) {
        let fields = fields_of(names, hi);
        let arity = names.len();
        let mut by_rows = store_with_known_rows(&fields, lookup, arity);
        let words = by_rows.layout().words();
        let before = by_rows.len();
        let found = found_in_parts(&by_rows, arity, spread);
        let firsts: Vec<usize> = (0..PARTS)
            .map(|part| part * STATES_A_PART_EXPANDS)
            .collect();
        for (part, rows) in found.iter().enumerate() {
            for (row, from) in rows {
                let parent = firsts[part] + usize::try_from(*from).expect("small");
                by_rows.place(row, hash_words(row), Some(parent), usize::MAX);
            }
        }
        let new = by_rows.len() - before;
        assert!(new > 100, "the parts find many new rows");
        assert!(
            found.iter().all(|rows| rows.len() > ROWS_A_PART_FINDS / 2),
            "most rows a part finds are not known before the level"
        );
        assert!(
            new < PARTS * ROWS_A_PART_FINDS,
            "some rows are found twice, within a part or across parts"
        );
        let filed: Vec<FiledPart> = found
            .iter()
            .zip(&firsts)
            .map(|(rows, first)| FiledPart::of(rows, *first, words))
            .collect();
        let fresh: Vec<Fresh<'_>> = filed.iter().map(FiledPart::fresh).collect();
        for (name, spread) in spreads() {
            let case = format!("{names:?}, {lookup:?}, {name}");
            let mut store = store_with_known_rows(&fields, lookup, arity);
            assert_eq!(
                store.place_level(&fresh, spread.as_ref(), before + new - 1),
                None,
                "{case}: one state short of the limit, nothing is placed"
            );
            assert_eq!(store.len(), before, "{case}: the level is taken back whole");
            assert_eq!(
                store.place_level(&fresh, spread.as_ref(), usize::MAX),
                Some(new),
                "{case}: every row new to the level, once"
            );
            assert_eq!(store.len(), by_rows.len(), "{case}: the counts agree");
            for id in 0..store.len() {
                assert_eq!(
                    store.parent(id),
                    by_rows.parent(id),
                    "{case}: parent of {id}"
                );
                assert_eq!(store.state(id), by_rows.state(id), "{case}: state of {id}");
            }
            for rows in &found {
                for (row, _) in rows {
                    assert!(
                        store.find(row, hash_words(row)).is_some(),
                        "{case}: a row of the level is known after it"
                    );
                }
            }
        }
    }

    #[test]
    fn a_level_of_one_word_keys_claimed_by_any_assignment_of_shards_to_threads_is_the_level_row_by_row()
     {
        any_assignment_of_shards_to_threads_is_the_level_row_by_row(
            &["a", "b", "c"],
            999,
            9,
            Lookup::Presence,
        );
    }

    #[test]
    fn a_level_of_rows_of_several_words_claimed_by_any_assignment_of_shards_to_threads_is_the_level_row_by_row()
     {
        any_assignment_of_shards_to_threads_is_the_level_row_by_row(
            &["a", "b"],
            1 << 40,
            23,
            Lookup::Presence,
        );
    }

    #[test]
    fn a_level_filed_by_id_with_any_assignment_of_shards_to_threads_is_the_level_row_by_row() {
        any_assignment_of_shards_to_threads_is_the_level_row_by_row(
            &["a", "b", "c"],
            999,
            9,
            Lookup::Id,
        );
    }

    #[test]
    fn no_two_shard_tables_share_a_pair_of_cache_lines() {
        let layouts = [
            (
                std::mem::align_of::<Shard<Key<u32>>>(),
                std::mem::size_of::<Shard<Key<u32>>>(),
            ),
            (
                std::mem::align_of::<Shard<Key<u64>>>(),
                std::mem::size_of::<Shard<Key<u64>>>(),
            ),
            (
                std::mem::align_of::<Shard<NarrowId>>(),
                std::mem::size_of::<Shard<NarrowId>>(),
            ),
            (
                std::mem::align_of::<Shard<WideId>>(),
                std::mem::size_of::<Shard<WideId>>(),
            ),
        ];
        for (align, size) in layouts {
            assert!(
                align >= CACHE_LINE_PAIR && size % CACHE_LINE_PAIR == 0,
                "the count a claim writes into one shard's table at every new row must not sit \
                 beside a neighbour's, claimed at the same time on another core: with 4 workers \
                 that made each row of the claim cost twice what it costs with 1"
            );
        }
    }
}
