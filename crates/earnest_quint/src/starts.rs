use crate::model::assigned;
use crate::reader::{Choice, Path, Read, refusal};
use earnest_ir::{Frame, eval, truth};
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

fn values_of(choice: &Choice, path: &Path, picked: &[Value], slots: usize) -> Read<Vec<Value>> {
    let Some(set) = &choice.from else {
        return Ok(choice.values.clone());
    };
    let mut frame = Frame::new(&[], slots);
    for (earlier, value) in path.choices.iter().zip(picked) {
        frame.put(earlier.slot, value.clone());
    }
    match eval(set, &mut frame).map_err(fault("a set `init` picks from"))? {
        Value::Set(items) => Ok(items.iter().cloned().collect()),
        _ => Err(refusal(
            "`init`",
            "`init` picks with `oneOf` from a value that is not a set",
        )),
    }
}

fn combinations(path: &Path, slots: usize) -> Read<Vec<Vec<Value>>> {
    let slots = slots_of(path, slots);
    let mut all: Vec<Vec<Value>> = vec![Vec::new()];
    for choice in &path.choices {
        let mut longer_all = Vec::new();
        for picked in &all {
            for value in values_of(choice, path, picked, slots)? {
                if u128::try_from(longer_all.len()).unwrap_or(u128::MAX) >= MOST_VALUES_LISTED {
                    return Err(refusal(
                        "`init`",
                        &format!("`init` picks among more than {MOST_VALUES_LISTED} combinations"),
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

fn fault(why: &str) -> impl Fn(earnest_ir::Fault) -> crate::reader::Refusal {
    move |fault| refusal("`init`", &format!("{why} {}", fault.describe()))
}

fn start_of(
    path: &Path,
    picked: &[Value],
    fields: &[String],
    slots: usize,
) -> Read<Option<Vec<Value>>> {
    let mut frame = Frame::new(&[], slots_of(path, slots));
    for (choice, value) in path.choices.iter().zip(picked) {
        frame.put(choice.slot, value.clone());
    }
    for guard in &path.guards {
        if !truth(guard, &mut frame).map_err(fault("a condition of `init`"))? {
            return Ok(None);
        }
    }
    let mut state = Vec::with_capacity(fields.len());
    for expr in assigned(path, fields, "init")? {
        state.push(eval(&expr, &mut frame).map_err(fault("a value `init` gives"))?);
    }
    Ok(Some(state))
}

pub(crate) fn start_states(
    paths: &[Path],
    fields: &[String],
    slots: usize,
) -> Read<Vec<Vec<Value>>> {
    let mut found = BTreeSet::new();
    for path in paths {
        for picked in combinations(path, slots)? {
            if let Some(state) = start_of(path, &picked, fields, slots)? {
                found.insert(state);
            }
        }
    }
    Ok(found.into_iter().collect())
}
