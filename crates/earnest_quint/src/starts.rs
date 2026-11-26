use crate::model::assigned;
use crate::reader::{Path, Read, refusal};
use earnest_ir::{Frame, eval, truth};
use earnest_value::{MOST_VALUES_LISTED, Value};
use std::collections::BTreeSet;

fn combinations(path: &Path) -> Read<Vec<Vec<Value>>> {
    let mut all: Vec<Vec<Value>> = vec![Vec::new()];
    for choice in &path.choices {
        let count = all.len().saturating_mul(choice.values.len());
        if u128::try_from(count).unwrap_or(u128::MAX) > MOST_VALUES_LISTED {
            return Err(refusal(
                "`init`",
                &format!("`init` picks among more than {MOST_VALUES_LISTED} combinations"),
            ));
        }
        all = all
            .iter()
            .flat_map(|picked| {
                choice.values.iter().map(|value| {
                    let mut longer = picked.clone();
                    longer.push(value.clone());
                    longer
                })
            })
            .collect();
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
    let slots = path
        .choices
        .iter()
        .map(|choice| choice.slot.saturating_add(1))
        .max()
        .unwrap_or(0)
        .max(slots);
    let mut frame = Frame::new(&[], slots);
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
        for picked in combinations(path)? {
            if let Some(state) = start_of(path, &picked, fields, slots)? {
                found.insert(state);
            }
        }
    }
    Ok(found.into_iter().collect())
}
