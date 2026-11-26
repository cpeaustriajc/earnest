use crate::explore::Search;
use crate::limits::asked;
use crate::outcome::{Path, RuleKind, RuleOutcome, Step, Verdict};
use earnest_ir::{Eventually, FairStep, Fairness, Fault, Frame};
use rustc_hash::FxHashMap;
use std::collections::VecDeque;

const NOWHERE: u32 = u32::MAX;
const STAYS: &str = "nothing changes the state from here, so the run can stay in it forever";
const STOPS: &str =
    "the run may stop here, because no step it must take can change the state from here";

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Edge {
    from: u32,
    to: u32,
    action: u32,
}

struct Edges {
    first: Vec<usize>,
    all: Vec<Edge>,
}

impl Edges {
    fn of(search: &Search<'_>, states: usize) -> Option<Self> {
        let mut all = Vec::with_capacity(search.arrows.len());
        for arrow in &search.arrows {
            if arrow.from != arrow.to {
                all.push(Edge {
                    from: u32::try_from(arrow.from).ok()?,
                    to: u32::try_from(arrow.to).ok()?,
                    action: u32::try_from(arrow.action).ok()?,
                });
            }
        }
        all.sort_unstable();
        all.dedup();
        let mut first = vec![0_usize; states.saturating_add(1)];
        for edge in &all {
            if let Some(count) = first.get_mut(usize::try_from(edge.from).ok()?.saturating_add(1)) {
                *count = count.saturating_add(1);
            }
        }
        let mut total = 0_usize;
        for count in &mut first {
            total = total.saturating_add(*count);
            *count = total;
        }
        Some(Self { first, all })
    }

    fn out(&self, from: u32) -> &[Edge] {
        let at = usize::try_from(from).unwrap_or(usize::MAX);
        let begin = self.first.get(at).copied().unwrap_or(0);
        let end = self
            .first
            .get(at.saturating_add(1))
            .copied()
            .unwrap_or(begin);
        self.all.get(begin..end).unwrap_or_default()
    }

    fn indexed(&self, from: u32) -> impl Iterator<Item = (usize, &Edge)> {
        let at = usize::try_from(from).unwrap_or(usize::MAX);
        let begin = self.first.get(at).copied().unwrap_or(0);
        self.out(from)
            .iter()
            .enumerate()
            .map(move |(offset, edge)| (begin.saturating_add(offset), edge))
    }

    fn position(&self, edge: Edge) -> Option<usize> {
        self.all.binary_search(&edge).ok()
    }

    fn first_action(&self, from: u32, to: u32) -> Option<u32> {
        self.out(from)
            .iter()
            .find(|edge| edge.to == to)
            .map(|edge| edge.action)
    }
}

fn at(flags: &[bool], state: u32) -> bool {
    usize::try_from(state)
        .ok()
        .and_then(|position| flags.get(position))
        .copied()
        .unwrap_or(false)
}

fn put<T>(list: &mut [T], state: u32, value: T) {
    if let Some(place) = usize::try_from(state)
        .ok()
        .and_then(|position| list.get_mut(position))
    {
        *place = value;
    }
}

fn read(list: &[u32], state: u32) -> u32 {
    usize::try_from(state)
        .ok()
        .and_then(|position| list.get(position))
        .copied()
        .unwrap_or(NOWHERE)
}

struct Judged {
    starts_here: Vec<bool>,
    reached: Vec<bool>,
    faulted: Option<(usize, Fault)>,
}

struct Groups {
    of: Vec<u32>,
    sizes: Vec<u32>,
}

fn groups(edges: &Edges, inside: &[bool]) -> Groups {
    let states = inside.len();
    let mut of = vec![NOWHERE; states];
    let mut sizes = Vec::new();
    let mut order = vec![NOWHERE; states];
    let mut low = vec![NOWHERE; states];
    let mut held: Vec<u32> = Vec::new();
    let mut on_hold = vec![false; states];
    let mut walk: Vec<(u32, usize)> = Vec::new();
    let mut next_order = 0_u32;
    let all = u32::try_from(states).unwrap_or(NOWHERE);
    for root in 0..all {
        if !at(inside, root) || read(&order, root) != NOWHERE {
            continue;
        }
        walk.push((root, 0));
        while let Some((state, tried)) = walk.pop() {
            if tried == 0 {
                put(&mut order, state, next_order);
                put(&mut low, state, next_order);
                next_order = next_order.saturating_add(1);
                held.push(state);
                put(&mut on_hold, state, true);
            }
            if let Some(edge) = edges.out(state).get(tried) {
                walk.push((state, tried.saturating_add(1)));
                if !at(inside, edge.to) {
                    continue;
                }
                if read(&order, edge.to) == NOWHERE {
                    walk.push((edge.to, 0));
                } else if at(&on_hold, edge.to) {
                    let lower = read(&low, state).min(read(&order, edge.to));
                    put(&mut low, state, lower);
                }
                continue;
            }
            if read(&low, state) == read(&order, state) {
                let group = u32::try_from(sizes.len()).unwrap_or(NOWHERE);
                let mut size = 0_u32;
                while let Some(member) = held.pop() {
                    put(&mut on_hold, member, false);
                    put(&mut of, member, group);
                    size = size.saturating_add(1);
                    if member == state {
                        break;
                    }
                }
                sizes.push(size);
            }
            if let Some((parent, _)) = walk.last() {
                let lower = read(&low, *parent).min(read(&low, state));
                put(&mut low, *parent, lower);
            }
        }
    }
    Groups { of, sizes }
}

struct Lasso {
    states: Vec<u32>,
    actions: Vec<Option<u32>>,
    repeats_from: Option<usize>,
    stops_with_ways_out: bool,
}

enum Hunted {
    Nothing,
    Run(Lasso),
    Lost,
}

struct Hunt<'h> {
    edges: &'h Edges,
    fair: &'h [Part],
    possible: &'h [Vec<bool>],
    stops: Stops,
    depth: &'h [u32],
    parents: &'h [u32],
}

#[derive(Clone, Copy)]
enum Stops {
    WhereNothingChangesTheState,
    WhereNothingFairChangesTheState,
}

struct Part {
    actions: Vec<u32>,
    arrows: Option<Vec<bool>>,
}

impl Part {
    fn counts(&self, at: usize, edge: &Edge) -> bool {
        match &self.arrows {
            Some(flags) => flags.get(at).copied().unwrap_or(false),
            None => self.actions.contains(&edge.action),
        }
    }
}

impl Hunt<'_> {
    fn stays(&self, state: u32) -> bool {
        match self.stops {
            Stops::WhereNothingFairChangesTheState => {
                !self.possible.iter().any(|flags| at(flags, state))
            }
            Stops::WhereNothingChangesTheState => self.edges.out(state).is_empty(),
        }
    }

    fn doomed(&self, inside: &[bool], groups: &Groups) -> Vec<bool> {
        let count = groups.sizes.len();
        let mut put_off = vec![vec![false; count]; self.fair.len()];
        let mut taken = vec![vec![false; count]; self.fair.len()];
        let all = u32::try_from(inside.len()).unwrap_or(NOWHERE);
        for state in 0..all {
            let group = read(&groups.of, state);
            if group == NOWHERE {
                continue;
            }
            for (number, part) in self.fair.iter().enumerate() {
                let can = self
                    .possible
                    .get(number)
                    .is_some_and(|flags| at(flags, state));
                if !can && let Some(flags) = put_off.get_mut(number) {
                    put(flags, group, true);
                }
                let inside_the_group = self
                    .edges
                    .indexed(state)
                    .any(|(at, edge)| part.counts(at, edge) && read(&groups.of, edge.to) == group);
                if inside_the_group && let Some(flags) = taken.get_mut(number) {
                    put(flags, group, true);
                }
            }
        }
        (0..all)
            .map(|state| {
                let group = read(&groups.of, state);
                if group == NOWHERE {
                    return false;
                }
                if self.stays(state) {
                    return true;
                }
                if read(&groups.sizes, group) < 2 {
                    return false;
                }
                (0..self.fair.len()).all(|number| {
                    let unfair_to_wait = put_off.get(number).is_some_and(|flags| at(flags, group));
                    let happens = taken.get(number).is_some_and(|flags| at(flags, group));
                    unfair_to_wait || happens
                })
            })
            .collect()
    }

    fn way_in(&self, inside: &[bool], triggers: &[bool], doomed: &[bool]) -> Option<Vec<u32>> {
        let all = u32::try_from(inside.len()).unwrap_or(NOWHERE);
        let mut sources: Vec<u32> = (0..all)
            .filter(|state| at(triggers, *state) && at(inside, *state))
            .collect();
        sources.sort_unstable_by_key(|state| (read(self.depth, *state), *state));
        let mut sources = sources.into_iter().peekable();
        let mut seen = vec![false; inside.len()];
        let mut via = vec![NOWHERE; inside.len()];
        let mut frontier: Vec<u32> = Vec::new();
        let mut level = read(self.depth, *sources.peek()?);
        loop {
            while let Some(source) = sources.next_if(|state| read(self.depth, *state) <= level) {
                if !at(&seen, source) {
                    put(&mut seen, source, true);
                    frontier.push(source);
                }
            }
            frontier.sort_unstable();
            if let Some(found) = frontier.iter().copied().find(|state| at(doomed, *state)) {
                let mut chain = vec![found];
                let mut state = found;
                while read(&via, state) != NOWHERE {
                    state = read(&via, state);
                    chain.push(state);
                }
                chain.reverse();
                return Some(chain);
            }
            let next_level = level.saturating_add(1);
            let mut next = Vec::new();
            while let Some(source) = sources.next_if(|state| read(self.depth, *state) <= next_level)
            {
                if !at(&seen, source) {
                    put(&mut seen, source, true);
                    next.push(source);
                }
            }
            for state in &frontier {
                for edge in self.edges.out(*state) {
                    if at(inside, edge.to) && !at(&seen, edge.to) {
                        put(&mut seen, edge.to, true);
                        put(&mut via, edge.to, *state);
                        next.push(edge.to);
                    }
                }
            }
            if next.is_empty() {
                level = read(self.depth, *sources.peek()?);
            } else {
                level = next_level;
            }
            frontier = next;
        }
    }

    fn route(
        &self,
        from: u32,
        group: u32,
        groups: &Groups,
        arrived: impl Fn(u32) -> bool,
    ) -> Option<Vec<u32>> {
        let mut via: FxHashMap<u32, u32> = FxHashMap::default();
        let mut queue = VecDeque::from([from]);
        let mut found = None;
        'search: while let Some(state) = queue.pop_front() {
            for edge in self.edges.out(state) {
                if read(&groups.of, edge.to) != group {
                    continue;
                }
                if arrived(edge.to) {
                    found = Some((edge.to, state));
                    break 'search;
                }
                if edge.to != from && !via.contains_key(&edge.to) {
                    via.insert(edge.to, state);
                    queue.push_back(edge.to);
                }
            }
        }
        let (goal, mut state) = found?;
        let mut chain = vec![goal, state];
        while state != from {
            state = *via.get(&state)?;
            chain.push(state);
        }
        chain.reverse();
        Some(chain)
    }

    fn extend(&self, lasso: &mut Lasso, chain: &[u32]) {
        for pair in chain.windows(2) {
            if let [from, to] = pair {
                lasso.states.push(*to);
                lasso.actions.push(self.edges.first_action(*from, *to));
            }
        }
    }

    fn turn(&self, lasso: &mut Lasso, groups: &Groups) -> Option<()> {
        let home = *lasso.states.last()?;
        let group = read(&groups.of, home);
        let begin = lasso.states.len().saturating_sub(1);
        for (number, part) in self.fair.iter().enumerate() {
            let can = |state: u32| {
                self.possible
                    .get(number)
                    .is_some_and(|flags| at(flags, state))
            };
            let here = *lasso.states.last()?;
            let waited = lasso.states.iter().skip(begin).any(|state| !can(*state));
            let happened = lasso
                .states
                .windows(2)
                .zip(lasso.actions.iter().skip(1))
                .skip(begin)
                .any(|(pair, taken)| {
                    let (Some(action), [from, to]) = (taken, pair) else {
                        return false;
                    };
                    let edge = Edge {
                        from: *from,
                        to: *to,
                        action: *action,
                    };
                    self.edges
                        .position(edge)
                        .is_some_and(|at| part.counts(at, &edge))
                });
            if waited || happened {
                continue;
            }
            let all = u32::try_from(groups.of.len()).unwrap_or(NOWHERE);
            let waits_somewhere =
                (0..all).any(|state| read(&groups.of, state) == group && !can(state));
            if waits_somewhere {
                let chain = self.route(here, group, groups, |state| !can(state))?;
                self.extend(lasso, &chain);
                continue;
            }
            let takes = |state: u32| {
                self.edges
                    .indexed(state)
                    .find(|(at, edge)| part.counts(*at, edge) && read(&groups.of, edge.to) == group)
                    .map(|(_, edge)| *edge)
            };
            if takes(here).is_none() {
                let chain = self.route(here, group, groups, |state| takes(state).is_some())?;
                self.extend(lasso, &chain);
            }
            let edge = takes(*lasso.states.last()?)?;
            lasso.states.push(edge.to);
            lasso.actions.push(Some(edge.action));
        }
        let here = *lasso.states.last()?;
        if here != home || lasso.states.len().saturating_sub(1) == begin {
            let chain = self.route(here, group, groups, |state| state == home)?;
            self.extend(lasso, &chain);
        }
        lasso.repeats_from = Some(begin);
        Some(())
    }

    fn entered(&self, chain: &[u32]) -> Option<Lasso> {
        let mut before = Vec::new();
        let mut state = *chain.first()?;
        before.push(state);
        while read(self.parents, state) != NOWHERE {
            state = read(self.parents, state);
            before.push(state);
        }
        before.reverse();
        let mut lasso = Lasso {
            actions: vec![None; before.len()],
            states: before,
            repeats_from: None,
            stops_with_ways_out: false,
        };
        self.extend(&mut lasso, chain);
        Some(lasso)
    }

    fn lasso(&self, judged: &Judged) -> Hunted {
        let inside: Vec<bool> = judged.reached.iter().map(|reached| !reached).collect();
        let groups = groups(self.edges, &inside);
        let doomed = self.doomed(&inside, &groups);
        let Some(chain) = self.way_in(&inside, &judged.starts_here, &doomed) else {
            return Hunted::Nothing;
        };
        let Some(mut lasso) = self.entered(&chain) else {
            return Hunted::Lost;
        };
        let last = lasso.states.last().copied().unwrap_or(NOWHERE);
        if self.stays(last) {
            lasso.stops_with_ways_out = !self.edges.out(last).is_empty();
            return Hunted::Run(lasso);
        }
        if self.turn(&mut lasso, &groups).is_none() {
            return Hunted::Lost;
        }
        Hunted::Run(lasso)
    }
}

struct Kept {
    edges: Edges,
    parents: Vec<u32>,
    depth: Vec<u32>,
    fair: Vec<Part>,
    possible: Vec<Vec<bool>>,
}

impl Kept {
    fn possible(&self, fair: &[Part]) -> Vec<Vec<bool>> {
        fair.iter()
            .map(|part| {
                let mut flags = vec![false; self.parents.len()];
                for (at, edge) in self.edges.all.iter().enumerate() {
                    if part.counts(at, edge) {
                        put(&mut flags, edge.from, true);
                    }
                }
                flags
            })
            .collect()
    }

    fn hunt<'k>(&'k self, fair: &'k [Part], possible: &'k [Vec<bool>], stops: Stops) -> Hunt<'k> {
        Hunt {
            edges: &self.edges,
            fair,
            possible,
            stops,
            depth: &self.depth,
            parents: &self.parents,
        }
    }
}

pub(crate) struct Eventual {
    pub(crate) rules: Vec<RuleOutcome>,
    pub(crate) trouble: Option<(usize, String)>,
}

impl Search<'_> {
    fn asked(&self) -> Vec<&Eventually> {
        let only_rule = self.limits.only_rule.as_deref();
        self.model
            .runs
            .eventually
            .iter()
            .filter(|rule| asked(only_rule, &rule.id))
            .collect()
    }

    fn judge_runs(&self, rules: &[&Eventually]) -> Vec<Judged> {
        let states = self.store.len();
        let mut judged: Vec<Judged> = rules
            .iter()
            .map(|_| Judged {
                starts_here: vec![false; states],
                reached: vec![false; states],
                faulted: None,
            })
            .collect();
        let (mut row, mut values) = (Vec::new(), Vec::new());
        for id in 0..states {
            self.store.read_values(id, &mut row, &mut values);
            let is_start = self.store.parent(id).is_none();
            for (rule, found) in rules.iter().zip(&mut judged) {
                if found.faulted.is_some() {
                    continue;
                }
                let mut frame = Frame::new(&values, rule.slots);
                let starts = match &rule.after {
                    Some(after) => after.truth(&mut frame),
                    None => Ok(is_start),
                };
                let reaches = rule.reaches.truth(&mut frame);
                match (starts, reaches) {
                    (Ok(starts), Ok(reaches)) => {
                        if let Some(flag) = found.starts_here.get_mut(id) {
                            *flag = starts;
                        }
                        if let Some(flag) = found.reached.get_mut(id) {
                            *flag = reaches;
                        }
                    }
                    (Err(fault), _) | (_, Err(fault)) => found.faulted = Some((id, fault)),
                }
            }
        }
        judged
    }

    fn lasso_path(&self, lasso: &Lasso) -> Path {
        let named = |state: &u32| {
            usize::try_from(*state).map_or_default(|id| self.named(&self.store.state(id)))
        };
        let steps: Vec<Step> = lasso
            .states
            .windows(2)
            .zip(lasso.actions.iter().skip(1))
            .filter_map(|(pair, action)| {
                let [from, to] = pair else { return None };
                let from = usize::try_from(*from).ok()?;
                let to = usize::try_from(*to).ok()?;
                let only = action.and_then(|action| usize::try_from(action).ok());
                self.step_by(from, to, only)
            })
            .collect();
        Path {
            start: lasso.states.first().map_or_default(named),
            steps,
        }
    }

    fn unknown(rule: &Eventually) -> RuleOutcome {
        RuleOutcome {
            id: rule.id.clone(),
            says: rule.says.clone(),
            verdict: Verdict::Unknown,
            path: None,
            kind: RuleKind::Eventually,
            reason: None,
            repeats_from: None,
        }
    }

    fn kept(&self) -> Option<Kept> {
        let states = self.store.len();
        let edges = Edges::of(self, states)?;
        let parents: Vec<u32> = (0..states)
            .map(|id| {
                self.store
                    .parent(id)
                    .and_then(|parent| u32::try_from(parent).ok())
                    .unwrap_or(NOWHERE)
            })
            .collect();
        let mut depth = vec![0_u32; states];
        for id in 0..states {
            let parent = parents.get(id).copied().unwrap_or(NOWHERE);
            if parent != NOWHERE {
                let above = read(&depth, parent).saturating_add(1);
                if let Some(place) = depth.get_mut(id) {
                    *place = above;
                }
            }
        }
        let fair: Vec<Part> = self
            .model
            .runs
            .fair
            .iter()
            .filter_map(|action| u32::try_from(*action).ok())
            .map(|action| Part {
                actions: vec![action],
                arrows: None,
            })
            .collect();
        let mut kept = Kept {
            edges,
            parents,
            depth,
            fair,
            possible: Vec::new(),
        };
        kept.possible = kept.possible(&kept.fair);
        Some(kept)
    }

    fn parts(&self, fairness: &Fairness, edges: &Edges) -> Vec<Part> {
        let actions = self.model.actions.len();
        fairness
            .actions
            .iter()
            .map(|part| {
                let narrowed = part.iter().any(|position| *position >= actions);
                Part {
                    actions: part
                        .iter()
                        .filter(|position| **position < actions)
                        .filter_map(|position| u32::try_from(*position).ok())
                        .collect(),
                    arrows: narrowed.then(|| self.counted(part, edges)),
                }
            })
            .collect()
    }

    fn counted(&self, part: &[usize], edges: &Edges) -> Vec<bool> {
        let actions = self.model.actions.len();
        let steps: Vec<&FairStep> = part
            .iter()
            .filter_map(|position| position.checked_sub(actions))
            .filter_map(|step| self.model.runs.fair_steps.get(step))
            .collect();
        edges
            .all
            .iter()
            .map(|edge| {
                let action = usize::try_from(edge.action).unwrap_or(usize::MAX);
                part.contains(&action)
                    || steps
                        .iter()
                        .any(|step| step.action == action && self.takes(step, edge))
            })
            .collect()
    }

    fn takes(&self, step: &FairStep, edge: &Edge) -> bool {
        let (Ok(from), Ok(to)) = (usize::try_from(edge.from), usize::try_from(edge.to)) else {
            return false;
        };
        let before = self.store.state(from);
        if !step.over.is_empty() {
            let after = self.store.state(to);
            let changes = step
                .over
                .iter()
                .any(|field| before.get(*field) != after.get(*field));
            if !changes {
                return false;
            }
        }
        if step.inputs.is_empty() {
            return true;
        }
        self.inputs_by(from, to, step.action)
            .into_iter()
            .filter_map(|input| self.input_values(step.action, input, &before))
            .any(|values| {
                step.inputs
                    .iter()
                    .all(|(at, value)| values.get(*at) == Some(value))
            })
    }

    fn broken(&self, rule: &Eventually, lasso: &Lasso) -> RuleOutcome {
        let reason = match lasso.repeats_from {
            Some(begin) => format!(
                "steps {} to {} can repeat forever",
                begin.saturating_add(1),
                lasso.states.len().saturating_sub(1)
            ),
            None if lasso.stops_with_ways_out => STOPS.to_owned(),
            None => STAYS.to_owned(),
        };
        RuleOutcome {
            verdict: Verdict::Broken,
            path: Some(self.lasso_path(lasso)),
            reason: Some(reason),
            repeats_from: lasso.repeats_from,
            ..Self::unknown(rule)
        }
    }

    pub(crate) fn eventually(&self) -> Eventual {
        let rules = self.asked();
        let whole = self.reason.is_none() && !self.cut_short && !rules.is_empty();
        let Some(kept) = whole.then(|| self.kept()).flatten() else {
            return Eventual {
                rules: rules.iter().map(|rule| Self::unknown(rule)).collect(),
                trouble: None,
            };
        };
        let hunt = kept.hunt(
            &kept.fair,
            &kept.possible,
            Stops::WhereNothingChangesTheState,
        );
        let mut trouble = None;
        let outcomes = rules
            .iter()
            .zip(self.judge_runs(&rules))
            .map(|(rule, judged)| {
                if let Some((id, fault)) = judged.faulted {
                    trouble.get_or_insert_with(|| {
                        let fault = fault.describe();
                        (id, format!("the rule `{}` {fault} in this state", rule.id))
                    });
                    return Self::unknown(rule);
                }
                let hunted = match &rule.fair {
                    None => hunt.lasso(&judged),
                    Some(fairness) => {
                        let fair = self.parts(fairness, &kept.edges);
                        let possible = kept.possible(&fair);
                        let stops = if fairness.step {
                            Stops::WhereNothingChangesTheState
                        } else {
                            Stops::WhereNothingFairChangesTheState
                        };
                        kept.hunt(&fair, &possible, stops).lasso(&judged)
                    }
                };
                match hunted {
                    Hunted::Run(lasso) => self.broken(rule, &lasso),
                    Hunted::Lost => Self::unknown(rule),
                    Hunted::Nothing => RuleOutcome {
                        verdict: Verdict::Holds,
                        ..Self::unknown(rule)
                    },
                }
            })
            .collect();
        Eventual {
            rules: outcomes,
            trouble,
        }
    }
}
