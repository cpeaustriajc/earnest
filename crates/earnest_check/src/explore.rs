mod choice;
mod encode;
mod here;
mod merge;
mod part;
mod starts;
mod workers;

use crate::expand::{Asked, Budget, Expand, FromRows, Mark, Move, Whence, Width};
use crate::helpers::Helpers;
use crate::limits::{Limits, TIME_LIMIT_REASON};
use crate::outcome::Outcome;
use crate::store::{Lookup, Placed, Store};
use earnest_ir::{Fault, Model};
use merge::{Full, Judged};
use part::{Part, Spares};
use starts::Starts;
use std::ops::Range;
use std::time::{Duration, Instant};

const STATES_BETWEEN_CLOCK_READS: usize = 64;

#[derive(Clone, Debug, Default)]
pub(crate) struct Note {
    pub(crate) end: bool,
    pub(crate) stuck: bool,
    pub(crate) outside: Option<usize>,
    pub(crate) broken: Vec<usize>,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Arrow {
    pub(crate) from: usize,
    pub(crate) to: usize,
    pub(crate) action: usize,
}

pub(crate) struct Search<'a> {
    pub(crate) model: &'a Model,
    pub(crate) limits: &'a Limits,
    expander: Expander<'a>,
    pub(crate) store: Store,
    pub(crate) notes: Vec<Note>,
    pub(crate) arrows: Vec<Arrow>,
    pub(crate) first_broken: Vec<Option<usize>>,
    pub(crate) unjudged: Vec<bool>,
    pub(crate) stuck: Option<usize>,
    pub(crate) outside: Option<(usize, usize)>,
    pub(crate) trouble: Option<(usize, Whence, Fault)>,
    pub(crate) reason: Option<String>,
    pub(crate) computed_beyond_32_bits: bool,
    pub(crate) cut_short: bool,
    judged: usize,
    last_spread_merge: SpreadMerge,
    state_time: Duration,
    spares: Spares,
}

struct Expander<'a> {
    moves: Vec<Move<'a>>,
    from_rows: FromRows,
    slots: usize,
}

impl Expander<'_> {
    fn expand<'s>(
        &'s self,
        model: &'s Model,
        limits: &'s Limits,
        deadline: Option<Instant>,
        asked: Asked,
    ) -> Expand<'s> {
        Expand {
            model,
            moves: &self.moves,
            from_rows: &self.from_rows,
            only_rule: limits.only_rule.as_deref(),
            slots: self.slots,
            asked,
            budget: Budget {
                deadline,
                successors: limits.max_states,
            },
        }
    }
}

const FEWEST_STATES_OF_A_ROUND: usize = 1;
const ROUND_GROWTH: usize = 2;

#[derive(Clone, Copy, Default)]
struct SpreadMerge {
    kept: usize,
    new: usize,
}

/// Visits every state `model` can reach from its start states, breadth first, and says what
/// it found.
///
/// A rule is checked in every state, so the path given for a broken rule is a shortest one.
/// A state out of range is counted and reported but not searched further. The search stops
/// early only at a limit, and then the outcome is unfinished unless something was found
/// broken. Every state that is counted has its rules judged, also at a limit: the states the
/// search found and had no time or room to go on from are judged before it gives its outcome,
/// and no action is tried from them.
#[must_use]
pub fn explore(model: &Model, limits: &Limits) -> Outcome {
    let slots = model
        .actions
        .iter()
        .map(|action| action.slots)
        .chain(model.rules.iter().map(|rule| rule.slots))
        .chain(model.ends.iter().map(|end| end.slots))
        .max()
        .unwrap_or(0);
    let store = Store::new(
        &model.fields,
        limits.max_states,
        if limits.keep_graph {
            Lookup::Id
        } else {
            Lookup::Presence
        },
    );
    let mut search = Search {
        model,
        limits,
        expander: Expander {
            moves: Move::all(model, store.layout(), &store.slots()),
            from_rows: FromRows::of(model, &store),
            slots,
        },
        store,
        notes: Vec::new(),
        arrows: Vec::new(),
        first_broken: vec![None; model.rules.len()],
        unjudged: vec![false; model.rules.len()],
        stuck: None,
        outside: None,
        trouble: None,
        reason: None,
        computed_beyond_32_bits: false,
        cut_short: false,
        judged: 0,
        last_spread_merge: SpreadMerge::default(),
        state_time: Duration::ZERO,
        spares: Spares::default(),
    };
    search.run();
    search.outcome()
}

impl Search<'_> {
    fn run(&mut self) {
        let began = Instant::now();
        let deadline = self
            .limits
            .time_limit
            .and_then(|limit| began.checked_add(limit));
        for start in Starts::new(&self.model.fields) {
            let placed =
                self.store
                    .place_values(start.into_boxed_slice(), None, self.limits.max_states);
            if placed == Placed::Full {
                self.reason = Some(self.limit_reason());
                break;
            }
        }
        let mut frontier = 0..self.store.len();
        while !frontier.is_empty() && self.reason.is_none() {
            let found_before = self.store.len();
            let level = self.level(frontier.clone(), deadline, Asked::Everything);
            if level.end == Round::Stopped {
                self.reason = Some(self.time_limit_reason());
                break;
            }
            if level.end == Round::Full {
                self.reason = Some(self.limit_reason());
                break;
            }
            frontier = found_before..self.store.len();
        }
        self.judge_the_rest(deadline);
    }

    fn judge_the_rest(&mut self, deadline: Option<Instant>) {
        let unjudged = self.judged..self.store.len();
        let level = self.level(unjudged, deadline, Asked::RulesOnly);
        if level.end == Round::Stopped {
            self.reason = Some(self.time_limit_reason());
            let kept = self.judged;
            self.store.truncate(kept);
            self.arrows
                .retain(|arrow| arrow.from < kept && arrow.to < kept);
        }
    }

    fn time_limit_reason(&self) -> String {
        let seconds = self.limits.time_limit.map_or(0, |limit| limit.as_secs());
        format!("{TIME_LIMIT_REASON} {seconds} s")
    }

    fn limit_reason(&self) -> String {
        format!("state limit of {}", self.limits.max_states)
    }

    fn expand(&self, deadline: Option<Instant>, asked: Asked) -> Expand<'_> {
        self.expander
            .expand(self.model, self.limits, deadline, asked)
    }

    fn level(&mut self, frontier: Range<usize>, deadline: Option<Instant>, asked: Asked) -> Ran {
        let lending = self.limits.helpers.as_ref();
        let mut judged = Judged::default();
        let mut work = Work::default();
        let mut rest = frontier.start;
        let mut way = Way::Spread {
            fewest: FEWEST_STATES_OF_A_ROUND,
            else_lending: lending,
        };
        let ran = loop {
            let states = rest..frontier.end;
            let ran = match way {
                Way::Spread {
                    fewest,
                    else_lending,
                } => {
                    let spread = self.spread_round(
                        &states,
                        frontier.len(),
                        deadline,
                        asked,
                        fewest,
                        &mut judged,
                    );
                    let Some(ran) = spread else {
                        way = Way::Here(else_lending);
                        continue;
                    };
                    ran
                }
                Way::Here(lending) => {
                    self.expand_here(states, deadline, asked, lending, &mut judged)
                }
            };
            work = work.and(ran.work);
            way = match (ran.end, way) {
                (Round::RestFrom(from), Way::Spread { .. }) => {
                    let taken = from.saturating_sub(rest);
                    rest = from;
                    Way::Spread {
                        fewest: taken.saturating_mul(ROUND_GROWTH),
                        else_lending: lending,
                    }
                }
                (Round::RestFrom(from), Way::Here(_)) => {
                    rest = from;
                    Way::Spread {
                        fewest: FEWEST_STATES_OF_A_ROUND,
                        else_lending: None,
                    }
                }
                (Round::RestAlone, _) => Way::Here(None),
                (Round::Done | Round::Stopped | Round::Full, _) => break ran,
            };
        };
        if asked == Asked::Everything
            && let Some(state_time) = work.per_state()
        {
            self.state_time = state_time;
        }
        if ran.end == Round::Stopped && asked == Asked::Everything {
            self.undo_level(&frontier);
        } else {
            self.record_all(&judged.marks, judged.through);
        }
        ran
    }

    fn spread_round(
        &mut self,
        states: &Range<usize>,
        width: usize,
        deadline: Option<Instant>,
        asked: Asked,
        fewest: usize,
        judged: &mut Judged,
    ) -> Option<Ran> {
        let mut expansion = self.spread_level(states, width, deadline, asked, fewest)?;
        let ran = Ran {
            end: expansion.end,
            work: expansion.work,
        };
        let undone = ran.end == Round::Stopped && asked == Asked::Everything;
        if ran.end == Round::RestAlone || undone {
            return Some(ran);
        }
        let before = self.store.len();
        let kept = expansion.parts.iter().map(Part::kept_rows).sum();
        let merged = self.merge_any(&mut expansion.parts, judged);
        self.spares.keep(expansion.parts, expansion.spare_parts);
        self.last_spread_merge = SpreadMerge {
            kept,
            new: self.store.len().saturating_sub(before),
        };
        Some(match merged {
            Ok(()) => ran,
            Err(Full) => Ran {
                end: Round::Full,
                ..ran
            },
        })
    }

    fn undo_level(&mut self, frontier: &Range<usize>) {
        self.store.truncate(frontier.end);
        self.arrows.retain(|arrow| arrow.from < frontier.start);
    }

    fn record_all(&mut self, marks: &[(usize, Mark)], through: usize) {
        for (id, mark) in marks {
            self.record(*id, mark);
        }
        self.judged = self.judged.max(through);
    }

    fn record(&mut self, id: usize, expanded: &Mark) {
        self.computed_beyond_32_bits |= expanded.width == Width::Beyond32Bits;
        self.cut_short |= expanded.cut_short;
        if let Some(field) = expanded.outside {
            self.outside.get_or_insert((id, field));
        }
        for rule in &expanded.broken {
            if let Some(first) = self.first_broken.get_mut(*rule) {
                first.get_or_insert(id);
            }
        }
        for rule in &expanded.unjudged {
            if let Some(unjudged) = self.unjudged.get_mut(*rule) {
                *unjudged = true;
            }
        }
        if let Some((whence, fault)) = expanded.trouble {
            self.trouble.get_or_insert((id, whence, fault));
        }
        if expanded.stuck {
            self.stuck.get_or_insert(id);
        }
        if !self.limits.keep_graph {
            return;
        }
        if self.notes.len() <= id {
            self.notes.resize_with(id.saturating_add(1), Note::default);
        }
        if let Some(note) = self.notes.get_mut(id) {
            *note = Note {
                end: expanded.ended,
                stuck: expanded.stuck,
                outside: expanded.outside,
                broken: expanded.broken.clone(),
            };
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Round {
    Done,
    RestFrom(usize),
    RestAlone,
    Stopped,
    Full,
}

#[derive(Clone, Copy)]
enum Way<'h> {
    Spread {
        fewest: usize,
        else_lending: Option<&'h Helpers>,
    },
    Here(Option<&'h Helpers>),
}

#[derive(Clone, Copy)]
struct Ran {
    end: Round,
    work: Work,
}

#[derive(Clone, Copy, Default)]
struct Work {
    states: usize,
    took: Duration,
}

impl Work {
    fn and(self, more: Self) -> Self {
        Self {
            states: self.states.saturating_add(more.states),
            took: self.took.saturating_add(more.took),
        }
    }

    fn per_state(self) -> Option<Duration> {
        let states = u128::try_from(self.states)
            .ok()
            .filter(|states| *states > 0)?;
        let nanos = self.took.as_nanos() / states;
        Some(Duration::from_nanos(
            u64::try_from(nanos).unwrap_or(u64::MAX),
        ))
    }
}
