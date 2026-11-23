use crate::store::{RangeSlot, RowLayout, hash_words};
use earnest_ir::{Action, Code, Fault, FieldStep, Frame, Input, Model, Scalar};
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
    digits: Vec<usize>,
    known: Known,
    seen: Seen,
}

#[derive(Debug, Default)]
struct Known {
    guard: Option<Result<bool, Fault>>,
    scalars: Vec<Option<Result<Scalar, Fault>>>,
    updates: Vec<Option<Result<Value, Fault>>>,
}

#[derive(Debug)]
struct Reuse {
    guard: usize,
    updates: Vec<usize>,
}

const CODE_BITS: u32 = 16;

#[derive(Debug, Default)]
struct Seen {
    codes: Vec<u64>,
    code_words_met: Vec<usize>,
    values: Vec<Value>,
    at: HashTable<usize>,
    rows: Vec<u64>,
    row_at: HashTable<(usize, u64)>,
    words: HashTable<(u64, u64)>,
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
    code_mask: u64,
    placed: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SeenBy {
    WrittenCodes,
    Rows,
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
    direct: Option<Vec<(FieldStep, RangeSlot)>>,
    seen_by: SeenBy,
    combinations: usize,
    tried: Tried,
    refill: Refill,
    reuse: Option<Reuse>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Refill {
    Turned,
    Every,
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
                let code_mask = u64::MAX
                    .checked_shr(u64::BITS.saturating_sub(slot.width))
                    .unwrap_or(0);
                Some(Write {
                    code,
                    slot,
                    holds,
                    code_mask,
                    placed: code_mask.checked_shl(slot.shift).unwrap_or(0),
                })
            })
            .collect::<Option<Vec<_>>>()
            .map_or(InPlace::SomeUpdateHasNoSlot, InPlace::Writes);
        let direct = match &in_place {
            InPlace::Writes(writes) if action.inputs.is_empty() => writes
                .iter()
                .map(|write| {
                    (write.holds == Holds::WholeNumber)
                        .then(|| write.code.field_step())
                        .flatten()
                        .map(|step| (step, write.slot))
                })
                .collect(),
            InPlace::Writes(_) | InPlace::SomeUpdateHasNoSlot => None,
        };
        let seen_by = match &in_place {
            InPlace::Writes(writes)
                if writes
                    .iter()
                    .try_fold(0_u32, |bits, write| bits.checked_add(write.slot.width))
                    .is_some_and(|bits| bits <= CODE_BITS) =>
            {
                SeenBy::WrittenCodes
            }
            InPlace::Writes(_) | InPlace::SomeUpdateHasNoSlot => SeenBy::Rows,
        };
        let refill = if action.inputs_may_be_overwritten() {
            Refill::Every
        } else {
            Refill::Turned
        };
        let reuse = (refill == Refill::Turned)
            .then(|| Reuse {
                guard: first_input_read(action, &action.guard),
                updates: updates
                    .iter()
                    .map(|(_, code)| first_input_read(action, code))
                    .collect(),
            })
            .filter(|reuse| {
                std::iter::once(&reuse.guard)
                    .chain(&reuse.updates)
                    .any(|first| *first > 0)
            });
        Self {
            action,
            updates,
            in_place,
            direct,
            seen_by,
            combinations,
            tried,
            refill,
            reuse,
        }
    }

    fn kept<'k, T>(&self, update: usize, known: &'k mut [Option<T>]) -> Option<&'k mut Option<T>> {
        let first = self.reuse.as_ref()?.updates.get(update)?;
        (*first > 0).then(|| known.get_mut(update)).flatten()
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

fn first_input_read(action: &Action, code: &Code) -> usize {
    let Some(slots) = code.slots_read() else {
        return 0;
    };
    let inputs = &action.inputs;
    if !slots
        .iter()
        .all(|slot| inputs.iter().any(|input| input.slot == *slot))
    {
        return 0;
    }
    inputs
        .iter()
        .position(|input| slots.contains(&input.slot))
        .unwrap_or(inputs.len())
}

fn known_or_scalar(
    known: Option<&mut Option<Result<Scalar, Fault>>>,
    code: &Code,
    frame: &mut Frame<'_>,
) -> Result<Scalar, Fault> {
    let Some(place) = known else {
        return code.scalar(frame);
    };
    if let Some(answer) = *place {
        return answer;
    }
    let answer = code.scalar(frame);
    *place = Some(answer);
    answer
}

fn known_or_eval(
    known: Option<&mut Option<Result<Value, Fault>>>,
    code: &Code,
    frame: &mut Frame<'_>,
) -> Result<Value, Fault> {
    let Some(place) = known else {
        return code.eval(frame);
    };
    if let Some(answer) = place {
        return answer.clone();
    }
    let answer = code.eval(frame);
    *place = Some(answer.clone());
    answer
}

impl Known {
    fn start(&mut self, reuse: Option<&Reuse>) {
        self.guard = None;
        self.scalars.clear();
        self.updates.clear();
        if let Some(reuse) = reuse {
            self.scalars.resize(reuse.updates.len(), None);
            self.updates.resize(reuse.updates.len(), None);
        }
    }

    fn forget_turned(&mut self, reuse: &Reuse, turned: usize) {
        if reuse.guard < turned {
            self.guard = None;
        }
        for ((scalar, place), first) in self
            .scalars
            .iter_mut()
            .zip(self.updates.iter_mut())
            .zip(&reuse.updates)
        {
            if *first < turned {
                *scalar = None;
                *place = None;
            }
        }
    }
}

#[cfg(test)]
fn told_apart_from_earlier(action: &Action, told: &[Vec<bool>], input: usize) -> bool {
    let mut rest = input;
    let digits: Vec<usize> = action
        .inputs
        .iter()
        .map(|each| {
            let count = each.values.len();
            let digit = rest.checked_rem(count).unwrap_or(usize::MAX);
            rest = rest.checked_div(count).unwrap_or(0);
            digit
        })
        .collect();
    told_apart_at(told, &digits)
}

fn told_apart_at(told: &[Vec<bool>], digits: &[usize]) -> bool {
    digits
        .iter()
        .zip(told)
        .all(|(digit, apart)| apart.get(*digit).copied().unwrap_or(true))
}

fn turn(digits: &mut [usize], inputs: &[Input]) -> usize {
    for (at, (digit, each)) in digits.iter_mut().zip(inputs).enumerate() {
        *digit = digit.saturating_add(1);
        if *digit < each.values.len() {
            return at.saturating_add(1);
        }
        *digit = 0;
    }
    digits.len()
}

fn put_inputs(inputs: &[Input], digits: &[usize], count: usize, frame: &mut Frame<'_>) {
    for (each, digit) in inputs.iter().zip(digits).take(count) {
        if let Some(found) = each.values.get(*digit) {
            frame.put_copy(each.slot, found);
        }
    }
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

    #[inline]
    fn first_codes(&mut self, codes: u64) -> bool {
        let at = usize::try_from(codes / u64::from(u64::BITS)).unwrap_or(usize::MAX);
        let bit = 1_u64 << (codes % u64::from(u64::BITS));
        let Some(word) = self.codes.get_mut(at) else {
            return self.first_codes_beyond_the_table(codes, at);
        };
        if *word & bit != 0 {
            return false;
        }
        if *word == 0 {
            self.code_words_met.push(at);
        }
        *word |= bit;
        true
    }

    #[cold]
    #[inline(never)]
    fn first_codes_beyond_the_table(&mut self, codes: u64, at: usize) -> bool {
        self.codes.resize(at.saturating_add(1), 0);
        self.first_codes(codes)
    }

    fn first_row(&mut self, row: &[u64], hash: u64) -> bool {
        if let [word] = row {
            return match self
                .words
                .entry(hash, |(met, _)| met == word, |(_, hash)| *hash)
            {
                hashbrown::hash_table::Entry::Occupied(_) => false,
                hashbrown::hash_table::Entry::Vacant(vacant) => {
                    vacant.insert((*word, hash));
                    true
                }
            };
        }
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
        for at in self.code_words_met.drain(..) {
            if let Some(word) = self.codes.get_mut(at) {
                *word = 0;
            }
        }
        self.at.clear();
        self.values.clear();
        self.row_at.clear();
        self.rows.clear();
        self.words.clear();
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
    mut known: Option<&mut [Option<Result<Value, Fault>>]>,
) -> Result<(), Fault> {
    updates.clear();
    for (at, (position, code)) in planned.updates.iter().enumerate() {
        if *position >= state.len() {
            break;
        }
        let place = known.as_deref_mut().and_then(|all| planned.kept(at, all));
        updates.push((*position, known_or_eval(place, code, frame)?));
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
        let inputs = planned.action.inputs.as_slice();
        let mut digits = std::mem::take(&mut self.scratch.digits);
        digits.clear();
        digits.resize(inputs.len(), 0);
        let mut turned = usize::MAX;
        let reuse = planned.reuse.as_ref();
        self.scratch.known.start(reuse);
        for input in 0..planned.combinations {
            if input > 0 {
                turned = turned.max(turn(&mut digits, inputs));
            }
            if !self.may_go_on_noting_a_time_stop() {
                break;
            }
            if let Some(told) = skipping
                && !told_apart_at(told, &digits)
            {
                continue;
            }
            let count = match planned.refill {
                Refill::Turned => turned,
                Refill::Every => inputs.len(),
            };
            put_inputs(inputs, &digits, count, &mut self.frame);
            if let Some(reuse) = reuse {
                self.scratch.known.forget_turned(reuse, turned);
            }
            turned = 0;
            self.try_input::<true>(position, planned, input);
        }
        self.scratch.digits = digits;
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
        let reusing = REPEATS && planned.reuse.is_some();
        let guard = match (reusing, self.scratch.known.guard) {
            (true, Some(answer)) => answer,
            (true, None) => {
                let answer = planned.action.guard.truth(&mut self.frame);
                self.scratch.known.guard = Some(answer);
                answer
            }
            (false, _) => planned.action.guard.truth(&mut self.frame),
        };
        match guard {
            Ok(true) => {}
            Ok(false) => return,
            Err(fault) => {
                self.found.note(Whence::Guard(position), fault);
                self.moving.unsure = true;
                return;
            }
        }
        self.moving.enabled = true;
        if !REPEATS
            && let (Some(steps), Some(row)) = (&planned.direct, self.base.row)
            && self.write_directly(steps, row)
        {
            self.offer_row_once::<false>(position, input, planned.seen_by, 0);
            return;
        }
        if let (InPlace::Writes(writes), Some(row)) = (&planned.in_place, self.base.row) {
            let next = &mut self.scratch.next;
            if let ([into], [word]) = (next.as_mut_slice(), row) {
                *into = *word;
            } else {
                next.clear();
                next.extend_from_slice(row);
            }
            let mut fits = true;
            let mut codes = 0_u64;
            for (at, write) in writes.iter().enumerate() {
                let place = reusing
                    .then(|| planned.kept(at, &mut self.scratch.known.scalars))
                    .flatten();
                let number = match (
                    write.holds,
                    known_or_scalar(place, write.code, &mut self.frame),
                ) {
                    (Holds::WholeNumber, Ok(Scalar::Int(number))) => number,
                    (Holds::TrueOrFalse, Ok(Scalar::Bool(flag))) => i64::from(flag),
                    (_, Err(fault)) => {
                        self.found.note(Whence::Update(position), fault);
                        return;
                    }
                    (_, Ok(_)) => {
                        fits = false;
                        break;
                    }
                };
                let (Some(code), Some(word)) =
                    (write.slot.code(number), next.get_mut(write.slot.word))
                else {
                    fits = false;
                    break;
                };
                *word = (*word & !write.placed)
                    | (code.checked_shl(write.slot.shift).unwrap_or(0) & write.placed);
                if REPEATS && planned.seen_by == SeenBy::WrittenCodes {
                    codes =
                        codes.checked_shl(write.slot.width).unwrap_or(0) | (code & write.code_mask);
                }
            }
            if fits {
                self.offer_row_once::<REPEATS>(position, input, planned.seen_by, codes);
                return;
            }
        }
        match next_updates(
            planned,
            self.base.values,
            &mut self.frame,
            &mut self.scratch.updates,
            reusing.then_some(self.scratch.known.updates.as_mut_slice()),
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

    #[inline]
    fn write_directly(&mut self, steps: &[(FieldStep, RangeSlot)], row: &[u64]) -> bool {
        let next = &mut self.scratch.next;
        if let ([into], [word]) = (next.as_mut_slice(), row) {
            *into = *word;
        } else {
            next.clear();
            next.extend_from_slice(row);
        }
        for (step, slot) in steps {
            if !step
                .apply(self.base.values)
                .is_some_and(|number| slot.write(next, number))
            {
                return false;
            }
        }
        true
    }

    #[inline]
    fn offer_row_once<const REPEATS: bool>(
        &mut self,
        position: usize,
        input: usize,
        seen_by: SeenBy,
        codes: u64,
    ) {
        let next = &self.scratch.next;
        let first = match (REPEATS, seen_by) {
            (false, _) => Some(hash_words(next)),
            (true, SeenBy::WrittenCodes) => self
                .scratch
                .seen
                .first_codes(codes)
                .then(|| hash_words(next)),
            (true, SeenBy::Rows) => {
                let hash = hash_words(next);
                self.scratch.seen.first_row(next, hash).then_some(hash)
            }
        };
        if let Some(hash) = first
            && self.sink.offer_row(position, input, next, hash)
        {
            self.offered = self.offered.saturating_add(1);
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
    use super::{Asked, Base, Budget, Expand, Move, Scratch, Successors, Whence, Width};
    use super::{Seen, chosen, input_count, told_apart_from_earlier, turn};
    use crate::store::{Lookup, Store};
    use earnest_ir::{Action, Expr, Input};
    use earnest_ir::{Code, Fault, Field, Frame, Math, Model, Op, Order, View, eval, truth};
    use earnest_value::Symbols;
    use earnest_value::{Domain, IntType, Value};

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
    fn turning_the_digits_walks_the_combinations_in_order_and_says_how_many_inputs_changed() {
        let two_by_three = action(vec![
            input("a", 0, numbers(&[1, 2])),
            input("b", 1, numbers(&[10, 20, 30])),
        ]);
        let mut digits = vec![0, 0];
        let mut changed = Vec::new();
        for combination in 1..6 {
            changed.push(turn(&mut digits, &two_by_three.inputs));
            let found: Vec<Option<&Value>> = two_by_three
                .inputs
                .iter()
                .zip(&digits)
                .map(|(each, digit)| each.values.get(*digit))
                .collect();
            let expected: Vec<Option<&Value>> = chosen(&two_by_three, combination).collect();
            assert_eq!(found, expected, "combination {combination}");
        }
        assert_eq!(
            changed,
            vec![1, 2, 1, 2, 1],
            "a carry turns the next input too"
        );
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

    const SEARCH_SLOTS: usize = 4;
    const FAST: usize = 0;
    const SLOW: usize = 1;
    const SLOWEST: usize = 2;
    const BOUND: usize = 3;

    type Next = (usize, usize, Vec<Value>);

    #[derive(Debug, PartialEq)]
    struct Expanded {
        next: Vec<Next>,
        trouble: Option<(Whence, Fault)>,
        cut_short: bool,
        stuck: bool,
        width: Width,
    }

    #[derive(Clone, Copy, Debug)]
    enum Handed {
        Packed,
        AsValues,
    }

    struct Gathered<'s> {
        store: &'s Store,
        next: Vec<Next>,
    }

    impl Successors for Gathered<'_> {
        fn offer(
            &mut self,
            action: usize,
            input: usize,
            base: Base<'_>,
            updates: &[(usize, Value)],
        ) -> bool {
            let mut next = base.values.to_vec();
            for (position, value) in updates {
                if let Some(place) = next.get_mut(*position) {
                    *place = value.clone();
                }
            }
            self.next.push((action, input, next));
            true
        }

        fn offer_row(&mut self, action: usize, input: usize, row: &[u64], _: u64) -> bool {
            self.next
                .push((action, input, self.store.values_of(row).into_vec()));
            true
        }
    }

    fn wheels() -> Vec<Field> {
        let mut symbols = Symbols::default();
        ["x", "y"]
            .iter()
            .map(|name| Field {
                name: (*name).to_owned(),
                sym: symbols.intern(name),
                domain: Domain::Int { lo: 0, hi: 99 },
                starts: vec![Value::Int(0)],
            })
            .collect()
    }

    fn model_of(actions: Vec<Action>) -> Model {
        Model {
            name: "Wheels".to_owned(),
            file: "wheels.model.ts".to_owned(),
            line: 1,
            symbols: Symbols::default(),
            fields: wheels(),
            actions,
            rules: Vec::new(),
            ends: Vec::new(),
            view: View::default(),
        }
    }

    fn acting(inputs: Vec<Input>, guard: Expr, updates: &[Option<Expr>]) -> Action {
        let exprs = std::iter::once(guard).chain(updates.iter().flatten().cloned());
        let mut codes = Code::together(exprs).into_iter();
        let guard = codes.next().expect("the guard comes first");
        let updates = updates
            .iter()
            .map(|update| update.as_ref().and_then(|_| codes.next()))
            .collect();
        Action {
            id: "act".to_owned(),
            label: "acts".to_owned(),
            line: 1,
            inputs,
            guard,
            updates,
            slots: SEARCH_SLOTS,
        }
    }

    fn int(number: i64) -> Expr {
        Expr::Const(Value::Int(number))
    }

    fn yes() -> Expr {
        Expr::Const(Value::Bool(true))
    }

    fn math(math: Math, left: Expr, right: Expr) -> Expr {
        Expr::Apply(Op::Math(math), vec![left, right])
    }

    fn plus(left: Expr, right: Expr) -> Expr {
        math(Math::Add, left, right)
    }

    fn below(left: Expr, right: Expr) -> Expr {
        Expr::Apply(Op::Order(Order::Less), vec![left, right])
    }

    fn states() -> Vec<Vec<Value>> {
        [(0, 0), (5, 9), (2, 3), (5, 9)]
            .iter()
            .map(|(x, y)| vec![Value::Int(*x), Value::Int(*y)])
            .collect()
    }

    fn searched(model: &Model, handed: Handed) -> Vec<Expanded> {
        let store = Store::new(&model.fields, 100, Lookup::Presence);
        let moves = Move::all(model, store.layout());
        let expand = Expand {
            model,
            moves: &moves,
            only_rule: None,
            slots: SEARCH_SLOTS,
            asked: Asked::Everything,
            budget: Budget {
                deadline: None,
                successors: usize::MAX,
            },
        };
        let mut scratch = Scratch::default();
        states()
            .iter()
            .map(|state| {
                let mut row = Vec::new();
                assert!(store.encode(state, &mut row), "the state is in the domain");
                let mut sink = Gathered {
                    store: &store,
                    next: Vec::new(),
                };
                let base = Base {
                    values: state,
                    row: match handed {
                        Handed::Packed => Some(row.as_slice()),
                        Handed::AsValues => None,
                    },
                };
                let mark = expand.state(base, None, &mut sink, &mut scratch);
                Expanded {
                    next: sink.next,
                    trouble: mark.trouble,
                    cut_short: mark.cut_short,
                    stuck: mark.stuck,
                    width: mark.width,
                }
            })
            .collect()
    }

    fn worked_out(model: &Model, state: &[Value]) -> Expanded {
        let mut found = Expanded {
            next: Vec::new(),
            trouble: None,
            cut_short: false,
            stuck: false,
            width: Width::Narrow,
        };
        let (mut enabled, mut unsure) = (false, false);
        for (position, action) in model.actions.iter().enumerate() {
            let mut met: Vec<Vec<Value>> = Vec::new();
            for combination in 0..input_count(action) {
                let mut frame = Frame::new(state, SEARCH_SLOTS);
                for (each, value) in action.inputs.iter().zip(chosen(action, combination)) {
                    frame.put(each.slot, value.cloned().unwrap());
                }
                let noted = |whence: Whence, fault: Fault, found: &mut Expanded| {
                    found.trouble.get_or_insert((whence, fault));
                    found.cut_short = true;
                };
                match truth(action.guard.expr(), &mut frame) {
                    Ok(true) => {}
                    Ok(false) => continue,
                    Err(fault) => {
                        noted(Whence::Guard(position), fault, &mut found);
                        unsure = true;
                        continue;
                    }
                }
                enabled = true;
                let mut next = state.to_vec();
                let mut faulted = false;
                for (field, update) in action.updates.iter().enumerate() {
                    let Some(update) = update else {
                        continue;
                    };
                    match eval(update.expr(), &mut frame) {
                        Ok(value) => {
                            if let Some(place) = next.get_mut(field) {
                                *place = value;
                            }
                        }
                        Err(fault) => {
                            noted(Whence::Update(position), fault, &mut found);
                            faulted = true;
                            break;
                        }
                    }
                }
                if frame.computed_beyond_32_bits() {
                    found.width = Width::Beyond32Bits;
                }
                if !faulted && !met.contains(&next) {
                    met.push(next.clone());
                    found.next.push((position, combination, next));
                }
            }
        }
        found.stuck = !enabled && !unsure;
        found
    }

    fn agrees_with_every_combination_worked_out(case: &str, actions: Vec<Action>) {
        let model = model_of(actions);
        let expected: Vec<Expanded> = states()
            .iter()
            .map(|state| worked_out(&model, state))
            .collect();
        for handed in [Handed::Packed, Handed::AsValues] {
            assert_eq!(
                searched(&model, handed),
                expected,
                "{case}, the state handed {handed:?}"
            );
        }
    }

    fn two_inputs() -> Vec<Input> {
        vec![
            input("a", FAST, numbers(&[0, 1, 2])),
            input("b", SLOW, numbers(&[0, 1, 2])),
        ]
    }

    fn three_inputs() -> Vec<Input> {
        vec![
            input("a", FAST, numbers(&[0, 1])),
            input("b", SLOW, numbers(&[0, 1, 2])),
            input("c", SLOWEST, numbers(&[0, 1])),
        ]
    }

    fn changed(how: Math, field: usize, number: i64) -> Expr {
        math(how, Expr::Field(field), int(number))
    }

    fn bounded(op: Op, inner: Expr) -> Expr {
        Expr::Apply(op, vec![inner])
    }

    #[test]
    fn an_action_without_inputs_that_changes_fields_by_constants_gives_what_its_updates_give() {
        let cases = [
            (
                "x up, y down below its dial",
                [
                    Some(changed(Math::Add, 0, 1)),
                    Some(changed(Math::Subtract, 1, 1)),
                ],
            ),
            (
                "x times three, y up to the end of its dial",
                [
                    Some(changed(Math::Multiply, 0, 3)),
                    Some(changed(Math::Add, 1, 90)),
                ],
            ),
            (
                "x divided by zero",
                [Some(changed(Math::Divide, 0, 0)), None],
            ),
            (
                "y remainder by zero",
                [None, Some(changed(Math::Remainder, 1, 0))],
            ),
            (
                "x halved, y remainder by three",
                [
                    Some(changed(Math::Divide, 0, 2)),
                    Some(changed(Math::Remainder, 1, 3)),
                ],
            ),
            ("x below zero", [Some(changed(Math::Subtract, 0, 7)), None]),
            (
                "x beyond 32 bits",
                [Some(changed(Math::Add, 0, 1 << 40)), None],
            ),
            (
                "x past 64 bits",
                [Some(changed(Math::Multiply, 0, i64::MAX)), None],
            ),
            (
                "x past the safe integers",
                [
                    Some(bounded(Op::Safe, changed(Math::Multiply, 0, 1 << 52))),
                    None,
                ],
            ),
            (
                "y past an i8",
                [
                    None,
                    Some(bounded(
                        Op::Fit(IntType::I8),
                        changed(Math::Multiply, 1, 20),
                    )),
                ],
            ),
            (
                "y within a u8",
                [
                    None,
                    Some(bounded(Op::Fit(IntType::U8), changed(Math::Add, 1, 2))),
                ],
            ),
            ("x from y", [Some(plus(Expr::Field(1), int(1))), None]),
        ];
        let both_low = Expr::And(
            Box::new(below(Expr::Field(0), int(5))),
            Box::new(Expr::Apply(
                Op::Order(Order::LessOrEqual),
                vec![Expr::Field(1), int(3)],
            )),
        );
        for guard in [yes(), below(Expr::Field(0), int(5)), both_low] {
            for (case, updates) in &cases {
                agrees_with_every_combination_worked_out(
                    case,
                    vec![acting(Vec::new(), guard.clone(), updates)],
                );
            }
        }
    }

    #[test]
    fn an_update_that_reads_only_the_slow_input_gives_what_every_combination_gives() {
        agrees_with_every_combination_worked_out(
            "x reads b, y reads a",
            vec![acting(
                two_inputs(),
                yes(),
                &[
                    Some(plus(Expr::Field(0), Expr::Local(SLOW))),
                    Some(Expr::Local(FAST)),
                ],
            )],
        );
    }

    #[test]
    fn an_update_that_reads_only_the_fast_input_gives_what_every_combination_gives() {
        agrees_with_every_combination_worked_out(
            "x reads a, y is kept",
            vec![acting(
                two_inputs(),
                yes(),
                &[Some(plus(Expr::Field(0), Expr::Local(FAST))), None],
            )],
        );
    }

    #[test]
    fn an_update_that_reads_no_input_gives_what_every_combination_gives() {
        agrees_with_every_combination_worked_out(
            "x reads no input, y reads a",
            vec![acting(
                two_inputs(),
                yes(),
                &[
                    Some(plus(Expr::Field(0), int(1))),
                    Some(plus(Expr::Field(1), Expr::Local(FAST))),
                ],
            )],
        );
    }

    #[test]
    fn a_guard_and_updates_on_slow_inputs_of_three_give_what_every_combination_gives() {
        agrees_with_every_combination_worked_out(
            "the guard reads b, x reads c, y reads b and c",
            vec![acting(
                three_inputs(),
                below(Expr::Local(SLOW), int(2)),
                &[
                    Some(plus(Expr::Field(0), Expr::Local(SLOWEST))),
                    Some(plus(
                        plus(Expr::Field(1), Expr::Local(SLOW)),
                        Expr::Local(SLOWEST),
                    )),
                ],
            )],
        );
    }

    #[test]
    fn an_update_that_reads_a_slot_the_guard_binds_gives_what_every_combination_gives() {
        let guard = Expr::Bind {
            slot: BOUND,
            value: Box::new(plus(Expr::Local(FAST), Expr::Local(SLOW))),
            body: Box::new(below(Expr::Local(BOUND), int(3))),
        };
        agrees_with_every_combination_worked_out(
            "the guard binds a + b, x reads it, y reads b",
            vec![acting(
                two_inputs(),
                guard,
                &[Some(Expr::Local(BOUND)), Some(Expr::Local(SLOW))],
            )],
        );
    }

    #[test]
    fn an_update_that_faults_for_some_values_of_a_slow_input_gives_what_every_combination_gives() {
        agrees_with_every_combination_worked_out(
            "x divides by b, y reads a",
            vec![acting(
                two_inputs(),
                yes(),
                &[
                    Some(math(Math::Divide, Expr::Field(0), Expr::Local(SLOW))),
                    Some(Expr::Local(FAST)),
                ],
            )],
        );
    }

    #[test]
    fn a_guard_that_faults_for_some_values_of_a_slow_input_gives_what_every_combination_gives() {
        agrees_with_every_combination_worked_out(
            "the guard divides by c, x reads a, y reads b",
            vec![acting(
                three_inputs(),
                below(
                    math(Math::Divide, Expr::Field(1), Expr::Local(SLOWEST)),
                    int(50),
                ),
                &[Some(Expr::Local(FAST)), Some(Expr::Local(SLOW))],
            )],
        );
    }

    #[test]
    fn an_action_that_overwrites_a_slow_input_gives_what_every_combination_gives() {
        let guard = Expr::Choose(
            Box::new(Expr::Apply(Op::Equal, vec![Expr::Local(FAST), int(0)])),
            Box::new(Expr::Bind {
                slot: SLOW,
                value: Box::new(int(7)),
                body: Box::new(yes()),
            }),
            Box::new(yes()),
        );
        agrees_with_every_combination_worked_out(
            "the guard puts 7 in the slot of b when a is 0, y reads b",
            vec![acting(
                two_inputs(),
                guard,
                &[Some(Expr::Local(FAST)), Some(Expr::Local(SLOW))],
            )],
        );
    }

    #[test]
    fn two_actions_one_after_the_other_each_give_what_every_combination_gives() {
        agrees_with_every_combination_worked_out(
            "the second action uses the slots of the first",
            vec![
                acting(
                    two_inputs(),
                    below(Expr::Local(SLOW), int(2)),
                    &[Some(plus(Expr::Field(0), Expr::Local(SLOW))), None],
                ),
                acting(
                    three_inputs(),
                    yes(),
                    &[None, Some(plus(Expr::Local(SLOW), Expr::Local(SLOWEST)))],
                ),
            ],
        );
    }

    fn wide_wheels_model_of(actions: Vec<Action>) -> Model {
        let mut model = model_of(actions);
        for field in &mut model.fields {
            field.domain = Domain::Int { lo: 0, hi: 999_999 };
        }
        model
    }

    fn agrees_in_both_widths(case: &str, actions: &[Action]) {
        for model in [
            model_of(actions.to_vec()),
            wide_wheels_model_of(actions.to_vec()),
        ] {
            let expected: Vec<Expanded> = states()
                .iter()
                .map(|state| worked_out(&model, state))
                .collect();
            for handed in [Handed::Packed, Handed::AsValues] {
                assert_eq!(
                    searched(&model, handed),
                    expected,
                    "{case}, fields up to {:?}, the state handed {handed:?}",
                    model.fields.first().map(|field| &field.domain)
                );
            }
        }
    }

    #[test]
    fn a_row_that_many_combinations_reach_is_offered_once_in_narrow_and_in_wide_rows() {
        agrees_in_both_widths(
            "x adds a and b, y adds b",
            &[acting(
                two_inputs(),
                yes(),
                &[
                    Some(plus(
                        plus(Expr::Field(0), Expr::Local(FAST)),
                        Expr::Local(SLOW),
                    )),
                    Some(plus(Expr::Field(1), Expr::Local(SLOW))),
                ],
            )],
        );
    }

    #[test]
    fn rows_of_three_inputs_that_meet_again_out_of_order_are_offered_once() {
        agrees_in_both_widths(
            "x adds c and a, y is b times c",
            &[acting(
                three_inputs(),
                yes(),
                &[
                    Some(plus(
                        plus(Expr::Field(0), Expr::Local(SLOWEST)),
                        Expr::Local(FAST),
                    )),
                    Some(math(
                        Math::Multiply,
                        Expr::Local(SLOW),
                        Expr::Local(SLOWEST),
                    )),
                ],
            )],
        );
    }

    #[test]
    fn a_row_that_leaves_the_domain_for_some_combinations_is_offered_once_either_way() {
        agrees_in_both_widths(
            "x adds sixty times a, y is the remainder of y and b by 2",
            &[acting(
                two_inputs(),
                yes(),
                &[
                    Some(plus(
                        Expr::Field(0),
                        math(Math::Multiply, Expr::Local(FAST), int(60)),
                    )),
                    Some(math(
                        Math::Remainder,
                        plus(Expr::Field(1), Expr::Local(SLOW)),
                        int(2),
                    )),
                ],
            )],
        );
    }

    #[test]
    fn rows_written_by_one_update_and_kept_by_the_other_are_offered_once() {
        agrees_in_both_widths(
            "x is the remainder of a and b by 2, y is kept",
            &[acting(
                three_inputs(),
                yes(),
                &[
                    Some(math(
                        Math::Remainder,
                        plus(Expr::Local(FAST), Expr::Local(SLOW)),
                        int(2),
                    )),
                    None,
                ],
            )],
        );
    }

    #[test]
    fn a_row_written_codes_tell_apart_is_new_once_until_they_are_forgotten() {
        let mut seen = Seen::default();
        assert!(
            seen.first_codes(70_000),
            "the first time, past the bits held"
        );
        assert!(seen.first_codes(3), "another");
        assert!(!seen.first_codes(70_000), "met before");
        assert!(!seen.first_codes(3), "met before");
        assert!(seen.first_codes(67), "another bit of a word in use");
        seen.forget();
        assert!(seen.first_codes(3), "new again once forgotten");
        assert!(seen.first_codes(70_000), "new again once forgotten");
    }

    #[test]
    fn rows_of_two_narrow_wheels_are_told_apart_by_written_codes_and_wide_ones_by_rows() {
        let actions = [acting(
            two_inputs(),
            yes(),
            &[Some(Expr::Local(FAST)), Some(Expr::Local(SLOW))],
        )];
        for (model, told) in [
            (model_of(actions.to_vec()), super::SeenBy::WrittenCodes),
            (wide_wheels_model_of(actions.to_vec()), super::SeenBy::Rows),
        ] {
            let store = Store::new(&model.fields, 100, Lookup::Presence);
            let moves = Move::all(&model, store.layout());
            assert_eq!(
                moves
                    .iter()
                    .map(|planned| planned.seen_by)
                    .collect::<Vec<_>>(),
                vec![told],
                "{:?}",
                model.fields.first().map(|field| &field.domain)
            );
        }
    }
}
