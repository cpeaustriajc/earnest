use crate::plot::Plot;
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
    into: Into,
    limit: usize,
    count: usize,
    reached: Vec<bool>,
    drawn: Vec<bool>,
    kept: Vec<bool>,
    route: Vec<usize>,
}

impl<P: Plot> Spending<'_, P> {
    fn route_to(&mut self, target: usize) -> bool {
        let into = &self.into.arrow;
        let room = self.limit.saturating_sub(self.count);
        if room == 0 {
            self.drawn[target] |= into[target].is_none_or(|arrow| self.kept[arrow]);
            return self.drawn[target];
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
        self.drawn[target]
    }

    fn nearest(&self, premise: usize, wanted: impl Fn(usize) -> bool) -> Option<usize> {
        self.plot
            .holding(premise)
            .filter(|at| *at < self.drawn.len() && wanted(*at))
            .filter_map(|at| self.into.rank[at].map(|rank| (rank, at)))
            .min()
            .map(|(_, at)| at)
    }

    fn premise(&mut self, premise: usize) -> Option<Found> {
        if let Some(drawn) = self.nearest(premise, |at| self.drawn[at]) {
            return Some(Found::Drawn(drawn));
        }
        let target = self.nearest(premise, |_| true)?;
        Some(if self.route_to(target) {
            Found::Drawn(target)
        } else {
            Found::Beyond
        })
    }
}

pub(crate) fn select(plot: &impl Plot, limit: usize) -> Selection {
    let boxes = plot.boxes();
    let leaving = Leaving::new(plot);
    let start = |at: usize| plot.look(at).start;
    let marked = |at: usize| plot.look(at).marked;
    let mut spending = Spending {
        plot,
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
    let premises = (0..plot.premises().len())
        .map(|premise| spending.premise(premise))
        .collect();
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
