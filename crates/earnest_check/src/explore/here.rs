use super::encode::{encode_next, with_updates};
use super::merge::Judged;
use super::{Arrow, Ran, Round, STATES_BETWEEN_CLOCK_READS, Search};
use crate::expand::{Asked, Base, Clock, Mark, Scratch, Successors};
use crate::helpers::Helpers;
use crate::store::{Placed, Store, hash_words};
use earnest_value::Value;
use rustc_hash::FxHashSet;
use std::ops::Range;
use std::time::Instant;

struct HereSink<'a> {
    store: &'a mut Store,
    arrows: &'a mut Vec<Arrow>,
    graph: bool,
    from: usize,
    most: usize,
    full: bool,
    row: Vec<u64>,
    after_full: FxHashSet<Box<[Value]>>,
}

impl HereSink<'_> {
    fn settle(
        &mut self,
        action: usize,
        placed: Placed,
        next: impl FnOnce(&Store) -> Box<[Value]>,
    ) -> bool {
        let (to, counted) = match placed {
            Placed::Known(found) => (found.id, false),
            Placed::New(to) => (Some(to), true),
            Placed::Full => {
                self.full = true;
                let next = next(self.store);
                self.after_full.insert(next);
                return true;
            }
        };
        if self.graph
            && let Some(to) = to
        {
            self.arrows.push(Arrow {
                from: self.from,
                to,
                action,
            });
        }
        counted
    }

    fn place_row(&mut self, action: usize, row: &[u64], hash: u64) -> bool {
        if self.full {
            let known = self.store.find(row, hash).is_some();
            return !known && self.after_full.insert(self.store.values_of(row));
        }
        let placed = self.store.place(row, hash, Some(self.from), self.most);
        self.settle(action, placed, |store| store.values_of(row))
    }
}

impl Successors for HereSink<'_> {
    fn offer(
        &mut self,
        action: usize,
        _: usize,
        base: Base<'_>,
        updates: &[(usize, Value)],
    ) -> bool {
        let mut row = std::mem::take(&mut self.row);
        let counted = if encode_next(self.store, base.row, &mut row, base.values, updates) {
            let hash = hash_words(&row);
            self.place_row(action, &row, hash)
        } else if self.full {
            let next = with_updates(base.values, updates);
            self.store.find_values(&next).is_none() && self.after_full.insert(next)
        } else {
            let next = with_updates(base.values, updates);
            let placed = self.store.place_values(next, Some(self.from), self.most);
            self.settle(action, placed, |_| with_updates(base.values, updates))
        };
        self.row = row;
        counted
    }

    fn offer_row(&mut self, action: usize, _: usize, row: &[u64], hash: u64) -> bool {
        self.place_row(action, row, hash)
    }
}

struct Here {
    end: Round,
    marks: Vec<(usize, Mark)>,
    through: usize,
}

impl Search<'_> {
    pub(super) fn expand_here(
        &mut self,
        frontier: Range<usize>,
        deadline: Option<Instant>,
        asked: Asked,
        lending: Option<&Helpers>,
        judged: &mut Judged,
    ) -> Ran {
        let expand = self
            .expander
            .expand(self.model, self.limits, deadline, asked);
        let mut here = Here {
            end: Round::Done,
            marks: Vec::new(),
            through: frontier.start,
        };
        let mut sink = HereSink {
            store: &mut self.store,
            arrows: &mut self.arrows,
            graph: self.limits.keep_graph,
            from: 0,
            most: self.limits.max_states,
            full: false,
            row: Vec::new(),
            after_full: FxHashSet::default(),
        };
        let mut scratch = Scratch::default();
        let mut state = Vec::new();
        let mut row = Vec::new();
        let stops_at = deadline.filter(|_| asked == Asked::Everything);
        for (position, id) in frontier.enumerate() {
            let clock_due = position.is_multiple_of(STATES_BETWEEN_CLOCK_READS);
            if clock_due && stops_at.is_some_and(|limit| Instant::now() >= limit) {
                here.end = Round::Stopped;
                break;
            }
            let idle = || lending.is_some_and(|helpers| helpers.idle() > 0);
            if clock_due && position > 0 && idle() {
                here.end = Round::RestFrom(here.through);
                break;
            }
            let is_row = sink.store.read_state(id, &mut row, &mut state);
            let outside = sink.store.outside(id, &row, &expand.model.fields);
            sink.from = id;
            let base = Base {
                values: &state,
                row: is_row.then_some(row.as_slice()),
            };
            let mark = expand.state(base, outside, &mut sink, &mut scratch);
            if mark.clock == Clock::Stopped {
                here.end = Round::Stopped;
                break;
            }
            if !mark.is_plain() {
                here.marks.push((id, mark));
            }
            here.through = id.saturating_add(1);
            if sink.full {
                here.end = Round::Full;
                break;
            }
        }
        judged.add(here.marks, here.through);
        Ran { end: here.end }
    }
}
