use crate::store::{Decoder, RangeSlot, RowLayout, Store, hash_words};
use earnest_ir::{
    Action, Code, Fault, Frame, Input, Model, PackedTest, PackedUpdate, Scalar, Slot,
};
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
    fn of(frame: Option<&Frame<'_>>) -> Self {
        if frame.is_some_and(Frame::computed_beyond_32_bits) {
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
    values: Vec<Value>,
    work: Work,
}

#[derive(Debug, Default)]
struct Work {
    rows: Vec<u64>,
    lit: Vec<bool>,
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
    packed: Option<Packed>,
    packed_inputs: Option<Box<[Option<Packed>]>>,
    seen_by: SeenBy,
    combinations: usize,
    tried: Tried,
    refill: Refill,
    reuse: Option<Reuse>,
}

#[derive(Debug)]
struct Packed {
    guard: PackedTest,
    updates: Vec<PackedUpdate>,
}

const MOST_PACKED_COMBINATIONS: usize = 64;

impl Packed {
    fn of(
        action: &Action,
        updates: &[(usize, &Code)],
        slots: &[Option<Slot>],
        inputs: &[(usize, Value)],
    ) -> Option<Self> {
        Some(Self {
            guard: action.guard.packed_test(slots, inputs)?,
            updates: updates
                .iter()
                .map(|(position, code)| code.packed_update(slots, *position, inputs))
                .collect::<Option<_>>()?,
        })
    }

    fn each_combination(
        action: &Action,
        updates: &[(usize, &Code)],
        slots: &[Option<Slot>],
        tried: &Tried,
    ) -> Option<Box<[Option<Self>]>> {
        let combinations = input_count(action);
        if action.inputs.is_empty()
            || combinations > MOST_PACKED_COMBINATIONS
            || action.inputs_may_be_overwritten()
        {
            return None;
        }
        (0..combinations)
            .map(|combination| {
                if let Tried::FirstOfEach(told) = tried
                    && !told_apart_at(told, &digits_of(action, combination))
                {
                    return Some(None);
                }
                let inputs: Vec<(usize, Value)> = action
                    .inputs
                    .iter()
                    .zip(chosen(action, combination))
                    .map(|(input, value)| Some((input.slot, value?.clone())))
                    .collect::<Option<_>>()?;
                Self::of(action, updates, slots, &inputs).map(Some)
            })
            .collect()
    }
}

fn digits_of(action: &Action, combination: usize) -> Vec<usize> {
    let mut rest = combination;
    action
        .inputs
        .iter()
        .map(|each| {
            let count = each.values.len();
            let digit = rest.checked_rem(count).unwrap_or(0);
            rest = rest.checked_div(count).unwrap_or(0);
            digit
        })
        .collect()
}

#[derive(Debug)]
pub(crate) struct FromRows {
    rules: Vec<Option<PackedTest>>,
    ends: Vec<Option<PackedTest>>,
    decoder: Option<Decoder>,
}

impl FromRows {
    pub(crate) fn of(model: &Model, store: &Store) -> Self {
        let slots = store.slots();
        Self {
            rules: model
                .rules
                .iter()
                .map(|rule| rule.holds.packed_test(&slots, &[]))
                .collect(),
            ends: model
                .ends
                .iter()
                .map(|end| end.when.packed_test(&slots, &[]))
                .collect(),
            decoder: store.decoder(),
        }
    }
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
    pub(crate) fn all(model: &'a Model, layout: &RowLayout, slots: &[Option<Slot>]) -> Vec<Self> {
        model
            .actions
            .iter()
            .map(|action| Self::of(action, model.fields.len(), layout, slots))
            .collect()
    }

    fn of(action: &'a Action, fields: usize, layout: &RowLayout, slots: &[Option<Slot>]) -> Self {
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
        let packed = action
            .inputs
            .is_empty()
            .then(|| Packed::of(action, &updates, slots, &[]))
            .flatten();
        let packed_inputs = Packed::each_combination(action, &updates, slots, &tried);
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
            packed,
            packed_inputs,
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

struct Values<'v> {
    row: Option<&'v [u64]>,
    given: &'v [Value],
    spare: Option<&'v mut Vec<Value>>,
    decoder: Option<&'v Decoder>,
    fields: usize,
    slots: usize,
    deadline: Option<Instant>,
    state: &'v [Value],
    frame: Option<Frame<'v>>,
}

impl<'v> Values<'v> {
    fn frame(&mut self) -> &mut Frame<'v> {
        let Self {
            row,
            given,
            spare,
            decoder,
            fields,
            slots,
            deadline,
            state,
            frame,
        } = self;
        frame.get_or_insert_with(|| {
            let values: &'v [Value] = match (*row, spare.take()) {
                (Some(row), Some(buffer)) if given.len() != *fields => {
                    match decoder {
                        Some(decoder) => decoder.decode(row, buffer),
                        None => buffer.clear(),
                    }
                    let decoded: &'v Vec<Value> = buffer;
                    decoded.as_slice()
                }
                _ => given,
            };
            *state = values;
            Frame::new(values, *slots).until(*deadline)
        })
    }

    fn base(&self) -> Base<'v> {
        Base {
            values: self.state,
            row: self.row,
        }
    }

    fn width(&self) -> Width {
        Width::of(self.frame.as_ref())
    }

    fn judge(&mut self, packed: Option<&PackedTest>, code: &Code) -> Result<bool, Fault> {
        if let (Some(test), Some(row)) = (packed, self.row)
            && let Some(answer) = test.holds(row)
        {
            return Ok(answer);
        }
        code.truth(self.frame())
    }
}

const TICKS_BETWEEN_CLOCK_READS: u32 = 1024;

struct Timer {
    deadline: Option<Instant>,
    ticks: u32,
    expired: bool,
}

impl Timer {
    #[inline]
    fn out_of_time(&mut self) -> bool {
        let Some(limit) = self.deadline else {
            return false;
        };
        self.ticks = self.ticks.wrapping_add(1);
        if !self.expired && self.ticks.is_multiple_of(TICKS_BETWEEN_CLOCK_READS) {
            self.expired = Instant::now() >= limit;
        }
        self.expired
    }
}

struct Expansion<'e, 'v, S> {
    values: &'e mut Values<'v>,
    timer: Timer,
    moving: Enabling,
    found: &'e mut Mark,
    budget: Budget,
    offered: usize,
    sink: &'e mut S,
    scratch: &'e mut Work,
}

impl<S: Successors> Expansion<'_, '_, S> {
    fn action(&mut self, position: usize, planned: &Move<'_>) {
        if let (Some(packed), Some(row)) = (&planned.packed, self.values.row) {
            if self.may_go_on_noting_a_time_stop() && !self.try_packed(position, packed, row) {
                self.try_input::<false>(position, planned, 0);
            }
            return;
        }
        if let (Some(combinations), Some(row)) = (&planned.packed_inputs, self.values.row)
            && self.try_each_packed(position, combinations, row)
        {
            return;
        }
        if planned.repeats() {
            self.every_input_told_apart(position, planned);
        } else if planned.combinations == 1 && self.may_go_on_noting_a_time_stop() {
            if !planned.action.inputs.is_empty() {
                set_inputs(planned.action, 0, self.values.frame());
            }
            self.try_input::<false>(position, planned, 0);
        }
    }

    #[inline]
    fn try_packed(&mut self, position: usize, packed: &Packed, row: &[u64]) -> bool {
        match packed.guard.holds(row) {
            Some(true) => {}
            Some(false) => return true,
            None => return false,
        }
        if let [word] = row {
            let mut next = *word;
            for update in &packed.updates {
                match update.write_word(row, next) {
                    Some(written) => next = written,
                    None => return false,
                }
            }
            self.moving.enabled = true;
            let hash = hash_words(std::slice::from_ref(&next));
            if self
                .sink
                .offer_row(position, 0, std::slice::from_ref(&next), hash)
            {
                self.offered = self.offered.saturating_add(1);
            }
            return true;
        }
        let next = &mut self.scratch.next;
        if let ([into], [word]) = (next.as_mut_slice(), row) {
            *into = *word;
        } else {
            next.clear();
            next.extend_from_slice(row);
        }
        for update in &packed.updates {
            if !update.write(row, next) {
                return false;
            }
        }
        self.moving.enabled = true;
        self.offer_row_once::<false>(position, 0, SeenBy::Rows, 0);
        true
    }

    fn try_each_packed(
        &mut self,
        position: usize,
        combinations: &[Option<Packed>],
        row: &[u64],
    ) -> bool {
        let words = row.len();
        let (rows, lit) = (&mut self.scratch.rows, &mut self.scratch.lit);
        rows.clear();
        lit.clear();
        for combination in combinations {
            let Some(packed) = combination else {
                lit.push(false);
                rows.extend_from_slice(row);
                continue;
            };
            let Some(enabled) = packed.guard.holds(row) else {
                return false;
            };
            lit.push(enabled);
            let start = rows.len();
            rows.extend_from_slice(row);
            if enabled {
                let Some(next) = rows.get_mut(start..) else {
                    return false;
                };
                if !packed.updates.iter().all(|update| update.write(row, next)) {
                    return false;
                }
            }
        }
        self.scratch.seen.forget();
        for (input, combination) in combinations.iter().enumerate() {
            if !self.may_go_on_noting_a_time_stop() {
                break;
            }
            if combination.is_none() || !self.scratch.lit.get(input).copied().unwrap_or(false) {
                continue;
            }
            self.moving.enabled = true;
            let start = input.saturating_mul(words);
            let Some(next) = self.scratch.rows.get(start..start.saturating_add(words)) else {
                continue;
            };
            let hash = hash_words(next);
            if self.scratch.seen.first_row(next, hash)
                && self.sink.offer_row(position, input, next, hash)
            {
                self.offered = self.offered.saturating_add(1);
            }
        }
        true
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
            put_inputs(inputs, &digits, count, self.values.frame());
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
        if self.timer.out_of_time() {
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
        let frame = self.values.frame();
        let guard = match (reusing, self.scratch.known.guard) {
            (true, Some(answer)) => answer,
            (true, None) => {
                let answer = planned.action.guard.truth(frame);
                self.scratch.known.guard = Some(answer);
                answer
            }
            (false, _) => planned.action.guard.truth(frame),
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
        if let (InPlace::Writes(writes), Some(row)) = (&planned.in_place, self.values.row) {
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
                    known_or_scalar(place, write.code, self.values.frame()),
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
        let base = self.values.base();
        match next_updates(
            planned,
            base.values,
            self.values.frame(),
            &mut self.scratch.updates,
            reusing.then_some(self.scratch.known.updates.as_mut_slice()),
        ) {
            Ok(()) => {
                if (!REPEATS || self.scratch.seen.first_updates(&self.scratch.updates))
                    && self
                        .sink
                        .offer(position, input, base, &self.scratch.updates)
                {
                    self.offered = self.offered.saturating_add(1);
                }
            }
            Err(fault) => self.found.note(Whence::Update(position), fault),
        }
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

    fn marked_end(&mut self, model: &Model, packed: &[Option<PackedTest>]) -> Enabling {
        let mut ending = Enabling::default();
        for (position, end) in model.ends.iter().enumerate() {
            match self
                .values
                .judge(packed.get(position).and_then(Option::as_ref), &end.when)
            {
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
    pub(crate) from_rows: &'a FromRows,
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
        let mut values = Values {
            row: base.row,
            given: base.values,
            spare: Some(&mut scratch.values),
            decoder: self.from_rows.decoder.as_ref(),
            fields: model.fields.len(),
            slots: self.slots,
            deadline: self.budget.deadline,
            state: base.values,
            frame: None,
        };
        for (position, rule) in model.rules.iter().enumerate() {
            if self.only_rule.is_some_and(|wanted| wanted != rule.id) {
                continue;
            }
            let packed = self.from_rows.rules.get(position).and_then(Option::as_ref);
            match values.judge(packed, &rule.holds) {
                Ok(true) => {}
                Ok(false) => found.broken.push(position),
                Err(fault) => {
                    found.note(Whence::Rule(position), fault);
                    found.unjudged.push(position);
                }
            }
        }
        found.width = values.width();
        if self.asked == Asked::RulesOnly || found.clock == Clock::Stopped {
            return found;
        }
        let mut expansion = Expansion {
            values: &mut values,
            timer: Timer {
                deadline: self.budget.deadline,
                ticks: 0,
                expired: false,
            },
            moving: Enabling::default(),
            found: &mut found,
            budget: self.budget,
            offered: 0,
            sink,
            scratch: &mut scratch.work,
        };
        for (position, planned) in self.moves.iter().enumerate() {
            expansion.action(position, planned);
            if expansion.found.clock == Clock::Stopped {
                return found;
            }
            if expansion.sink.done() {
                break;
            }
        }
        let moving = expansion.moving;
        if !moving.enabled {
            let ending = expansion.marked_end(model, &self.from_rows.ends);
            expansion.found.ended = ending.enabled;
            expansion.found.stuck = !ending.enabled && !ending.unsure && !moving.unsure;
        }
        found.width = values.width();
        found
    }
}

#[cfg(test)]
mod tests {
    use super::{Asked, Base, Budget, Expand, FromRows, Move, Scratch, Successors, Whence, Width};
    use super::{Seen, chosen, input_count, told_apart_from_earlier, turn};
    use crate::store::{Lookup, Store};
    use earnest_ir::{Action, Expr, Input};
    use earnest_ir::{
        Code, End, Fault, Field, Frame, Math, Model, Op, Order, Rule, SeqOp, View, eval, truth,
    };
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
        let moves = Move::all(model, store.layout(), &store.slots());
        let from_rows = FromRows::of(model, &store);
        let expand = Expand {
            model,
            moves: &moves,
            from_rows: &from_rows,
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
            let moves = Move::all(&model, store.layout(), &store.slots());
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

    struct Seed(u64);

    impl Seed {
        fn next(&mut self) -> u64 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            self.0
        }

        fn below(&mut self, count: usize) -> usize {
            usize::try_from(self.next() % u64::try_from(count).expect("a count")).expect("an index")
        }
    }

    #[derive(Debug, PartialEq)]
    struct Judged {
        next: Vec<Next>,
        broken: Vec<usize>,
        unjudged: Vec<usize>,
        trouble: Option<(Whence, Fault)>,
        ended: bool,
        stuck: bool,
        cut_short: bool,
        width: Width,
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum Path {
        FromRows,
        General,
    }

    #[derive(Clone, Copy, Debug)]
    enum Given {
        RowAlone,
        RowAndValues,
        ValuesAlone,
    }

    fn judged(model: &Model, path: Path, given: Given, states: &[Vec<Value>]) -> Vec<Judged> {
        let store = Store::new(&model.fields, 100, Lookup::Presence);
        let slots = match path {
            Path::FromRows => store.slots(),
            Path::General => vec![None; model.fields.len()],
        };
        let moves = Move::all(model, store.layout(), &slots);
        let from_rows = match path {
            Path::FromRows => FromRows::of(model, &store),
            Path::General => FromRows {
                rules: vec![None; model.rules.len()],
                ends: vec![None; model.ends.len()],
                decoder: store.decoder(),
            },
        };
        let expand = Expand {
            model,
            moves: &moves,
            from_rows: &from_rows,
            only_rule: None,
            slots: SEARCH_SLOTS,
            asked: Asked::Everything,
            budget: Budget {
                deadline: None,
                successors: usize::MAX,
            },
        };
        let mut scratch = Scratch::default();
        states
            .iter()
            .map(|state| {
                let mut row = Vec::new();
                assert!(store.encode(state, &mut row), "the state is in the domain");
                let mut sink = Gathered {
                    store: &store,
                    next: Vec::new(),
                };
                let base = match given {
                    Given::RowAlone => Base {
                        values: &[],
                        row: Some(row.as_slice()),
                    },
                    Given::RowAndValues => Base {
                        values: state,
                        row: Some(row.as_slice()),
                    },
                    Given::ValuesAlone => Base {
                        values: state,
                        row: None,
                    },
                };
                let mark = expand.state(base, None, &mut sink, &mut scratch);
                Judged {
                    next: sink.next,
                    broken: mark.broken,
                    unjudged: mark.unjudged,
                    trouble: mark.trouble,
                    ended: mark.ended,
                    stuck: mark.stuck,
                    cut_short: mark.cut_short,
                    width: mark.width,
                }
            })
            .collect()
    }

    fn colours(symbols: &mut Symbols) -> Vec<Value> {
        ["Red", "Green", "Amber"]
            .iter()
            .map(|name| Value::Str(symbols.intern(name)))
            .collect()
    }

    const WIDE: i64 = 1 << 40;

    fn random_domain(seed: &mut Seed, colours: &[Value]) -> Domain {
        match seed.below(9) {
            0 => Domain::Bool,
            7 => Domain::Seq {
                item: Box::new(Domain::OneOf(colours.to_vec())),
                max: 2,
            },
            1 => Domain::OneOf(colours.to_vec()),
            2 => Domain::Int { lo: -5, hi: 5 },
            3 => Domain::Int {
                lo: WIDE,
                hi: WIDE + 9,
            },
            4 => Domain::Int {
                lo: i64::MAX - 4,
                hi: i64::MAX,
            },
            5 => Domain::Int { lo: 0, hi: 1 << 33 },
            _ => Domain::Int { lo: 0, hi: 3 },
        }
    }

    fn samples(domain: &Domain) -> Vec<Value> {
        match domain {
            Domain::Int { lo, hi } if hi.abs_diff(*lo) < 12 => {
                (*lo..=*hi).map(Value::Int).collect()
            }
            Domain::Int { lo, hi } => [*lo, *lo + 1, *hi - 1, *hi]
                .into_iter()
                .map(Value::Int)
                .collect(),
            other => other.values(),
        }
    }

    struct Shapes<'f> {
        fields: &'f [Field],
        seed: Seed,
        locals: Vec<usize>,
    }

    impl Shapes<'_> {
        fn of_kind(&mut self, wanted: fn(&Domain) -> bool) -> Option<usize> {
            let all: Vec<usize> = (0..self.fields.len())
                .filter(|at| wanted(&self.fields[*at].domain))
                .collect();
            (!all.is_empty()).then(|| all[self.seed.below(all.len())])
        }

        fn whole(&mut self) -> Option<usize> {
            self.of_kind(|domain| matches!(domain, Domain::Int { .. }))
        }

        fn constant(&mut self, field: usize) -> i64 {
            let Domain::Int { lo, hi } = self.fields[field].domain else {
                return 0;
            };
            let near = [
                lo,
                hi,
                lo.saturating_sub(1),
                hi.saturating_add(1),
                0,
                1,
                -1,
                1 << 31,
            ];
            near[self.seed.below(near.len())]
        }

        fn number(&mut self, depth: usize) -> Expr {
            let Some(field) = self.whole() else {
                return int(1);
            };
            if !self.locals.is_empty() && self.seed.below(4) == 0 {
                return Expr::Local(self.locals[self.seed.below(self.locals.len())]);
            }
            match (depth, self.seed.below(5)) {
                (0, _) | (_, 0 | 1) => Expr::Field(field),
                (_, 2) => int(self.constant(field)),
                (_, 3) => math(
                    [Math::Add, Math::Subtract, Math::Divide][self.seed.below(3)],
                    self.number(depth - 1),
                    self.number(depth - 1),
                ),
                _ => bounded(Op::Fit(IntType::I8), self.number(depth - 1)),
            }
        }

        fn test(&mut self, depth: usize) -> Expr {
            let choice = if depth == 0 {
                self.seed.below(4)
            } else {
                self.seed.below(9)
            };
            match choice {
                7 => match self.of_kind(|domain| matches!(domain, Domain::Seq { .. })) {
                    Some(field) => Expr::Apply(
                        Op::Order(Order::Less),
                        vec![
                            Expr::Apply(Op::Seq(SeqOp::Size), vec![Expr::Field(field)]),
                            int(i64::try_from(self.seed.below(3)).expect("a small number")),
                        ],
                    ),
                    None => yes(),
                },
                8 => match self.whole() {
                    Some(field) => Expr::Bind {
                        slot: BOUND,
                        value: Box::new(plus(Expr::Field(field), int(1))),
                        body: Box::new(Expr::Apply(
                            Op::Order(Order::Greater),
                            vec![Expr::Local(BOUND), int(self.constant(field))],
                        )),
                    },
                    None => yes(),
                },
                0 => match self.whole() {
                    Some(field) => Expr::Apply(
                        Op::Order(
                            [
                                Order::Less,
                                Order::LessOrEqual,
                                Order::Greater,
                                Order::GreaterOrEqual,
                            ][self.seed.below(4)],
                        ),
                        vec![Expr::Field(field), int(self.constant(field))],
                    ),
                    None => yes(),
                },
                1 => Expr::Apply(
                    [Op::Equal, Op::Differ][self.seed.below(2)],
                    vec![self.number(1), self.number(1)],
                ),
                2 => match self.of_kind(|domain| matches!(domain, Domain::Bool)) {
                    Some(field) => Expr::Field(field),
                    None => yes(),
                },
                3 => match self.of_kind(|domain| matches!(domain, Domain::OneOf(_))) {
                    Some(field) => {
                        let all = samples(&self.fields[field].domain);
                        let value = all[self.seed.below(all.len())].clone();
                        Expr::Apply(
                            [Op::Equal, Op::Differ][self.seed.below(2)],
                            vec![Expr::Field(field), Expr::Const(value)],
                        )
                    }
                    None => yes(),
                },
                4 => Expr::And(
                    Box::new(self.test(depth - 1)),
                    Box::new(self.test(depth - 1)),
                ),
                5 => Expr::Or(
                    Box::new(self.test(depth - 1)),
                    Box::new(self.test(depth - 1)),
                ),
                _ => Expr::Apply(Op::Not, vec![self.test(depth - 1)]),
            }
        }

        fn update(&mut self, into: usize) -> Expr {
            match &self.fields[into].domain {
                Domain::Bool => self.test(1),
                Domain::Seq { .. } => match self.seed.below(3) {
                    0 => Expr::Apply(Op::Seq(SeqOp::Shift), vec![Expr::Field(into)]),
                    1 => Expr::SeqOf(Vec::new()),
                    _ => {
                        let item = samples(&self.fields[into].domain)
                            .into_iter()
                            .find_map(|value| match value {
                                Value::Seq(items) => items.first().cloned(),
                                _ => None,
                            })
                            .unwrap_or(Value::Null);
                        Expr::Apply(
                            Op::Seq(SeqOp::Push),
                            vec![Expr::Field(into), Expr::Const(item)],
                        )
                    }
                },
                Domain::OneOf(values) => {
                    let values = values.clone();
                    if self.seed.below(2) == 0 {
                        Expr::Const(values[self.seed.below(values.len())].clone())
                    } else {
                        Expr::Choose(
                            Box::new(self.test(1)),
                            Box::new(Expr::Const(values[self.seed.below(values.len())].clone())),
                            Box::new(Expr::Const(values[self.seed.below(values.len())].clone())),
                        )
                    }
                }
                _ => match self.seed.below(4) {
                    0 => plus(Expr::Field(into), int(1)),
                    1 => math(Math::Subtract, Expr::Field(into), int(1)),
                    2 => int(self.constant(into)),
                    _ => self.number(2),
                },
            }
        }

        fn action(&mut self, at: usize) -> Action {
            let inputs = match self.seed.below(3) {
                0 => vec![input("a", FAST, numbers(&[0, 1, 2]))],
                1 => vec![
                    input("a", FAST, numbers(&[0, 1])),
                    input("b", SLOW, numbers(&[0, 1, 2])),
                ],
                _ => Vec::new(),
            };
            self.locals = inputs.iter().map(|each| each.slot).collect();
            let guard = self.test(2);
            let updates: Vec<Option<Expr>> = (0..self.fields.len())
                .map(|into| (self.seed.below(3) == 0).then(|| self.update(into)))
                .collect();
            self.locals.clear();
            let mut made = acting(inputs, guard, &updates);
            made.id = format!("act{at}");
            made
        }
    }

    fn random_model(seed: u64) -> (Model, Vec<Vec<Value>>) {
        let mut symbols = Symbols::default();
        let colours = colours(&mut symbols);
        let mut seed = Seed(seed);
        let count = 2 + seed.below(3);
        let fields: Vec<Field> = (0..count)
            .map(|at| {
                let domain = random_domain(&mut seed, &colours);
                let name = format!("f{at}");
                Field {
                    sym: symbols.intern(&name),
                    name,
                    starts: vec![samples(&domain)[0].clone()],
                    domain,
                }
            })
            .collect();
        let mut shapes = Shapes {
            fields: &fields,
            seed,
            locals: Vec::new(),
        };
        let actions = (0..3).map(|at| shapes.action(at)).collect();
        let rules = (0..2)
            .map(|at| Rule {
                id: format!("rule{at}"),
                says: "holds".to_owned(),
                line: 1,
                text: String::new(),
                holds: Code::new(shapes.test(2)),
                slots: SEARCH_SLOTS,
            })
            .collect();
        let ends = vec![End {
            id: "done".to_owned(),
            says: "done".to_owned(),
            when: Code::new(shapes.test(1)),
            slots: SEARCH_SLOTS,
        }];
        let mut seed = shapes.seed;
        let states = (0..60)
            .map(|_| {
                fields
                    .iter()
                    .map(|field| {
                        let all = samples(&field.domain);
                        all[seed.below(all.len())].clone()
                    })
                    .collect()
            })
            .collect();
        let mut model = model_of(actions);
        model.fields = fields;
        model.rules = rules;
        model.ends = ends;
        (model, states)
    }

    #[test]
    fn random_models_expand_from_rows_as_the_general_path_does() {
        let (mut taken, mut taken_with_inputs) = (0, 0);
        for seed in 1..=300 {
            let (model, states) = random_model(0x9E37_79B9_7F4A_7C15 ^ seed);
            let store = Store::new(&model.fields, 100, Lookup::Presence);
            for planned in Move::all(&model, store.layout(), &store.slots()) {
                taken += usize::from(planned.packed.is_some());
                taken_with_inputs += usize::from(planned.packed_inputs.is_some());
            }
            let expected = judged(&model, Path::General, Given::ValuesAlone, &states);
            for path in [Path::FromRows, Path::General] {
                for given in [Given::RowAlone, Given::RowAndValues, Given::ValuesAlone] {
                    assert_eq!(
                        judged(&model, path, given, &states),
                        expected,
                        "model {seed} by {path:?} with {given:?}: {:?}",
                        model.actions
                    );
                }
            }
        }
        assert!(
            taken > 200 && taken_with_inputs > 100,
            "many random actions are taken from rows: {taken} without inputs and {taken_with_inputs} with"
        );
    }

    fn ranged(order: Order, field: usize, number: i64) -> Expr {
        Expr::Apply(Op::Order(order), vec![Expr::Field(field), int(number)])
    }

    fn agrees_from_rows(case: &str, model: &Model, states: &[Vec<Value>]) {
        let expected = judged(model, Path::General, Given::ValuesAlone, states);
        for given in [Given::RowAlone, Given::RowAndValues] {
            assert_eq!(
                judged(model, Path::FromRows, given, states),
                expected,
                "{case} with {given:?}"
            );
        }
    }

    fn wheel_states() -> Vec<Vec<Value>> {
        (0..=99)
            .step_by(9)
            .flat_map(|x| [0, 1, 50, 98, 99].map(|y| numbers(&[x, y])))
            .collect()
    }

    #[test]
    fn a_guard_that_faults_a_rule_that_breaks_an_end_and_a_stuck_state_are_marked_as_the_general_path_marks_them()
     {
        let faulting = acting(
            Vec::new(),
            Expr::Apply(
                Op::Order(Order::Less),
                vec![math(Math::Divide, int(10), Expr::Field(1)), int(3)],
            ),
            &[Some(plus(Expr::Field(0), int(1))), None],
        );
        let up_to_the_top = acting(
            Vec::new(),
            ranged(Order::Less, 0, 99),
            &[Some(plus(Expr::Field(0), int(1))), Some(Expr::Field(0))],
        );
        let past_the_top = acting(
            Vec::new(),
            yes(),
            &[None, Some(plus(Expr::Field(1), int(1)))],
        );
        let below_the_bottom = acting(
            Vec::new(),
            ranged(Order::Greater, 1, 49),
            &[Some(math(Math::Subtract, Expr::Field(0), int(1))), None],
        );
        let beyond_32_bits = acting(
            Vec::new(),
            yes(),
            &[
                Some(math(
                    Math::Subtract,
                    plus(Expr::Field(0), int(1 << 40)),
                    int(1 << 40),
                )),
                None,
            ],
        );
        for (case, actions) in [
            ("a guard that faults where y is 0", vec![faulting]),
            ("x up to 99", vec![up_to_the_top.clone()]),
            ("y past 99", vec![past_the_top]),
            ("x below 0 when y is over 49", vec![below_the_bottom]),
            ("x through a number beyond 32 bits", vec![beyond_32_bits]),
        ] {
            let mut model = model_of(actions);
            model.rules = vec![Rule {
                id: "low".to_owned(),
                says: "x stays below 90".to_owned(),
                line: 1,
                text: String::new(),
                holds: Code::new(ranged(Order::Less, 0, 90)),
                slots: SEARCH_SLOTS,
            }];
            model.ends = vec![End {
                id: "top".to_owned(),
                says: "x is at the top".to_owned(),
                when: Code::new(Expr::Apply(Op::Equal, vec![Expr::Field(0), int(99)])),
                slots: SEARCH_SLOTS,
            }];
            agrees_from_rows(case, &model, &wheel_states());
        }
        let store = Store::new(&model_of(Vec::new()).fields, 100, Lookup::Presence);
        let model = model_of(vec![up_to_the_top]);
        assert!(
            Move::all(&model, store.layout(), &store.slots())[0]
                .packed
                .is_some(),
            "a guard and updates of these shapes are taken from rows"
        );
    }
}
