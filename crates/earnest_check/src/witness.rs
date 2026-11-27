use crate::explore::{Search, Starts};
use crate::outcome::{Change, Named, Path, RuleKind, RuleOutcome, Step, Verdict};
use crate::picks::{combinations_of, sets_for, sets_of};
use earnest_ir::{Action, Code, Falls, Fault, Field, Frame, Model, Pick, Plan, Script, Witness};
use earnest_value::Value;
use rustc_hash::FxHashSet;

const MOST_BRANCHES: usize = 100_000;
const FIRST_CANNOT_HAPPEN: &str =
    "the run's first action can't happen: it gives no state to start from";
const NONE_GETS_THROUGH: &str = "no value the run's `nondet`s pick gets the run through";

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

#[derive(Default)]
struct Ends {
    through: bool,
    fell: Option<(Path, String)>,
}

struct Course<'a> {
    skips: bool,
    last_false: bool,
    fields: &'a [Field],
    starts: Option<&'a [Vec<Value>]>,
    start: Option<&'a Code>,
    legs: Vec<Leg<'a>>,
    witness_expects: Option<&'a Code>,
    must_not_finish: bool,
    slots: usize,
}

impl<'a> Course<'a> {
    fn of_witness(witness: &'a Witness, model: &'a Model) -> Self {
        Self {
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
        }
    }

    fn of_plan(
        plan: &'a Plan,
        (script, model): (&'a Script, &'a Model),
        picked: (&[Pick], usize),
    ) -> Self {
        let (picks, number) = picked;
        let (skips, last_false) = match &plan.falls {
            Falls::Retried { last_step, .. } => (true, *last_step),
            _ => (false, false),
        };
        Self {
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
        }
    }
}

struct Combination<'p> {
    chosen: &'p [(String, Value)],
    retried: &'p [bool],
    ends: Ends,
}

fn judged(all: &[Combination<'_>], depth: usize) -> (bool, Option<usize>) {
    if all.iter().all(|each| each.chosen.len() <= depth) {
        let through = all.iter().any(|each| each.ends.through);
        return (
            through,
            all.iter().position(|each| each.ends.fell.is_some()),
        );
    }
    let retried = all
        .first()
        .and_then(|first| first.retried.get(depth))
        .copied()
        .unwrap_or(false);
    let (mut through, mut first_fell, mut every_fell) = (false, None, true);
    let mut done: usize = 0;
    let mut rest = all;
    while let Some(first) = rest.first() {
        let value = first.chosen.get(depth).map(|(_, value)| value);
        let same = rest
            .iter()
            .take_while(|each| each.chosen.get(depth).map(|(_, value)| value) == value)
            .count();
        let (group, after) = rest.split_at(same);
        let (can_get_through, fell) = judged(group, depth.saturating_add(1));
        through |= can_get_through;
        match fell {
            Some(position) => {
                first_fell.get_or_insert_with(|| done.saturating_add(position));
            }
            None => every_fell = false,
        }
        done = done.saturating_add(same);
        rest = after;
    }
    let fell = if retried && !every_fell {
        None
    } else {
        first_fell
    };
    (through, fell)
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

    fn none_happened(&mut self, number: usize, leg: &Leg<'_>, path: &Path) -> Result<(), Stop> {
        if leg.refused {
            self.ends.through = true;
            return Ok(());
        }
        let why = none_can_happen(number, leg);
        if !self.course.last_false || number < self.course.legs.len() {
            return Err(Stop::Broken(path.clone(), why));
        }
        self.fall(path, why)
    }

    fn finish(&mut self, state: &[Value], path: &Path) -> Result<(), Stop> {
        if let Some(expects) = self.course.witness_expects {
            self.check(expects, state, path, "what the witness expects")?;
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
        let mut happened = false;
        for choice in &leg.choices {
            let Some(action) = choice.action else {
                return Err(Stop::Broken(
                    path.clone(),
                    format!("step {number} is no action of the model"),
                ));
            };
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
                self.played = self.played.saturating_add(1);
                if self.played > MOST_BRANCHES {
                    return Err(Stop::TooManyBranches);
                }
                let after = match Search::take(action, &input, state) {
                    Taken::Moved(after) => after,
                    Taken::Refused => continue,
                    Taken::Faulted(part, fault) => {
                        let fault = fault.describe();
                        return Err(Stop::Broken(
                            path.clone(),
                            format!("step {number}: the {part} of the action `{id}` {fault} here"),
                        ));
                    }
                };
                happened = true;
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
        }
        if !happened {
            self.none_happened(number, leg, path)?;
        }
        self.held.insert(seen);
        Ok(())
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
        if !started && course.starts.is_some() {
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

    fn with_chosen(&self, chosen: &[(String, Value)], why: &str) -> String {
        if chosen.is_empty() {
            return why.to_owned();
        }
        let chosen: Vec<String> = chosen
            .iter()
            .map(|(name, value)| format!("`{name}` = {}", self.shown(value).text))
            .collect();
        format!("with {}: {why}", chosen.join(", "))
    }

    fn play_script(&self, number: usize, script: &Script) -> Ended {
        let picked = (self.model.runs.picks.as_slice(), number);
        let mut combinations = Vec::new();
        for plan in &script.plans {
            let mut ends = Ends::default();
            let course = Course::of_plan(plan, (script, self.model), picked);
            match self.play_course(&course, &mut ends) {
                Ended::Held => {}
                Ended::Broken(path, why) => {
                    return Ended::Broken(path, self.with_chosen(&plan.chosen, &why));
                }
                other => return other,
            }
            if let Falls::Retried { retried, .. } = &plan.falls {
                combinations.push(Combination {
                    chosen: &plan.chosen,
                    retried,
                    ends,
                });
            }
        }
        if combinations.is_empty() {
            return Ended::Held;
        }
        let (through, fell) = judged(&combinations, 0);
        let found = fell
            .and_then(|position| combinations.get_mut(position))
            .and_then(|found| Some((found.chosen, found.retried, found.ends.fell.take()?)));
        match found {
            None if through => Ended::Held,
            None => Ended::Broken(
                Path {
                    start: Vec::new(),
                    steps: Vec::new(),
                },
                NONE_GETS_THROUGH.to_owned(),
            ),
            Some((chosen, retried, (path, why))) => {
                let after = if retried.iter().all(|each| *each) {
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
