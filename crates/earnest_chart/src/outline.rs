use crate::select::Selection;
use earnest_check::Outline;
use std::cmp::Reverse;
use std::collections::{BinaryHeap, VecDeque};

const NO_ARROW: u32 = u32::MAX;

struct Leaving {
    first: Vec<u32>,
    arrows: Vec<u32>,
}

impl Leaving {
    fn new(outline: &Outline) -> Option<Self> {
        let boxes = outline.boxes();
        let mut first = vec![0_u32; boxes.saturating_add(1)];
        for at in 0..outline.arrows() {
            let (from, _) = outline.ends(at)?;
            let count = first.get_mut(from.saturating_add(1))?;
            *count = count.saturating_add(1);
        }
        for at in 1..first.len() {
            first[at] = first[at].saturating_add(first[at - 1]);
        }
        let mut next = first.clone();
        let mut arrows = vec![0_u32; outline.arrows()];
        for at in 0..outline.arrows() {
            let (from, _) = outline.ends(at)?;
            let place = next.get_mut(from)?;
            *arrows.get_mut(usize::try_from(*place).ok()?)? = u32::try_from(at).ok()?;
            *place = place.saturating_add(1);
        }
        Some(Self { first, arrows })
    }

    fn of(&self, from: usize) -> impl Iterator<Item = usize> {
        let begin = self.first.get(from).copied().unwrap_or(0);
        let end = self
            .first
            .get(from.saturating_add(1))
            .copied()
            .unwrap_or(begin);
        let range = usize::try_from(begin).unwrap_or(0)..usize::try_from(end).unwrap_or(0);
        self.arrows
            .get(range)
            .unwrap_or_default()
            .iter()
            .filter_map(|at| usize::try_from(*at).ok())
    }

    fn none_from(&self, from: usize) -> bool {
        self.of(from).next().is_none()
    }
}

fn arrows_into(outline: &Outline, leaving: &Leaving) -> Vec<u32> {
    let boxes = outline.boxes();
    let mut seen: Vec<bool> = (0..boxes).map(|at| outline.start(at)).collect();
    let mut into = vec![NO_ARROW; boxes];
    let mut queue: VecDeque<usize> = (0..boxes).filter(|at| seen[*at]).collect();
    while let Some(at) = queue.pop_front() {
        for arrow in leaving.of(at) {
            let (_, target) = outline.ends(arrow).unwrap_or_default();
            if !seen[target] {
                seen[target] = true;
                into[target] = u32::try_from(arrow).unwrap_or(NO_ARROW);
                queue.push_back(target);
            }
        }
    }
    into
}

fn arrow_into(into: &[u32], at: usize) -> Option<usize> {
    into.get(at)
        .filter(|arrow| **arrow != NO_ARROW)
        .and_then(|arrow| usize::try_from(*arrow).ok())
}

pub(crate) fn select_outline(outline: &Outline, limit: usize) -> Selection {
    let Some(leaving) = Leaving::new(outline) else {
        return Selection {
            states: Vec::new(),
            arrows: Vec::new(),
            beyond: 0,
            shown: 0,
        };
    };
    let boxes = outline.boxes();
    let into = arrows_into(outline, &leaving);
    let mut reached: Vec<bool> = (0..boxes).map(|at| outline.start(at)).collect();
    let mut drawn: Vec<bool> = (0..boxes)
        .map(|at| outline.start(at) && leaving.none_from(at))
        .collect();
    let mut kept = vec![false; outline.arrows()];
    let mut count = 0_usize;
    let mut route = Vec::new();
    for target in 0..boxes {
        if !outline.marked(target)
            || (!outline.start(target) && arrow_into(&into, target).is_none())
        {
            continue;
        }
        let room = limit.saturating_sub(count);
        if room == 0 {
            drawn[target] |= arrow_into(&into, target).is_none_or(|arrow| kept[arrow]);
            continue;
        }
        route.clear();
        let mut at = target;
        while let Some(arrow) = arrow_into(&into, at).filter(|arrow| !kept[*arrow]) {
            route.push(arrow);
            at = outline.ends(arrow).unwrap_or_default().0;
        }
        let left_out = route.len().saturating_sub(room);
        let taken = route.get(left_out..).unwrap_or_default();
        count = count.saturating_add(taken.len());
        if left_out == 0 {
            drawn[target] = true;
        }
        for arrow in taken {
            let (from, to) = outline.ends(*arrow).unwrap_or_default();
            kept[*arrow] = true;
            drawn[from] = true;
            drawn[to] = true;
            reached[to] = true;
        }
    }
    let mut next: BinaryHeap<Reverse<usize>> = (0..boxes)
        .filter(|at| reached[*at])
        .flat_map(|at| leaving.of(at).map(Reverse))
        .collect();
    while count < limit {
        let Some(Reverse(arrow)) = next.pop() else {
            break;
        };
        if kept[arrow] {
            continue;
        }
        let (from, to) = outline.ends(arrow).unwrap_or_default();
        kept[arrow] = true;
        drawn[from] = true;
        drawn[to] = true;
        if !reached[to] {
            reached[to] = true;
            next.extend(leaving.of(to).map(Reverse));
        }
        count = count.saturating_add(1);
    }
    let reachable = |at: usize| outline.start(at) || arrow_into(&into, at).is_some();
    Selection {
        states: (0..boxes).filter(|at| drawn[*at]).collect(),
        arrows: (0..outline.arrows()).filter(|arrow| kept[*arrow]).collect(),
        beyond: (0..boxes)
            .filter(|at| outline.marked(*at) && !drawn[*at] && reachable(*at))
            .count(),
        shown: (0..boxes)
            .filter(|at| outline.marked(*at) && drawn[*at])
            .count(),
    }
}
