use super::Search;
use crate::expand::{Asked, Base, Scratch, Successors};
use earnest_value::Value;

struct Choice<'a> {
    target: &'a [Value],
    target_row: Option<&'a [u64]>,
    found: Option<(usize, usize)>,
}

impl Successors for Choice<'_> {
    fn offer(
        &mut self,
        action: usize,
        input: usize,
        base: Base<'_>,
        updates: &[(usize, Value)],
    ) -> bool {
        let state = base.values;
        let mut changed = updates.iter().peekable();
        let same = state.len() == self.target.len()
            && state
                .iter()
                .zip(self.target)
                .enumerate()
                .all(|(position, (old, wanted))| {
                    let value = changed
                        .next_if(|(at, _)| *at == position)
                        .map_or(old, |(_, new)| new);
                    value == wanted
                });
        if same && self.found.is_none() {
            self.found = Some((action, input));
        }
        false
    }

    fn offer_row(&mut self, action: usize, input: usize, row: &[u64], _: u64) -> bool {
        if self.target_row == Some(row) && self.found.is_none() {
            self.found = Some((action, input));
        }
        false
    }

    fn done(&self) -> bool {
        self.found.is_some()
    }
}

impl Search<'_> {
    pub(crate) fn choice(&self, from: usize, to: usize) -> Option<(usize, usize)> {
        let state = self.store.state(from);
        let target = self.store.state(to);
        let mut target_row = Vec::new();
        let target_is_row = self.store.read_state(to, &mut target_row, &mut Vec::new());
        let mut choice = Choice {
            target: &target,
            target_row: target_is_row.then_some(target_row.as_slice()),
            found: None,
        };
        let mut row = Vec::new();
        let is_row = self.store.read_state(from, &mut row, &mut Vec::new());
        let outside = self.store.outside(from, &row, &self.model.fields);
        let base = Base {
            values: &state,
            row: is_row.then_some(row.as_slice()),
        };
        self.expand(None, Asked::Everything).state(
            base,
            outside,
            &mut choice,
            &mut Scratch::default(),
        );
        choice.found
    }
}
