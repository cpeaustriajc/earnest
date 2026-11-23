use super::part::Part;
use super::{Round, Search, SpreadMerge, Work};
use crate::cut::cut_points;
use crate::expand::Asked;
use crate::helpers::{Helpers, Loan};
use crate::limits::WORKER_STACK;
use crate::store::Spread;
use earnest_threads::Joined;
use std::any::Any;
use std::ops::Range;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Mutex, PoisonError};
use std::thread::Scope;
use std::time::{Duration, Instant};

const LEAST_TIME_OF_A_STATE: Duration = Duration::from_nanos(500);
const TIME_A_PART_PAYS_FOR: Duration = Duration::from_micros(512);
const MOST_PARTS: usize = 64;
const SMALLEST_LEVEL_A_LONE_THREAD_TAKES_IN_PARTS: usize = 16_384;
const PARTS_OF_A_LONE_ROUND: usize = MOST_PARTS / 2;

fn wide(count: usize) -> u128 {
    u128::try_from(count).unwrap_or(u128::MAX)
}

fn parts_of(states: usize, state_time: Duration) -> usize {
    let work = wide(states).saturating_mul(state_time.max(LEAST_TIME_OF_A_STATE).as_nanos());
    let parts = work / TIME_A_PART_PAYS_FOR.as_nanos().max(1);
    usize::try_from(parts)
        .unwrap_or(usize::MAX)
        .min(MOST_PARTS)
        .min(states)
        .max(1)
}

struct Hands {
    threads: usize,
    keep_graph: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Shape {
    part_states: usize,
    round_parts: usize,
    keeps_spares: bool,
}

impl Hands {
    fn take_in_parts(&self, width: usize, state_time: Duration) -> Option<Shape> {
        let parts = parts_of(width, state_time);
        let part_states = width.div_ceil(parts);
        if self.threads > 1 {
            return (parts > 1).then_some(Shape {
                part_states,
                round_parts: parts,
                keeps_spares: false,
            });
        }
        let taken = !self.keep_graph && width >= SMALLEST_LEVEL_A_LONE_THREAD_TAKES_IN_PARTS;
        taken.then_some(Shape {
            part_states,
            round_parts: PARTS_OF_A_LONE_ROUND,
            keeps_spares: true,
        })
    }
}

pub(super) struct Expansion {
    pub(super) parts: Vec<Part>,
    pub(super) end: Round,
    pub(super) work: Work,
    pub(super) spare_parts: usize,
}

struct Level {
    frontier: Range<usize>,
    cuts: Vec<usize>,
    round_parts: usize,
    keeps_spares: bool,
    deadline: Option<Instant>,
    asked: Asked,
    kept_rows_that_fit: usize,
    fewest_states: usize,
}

impl Level {
    fn parts(&self) -> usize {
        self.cuts.len().saturating_sub(1)
    }

    fn parts_of_this_round(&self) -> usize {
        self.parts().min(self.round_parts)
    }
}

#[derive(Default)]
struct Shared {
    next: AtomicUsize,
    stopped: AtomicBool,
    running: AtomicUsize,
    kept_rows: AtomicUsize,
    helpers_taken_up: AtomicUsize,
    done: Mutex<Vec<(usize, Part)>>,
    panic: Mutex<Option<Box<dyn Any + Send>>>,
}

impl Shared {
    fn holds_back(&self, level: &Level) -> bool {
        self.holds_back_at(level, self.next.load(Ordering::SeqCst))
    }

    fn holds_back_at(&self, level: &Level, part: usize) -> bool {
        let taken = level.cuts.get(part).copied().unwrap_or(usize::MAX);
        taken >= level.fewest_states
            && self.kept_rows.load(Ordering::SeqCst) > level.kept_rows_that_fit
    }

    fn wait_for(&self, workers: Vec<Joined<'_, ()>>) {
        for worker in workers {
            if let Err(panic) = worker.join() {
                self.panic
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .get_or_insert(panic);
            }
        }
    }

    fn take_part(&self, level: &Level) -> Option<usize> {
        self.next
            .try_update(Ordering::SeqCst, Ordering::SeqCst, |part| {
                let open = part < level.parts_of_this_round()
                    && !self.stopped.load(Ordering::SeqCst)
                    && !self.holds_back_at(level, part);
                open.then(|| part.saturating_add(1))
            })
            .ok()
    }
}

impl Search<'_> {
    fn estimated_kept_rows_that_fit(&self) -> usize {
        let room = self.limits.max_states.saturating_sub(self.store.len());
        let SpreadMerge { kept, new } = self.last_spread_merge;
        if kept == 0 {
            return room;
        }
        wide(room)
            .saturating_mul(wide(kept))
            .checked_div(wide(new))
            .map_or(usize::MAX, |scaled| {
                usize::try_from(scaled).unwrap_or(usize::MAX)
            })
    }

    pub(super) fn spread_level(
        &self,
        frontier: &Range<usize>,
        width: usize,
        deadline: Option<Instant>,
        asked: Asked,
        fewest_states: usize,
    ) -> Option<Expansion> {
        let own = self.limits.jobs.max(1);
        let helpers = self.limits.helpers.as_ref();
        let hands = Hands {
            threads: own.saturating_add(helpers.map_or(0, Helpers::idle)),
            keep_graph: self.limits.keep_graph,
        };
        let shape = hands.take_in_parts(width, self.state_time)?;
        let level = Level {
            frontier: frontier.clone(),
            cuts: cut_points(frontier.len(), shape.part_states),
            round_parts: shape.round_parts,
            keeps_spares: shape.keeps_spares,
            deadline,
            asked,
            kept_rows_that_fit: self.estimated_kept_rows_that_fit(),
            fewest_states,
        };
        let parts = level.parts();
        let round = level.parts_of_this_round();
        let shared = Shared::default();
        std::thread::scope(|scope| {
            let mut started = Vec::new();
            for _ in 1..own.min(round) {
                started.push(self.start_worker(scope, &level, &shared, None));
            }
            for _ in 0..round.saturating_sub(own) {
                let Some(loan) = helpers
                    .map(|helpers| helpers.borrow(1))
                    .filter(|loan| loan.threads() > 0)
                else {
                    break;
                };
                started.push(self.start_worker(scope, &level, &shared, Some(loan)));
            }
            self.work(scope, &level, &shared, None);
            shared.wait_for(started);
        });
        if let Some(panic) = shared
            .panic
            .into_inner()
            .unwrap_or_else(PoisonError::into_inner)
        {
            std::panic::resume_unwind(panic);
        }
        let mut finished = shared
            .done
            .into_inner()
            .unwrap_or_else(PoisonError::into_inner);
        finished.sort_by_key(|(part, _)| *part);
        let crowded = finished.iter().any(|(_, part)| part.crowded);
        let mut taken = Vec::with_capacity(finished.len());
        let mut stopped = false;
        for (expected, (index, part)) in finished.into_iter().enumerate() {
            if index != expected {
                stopped = true;
                break;
            }
            stopped = part.stopped;
            taken.push(part);
            if stopped {
                break;
            }
        }
        let end = if crowded {
            Round::RestAlone
        } else if stopped {
            Round::Stopped
        } else {
            level
                .cuts
                .get(taken.len())
                .filter(|_| (1..parts).contains(&taken.len()))
                .map_or(Round::Done, |cut| {
                    Round::RestFrom(frontier.start.saturating_add(*cut))
                })
        };
        let work = Work {
            states: taken.iter().map(|part| part.expanded).sum(),
            took: taken.iter().map(|part| part.took).sum(),
        };
        Some(Expansion {
            parts: taken,
            end,
            work,
            spare_parts: if shape.keeps_spares {
                shape.round_parts
            } else {
                0
            },
        })
    }

    fn start_worker<'scope, 'env>(
        &'env self,
        scope: &'scope Scope<'scope, 'env>,
        level: &'env Level,
        shared: &'env Shared,
        loan: Option<Loan<'env>>,
    ) -> Joined<'scope, ()> {
        earnest_threads::start(scope, Some(WORKER_STACK), move || {
            self.work(scope, level, shared, loan);
        })
        .expect("a worker thread starts")
    }

    fn work<'scope, 'env>(
        &'env self,
        scope: &'scope Scope<'scope, 'env>,
        level: &'env Level,
        shared: &'env Shared,
        loan: Option<Loan<'env>>,
    ) {
        shared.running.fetch_add(1, Ordering::SeqCst);
        let mut started = Vec::new();
        let outcome = catch_unwind(AssertUnwindSafe(|| {
            self.take_parts(scope, level, shared, &mut started);
        }));
        drop(loan);
        shared.running.fetch_sub(1, Ordering::SeqCst);
        if let Err(panic) = outcome {
            shared.stopped.store(true, Ordering::SeqCst);
            shared
                .panic
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .get_or_insert(panic);
        }
        shared.wait_for(started);
    }

    fn take_parts<'scope, 'env>(
        &'env self,
        scope: &'scope Scope<'scope, 'env>,
        level: &'env Level,
        shared: &'env Shared,
        started: &mut Vec<Joined<'scope, ()>>,
    ) {
        let mut kit = self.spares.kit();
        loop {
            self.take_up_idle_threads(scope, level, shared, started);
            let Some(part) = shared.take_part(level) else {
                break;
            };
            let ids = level
                .cuts
                .get(part)
                .zip(level.cuts.get(part.saturating_add(1)))
                .map(|(from, to)| {
                    level.frontier.start.saturating_add(*from)
                        ..level.frontier.start.saturating_add(*to)
                });
            let Some(ids) = ids else {
                break;
            };
            let began = Instant::now();
            let spare = self.spares.part();
            let mut expansion = self.expand_ids(ids, level.deadline, level.asked, &mut kit, spare);
            expansion.took = began.elapsed();
            if !level.keeps_spares {
                kit.drop_spare_rows();
            }
            if expansion.stopped {
                shared.stopped.store(true, Ordering::SeqCst);
            }
            shared
                .kept_rows
                .fetch_add(expansion.kept_rows(), Ordering::SeqCst);
            shared
                .done
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .push((part, expansion));
        }
        if level.keeps_spares {
            self.spares.keep_kit(kit);
        } else {
            self.spares.keep_table_of(kit);
        }
    }

    fn take_up_idle_threads<'scope, 'env>(
        &'env self,
        scope: &'scope Scope<'scope, 'env>,
        level: &'env Level,
        shared: &'env Shared,
        started: &mut Vec<Joined<'scope, ()>>,
    ) {
        let Some(helpers) = self.limits.helpers.as_ref() else {
            return;
        };
        let parts = level.parts_of_this_round();
        while helpers.idle() > 0
            && !shared.holds_back(level)
            && shared.helpers_taken_up.load(Ordering::SeqCst) < parts
            && shared.running.load(Ordering::SeqCst)
                < parts.saturating_sub(shared.next.load(Ordering::SeqCst))
        {
            let loan = helpers.borrow(1);
            if loan.threads() == 0 {
                return;
            }
            shared.helpers_taken_up.fetch_add(1, Ordering::SeqCst);
            started.push(self.start_worker(scope, level, shared, Some(loan)));
        }
    }
}

pub(super) struct Crew<'a> {
    pub(super) own: usize,
    pub(super) helpers: Option<&'a Helpers>,
    pub(super) most_borrowed: usize,
}

impl Spread for Crew<'_> {
    fn run(&self, tasks: usize, job: &(dyn Fn(usize) + Sync)) {
        let next = AtomicUsize::new(0);
        let work = || {
            loop {
                let task = next.fetch_add(1, Ordering::Relaxed);
                if task >= tasks {
                    return;
                }
                job(task);
            }
        };
        let own = self.own.min(tasks);
        let loan = self
            .helpers
            .filter(|_| self.most_borrowed > 0)
            .map(|helpers| helpers.borrow(tasks.saturating_sub(own).min(self.most_borrowed)));
        let borrowed = loan.as_ref().map_or(0, Loan::threads);
        std::thread::scope(|scope| {
            let started: Vec<_> = (1..own.saturating_add(borrowed))
                .map(|_| {
                    earnest_threads::start(scope, None, work).expect("a thread of the crew starts")
                })
                .collect();
            work();
            for worker in started {
                if let Err(panic) = worker.join() {
                    std::panic::resume_unwind(panic);
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Hands, LEAST_TIME_OF_A_STATE, Level, MOST_PARTS, PARTS_OF_A_LONE_ROUND,
        SMALLEST_LEVEL_A_LONE_THREAD_TAKES_IN_PARTS, Shape, Shared, TIME_A_PART_PAYS_FOR, parts_of,
    };
    use crate::expand::Asked;
    use std::sync::atomic::Ordering;
    use std::time::Duration;

    const LONE_LEVEL: usize = SMALLEST_LEVEL_A_LONE_THREAD_TAKES_IN_PARTS;
    const CHEAP: Duration = Duration::ZERO;
    const SLOW_STATE: Duration = Duration::from_micros(65);

    fn cheap_states_a_part_pays_for() -> usize {
        let states = TIME_A_PART_PAYS_FOR.as_nanos() / LEAST_TIME_OF_A_STATE.as_nanos();
        usize::try_from(states).unwrap_or(usize::MAX)
    }

    #[test]
    fn a_lone_thread_that_keeps_no_graph_takes_a_level_in_parts_from_its_size_alone() {
        let lone = Hands {
            threads: 1,
            keep_graph: false,
        };
        assert!(
            lone.take_in_parts(LONE_LEVEL - 1, CHEAP).is_none(),
            "just short"
        );
        assert!(
            lone.take_in_parts(LONE_LEVEL, CHEAP).is_some(),
            "at the size"
        );
        assert!(
            lone.take_in_parts(LONE_LEVEL - 1, SLOW_STATE).is_none(),
            "a slow level short of the size"
        );
        assert!(
            lone.take_in_parts(10 * LONE_LEVEL, CHEAP).is_some(),
            "far above"
        );
    }

    #[test]
    fn a_lone_thread_takes_a_level_in_rounds_of_a_few_parts_and_keeps_what_a_round_took() {
        let lone = Hands {
            threads: 1,
            keep_graph: false,
        };
        let part = cheap_states_a_part_pays_for();
        assert_eq!(
            lone.take_in_parts(LONE_LEVEL, CHEAP),
            Some(Shape {
                part_states: part,
                round_parts: PARTS_OF_A_LONE_ROUND,
                keeps_spares: true,
            }),
            "the narrowest level it takes in parts"
        );
        assert_eq!(
            lone.take_in_parts(MOST_PARTS * part * 10, CHEAP),
            Some(Shape {
                part_states: part * 10,
                round_parts: PARTS_OF_A_LONE_ROUND,
                keeps_spares: true,
            }),
            "a level of the most parts, ten shares each"
        );
        assert_eq!(
            lone.take_in_parts(MOST_PARTS * part * 10 + 1, CHEAP)
                .map(|shape| shape.part_states),
            Some(part * 10 + 1),
            "one state more makes every part a state wider"
        );
    }

    #[test]
    fn a_lone_round_is_half_the_most_parts_as_a_smaller_one_brings_too_few_rows_to_each_shard_table()
     {
        assert_eq!(
            PARTS_OF_A_LONE_ROUND, 32,
            "rounds of 14 parts made 100 million states 6% slower, of 24 still 10 million states, \
             since each round's claim met each shard's table with fewer rows"
        );
    }

    #[test]
    fn several_threads_take_a_level_in_one_round_and_keep_nothing() {
        let part = cheap_states_a_part_pays_for();
        for threads in [2, 4, 10] {
            let crew = Hands {
                threads,
                keep_graph: false,
            };
            for parts in [2, PARTS_OF_A_LONE_ROUND + 1, MOST_PARTS] {
                assert_eq!(
                    crew.take_in_parts(parts * part, CHEAP),
                    Some(Shape {
                        part_states: part,
                        round_parts: parts,
                        keeps_spares: false,
                    }),
                    "{threads} threads, {parts} parts"
                );
            }
        }
    }

    #[test]
    fn a_lone_thread_that_keeps_the_graph_takes_every_level_row_by_row() {
        let lone = Hands {
            threads: 1,
            keep_graph: true,
        };
        assert!(lone.take_in_parts(10 * LONE_LEVEL, SLOW_STATE).is_none());
    }

    #[test]
    fn several_threads_take_a_level_of_cheap_states_in_parts_once_it_fills_two_parts() {
        let part = cheap_states_a_part_pays_for();
        assert_eq!(part, 1024, "the cheap states a part pays for");
        for keep_graph in [false, true] {
            let crew = Hands {
                threads: 2,
                keep_graph,
            };
            assert!(
                crew.take_in_parts(2 * part - 1, CHEAP).is_none(),
                "short of two parts, graph kept: {keep_graph}"
            );
            assert!(
                crew.take_in_parts(2 * part, CHEAP).is_some(),
                "two parts, graph kept: {keep_graph}"
            );
            assert!(
                crew.take_in_parts(2 * part, LEAST_TIME_OF_A_STATE / 2)
                    .is_some(),
                "states measured below the least time count at the least time, graph kept: \
                 {keep_graph}"
            );
        }
    }

    #[test]
    fn several_threads_take_a_narrow_level_of_slow_states_in_parts() {
        let crew = Hands {
            threads: 4,
            keep_graph: false,
        };
        assert!(
            crew.take_in_parts(190, SLOW_STATE).is_some(),
            "190 states of 65 µs"
        );
        assert!(crew.take_in_parts(190, CHEAP).is_none(), "190 cheap states");
        assert!(
            crew.take_in_parts(1, Duration::from_secs(1)).is_none(),
            "a single state is one part however slow"
        );
    }

    #[test]
    fn a_level_has_a_part_for_every_share_of_work_a_part_pays_for_up_to_the_most_parts() {
        let part = cheap_states_a_part_pays_for();
        assert_eq!(parts_of(0, CHEAP), 1, "an empty level");
        assert_eq!(parts_of(part - 1, CHEAP), 1, "short of one share");
        assert_eq!(parts_of(3 * part - 1, CHEAP), 2, "short of three shares");
        assert_eq!(parts_of(3 * part, CHEAP), 3, "three shares");
        assert_eq!(
            parts_of(MOST_PARTS * part, CHEAP),
            MOST_PARTS,
            "the most parts"
        );
        assert_eq!(
            parts_of(10 * MOST_PARTS * part, CHEAP),
            MOST_PARTS,
            "far more shares than parts"
        );
        assert_eq!(
            parts_of(190, SLOW_STATE),
            24,
            "190 states of 65 µs: 12.35 ms of work"
        );
        assert_eq!(
            parts_of(10, Duration::from_secs(1)),
            10,
            "no more parts than states"
        );
    }

    const KEPT_ROWS_THAT_FIT: usize = 5;

    fn level_of_three_parts() -> Level {
        Level {
            frontier: 0..30,
            cuts: vec![0, 10, 20, 30],
            round_parts: MOST_PARTS,
            keeps_spares: false,
            deadline: None,
            asked: Asked::Everything,
            kept_rows_that_fit: KEPT_ROWS_THAT_FIT,
            fewest_states: 1,
        }
    }

    fn shared_after_the_first_part(kept_rows: usize) -> Shared {
        let shared = Shared::default();
        shared.next.store(1, Ordering::SeqCst);
        shared.kept_rows.store(kept_rows, Ordering::SeqCst);
        shared
    }

    #[test]
    fn a_part_is_not_taken_once_the_kept_rows_outgrow_what_fits() {
        let shared = shared_after_the_first_part(KEPT_ROWS_THAT_FIT + 1);
        assert_eq!(
            shared.take_part(&level_of_three_parts()),
            None,
            "the rest of the level is held back"
        );
        assert_eq!(
            shared.next.load(Ordering::SeqCst),
            1,
            "a part held back is left for a later round, not claimed"
        );
    }

    #[test]
    fn no_part_past_the_end_of_a_round_is_taken() {
        let level = Level {
            round_parts: 2,
            kept_rows_that_fit: usize::MAX,
            ..level_of_three_parts()
        };
        let shared = Shared::default();
        assert_eq!(shared.take_part(&level), Some(0), "the first part");
        assert_eq!(
            shared.take_part(&level),
            Some(1),
            "the last part of the round"
        );
        assert_eq!(
            shared.take_part(&level),
            None,
            "the next round's first part"
        );
        assert_eq!(
            shared.next.load(Ordering::SeqCst),
            2,
            "a part of the next round is left to it, not claimed"
        );
    }

    #[test]
    fn a_part_is_taken_while_the_kept_rows_fit() {
        let shared = shared_after_the_first_part(KEPT_ROWS_THAT_FIT);
        assert_eq!(
            shared.take_part(&level_of_three_parts()),
            Some(1),
            "the next part"
        );
        assert_eq!(
            shared.next.load(Ordering::SeqCst),
            2,
            "the part taken is claimed"
        );
    }
}
