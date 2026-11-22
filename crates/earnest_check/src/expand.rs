use crate::store::{RangeSlot, RowLayout, hash_words};
use earnest_ir::{Action, Code, Fault, Frame, Model};
use earnest_value::Value;
use hashbrown::HashTable;
use rustc_hash::FxBuildHasher;
use std::hash::BuildHasher;
use std::time::Instant;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Whence {
    Rule(usize),
    Guard(usize),
    Update(usize),
    End(usize),
}

#[derive(Debug, Default)]
pub(crate) struct Mark {
    pub(crate) broken: Vec<usize>,
    pub(crate) unjudged: Vec<usize>,
    pub(crate) outside: Option<usize>,
    pub(crate) trouble: Option<(Whence, Fault)>,
    pub(crate) ended: bool,
    pub(crate) stuck: bool,
    pub(crate) cut_short: bool,
    pub(crate) clock: Clock,
    pub(crate) width: Width,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Clock {
    #[default]
    Running,
    Stopped,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Width {
    #[default]
    Narrow,
    Beyond32Bits,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Budget {
    pub(crate) deadline: Option<Instant>,
    pub(crate) successors: usize,
}

impl Width {
    fn of(frame: &Frame<'_>) -> Self {
        if frame.computed_beyond_32_bits() {
            Self::Beyond32Bits
        } else {
            Self::Narrow
        }
    }
}

impl Mark {
    fn note(&mut self, whence: Whence, fault: Fault) {
        if fault == Fault::Stopped {
            self.clock = Clock::Stopped;
            return;
        }
        self.cut_short |= !matches!(whence, Whence::Rule(_));
        if self.trouble.is_none() {
            self.trouble = Some((whence, fault));
        }
    }

    pub(crate) fn is_plain(&self) -> bool {
        self.broken.is_empty()
            && self.unjudged.is_empty()
            && self.outside.is_none()
            && self.trouble.is_none()
            && !self.ended
            && !self.stuck
            && !self.cut_short
            && self.clock == Clock::Running
            && self.width == Width::Narrow
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Asked {
    Everything,
    RulesOnly,
}

#[derive(Clone, Copy, Default)]
struct Enabling {
    enabled: bool,
    unsure: bool,
}

#[derive(Clone, Copy)]
pub(crate) struct Base<'s> {
    pub(crate) values: &'s [Value],
    pub(crate) row: Option<&'s [u64]>,
}

pub(crate) trait Successors {
    fn offer(
        &mut self,
        action: usize,
        input: usize,
        base: Base<'_>,
        updates: &[(usize, Value)],
    ) -> bool;

    fn offer_row(&mut self, action: usize, input: usize, row: &[u64], hash: u64) -> bool;

    fn done(&self) -> bool {
        false
    }
}

#[derive(Debug, Default)]
pub(crate) struct Scratch {
    updates: Vec<(usize, Value)>,
    next: Vec<u64>,
    seen: Seen,
}

#[derive(Debug, Default)]
struct Seen {
    values: Vec<Value>,
    at: HashTable<usize>,
    rows: Vec<u64>,
    row_at: HashTable<(usize, u64)>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Holds {
    WholeNumber,
    TrueOrFalse,
}

#[derive(Clone, Copy, Debug)]
struct Write<'a> {
    code: &'a Code,
    slot: RangeSlot,
    holds: Holds,
}

#[derive(Debug)]
enum InPlace<'a> {
    Writes(Vec<Write<'a>>),
    SomeUpdateHasNoSlot,
}

#[derive(Debug)]
pub(crate) struct Move<'a> {
    action: &'a Action,
    updates: Vec<(usize, &'a Code)>,
    in_place: InPlace<'a>,
    combinations: usize,
    tried: Tried,
}

#[derive(Debug)]
enum Tried {
    Once,
    Every,
    FirstOfEach(Vec<Vec<bool>>),
}

impl<'a> Move<'a> {
    pub(crate) fn all(model: &'a Model, layout: &RowLayout) -> Vec<Self> {
        model
            .actions
            .iter()
            .map(|action| Self::of(action, model.fields.len(), layout))
            .collect()
    }

    fn of(action: &'a Action, fields: usize, layout: &RowLayout) -> Self {
        let combinations = input_count(action);
        let tried = if combinations <= 1 {
            Tried::Once
        } else {
            let told = action.told_apart();
            if told.iter().flatten().all(|apart| *apart) {
                Tried::Every
            } else {
                Tried::FirstOfEach(told)
            }
        };
        let updates = action
            .updates
            .iter()
            .take(fields)
            .enumerate()
            .filter_map(|(position, update)| Some((position, update.as_ref()?)))
            .collect::<Vec<_>>();
        let in_place = updates
            .iter()
            .map(|(position, code)| {
                let (slot, holds) = match (layout.number(*position), layout.truth(*position)) {
                    (Some(slot), _) => (slot, Holds::WholeNumber),
                    (None, Some(slot)) => (slot, Holds::TrueOrFalse),
                    (None, None) => return None,
                };
                Some(Write { code, slot, holds })
            })
            .collect::<Option<Vec<_>>>()
            .map_or(InPlace::SomeUpdateHasNoSlot, InPlace::Writes);
        Self {
            action,
            updates,
            in_place,
            combinations,
            tried,
        }
    }

    fn repeats(&self) -> bool {
        !matches!(self.tried, Tried::Once)
    }

    fn skipping(&self) -> Option<&[Vec<bool>]> {
        match &self.tried {
            Tried::FirstOfEach(told) => Some(told),
            Tried::Once | Tried::Every => None,
        }
    }
}

fn told_apart_from_earlier(action: &Action, told: &[Vec<bool>], input: usize) -> bool {
    let mut rest = input;
    action.inputs.iter().zip(told).all(|(each, apart)| {
        let count = each.values.len();
        let digit = rest.checked_rem(count);
        rest = rest.checked_div(count).unwrap_or(0);
        digit.and_then(|at| apart.get(at)).copied().unwrap_or(true)
    })
}

pub(crate) fn same_words(one: &[u64], other: &[u64]) -> bool {
    one.len() == other.len() && one.iter().zip(other).all(|(left, right)| left == right)
}

impl Seen {
    fn first_updates(&mut self, updates: &[(usize, Value)]) -> bool {
        let width = updates.len();
        let start = self.values.len();
        self.values
            .extend(updates.iter().map(|(_, value)| value.clone()));
        let (seen, values) = (&mut self.at, &self.values);
        let tuple = |at: usize| values.get(at..at.saturating_add(width)).unwrap_or(&[]);
        let Some(candidate) = values.get(start..) else {
            return false;
        };
        let hash = FxBuildHasher.hash_one(candidate);
        match seen.entry(
            hash,
            |at| tuple(*at) == candidate,
            |at| FxBuildHasher.hash_one(tuple(*at)),
        ) {
            hashbrown::hash_table::Entry::Occupied(_) => {
                self.values.truncate(start);
                false
            }
            hashbrown::hash_table::Entry::Vacant(vacant) => {
                vacant.insert(start);
                true
            }
        }
    }

    fn first_row(&mut self, row: &[u64], hash: u64) -> bool {
        let width = row.len();
        let rows = &self.rows;
        let earlier = |at: usize| rows.get(at..at.saturating_add(width)).unwrap_or(&[]);
        match self.row_at.entry(
            hash,
            |(at, _)| same_words(earlier(*at), row),
            |(_, hash)| *hash,
        ) {
            hashbrown::hash_table::Entry::Occupied(_) => false,
            hashbrown::hash_table::Entry::Vacant(vacant) => {
                vacant.insert((self.rows.len(), hash));
                self.rows.extend_from_slice(row);
                true
            }
        }
    }

    fn forget(&mut self) {
        self.at.clear();
        self.values.clear();
        self.row_at.clear();
        self.rows.clear();
    }
}

pub(crate) fn chosen(action: &Action, input: usize) -> impl Iterator<Item = Option<&Value>> {
    let mut rest = input;
    action.inputs.iter().map(move |each| {
        let count = each.values.len();
        let digit = rest.checked_rem(count)?;
        rest = rest.checked_div(count)?;
        each.values.get(digit)
    })
}

fn input_count(action: &Action) -> usize {
    action.inputs.iter().fold(1_usize, |total, each| {
        total.saturating_mul(each.values.len())
    })
}

fn set_inputs(action: &Action, input: usize, frame: &mut Frame<'_>) {
    for (each, value) in action.inputs.iter().zip(chosen(action, input)) {
        if let Some(found) = value {
            frame.put(each.slot, found.clone());
        }
    }
}

fn next_updates(
    planned: &Move<'_>,
    state: &[Value],
    frame: &mut Frame<'_>,
    updates: &mut Vec<(usize, Value)>,
) -> Result<(), Fault> {
    updates.clear();
    for (position, code) in &planned.updates {
        if *position >= state.len() {
            break;
        }
        updates.push((*position, code.eval(frame)?));
    }
    Ok(())
}

struct Expansion<'s, 'f, S> {
    base: Base<'s>,
    moving: Enabling,
    frame: Frame<'f>,
    found: Mark,
    budget: Budget,
    offered: usize,
    sink: &'s mut S,
    scratch: &'s mut Scratch,
}

impl<S: Successors> Expansion<'_, '_, S> {
    fn action(&mut self, position: usize, planned: &Move<'_>) {
        if planned.repeats() {
            self.every_input_told_apart(position, planned);
        } else if planned.combinations == 1 && self.may_go_on_noting_a_time_stop() {
            if !planned.action.inputs.is_empty() {
                set_inputs(planned.action, 0, &mut self.frame);
            }
            self.try_input::<false>(position, planned, 0);
        }
    }

    fn every_input_told_apart(&mut self, position: usize, planned: &Move<'_>) {
        self.scratch.seen.forget();
        let skipping = planned.skipping();
        for input in 0..planned.combinations {
            if !self.may_go_on_noting_a_time_stop() {
                break;
            }
            if let Some(told) = skipping
                && !told_apart_from_earlier(planned.action, told, input)
            {
                continue;
            }
            set_inputs(planned.action, input, &mut self.frame);
            self.try_input::<true>(position, planned, input);
        }
    }

    #[inline]
    fn may_go_on_noting_a_time_stop(&mut self) -> bool {
        if self.frame.out_of_time() {
            self.found.clock = Clock::Stopped;
            return false;
        }
        self.offered <= self.budget.successors && !self.sink.done()
    }

    #[inline]
    fn try_input<const REPEATS: bool>(
        &mut self,
        position: usize,
        planned: &Move<'_>,
        input: usize,
    ) {
        match planned.action.guard.truth(&mut self.frame) {
            Ok(true) => {}
            Ok(false) => return,
            Err(fault) => {
                self.found.note(Whence::Guard(position), fault);
                self.moving.unsure = true;
                return;
            }
        }
        self.moving.enabled = true;
        if let (InPlace::Writes(writes), Some(row)) = (&planned.in_place, self.base.row) {
            let next = &mut self.scratch.next;
            next.clear();
            next.extend_from_slice(row);
            let mut fits = true;
            for write in writes {
                let number = match (write.holds, write.code.eval(&mut self.frame)) {
                    (Holds::WholeNumber, Ok(Value::Int(number))) => number,
                    (Holds::TrueOrFalse, Ok(Value::Bool(flag))) => i64::from(flag),
                    (_, Err(fault)) => {
                        self.found.note(Whence::Update(position), fault);
                        return;
                    }
                    (_, Ok(_)) => {
                        fits = false;
                        break;
                    }
                };
                if !write.slot.write(next, number) {
                    fits = false;
                    break;
                }
            }
            if fits {
                let hash = hash_words(next);
                if (!REPEATS || self.scratch.seen.first_row(next, hash))
                    && self.sink.offer_row(position, input, next, hash)
                {
                    self.offered = self.offered.saturating_add(1);
                }
                return;
            }
        }
        match next_updates(
            planned,
            self.base.values,
            &mut self.frame,
            &mut self.scratch.updates,
        ) {
            Ok(()) => {
                if (!REPEATS || self.scratch.seen.first_updates(&self.scratch.updates))
                    && self
                        .sink
                        .offer(position, input, self.base, &self.scratch.updates)
                {
                    self.offered = self.offered.saturating_add(1);
                }
            }
            Err(fault) => self.found.note(Whence::Update(position), fault),
        }
    }

    fn marked_end(&mut self, model: &Model) -> Enabling {
        let mut ending = Enabling::default();
        for (position, end) in model.ends.iter().enumerate() {
            match end.when.truth(&mut self.frame) {
                Ok(true) => ending.enabled = true,
                Ok(false) => {}
                Err(fault) => {
                    self.found.note(Whence::End(position), fault);
                    ending.unsure = true;
                }
            }
        }
        ending
    }
}

pub(crate) struct Expand<'a> {
    pub(crate) model: &'a Model,
    pub(crate) moves: &'a [Move<'a>],
    pub(crate) only_rule: Option<&'a str>,
    pub(crate) slots: usize,
    pub(crate) asked: Asked,
    pub(crate) budget: Budget,
}

impl Expand<'_> {
    pub(crate) fn state<S: Successors>(
        &self,
        base: Base<'_>,
        outside: Option<usize>,
        sink: &mut S,
        scratch: &mut Scratch,
    ) -> Mark {
        let model = self.model;
        let mut found = Mark {
            outside,
            ..Mark::default()
        };
        if found.outside.is_some() {
            found.cut_short = true;
            return found;
        }
        let mut frame = Frame::new(base.values, self.slots).until(self.budget.deadline);
        for (position, rule) in model.rules.iter().enumerate() {
            if self.only_rule.is_some_and(|wanted| wanted != rule.id) {
                continue;
            }
            match rule.holds.truth(&mut frame) {
                Ok(true) => {}
                Ok(false) => found.broken.push(position),
                Err(fault) => {
                    found.note(Whence::Rule(position), fault);
                    found.unjudged.push(position);
                }
            }
        }
        found.width = Width::of(&frame);
        if self.asked == Asked::RulesOnly || found.clock == Clock::Stopped {
            return found;
        }
        let mut expansion = Expansion {
            base,
            moving: Enabling::default(),
            frame,
            found,
            budget: self.budget,
            offered: 0,
            sink,
            scratch,
        };
        for (position, planned) in self.moves.iter().enumerate() {
            expansion.action(position, planned);
            if expansion.found.clock == Clock::Stopped {
                return expansion.found;
            }
            if expansion.sink.done() {
                break;
            }
        }
        let moving = expansion.moving;
        if !moving.enabled {
            let ending = expansion.marked_end(model);
            expansion.found.ended = ending.enabled;
            expansion.found.stuck = !ending.enabled && !ending.unsure && !moving.unsure;
        }
        expansion.found.width = Width::of(&expansion.frame);
        expansion.found
    }
}

#[cfg(test)]
mod tests {
    use super::{Seen, chosen, input_count, told_apart_from_earlier};
    use earnest_ir::{Action, Expr, Input};
    use earnest_value::{Domain, Value};

    fn input(name: &str, slot: usize, values: Vec<Value>) -> Input {
        Input {
            name: name.to_owned(),
            slot,
            domain: Domain::OneOf(values.clone()),
            values,
        }
    }

    fn action(inputs: Vec<Input>) -> Action {
        Action {
            id: "act".to_owned(),
            label: "acts".to_owned(),
            line: 1,
            inputs,
            guard: Expr::Const(Value::Bool(true)).into(),
            updates: Vec::new(),
            slots: 2,
        }
    }

    fn numbers(all: &[i64]) -> Vec<Value> {
        all.iter().copied().map(Value::Int).collect()
    }

    #[test]
    fn a_row_is_new_once_until_the_seen_rows_are_forgotten() {
        let mut seen = Seen::default();
        assert!(seen.first_row(&[7, 1], 3), "the first time");
        assert!(
            seen.first_row(&[7, 2], 3),
            "another row under the same hash"
        );
        assert!(!seen.first_row(&[7, 1], 3), "met before");
        seen.forget();
        assert!(seen.first_row(&[7, 1], 3), "new again once forgotten");
    }

    #[test]
    fn an_action_without_inputs_is_tried_once() {
        assert_eq!(input_count(&action(Vec::new())), 1);
    }

    #[test]
    fn an_action_is_tried_once_per_combination_of_input_values() {
        let two_by_three = action(vec![
            input("a", 0, numbers(&[1, 2])),
            input("b", 1, numbers(&[10, 20, 30])),
        ]);
        assert_eq!(input_count(&two_by_three), 6);
    }

    #[test]
    fn an_input_without_values_leaves_nothing_to_try() {
        let none = action(vec![input("a", 0, Vec::new())]);
        assert_eq!(input_count(&none), 0);
    }

    #[test]
    fn every_combination_number_names_a_different_choice_and_the_first_input_turns_fastest() {
        let two_by_three = action(vec![
            input("a", 0, numbers(&[1, 2])),
            input("b", 1, numbers(&[10, 20, 30])),
        ]);
        let all: Vec<Vec<Value>> = (0..6)
            .map(|combination| {
                chosen(&two_by_three, combination)
                    .map(|value| value.cloned().unwrap())
                    .collect()
            })
            .collect();
        let expected: Vec<Vec<Value>> = [[1, 10], [2, 10], [1, 20], [2, 20], [1, 30], [2, 30]]
            .iter()
            .map(|pair| numbers(pair))
            .collect();
        assert_eq!(all, expected);
    }

    #[test]
    fn a_combination_is_tried_only_when_each_of_its_values_is_told_apart_from_earlier_ones() {
        let two_by_three = action(vec![
            input("a", 0, numbers(&[1, 2])),
            input("b", 1, numbers(&[10, 20, 30])),
        ]);
        let told = [vec![true, true], vec![true, false, true]];
        let tried: Vec<usize> = (0..6)
            .filter(|combination| told_apart_from_earlier(&two_by_three, &told, *combination))
            .collect();
        assert_eq!(
            tried,
            vec![0, 1, 4, 5],
            "the combinations with b at 20 repeat those with b at 10"
        );
    }
}
