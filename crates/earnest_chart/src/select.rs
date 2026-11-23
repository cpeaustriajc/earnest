use crate::plot::Plot;
use std::cmp::Reverse;
use std::collections::{BinaryHeap, VecDeque};

/// The most arrows a chart draws, not counting the arrows from and to `[*]`.
pub const ARROW_LIMIT: usize = 20;

pub(crate) struct Selection {
    pub(crate) states: Vec<usize>,
    pub(crate) arrows: Vec<usize>,
    pub(crate) beyond: usize,
    pub(crate) shown: usize,
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

fn arrows_into(plot: &impl Plot, leaving: &Leaving) -> Vec<Option<usize>> {
    let boxes = plot.boxes();
    let mut seen: Vec<bool> = (0..boxes).map(|at| plot.look(at).start).collect();
    let mut into = vec![None; boxes];
    let mut queue: VecDeque<usize> = (0..boxes).filter(|at| seen[*at]).collect();
    while let Some(at) = queue.pop_front() {
        for arrow in leaving.of(at) {
            let (_, target) = plot.ends(*arrow);
            if !seen[target] {
                seen[target] = true;
                into[target] = Some(*arrow);
                queue.push_back(target);
            }
        }
    }
    into
}

pub(crate) fn select(plot: &impl Plot, limit: usize) -> Selection {
    let boxes = plot.boxes();
    let leaving = Leaving::new(plot);
    let into = arrows_into(plot, &leaving);
    let start = |at: usize| plot.look(at).start;
    let marked = |at: usize| plot.look(at).marked;
    let mut reached: Vec<bool> = (0..boxes).map(start).collect();
    let mut drawn: Vec<bool> = (0..boxes)
        .map(|at| start(at) && leaving.of(at).is_empty())
        .collect();
    let mut kept = vec![false; plot.arrows()];
    let mut count = 0_usize;
    let mut route = Vec::new();
    for target in 0..boxes {
        if !marked(target) || (!start(target) && into[target].is_none()) {
            continue;
        }
        let room = limit.saturating_sub(count);
        if room == 0 {
            drawn[target] |= into[target].is_none_or(|arrow| kept[arrow]);
            continue;
        }
        route.clear();
        let mut at = target;
        while let Some(arrow) = into[at].filter(|arrow| !kept[*arrow]) {
            route.push(arrow);
            at = plot.ends(arrow).0;
        }
        let left_out = route.len().saturating_sub(room);
        let taken = route.get(left_out..).unwrap_or_default();
        count = count.saturating_add(taken.len());
        if left_out == 0 {
            drawn[target] = true;
        }
        for arrow in taken {
            let (from, to) = plot.ends(*arrow);
            kept[*arrow] = true;
            drawn[from] = true;
            drawn[to] = true;
            reached[to] = true;
        }
    }
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
            .filter(|at| marked(*at) && !drawn[*at] && (start(*at) || into[*at].is_some()))
            .count(),
        shown: (0..boxes).filter(|at| marked(*at) && drawn[*at]).count(),
    }
}
