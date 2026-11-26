use crate::plot::Plot;
use earnest_check::PremiseKind;
use std::cmp::Reverse;
use std::collections::{BinaryHeap, VecDeque};

/// The most arrows a chart draws, not counting the arrows from and to `[*]`.
pub const ARROW_LIMIT: usize = 20;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Found {
    Drawn(usize),
    Beyond,
}

pub(crate) struct Selection {
    pub(crate) states: Vec<usize>,
    pub(crate) arrows: Vec<usize>,
    pub(crate) beyond: usize,
    pub(crate) shown: usize,
    pub(crate) premises: Vec<Option<Found>>,
}

struct Leaving {
    first: Vec<usize>,
    arrows: Vec<usize>,
}

impl Leaving {
    fn new(plot: &impl Plot) -> Self {
        let boxes = plot.boxes();
        let sound = |at: usize| {
            let (from, to) = plot.ends(at);
            (from < boxes && to < boxes).then_some(from)
        };
        let mut first = vec![0_usize; boxes.saturating_add(1)];
        for from in (0..plot.arrows()).filter_map(sound) {
            first[from] = first[from].saturating_add(1);
        }
        let mut total = 0_usize;
        for end in &mut first {
            total = total.saturating_add(*end);
            *end = total;
        }
        let mut arrows = vec![0_usize; total];
        for at in (0..plot.arrows()).rev() {
            if let Some(from) = sound(at) {
                first[from] = first[from].saturating_sub(1);
                arrows[first[from]] = at;
            }
        }
        Self { first, arrows }
    }

    fn of(&self, from: usize) -> &[usize] {
        &self.arrows[self.first[from]..self.first[from.saturating_add(1)]]
    }
}

struct Into {
    arrow: Vec<Option<usize>>,
    rank: Vec<Option<usize>>,
}

fn arrows_into(plot: &impl Plot, leaving: &Leaving) -> Into {
    let boxes = plot.boxes();
    let mut seen: Vec<bool> = (0..boxes).map(|at| plot.look(at).start).collect();
    let mut into = Into {
        arrow: vec![None; boxes],
        rank: vec![None; boxes],
    };
    let mut queue: VecDeque<usize> = (0..boxes).filter(|at| seen[*at]).collect();
    let mut next_rank = 0_usize;
    while let Some(at) = queue.pop_front() {
        into.rank[at] = Some(next_rank);
        next_rank = next_rank.saturating_add(1);
        for arrow in leaving.of(at) {
            let (_, target) = plot.ends(*arrow);
            if !seen[target] {
                seen[target] = true;
                into.arrow[target] = Some(*arrow);
                queue.push_back(target);
            }
        }
    }
    into
}

struct Spending<'p, P> {
    plot: &'p P,
    leaving: &'p Leaving,
    into: Into,
    limit: usize,
    count: usize,
    reached: Vec<bool>,
    drawn: Vec<bool>,
    kept: Vec<bool>,
    route: Vec<usize>,
}

impl<P: Plot> Spending<'_, P> {
    fn route_to(&mut self, target: usize) {
        let into = &self.into.arrow;
        let room = self.limit.saturating_sub(self.count);
        if room == 0 {
            self.drawn[target] |= into[target].is_none_or(|arrow| self.kept[arrow]);
            return;
        }
        self.route.clear();
        let mut at = target;
        while let Some(arrow) = into[at].filter(|arrow| !self.kept[*arrow]) {
            self.route.push(arrow);
            at = self.plot.ends(arrow).0;
        }
        let left_out = self.route.len().saturating_sub(room);
        let taken = self.route.get(left_out..).unwrap_or_default();
        self.count = self.count.saturating_add(taken.len());
        if left_out == 0 {
            self.drawn[target] = true;
        }
        for arrow in taken {
            let (from, to) = self.plot.ends(*arrow);
            self.kept[*arrow] = true;
            self.drawn[from] = true;
            self.drawn[to] = true;
            self.reached[to] = true;
        }
    }

    fn keep_whole(&mut self, target: usize, route: &[usize]) -> Found {
        let new = route.iter().filter(|arrow| !self.kept[**arrow]).count();
        if new > self.limit.saturating_sub(self.count) {
            return Found::Beyond;
        }
        self.count = self.count.saturating_add(new);
        for arrow in route {
            let (from, to) = self.plot.ends(*arrow);
            self.kept[*arrow] = true;
            self.drawn[from] = true;
            self.drawn[to] = true;
            self.reached[to] = true;
        }
        self.drawn[target] = true;
        Found::Drawn(target)
    }

    fn nearest_to_a_start(&mut self, premise: usize) -> Option<Found> {
        let boxes = self.drawn.len();
        let target = self
            .plot
            .holding(premise)
            .filter(|at| *at < boxes)
            .filter_map(|at| self.into.rank[at].map(|rank| (rank, at)))
            .min()
            .map(|(_, at)| at)?;
        if self.drawn[target] {
            return Some(Found::Drawn(target));
        }
        let mut route = Vec::new();
        let mut at = target;
        while let Some(arrow) = self.into.arrow[at].filter(|arrow| !self.kept[*arrow]) {
            route.push(arrow);
            at = self.plot.ends(arrow).0;
        }
        Some(self.keep_whole(target, &route))
    }

    fn nearest_after(&mut self, origin: usize, premise: usize) -> Option<Found> {
        let boxes = self.drawn.len();
        let mut holds = vec![false; boxes];
        for at in self.plot.holding(premise).filter(|at| *at < boxes) {
            holds[at] = true;
        }
        if holds[origin] {
            return Some(Found::Drawn(origin));
        }
        let mut parent: Vec<Option<usize>> = vec![None; boxes];
        let mut seen = vec![false; boxes];
        seen[origin] = true;
        let mut queue = VecDeque::from([origin]);
        let mut found = None;
        'walk: while let Some(at) = queue.pop_front() {
            for arrow in self.leaving.of(at) {
                let (_, to) = self.plot.ends(*arrow);
                if seen[to] {
                    continue;
                }
                seen[to] = true;
                parent[to] = Some(*arrow);
                if holds[to] {
                    found = Some(to);
                    break 'walk;
                }
                queue.push_back(to);
            }
        }
        let target = found?;
        let mut path = Vec::new();
        let mut at = target;
        while let Some(arrow) = parent[at] {
            path.push(arrow);
            at = self.plot.ends(arrow).0;
        }
        Some(self.keep_whole(target, &path))
    }
}

fn premise_targets<P: Plot>(spending: &mut Spending<'_, P>) -> Vec<Option<Found>> {
    let premises = spending.plot.premises();
    let mut found: Vec<Option<Found>> = Vec::with_capacity(premises.len());
    for (at, premise) in premises.iter().enumerate() {
        let started = at
            .checked_sub(1)
            .and_then(|before| premises.get(before).zip(found.get(before)))
            .filter(|(before, _)| before.rule == premise.rule && before.kind == PremiseKind::After)
            .map(|(_, start)| *start);
        let target = match (premise.kind, started) {
            (PremiseKind::Reaches, Some(Some(Found::Drawn(origin)))) => {
                spending.nearest_after(origin, at)
            }
            (PremiseKind::Reaches, Some(_)) => None,
            _ => spending.nearest_to_a_start(at),
        };
        found.push(target);
    }
    found
}

pub(crate) fn select(plot: &impl Plot, limit: usize) -> Selection {
    let boxes = plot.boxes();
    let leaving = Leaving::new(plot);
    let start = |at: usize| plot.look(at).start;
    let marked = |at: usize| plot.look(at).marked;
    let mut spending = Spending {
        plot,
        leaving: &leaving,
        into: arrows_into(plot, &leaving),
        limit,
        count: 0,
        reached: (0..boxes).map(start).collect(),
        drawn: (0..boxes)
            .map(|at| start(at) && leaving.of(at).is_empty())
            .collect(),
        kept: vec![false; plot.arrows()],
        route: Vec::new(),
    };
    for target in 0..boxes {
        if marked(target) && (start(target) || spending.into.arrow[target].is_some()) {
            spending.route_to(target);
        }
    }
    let premises = premise_targets(&mut spending);
    let Spending {
        into,
        mut count,
        mut reached,
        mut drawn,
        mut kept,
        ..
    } = spending;
    let mut next: BinaryHeap<Reverse<usize>> = (0..boxes)
        .filter(|at| reached[*at])
        .flat_map(|at| leaving.of(at).iter().copied().map(Reverse))
        .collect();
    while count < limit {
        let Some(Reverse(arrow)) = next.pop() else {
            break;
        };
        if kept[arrow] {
            continue;
        }
        let (from, to) = plot.ends(arrow);
        kept[arrow] = true;
        drawn[from] = true;
        drawn[to] = true;
        if !reached[to] {
            reached[to] = true;
            next.extend(leaving.of(to).iter().copied().map(Reverse));
        }
        count = count.saturating_add(1);
    }
    Selection {
        states: (0..boxes).filter(|at| drawn[*at]).collect(),
        arrows: (0..plot.arrows()).filter(|arrow| kept[*arrow]).collect(),
        beyond: (0..boxes)
            .filter(|at| marked(*at) && !drawn[*at] && (start(*at) || into.arrow[*at].is_some()))
            .count(),
        shown: (0..boxes).filter(|at| marked(*at) && drawn[*at]).count(),
        premises,
    }
}
