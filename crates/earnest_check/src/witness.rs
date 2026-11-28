use crate::explore::{Search, Starts};
use crate::outcome::{Change, Named, Path, RuleKind, RuleOutcome, Step, Verdict};
use crate::picks::{combinations_of, sets_for, sets_of};
use earnest_ir::{
    Action, Call, Code, Falls, Fault, Field, Frame, Model, Pick, Plan, RETRIED_BELOW, Script,
    Witness,
};
use earnest_value::Value;
use rustc_hash::FxHashSet;

const MOST_BRANCHES: usize = 100_000;
const FIRST_CANNOT_HAPPEN: &str =
    "the run's first action can't happen: it gives no state to start from";
const NONE_GETS_THROUGH: &str = "no value the run's `nondet`s pick gets the run through";
const NOT_RETRIED: &str =
    "and `quint test` tries no other value of a `nondet` over 100 values or more";

enum Ended {
    Held,
    Broken(Path, String),
    TooManyStarts,
    TooManyBranches,
}

enum Stop {
    Broken(Path, String),
    TooManyBranches,
}

enum Taken {
    Moved(Vec<Value>),
    Refused,
    Faulted(&'static str, Fault),
}

struct Alternative<'a> {
    action: Option<&'a Action>,
    input: Option<&'a [Value]>,
    sets: Option<Box<[Option<Code>]>>,
}

struct Leg<'a> {
    choices: Vec<Alternative<'a>>,
    asserts: Option<&'a Code>,
    expect: Option<&'a Code>,
    refused: bool,
}

struct Move<'a> {
    alternative: usize,
    order: Vec<usize>,
    action: &'a Action,
    input: Vec<Value>,
    after: Vec<Value>,
}

struct Fell {
    id: String,
    picked: Vec<(String, Value)>,
}

#[derive(Default)]
struct Ends {
    through: bool,
    fell: Option<(Path, String)>,
    called: Vec<(usize, Path, String)>,
    error: Option<(Path, String)>,
}

struct Course<'a> {
    start_falls: &'a [(String, Value)],
    skips: bool,
    last_false: bool,
    fields: &'a [Field],
    starts: Option<&'a [Vec<Value>]>,
    start: Option<&'a Code>,
    legs: Vec<Leg<'a>>,
    witness_expects: Option<&'a Code>,
    must_not_finish: bool,
    slots: usize,
    calls: &'a [Call],
    closing: Vec<Option<usize>>,
}

impl<'a> Course<'a> {
    fn of_witness(witness: &'a Witness, model: &'a Model) -> Self {
        Self {
            start_falls: &[],
            skips: false,
            last_false: false,
            fields: &model.fields,
            starts: None,
            start: None,
            legs: witness
                .steps
                .iter()
                .map(|played| Leg {
                    choices: vec![Alternative {
                        action: model.actions.get(played.action),
                        input: Some(&played.input),
                        sets: sets_of(model, played.action),
                    }],
                    asserts: None,
                    expect: None,
                    refused: false,
                })
                .collect(),
            witness_expects: Some(&witness.expect),
            must_not_finish: false,
            slots: witness.slots,
            calls: &[],
            closing: Vec::new(),
        }
    }

    fn of_plan(
        plan: &'a Plan,
        (script, model): (&'a Script, &'a Model),
        picked: (&[Pick], usize),
    ) -> Self {
        let (picks, number) = picked;
        let (skips, last_false, calls): (bool, bool, &[Call]) = match &plan.falls {
            Falls::Retried {
                last_step, calls, ..
            } => (true, *last_step, calls),
            _ => (false, false, &[]),
        };
        let mut closing = vec![None; plan.steps.len().saturating_add(1)];
        for (position, call) in calls.iter().enumerate() {
            if let Some(slot) = call.last.and_then(|last| closing.get_mut(last)) {
                *slot = Some(position);
            }
        }
        Self {
            start_falls: &plan.start_falls,
            skips,
            last_false,
            fields: if script.fields.is_empty() {
                &model.fields
            } else {
                &script.fields
            },
            starts: plan.starts.as_deref(),
            start: plan.start.as_ref(),
            legs: plan
                .steps
                .iter()
                .map(|stage| Leg {
                    choices: stage
                        .choices
                        .iter()
                        .map(|position| {
                            let action = script.actions.get(*position);
                            Alternative {
                                action,
                                input: None,
                                sets: action.and_then(|found| {
                                    sets_for(picks, Some(number), *position, found.inputs.len())
                                }),
                            }
                        })
                        .collect(),
                    asserts: stage.asserts.as_ref(),
                    expect: stage.expect.as_ref(),
                    refused: stage.refused,
                })
                .collect(),
            witness_expects: None,
            must_not_finish: plan.must_not_finish,
            slots: script.slots,
            calls,
            closing,
        }
    }
}

struct Combination<'p> {
    chosen: &'p [(String, Value)],
    retried: &'p [bool],
    within: &'p [Option<usize>],
    calls: &'p [Call],
    ordered: &'p [bool],
    ends: Ends,
}

type Record = (usize, Option<usize>);

#[derive(Default)]
struct Judged {
    through: bool,
    fell: Option<Record>,
    called: Vec<(usize, usize)>,
    error: Option<Record>,
}

fn at_leaf(all: &[Combination<'_>]) -> Judged {
    let mut judged = Judged {
        through: all.iter().any(|each| each.ends.through),
        ..Judged::default()
    };
    for (position, each) in all.iter().enumerate() {
        if each.ends.error.is_some() {
            judged.error = Some((position, None));
            return judged;
        }
        if judged.fell.is_none() && each.ends.fell.is_some() {
            judged.fell = Some((position, None));
        }
        for (call, ..) in &each.ends.called {
            let picks = each.within.contains(&Some(*call));
            let ends_the_run = each
                .calls
                .get(*call)
                .is_some_and(|found| found.ends_the_run);
            if ends_the_run && !picks {
                judged.fell.get_or_insert((position, Some(*call)));
            } else if !judged.called.iter().any(|(kept, _)| kept == call) {
                judged.called.push((*call, position));
            }
        }
    }
    judged
}

fn resolved(mut judged: Judged, call: usize, calls: &[Call]) -> Judged {
    let Some(found) = judged.called.iter().position(|(kept, _)| *kept == call) else {
        return judged;
    };
    let (_, position) = judged.called.remove(found);
    let record = (position, Some(call));
    if calls.get(call).is_some_and(|each| each.ends_the_run) {
        if judged.fell.is_none_or(|(kept, _)| position < kept) {
            judged.fell = Some(record);
        }
    } else {
        judged.error = Some(record);
    }
    judged
}

fn groups<'c, 'p>(
    all: &'c [Combination<'p>],
    depth: usize,
) -> impl Iterator<Item = (usize, &'c [Combination<'p>])> {
    let mut rest = all;
    let mut done = 0_usize;
    std::iter::from_fn(move || {
        let first = rest.first()?;
        let value = first.chosen.get(depth).map(|(_, value)| value);
        let same = rest
            .iter()
            .take_while(|each| each.chosen.get(depth).map(|(_, value)| value) == value)
            .count();
        let (group, after) = rest.split_at(same);
        let at = done;
        done = done.saturating_add(same);
        rest = after;
        Some((at, group))
    })
}

fn in_order(all: &[Combination<'_>], depth: usize) -> Judged {
    let mut found = Judged::default();
    for (done, group) in groups(all, depth) {
        let under = judged(group, depth.saturating_add(1));
        let moved = |(position, record): Record| (done.saturating_add(position), record);
        if let Some(error) = under.error {
            found.error = Some(moved(error));
            return found;
        }
        found.through |= under.through;
        for (call, position) in under.called {
            if !found.called.iter().any(|(kept, _)| *kept == call) {
                found.called.push((call, done.saturating_add(position)));
            }
        }
        found.fell = under.fell.map(moved);
        if found.fell.is_none() {
            break;
        }
    }
    found
}

fn judged(all: &[Combination<'_>], depth: usize) -> Judged {
    let Some(deep) = all.iter().find(|each| each.chosen.len() > depth) else {
        return at_leaf(all);
    };
    if deep.ordered.get(depth).copied().unwrap_or(false) {
        return in_order(all, depth);
    }
    let retried = deep.retried.get(depth).copied().unwrap_or(false);
    let scope = deep.within.get(depth).copied().flatten();
    let mut found = Judged::default();
    let (mut every_fell, mut done) = (true, 0_usize);
    let mut rest = all;
    while let Some(first) = rest.first() {
        let value = first.chosen.get(depth).map(|(_, value)| value);
        let same = rest
            .iter()
            .take_while(|each| each.chosen.get(depth).map(|(_, value)| value) == value)
            .count();
        let (group, after) = rest.split_at(same);
        let under = judged(group, depth.saturating_add(1));
        let moved = |(position, record): Record| (done.saturating_add(position), record);
        if let Some(error) = under.error {
            found.error = Some(moved(error));
            return found;
        }
        found.through |= under.through;
        if let Some(fell) = under.fell {
            found.fell.get_or_insert(moved(fell));
        }
        let fell_here = match scope {
            None => under.fell.is_some(),
            Some(call) => under.called.iter().any(|(kept, _)| *kept == call),
        };
        every_fell &= fell_here;
        for (call, position) in under.called {
            if !found.called.iter().any(|(kept, _)| *kept == call) {
                found.called.push((call, done.saturating_add(position)));
            }
        }
        done = done.saturating_add(same);
        rest = after;
    }
    if retried && !every_fell {
        match scope {
            None => found.fell = None,
            Some(call) => found.called.retain(|(kept, _)| *kept != call),
        }
    }
    match scope {
        Some(call)
            if !deep
                .within
                .iter()
                .take(depth)
                .any(|each| *each == Some(call)) =>
        {
            resolved(found, call, deep.calls)
        }
        _ => found,
    }
}

fn every_input(action: &Action, given: Option<&[Value]>) -> Option<Vec<Vec<Value>>> {
    if let Some(given) = given {
        return Some(vec![given.to_vec()]);
    }
    let count = action.inputs.iter().try_fold(1_usize, |count, input| {
        count.checked_mul(input.values.len())
    })?;
    if count > MOST_BRANCHES {
        return None;
    }
    let mut all: Vec<Vec<Value>> = vec![Vec::with_capacity(action.inputs.len())];
    for input in &action.inputs {
        all = all
            .iter()
            .flat_map(|picked| {
                input.values.iter().map(|value| {
                    let mut longer = picked.clone();
                    longer.push(value.clone());
                    longer
                })
            })
            .collect();
    }
    Some(all)
}

fn none_can_happen(number: usize, leg: &Leg<'_>) -> String {
    let mut ids: Vec<&str> = Vec::with_capacity(leg.choices.len());
    for action in leg.choices.iter().filter_map(|choice| choice.action) {
        if !ids.contains(&action.id.as_str()) {
            ids.push(&action.id);
        }
    }
    if let [id] = ids.as_slice() {
        return format!("step {number}, `{id}`, can't happen here");
    }
    let listed: Vec<String> = ids.iter().map(|id| format!("`{id}`")).collect();
    format!(
        "step {number}: none of {} can happen here",
        listed.join(", ")
    )
}

struct Player<'s, 'a> {
    search: &'s Search<'s>,
    course: &'s Course<'a>,
    played: usize,
    held: FxHashSet<(usize, Vec<Value>)>,
    ends: Ends,
}

impl Player<'_, '_> {
    fn check(
        &self,
        condition: &Code,
        state: &[Value],
        path: &Path,
        what: &str,
    ) -> Result<(), Stop> {
        match condition.truth(&mut Frame::new(state, self.course.slots)) {
            Ok(true) => Ok(()),
            Ok(false) => Err(Stop::Broken(path.clone(), format!("{what} is false here"))),
            Err(fault) => Err(Stop::Broken(
                path.clone(),
                format!("{what} {} here", fault.describe()),
            )),
        }
    }

    fn fall(&mut self, path: &Path, why: String) -> Result<(), Stop> {
        if !self.course.skips {
            return Err(Stop::Broken(path.clone(), why));
        }
        if self.ends.fell.is_none() {
            self.ends.fell = Some((path.clone(), why));
        }
        Ok(())
    }

    fn closes(&self, done: usize) -> Option<usize> {
        self.course.closing.get(done).copied().flatten()
    }

    fn fall_in(&mut self, call: usize, path: &Path, why: String) -> Result<(), Stop> {
        if !self.course.skips {
            return Err(Stop::Broken(path.clone(), why));
        }
        if !self.ends.called.iter().any(|(kept, ..)| *kept == call) {
            self.ends.called.push((call, path.clone(), why));
        }
        Ok(())
    }

    fn none_happened(&mut self, number: usize, leg: &Leg<'_>, path: &Path) -> Result<(), Stop> {
        if leg.refused {
            self.ends.through = true;
            return Ok(());
        }
        let why = none_can_happen(number, leg);
        if let Some(call) = self.closes(number.saturating_sub(1)) {
            return self.fall_in(call, path, why);
        }
        if !self.course.last_false || number < self.course.legs.len() {
            return Err(Stop::Broken(path.clone(), why));
        }
        self.fall(path, why)
    }

    fn finish(&mut self, state: &[Value], path: &Path) -> Result<(), Stop> {
        if let Some(expects) = self.course.witness_expects {
            self.check(expects, state, path, "what the witness expects")?;
        }
        if let Some(call) = self.closes(self.course.legs.len()) {
            let reason = self
                .course
                .calls
                .get(call)
                .map_or("the run is false here", |found| found.reason.as_str());
            return self.fall_in(call, path, reason.to_owned());
        }
        if !self.course.must_not_finish {
            self.ends.through = true;
            return Ok(());
        }
        let why = if self.course.legs.is_empty() {
            "the run is false"
        } else {
            "the run gets through every step, and its `fail()` expects it not to"
        };
        self.fall(path, why.to_owned())
    }

    fn outside(&self, after: &[Value]) -> Option<&str> {
        self.course
            .fields
            .iter()
            .zip(after)
            .find(|(field, value)| !field.domain.contains(value))
            .map(|(field, _)| field.name.as_str())
    }

    fn taken(&self, chosen: (&Action, &[Value]), before: &[Value], after: &[Value]) -> Step {
        self.search
            .step_taken(chosen, self.course.fields, before, after)
    }

    fn walk(&mut self, done: usize, state: &[Value], path: &mut Path) -> Result<(), Stop> {
        let course = self.course;
        let Some(leg) = course.legs.get(done) else {
            return self.finish(state, path);
        };
        let seen = (done, state.to_vec());
        if self.held.contains(&seen) {
            return Ok(());
        }
        let number = done.saturating_add(1);
        if let Some(asserts) = leg.asserts {
            self.check(
                asserts,
                state,
                path,
                &format!("the assertion of step {number}"),
            )?;
        }
        let (mut moves, fell) = if course.witness_expects.is_none() {
            let members: Vec<usize> = (0..leg.choices.len()).collect();
            let mut moves = Vec::new();
            let mut bound = Vec::new();
            let fell = self.sampled(
                (leg, number, state, &*path),
                &members,
                (&mut bound, &mut Vec::new()),
                &mut moves,
            )?;
            moves.sort_by(|one, other| {
                (one.alternative, &one.order).cmp(&(other.alternative, &other.order))
            });
            (moves, fell)
        } else {
            (self.listed((leg, number, state, &*path))?, None)
        };
        if moves.is_empty() {
            self.none_happened(number, leg, path)?;
            self.held.insert(seen);
            return Ok(());
        }
        if let Some(fell) = fell.filter(|_| !leg.refused) {
            self.sampled_fall(number, &fell, path)?;
        }
        for Move {
            action,
            input,
            after,
            ..
        } in moves.drain(..)
        {
            let id = &action.id;
            path.steps.push(self.taken((action, &input), state, &after));
            if leg.refused {
                self.fall(
                    path,
                    format!(
                        "step {number}, `{id}`, can happen here, and the run's `fail()` \
                         expects it can't"
                    ),
                )?;
                path.steps.pop();
                continue;
            }
            if let Some(name) = self.outside(&after) {
                return Err(Stop::Broken(
                    path.clone(),
                    format!(
                        "step {number}, `{id}`, takes the field `{name}` out of the values it declares"
                    ),
                ));
            }
            if let Some(expect) = leg.expect {
                self.check(
                    expect,
                    &after,
                    path,
                    &format!("what the run expects after step {number}"),
                )?;
            }
            self.walk(done.saturating_add(1), &after, path)?;
            path.steps.pop();
        }
        self.held.insert(seen);
        Ok(())
    }

    fn sampled_fall(&mut self, number: usize, fell: &Fell, path: &Path) -> Result<(), Stop> {
        let picked = self.search.picked(&fell.picked);
        let why = format!(
            "with {picked}, step {number}, `{}`, can't happen here, {NOT_RETRIED}",
            fell.id
        );
        if let Some(call) = self.closes(number.saturating_sub(1)) {
            return self.fall_in(call, path, why);
        }
        if !self.course.last_false || number < self.course.legs.len() {
            return Err(Stop::Broken(path.clone(), why));
        }
        self.fall(path, why)
    }

    fn counted(&mut self) -> Result<(), Stop> {
        self.played = self.played.saturating_add(1);
        if self.played > MOST_BRANCHES {
            return Err(Stop::TooManyBranches);
        }
        Ok(())
    }

    fn moved(
        &mut self,
        (action, input): (&Action, &[Value]),
        (number, state, path): (usize, &[Value], &Path),
    ) -> Result<Option<Vec<Value>>, Stop> {
        self.counted()?;
        match Search::take(action, input, state) {
            Taken::Moved(after) => Ok(Some(after)),
            Taken::Refused => Ok(None),
            Taken::Faulted(part, fault) => {
                let (id, fault) = (&action.id, fault.describe());
                Err(Stop::Broken(
                    path.clone(),
                    format!("step {number}: the {part} of the action `{id}` {fault} here"),
                ))
            }
        }
    }

    fn listed<'l>(
        &mut self,
        (leg, number, state, path): (&Leg<'l>, usize, &[Value], &Path),
    ) -> Result<Vec<Move<'l>>, Stop> {
        let mut moves = Vec::new();
        for (alternative, choice) in leg.choices.iter().enumerate() {
            let action = Self::action_of(choice, number, path)?;
            let id = &action.id;
            let inputs = match &choice.sets {
                Some(sets) => match combinations_of(action, sets, state) {
                    (_, Some(fault)) => {
                        let fault = fault.describe();
                        return Err(Stop::Broken(
                            path.clone(),
                            format!(
                                "step {number}: the set the action `{id}` picks from {fault} here"
                            ),
                        ));
                    }
                    (all, None) => match choice.input {
                        Some(given) => all.into_iter().filter(|each| each == given).collect(),
                        None => all,
                    },
                },
                None => every_input(action, choice.input).ok_or(Stop::TooManyBranches)?,
            };
            for input in inputs {
                if let Some(after) = self.moved((action, &input), (number, state, path))? {
                    moves.push(Move {
                        alternative,
                        order: Vec::new(),
                        action,
                        input,
                        after,
                    });
                }
            }
        }
        Ok(moves)
    }

    fn action_of<'l>(
        choice: &Alternative<'l>,
        number: usize,
        path: &Path,
    ) -> Result<&'l Action, Stop> {
        choice.action.ok_or_else(|| {
            Stop::Broken(
                path.clone(),
                format!("step {number} is no action of the model"),
            )
        })
    }

    fn values_of(
        (choice, action): (&Alternative<'_>, &Action),
        depth: usize,
        (number, state, path): (usize, &[Value], &Path),
        bound: &[Value],
    ) -> Result<Vec<Value>, Stop> {
        let set = choice
            .sets
            .as_ref()
            .and_then(|sets| sets.get(depth))
            .and_then(Option::as_ref);
        let Some(set) = set else {
            return Ok(action
                .inputs
                .get(depth)
                .map_or_else(Vec::new, |input| input.values.clone()));
        };
        let mut frame = Frame::new(state, action.slots);
        for (input, value) in action.inputs.iter().zip(bound) {
            frame.put(input.slot, value.clone());
        }
        let fault = match set.eval(&mut frame) {
            Ok(Value::Set(items)) => return Ok(items.iter().cloned().collect()),
            Ok(_) => Fault::WrongKind,
            Err(fault) => fault,
        };
        let (id, fault) = (&action.id, fault.describe());
        Err(Stop::Broken(
            path.clone(),
            format!("step {number}: the set the action `{id}` picks from {fault} here"),
        ))
    }

    fn sampled<'l>(
        &mut self,
        (leg, number, state, path): (&Leg<'l>, usize, &[Value], &Path),
        members: &[usize],
        (bound, order): (&mut Vec<Value>, &mut Vec<usize>),
        moves: &mut Vec<Move<'l>>,
    ) -> Result<Option<Fell>, Stop> {
        let depth = bound.len();
        let (mut fell, mut every) = (None, true);
        let mut groups: Vec<(usize, Vec<Value>, Vec<usize>)> = Vec::new();
        for member in members {
            let Some(choice) = leg.choices.get(*member) else {
                continue;
            };
            let action = Self::action_of(choice, number, path)?;
            let Some(input) = action.inputs.get(depth) else {
                match self.moved((action, bound), (number, state, path))? {
                    Some(after) => {
                        every = false;
                        moves.push(Move {
                            alternative: *member,
                            order: order.clone(),
                            action,
                            input: bound.clone(),
                            after,
                        });
                    }
                    None => {
                        fell.get_or_insert_with(|| Fell {
                            id: action.id.clone(),
                            picked: Self::named(action, bound),
                        });
                    }
                }
                continue;
            };
            let values = Self::values_of((choice, action), depth, (number, state, path), bound)?;
            match groups
                .iter_mut()
                .find(|(slot, listed, _)| *slot == input.slot && *listed == values)
            {
                Some((.., found)) => found.push(*member),
                None => groups.push((input.slot, values, vec![*member])),
            }
        }
        for (_, values, group) in groups {
            let first_action = group
                .first()
                .and_then(|member| leg.choices.get(*member))
                .and_then(|choice| choice.action);
            let retried = values.len() < RETRIED_BELOW;
            let mut first = None;
            let mut all_fell = true;
            if values.is_empty() {
                first = first_action.map(|action| Fell {
                    id: action.id.clone(),
                    picked: Self::named(action, bound),
                });
            }
            for (position, value) in values.into_iter().enumerate() {
                bound.push(value);
                order.push(position);
                let under = self.sampled(
                    (leg, number, state, path),
                    &group,
                    (&mut *bound, &mut *order),
                    moves,
                );
                bound.pop();
                order.pop();
                match under? {
                    Some(found) => {
                        first.get_or_insert(found);
                    }
                    None => all_fell = false,
                }
            }
            let falls = first.is_some() && (all_fell || !retried);
            match first {
                Some(found) if falls => {
                    if fell
                        .as_ref()
                        .is_none_or(|kept: &Fell| kept.picked.len() < found.picked.len())
                    {
                        fell = Some(found);
                    }
                }
                _ => every = false,
            }
        }
        Ok(if every { fell } else { None })
    }

    fn named(action: &Action, bound: &[Value]) -> Vec<(String, Value)> {
        action
            .inputs
            .iter()
            .zip(bound)
            .map(|(input, value)| (input.name.clone(), value.clone()))
            .collect()
    }
}

impl Search<'_> {
    fn take(action: &Action, input: &[Value], state: &[Value]) -> Taken {
        let mut frame = Frame::new(state, action.slots);
        for (each, value) in action.inputs.iter().zip(input) {
            frame.put(each.slot, value.clone());
        }
        match action.guard.truth(&mut frame) {
            Ok(true) => {}
            Ok(false) => return Taken::Refused,
            Err(fault) => return Taken::Faulted("condition", fault),
        }
        let mut next = state.to_vec();
        for (field, update) in next.iter_mut().zip(&action.updates) {
            let Some(code) = update else { continue };
            match code.eval(&mut frame) {
                Ok(value) => *field = value,
                Err(fault) => return Taken::Faulted("change", fault),
            }
        }
        Taken::Moved(next)
    }

    fn named_over(&self, fields: &[Field], state: &[Value]) -> Vec<Named> {
        fields
            .iter()
            .zip(state)
            .map(|(field, value)| Named {
                name: field.name.clone(),
                value: self.shown(value),
            })
            .collect()
    }

    fn step_taken(
        &self,
        (action, input): (&Action, &[Value]),
        fields: &[Field],
        before: &[Value],
        after: &[Value],
    ) -> Step {
        Step {
            action: action.id.clone(),
            label: action.label.clone(),
            input: action
                .inputs
                .iter()
                .zip(input)
                .map(|(each, value)| Named {
                    name: each.name.clone(),
                    value: self.shown(value),
                })
                .collect(),
            changes: fields
                .iter()
                .zip(before.iter().zip(after))
                .filter(|(_, (old, new))| old != new)
                .map(|(field, (old, new))| Change {
                    field: field.name.clone(),
                    before: self.shown(old),
                    after: self.shown(new),
                })
                .collect(),
            state: self.named_over(fields, after),
        }
    }

    fn play_course(&self, course: &Course<'_>, ends: &mut Ends) -> Ended {
        let most = self.limits.max_states;
        let mut held = FxHashSet::default();
        let starts: Box<dyn Iterator<Item = Vec<Value>>> = match course.starts {
            Some(listed) => Box::new(listed.iter().cloned()),
            None => Box::new(Starts::of(self.model)),
        };
        if !course.start_falls.is_empty() && !course.must_not_finish {
            let path = Path {
                start: Vec::new(),
                steps: Vec::new(),
            };
            let why = format!(
                "with {}, the run's first action can't happen, {NOT_RETRIED}",
                self.picked(course.start_falls)
            );
            if !(course.skips && course.last_false && course.legs.is_empty()) {
                return Ended::Broken(path, why);
            }
            ends.fell.get_or_insert((path, why));
        }
        let mut started = false;
        for (played, start) in starts.enumerate() {
            if played >= most {
                return Ended::TooManyStarts;
            }
            let mut path = Path {
                start: self.named_over(course.fields, &start),
                steps: Vec::with_capacity(course.legs.len()),
            };
            let mut player = Player {
                search: self,
                course,
                played: 0,
                held,
                ends: std::mem::take(ends),
            };
            let walked = match course.start {
                Some(start_holds) => player
                    .check(
                        start_holds,
                        &start,
                        &path,
                        "what the run expects at the start",
                    )
                    .and_then(|()| player.walk(0, &start, &mut path)),
                None => player.walk(0, &start, &mut path),
            };
            match walked {
                Ok(()) => {}
                Err(Stop::Broken(path, why)) => return Ended::Broken(path, why),
                Err(Stop::TooManyBranches) => return Ended::TooManyBranches,
            }
            held = player.held;
            *ends = player.ends;
            started = true;
        }
        if !started {
            let path = Path {
                start: Vec::new(),
                steps: Vec::new(),
            };
            if course.must_not_finish && course.legs.is_empty() {
                ends.through = true;
            } else if course.skips && course.last_false && course.legs.is_empty() {
                ends.fell
                    .get_or_insert_with(|| (path, FIRST_CANNOT_HAPPEN.to_owned()));
            } else {
                return Ended::Broken(path, FIRST_CANNOT_HAPPEN.to_owned());
            }
        }
        Ended::Held
    }

    fn picked(&self, chosen: &[(String, Value)]) -> String {
        let chosen: Vec<String> = chosen
            .iter()
            .map(|(name, value)| format!("`{name}` = {}", self.shown(value).text))
            .collect();
        chosen.join(", ")
    }

    fn with_chosen(&self, chosen: &[(String, Value)], why: &str) -> String {
        if chosen.is_empty() {
            return why.to_owned();
        }
        format!("with {}: {why}", self.picked(chosen))
    }

    fn play_script(&self, number: usize, script: &Script) -> Ended {
        let picked = (self.model.runs.picks.as_slice(), number);
        let mut combinations = Vec::new();
        for plan in &script.plans {
            let mut ends = Ends::default();
            let course = Course::of_plan(plan, (script, self.model), picked);
            let played = self.play_course(&course, &mut ends);
            let Falls::Retried {
                retried,
                within,
                calls,
                ordered,
                ..
            } = &plan.falls
            else {
                match played {
                    Ended::Held => continue,
                    Ended::Broken(path, why) => {
                        return Ended::Broken(path, self.with_chosen(&plan.chosen, &why));
                    }
                    other => return other,
                }
            };
            let later_part = ordered
                .iter()
                .zip(&plan.chosen)
                .any(|(part, (_, value))| *part && *value != Value::Int(1));
            match played {
                Ended::Held => {}
                Ended::Broken(path, why) if later_part => ends.error = Some((path, why)),
                Ended::Broken(path, why) => {
                    return Ended::Broken(path, self.with_chosen(&plan.chosen, &why));
                }
                other => return other,
            }
            combinations.push(Combination {
                chosen: &plan.chosen,
                retried,
                within,
                calls,
                ordered,
                ends,
            });
        }
        if combinations.is_empty() {
            return Ended::Held;
        }
        let mut judged = judged(&combinations, 0);
        while judged.error.is_none()
            && let Some((call, _)) = judged.called.first().copied()
        {
            let calls = combinations.first().map_or(&[][..], |first| first.calls);
            judged = resolved(judged, call, calls);
        }
        match judged.error {
            Some((position, Some(call))) => {
                return Self::called_error(&mut combinations, (position, call), |chosen, why| {
                    self.with_chosen(chosen, why)
                });
            }
            Some((position, None)) => {
                if let Some(found) = combinations.get_mut(position)
                    && let Some((path, why)) = found.ends.error.take()
                {
                    return Ended::Broken(path, self.with_chosen(found.chosen, &why));
                }
            }
            None => {}
        }
        let found = judged.fell.and_then(|(position, record)| {
            let found = combinations.get_mut(position)?;
            let ends = &mut found.ends;
            let taken = match record {
                None => ends.fell.take(),
                Some(call) => {
                    let at = ends.called.iter().position(|(kept, ..)| *kept == call)?;
                    let (_, path, why) = ends.called.remove(at);
                    Some((path, why))
                }
            };
            Some((
                found.chosen,
                found.retried,
                found.within,
                found.ordered,
                taken?,
            ))
        });
        match found {
            None if judged.through => Ended::Held,
            None => Ended::Broken(
                Path {
                    start: Vec::new(),
                    steps: Vec::new(),
                },
                NONE_GETS_THROUGH.to_owned(),
            ),
            Some((chosen, retried, within, ordered, (path, why))) => {
                let mut own = retried
                    .iter()
                    .zip(within.iter().chain(std::iter::repeat(&None)))
                    .zip(ordered.iter().chain(std::iter::repeat(&false)))
                    .filter(|((_, scope), part)| scope.is_none() && !**part);
                let after = if own.all(|((each, _), _)| *each) {
                    "; the run is false for every value its `nondet`s pick, or can be, so \
                     `quint test` fails it"
                } else {
                    "; the run is false here, and `quint test` tries no other value of a \
                     `nondet` over 100 values or more"
                };
                Ended::Broken(path, self.with_chosen(chosen, &format!("{why}{after}")))
            }
        }
    }

    fn called_error(
        combinations: &mut [Combination<'_>],
        (position, call): (usize, usize),
        with_chosen: impl Fn(&[(String, Value)], &str) -> String,
    ) -> Ended {
        let unplayed = || {
            Ended::Broken(
                Path {
                    start: Vec::new(),
                    steps: Vec::new(),
                },
                "a run it calls is false, and something comes after it".to_owned(),
            )
        };
        let Some(found) = combinations.get_mut(position) else {
            return unplayed();
        };
        let says = found
            .calls
            .get(call)
            .map_or("the run it calls", |each| each.says.as_str());
        let Some(at) = found
            .ends
            .called
            .iter()
            .position(|(kept, ..)| *kept == call)
        else {
            return unplayed();
        };
        let (_, path, why) = found.ends.called.remove(at);
        if !found.within.contains(&Some(call)) {
            return Ended::Broken(path, with_chosen(found.chosen, &why));
        }
        let retried = found
            .retried
            .iter()
            .zip(found.within)
            .filter(|(_, scope)| **scope == Some(call))
            .all(|(each, _)| *each);
        let after = if retried {
            format!(
                "; {says} is false for every value its `nondet`s pick, or can be, and something \
                 comes after it, so `quint test` stops the run there with an error"
            )
        } else {
            format!(
                "; {says} is false here, `quint test` tries no other value of a `nondet` over 100 \
                 values or more, and something comes after it, so it stops the run there with an \
                 error"
            )
        };
        Ended::Broken(path, with_chosen(found.chosen, &format!("{why}{after}")))
    }

    fn played_outcome(id: &str, says: &str, ended: Ended) -> RuleOutcome {
        let (verdict, path, reason) = match ended {
            Ended::Held => (Verdict::Holds, None, None),
            Ended::Broken(path, why) => (Verdict::Broken, Some(path), Some(why)),
            Ended::TooManyStarts => (Verdict::Unknown, None, None),
            Ended::TooManyBranches => (
                Verdict::Unknown,
                None,
                Some(format!(
                    "the run has more than {MOST_BRANCHES} branches from one start state, and \
                     the checker stopped after playing that many"
                )),
            ),
        };
        RuleOutcome {
            id: id.to_owned(),
            says: says.to_owned(),
            verdict,
            path,
            kind: RuleKind::Witness,
            reason,
            repeats_from: None,
        }
    }

    pub(crate) fn unplayed(rules: &[RuleOutcome]) -> Option<String> {
        rules
            .iter()
            .find(|rule| rule.kind == RuleKind::Witness && rule.verdict == Verdict::Unknown)
            .map(|rule| match &rule.reason {
                Some(_) => format!(
                    "the run `{}` has more than {MOST_BRANCHES} branches from one start state",
                    rule.id
                ),
                None => format!(
                    "the witness `{}` has more start states than the state limit",
                    rule.id
                ),
            })
    }

    pub(crate) fn witnesses(&self) -> Vec<RuleOutcome> {
        let only_rule = self.limits.only_rule.as_deref();
        let wanted = |id: &str| only_rule.is_none_or(|wanted| wanted == id);
        let runs = &self.model.runs;
        let witnesses = runs
            .witnesses
            .iter()
            .filter(|witness| witness.setup == runs.chosen)
            .filter(|witness| wanted(&witness.id))
            .map(|witness| {
                let course = Course::of_witness(witness, self.model);
                let ended = self.play_course(&course, &mut Ends::default());
                Self::played_outcome(&witness.id, &witness.says, ended)
            });
        let scripts = runs
            .scripts
            .iter()
            .enumerate()
            .filter(|(_, script)| wanted(&script.id))
            .map(|(number, script)| {
                let ended = self.play_script(number, script);
                Self::played_outcome(&script.id, &script.says, ended)
            });
        witnesses.chain(scripts).collect()
    }
}
