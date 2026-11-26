use crate::explore::{Search, Starts};
use crate::outcome::{Change, Named, Path, RuleKind, RuleOutcome, Step, Verdict};
use earnest_ir::{Action, Code, Fault, Frame, Plan, Script, Witness};
use earnest_value::Value;
use rustc_hash::FxHashSet;

const MOST_BRANCHES: usize = 100_000;

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
}

struct Leg<'a> {
    choices: Vec<Alternative<'a>>,
    asserts: Option<&'a Code>,
    expect: Option<&'a Code>,
    refused: bool,
}

struct Course<'a> {
    start: Option<&'a Code>,
    legs: Vec<Leg<'a>>,
    witness_expects: Option<&'a Code>,
    must_not_finish: bool,
    slots: usize,
}

impl<'a> Course<'a> {
    fn of_witness(witness: &'a Witness, actions: &'a [Action]) -> Self {
        Self {
            start: None,
            legs: witness
                .steps
                .iter()
                .map(|played| Leg {
                    choices: vec![Alternative {
                        action: actions.get(played.action),
                        input: Some(&played.input),
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

    fn of_plan(plan: &'a Plan, script: &'a Script) -> Self {
        Self {
            start: plan.start.as_ref(),
            legs: plan
                .steps
                .iter()
                .map(|stage| Leg {
                    choices: stage
                        .choices
                        .iter()
                        .map(|position| Alternative {
                            action: script.actions.get(*position),
                            input: None,
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

    fn finish(&self, state: &[Value], path: &Path) -> Result<(), Stop> {
        if let Some(expects) = self.course.witness_expects {
            self.check(expects, state, path, "what the witness expects")?;
        }
        if !self.course.must_not_finish {
            return Ok(());
        }
        let why = if self.course.legs.is_empty() {
            "the run is false"
        } else {
            "the run gets through every step, and its `fail()` expects it not to"
        };
        Err(Stop::Broken(path.clone(), why.to_owned()))
    }

    fn outside(&self, after: &[Value]) -> Option<&str> {
        self.search
            .model
            .fields
            .iter()
            .zip(after)
            .find(|(field, value)| !field.domain.contains(value))
            .map(|(field, _)| field.name.as_str())
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
            let inputs = every_input(action, choice.input).ok_or(Stop::TooManyBranches)?;
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
                path.steps
                    .push(self.search.step_taken(action, &input, state, &after));
                if leg.refused {
                    return Err(Stop::Broken(
                        path.clone(),
                        format!(
                            "step {number}, `{id}`, can happen here, and the run's `fail()` \
                             expects it can't"
                        ),
                    ));
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
        if !happened && !leg.refused {
            return Err(Stop::Broken(path.clone(), none_can_happen(number, leg)));
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

    fn step_taken(
        &self,
        action: &Action,
        input: &[Value],
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
            changes: self
                .model
                .fields
                .iter()
                .zip(before.iter().zip(after))
                .filter(|(_, (old, new))| old != new)
                .map(|(field, (old, new))| Change {
                    field: field.name.clone(),
                    before: self.shown(old),
                    after: self.shown(new),
                })
                .collect(),
            state: self.named(after),
        }
    }

    fn play_course(&self, course: &Course<'_>) -> Ended {
        let most = self.limits.max_states;
        let mut held = FxHashSet::default();
        for (played, start) in Starts::of(self.model).enumerate() {
            if played >= most {
                return Ended::TooManyStarts;
            }
            let mut path = Path {
                start: self.named(&start),
                steps: Vec::with_capacity(course.legs.len()),
            };
            let mut player = Player {
                search: self,
                course,
                played: 0,
                held,
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
        }
        Ended::Held
    }

    fn play_script(&self, script: &Script) -> Ended {
        for plan in &script.plans {
            match self.play_course(&Course::of_plan(plan, script)) {
                Ended::Held => {}
                Ended::Broken(path, why) if !plan.chosen.is_empty() => {
                    let chosen: Vec<String> = plan
                        .chosen
                        .iter()
                        .map(|(name, value)| format!("`{name}` = {}", self.shown(value).text))
                        .collect();
                    return Ended::Broken(path, format!("with {}: {why}", chosen.join(", ")));
                }
                other => return other,
            }
        }
        Ended::Held
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
                let course = Course::of_witness(witness, &self.model.actions);
                Self::played_outcome(&witness.id, &witness.says, self.play_course(&course))
            });
        let scripts = runs
            .scripts
            .iter()
            .filter(|script| wanted(&script.id))
            .map(|script| Self::played_outcome(&script.id, &script.says, self.play_script(script)));
        witnesses.chain(scripts).collect()
    }
}
