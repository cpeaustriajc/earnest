use crate::model::assigned;
use crate::reader::{Choice, Path, Read, refusal};
use earnest_ir::{Fault, Frame, eval, truth};
use earnest_value::{MOST_VALUES_LISTED, Value};
use std::collections::BTreeSet;

fn slots_of(path: &Path, slots: usize) -> usize {
    path.choices
        .iter()
        .map(|choice| choice.slot.saturating_add(1))
        .max()
        .unwrap_or(0)
        .max(slots)
}

fn values_of(
    choice: &Choice,
    (path, what): (&Path, &str),
    picked: &[Value],
    slots: usize,
) -> Read<Vec<Value>> {
    let Some(set) = &choice.from else {
        return Ok(choice.values.clone());
    };
    let mut frame = Frame::new(&[], slots);
    for (earlier, value) in path.choices.iter().zip(picked) {
        frame.put(earlier.slot, value.clone());
    }
    match eval(set, &mut frame).map_err(fault(what, &format!("a set {what} picks from")))? {
        Value::Set(items) => Ok(items.iter().cloned().collect()),
        _ => Err(refusal(
            what,
            &format!("{what} picks with `oneOf` from a value that is not a set"),
        )),
    }
}

fn combinations((path, what): (&Path, &str), slots: usize) -> Read<Vec<Vec<Value>>> {
    let slots = slots_of(path, slots);
    let mut all: Vec<Vec<Value>> = vec![Vec::new()];
    for choice in &path.choices {
        let mut longer_all = Vec::new();
        for picked in &all {
            for value in values_of(choice, (path, what), picked, slots)? {
                if u128::try_from(longer_all.len()).unwrap_or(u128::MAX) >= MOST_VALUES_LISTED {
                    return Err(refusal(
                        what,
                        &format!("{what} picks among more than {MOST_VALUES_LISTED} combinations"),
                    ));
                }
                let mut longer = picked.clone();
                longer.push(value);
                longer_all.push(longer);
            }
        }
        all = longer_all;
    }
    Ok(all)
}

fn fault(what: &str, why: &str) -> impl Fn(earnest_ir::Fault) -> crate::reader::Refusal + use<> {
    let (what, why) = (what.to_owned(), why.to_owned());
    move |fault| refusal(&what, &format!("{why} {}", fault.describe()))
}

/// What a false `assert` among the conditions of a first action does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Asserts {
    /// The model is refused, as for `init`, where Quint's backends differ.
    Refuse,
    /// The run has no state to start from, so it is broken, as `quint test` fails it.
    EndTheRun,
}

enum Start {
    State(Vec<Value>),
    NotPossible,
    Asserted,
}

fn start_of(
    (path, what): (&Path, &str),
    picked: &[Value],
    (fields, slots): (&[String], usize),
    asserts: Asserts,
) -> Read<Start> {
    let mut frame = Frame::new(&[], slots_of(path, slots));
    for (choice, value) in path.choices.iter().zip(picked) {
        frame.put(choice.slot, value.clone());
    }
    for guard in &path.guards {
        match truth(guard, &mut frame) {
            Ok(true) => {}
            Ok(false) => return Ok(Start::NotPossible),
            Err(Fault::Asserted) if asserts == Asserts::EndTheRun => return Ok(Start::Asserted),
            Err(found) => return Err(fault(what, &format!("a condition of {what}"))(found)),
        }
    }
    let mut state = Vec::with_capacity(fields.len());
    for expr in assigned(path, fields, what.trim_matches('`'))? {
        let given = fault(what, &format!("a value {what} gives"));
        state.push(eval(&expr, &mut frame).map_err(given)?);
    }
    Ok(Start::State(state))
}

pub(crate) fn start_states(
    (paths, what): (&[Path], &str),
    fields: &[String],
    slots: usize,
    asserts: Asserts,
) -> Read<Vec<Vec<Value>>> {
    let mut found = BTreeSet::new();
    for path in paths {
        for picked in combinations((path, what), slots)? {
            match start_of((path, what), &picked, (fields, slots), asserts)? {
                Start::State(state) => {
                    found.insert(state);
                }
                Start::NotPossible => {}
                Start::Asserted => return Ok(Vec::new()),
            }
        }
    }
    Ok(found.into_iter().collect())
}
