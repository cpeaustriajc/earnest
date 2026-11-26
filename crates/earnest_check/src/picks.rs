use earnest_ir::{Code, Fault, Frame, Input, Model};
use earnest_value::Value;

pub(crate) fn sets_of(model: &Model, action: usize) -> Option<Box<[Option<Code>]>> {
    let inputs = model.actions.get(action)?.inputs.len();
    let mut sets: Vec<Option<Code>> = vec![None; inputs];
    let mut picked = false;
    for pick in model.runs.picks.iter().filter(|pick| pick.action == action) {
        if let Some(place) = sets.get_mut(pick.input) {
            *place = Some(pick.from.clone());
            picked = true;
        }
    }
    picked.then(|| sets.into_boxed_slice())
}

pub(crate) enum Next {
    Ready,
    Faulted(Fault),
    Done,
}

pub(crate) struct Cursor<'a> {
    inputs: &'a [Input],
    sets: &'a [Option<Code>],
    lists: Vec<Vec<Value>>,
    digits: Vec<usize>,
    descending: bool,
}

fn items(set: Result<Value, Fault>) -> Result<Vec<Value>, Fault> {
    match set? {
        Value::Set(items) => Ok(items.iter().cloned().collect()),
        _ => Err(Fault::WrongKind),
    }
}

impl<'a> Cursor<'a> {
    pub(crate) fn new(inputs: &'a [Input], sets: &'a [Option<Code>]) -> Self {
        Self {
            inputs,
            sets,
            lists: Vec::with_capacity(inputs.len()),
            digits: Vec::with_capacity(inputs.len()),
            descending: true,
        }
    }

    fn put_chosen(&self, frame: &mut Frame<'_>) {
        for (input, (list, digit)) in self.inputs.iter().zip(self.lists.iter().zip(&self.digits)) {
            if let Some(value) = list.get(*digit) {
                frame.put_copy(input.slot, value);
            }
        }
    }

    fn chosen(&self) -> Vec<Value> {
        self.lists
            .iter()
            .zip(&self.digits)
            .filter_map(|(list, digit)| list.get(*digit).cloned())
            .collect()
    }

    pub(crate) fn next(&mut self, frame: &mut Frame<'_>) -> Next {
        loop {
            if self.descending {
                let depth = self.lists.len();
                self.put_chosen(frame);
                let Some(input) = self.inputs.get(depth) else {
                    self.descending = false;
                    return Next::Ready;
                };
                let listed = match self.sets.get(depth).and_then(Option::as_ref) {
                    Some(set) => items(set.eval(frame)),
                    None => Ok(input.values.clone()),
                };
                match listed {
                    Ok(values) => {
                        self.descending = !values.is_empty();
                        self.lists.push(values);
                        self.digits.push(0);
                    }
                    Err(fault) => {
                        self.descending = false;
                        return Next::Faulted(fault);
                    }
                }
            } else {
                let (Some(digit), Some(list)) = (self.digits.last_mut(), self.lists.last()) else {
                    return Next::Done;
                };
                *digit = digit.saturating_add(1);
                if *digit < list.len() {
                    self.descending = true;
                } else {
                    self.digits.pop();
                    self.lists.pop();
                }
            }
        }
    }
}

/// Every combination of values the inputs of the action at `action` in [`Model::actions`] take
/// in `state`, each in the order of the action's inputs, in the order the search numbers them:
/// the first input turns slowest, and each input's set is worked out with the earlier inputs
/// in their slots. An input listed in [`earnest_ir::Runs::picks`] takes the items of its set in
/// that state; any other input takes its listed values. A set that faults leaves out the
/// combinations under it, and the first such fault is given beside the list. Empty for a
/// position past the last action.
pub(crate) fn combinations(
    model: &Model,
    action: usize,
    state: &[Value],
) -> (Vec<Vec<Value>>, Option<Fault>) {
    let Some(found) = model.actions.get(action) else {
        return (Vec::new(), None);
    };
    let sets =
        sets_of(model, action).unwrap_or_else(|| vec![None; found.inputs.len()].into_boxed_slice());
    let mut frame = Frame::new(state, found.slots);
    let mut cursor = Cursor::new(&found.inputs, &sets);
    let mut all = Vec::new();
    let mut first_fault = None;
    loop {
        match cursor.next(&mut frame) {
            Next::Ready => all.push(cursor.chosen()),
            Next::Faulted(fault) => {
                first_fault.get_or_insert(fault);
            }
            Next::Done => return (all, first_fault),
        }
    }
}
