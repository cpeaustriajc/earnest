use crate::explore::{Search, Starts};
use crate::outcome::{Change, Named, Path, RuleKind, RuleOutcome, Step, Verdict};
use earnest_ir::{Action, Fault, Frame, Played, Witness};
use earnest_value::Value;

enum Ended {
    Held,
    Broken(Path, String),
    TooManyStarts,
}

enum Taken {
    Moved(Vec<Value>),
    Refused,
    Faulted(&'static str, Fault),
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

    fn play_from(&self, witness: &Witness, start: Vec<Value>) -> Result<(), (Path, String)> {
        let mut state = start;
        let mut path = Path {
            start: self.named(&state),
            steps: Vec::with_capacity(witness.steps.len()),
        };
        for (number, Played { action, input }) in (1_usize..).zip(&witness.steps) {
            let Some(action) = self.model.actions.get(*action) else {
                return Err((path, format!("step {number} is no action of the model")));
            };
            let id = &action.id;
            let after = match Self::take(action, input, &state) {
                Taken::Moved(after) => after,
                Taken::Refused => {
                    return Err((path, format!("step {number}, `{id}`, can't happen here")));
                }
                Taken::Faulted(part, fault) => {
                    let fault = fault.describe();
                    return Err((
                        path,
                        format!("step {number}: the {part} of the action `{id}` {fault} here"),
                    ));
                }
            };
            path.steps
                .push(self.step_taken(action, input, &state, &after));
            let outside = self
                .model
                .fields
                .iter()
                .zip(&after)
                .find(|(field, value)| !field.domain.contains(value));
            if let Some((field, _)) = outside {
                let name = &field.name;
                return Err((
                    path,
                    format!(
                        "step {number}, `{id}`, takes the field `{name}` out of the values it declares"
                    ),
                ));
            }
            state = after;
        }
        match witness.expect.truth(&mut Frame::new(&state, witness.slots)) {
            Ok(true) => Ok(()),
            Ok(false) => Err((path, "what the witness expects is false here".to_owned())),
            Err(fault) => Err((
                path,
                format!("what the witness expects {} here", fault.describe()),
            )),
        }
    }

    fn play(&self, witness: &Witness) -> Ended {
        let most = self.limits.max_states;
        for (played, start) in Starts::of(self.model).enumerate() {
            if played >= most {
                return Ended::TooManyStarts;
            }
            if let Err((path, why)) = self.play_from(witness, start) {
                return Ended::Broken(path, why);
            }
        }
        Ended::Held
    }

    pub(crate) fn witnesses(&self) -> Vec<RuleOutcome> {
        let only_rule = self.limits.only_rule.as_deref();
        let runs = &self.model.runs;
        runs.witnesses
            .iter()
            .filter(|witness| witness.setup == runs.chosen)
            .filter(|witness| only_rule.is_none_or(|wanted| wanted == witness.id))
            .map(|witness| {
                let (verdict, path, reason) = match self.play(witness) {
                    Ended::Held => (Verdict::Holds, None, None),
                    Ended::Broken(path, why) => (Verdict::Broken, Some(path), Some(why)),
                    Ended::TooManyStarts => (Verdict::Unknown, None, None),
                };
                RuleOutcome {
                    id: witness.id.clone(),
                    says: witness.says.clone(),
                    verdict,
                    path,
                    kind: RuleKind::Witness,
                    reason,
                    repeats_from: None,
                }
            })
            .collect()
    }
}
