use crate::eval::field_of;
use crate::parts::{Binder, Binders, Scope};
use crate::{Action, Code, Expr, Fault};
use earnest_value::{Sym, Value};
use std::collections::HashSet;

impl Action {
    /// For each input, in order, and each of its values, in order: whether the guard and the
    /// updates can tell the value from every value before it.
    ///
    /// They read some inputs only through some of their fields, such as a record whose other
    /// fields no expression names. A value that agrees with an earlier one on every field they
    /// read gives the same guard, the same updates and the same faults as that value in every
    /// state and with every choice of the other inputs, so a search that tries the earlier one
    /// learns nothing new from it. Every value is told apart when an expression reads a slot
    /// it has not bound, or reads the input whole.
    #[must_use]
    pub fn told_apart(&self) -> Vec<Vec<bool>> {
        let exprs: Vec<&Expr> = std::iter::once(&self.guard)
            .chain(self.updates.iter().flatten())
            .map(Code::expr)
            .collect();
        let binders = Binders::of(exprs.iter().copied());
        let scoped = exprs
            .iter()
            .all(|expr| binders.in_scope(expr, &mut Scope::default()));
        self.inputs
            .iter()
            .map(|input| {
                let read = if scoped {
                    binders.fields_read(&exprs, input.slot)
                } else {
                    None
                };
                match read {
                    Some(names) => first_of_each(&input.values, &names),
                    None => vec![true; input.values.len()],
                }
            })
            .collect()
    }

    /// Whether the guard or an update binds a value to the slot of an input, or two inputs
    /// share a slot. Either way a slot of an input may hold another value after the action is
    /// evaluated than the one put there, so a search that tries the next combination puts every
    /// input again, not only those whose value changed.
    #[must_use]
    pub fn inputs_may_be_overwritten(&self) -> bool {
        let binders = Binders::of(
            std::iter::once(&self.guard)
                .chain(self.updates.iter().flatten())
                .map(Code::expr),
        );
        self.inputs.iter().enumerate().any(|(at, input)| {
            binders.bound(input.slot)
                || self
                    .inputs
                    .iter()
                    .skip(at.saturating_add(1))
                    .any(|other| other.slot == input.slot)
        })
    }
}

impl Binders<'_> {
    fn in_scope(&self, expr: &Expr, scope: &mut Scope) -> bool {
        match expr {
            Expr::Local(slot) => !self.bound(*slot) || scope.holds(*slot),
            Expr::Bind {
                slot,
                value: first,
                body: then,
            }
            | Expr::Sweep {
                slot,
                source: first,
                body: then,
                ..
            } => {
                if !self.in_scope(first, scope) {
                    return false;
                }
                scope.open(*slot);
                let scoped = self.in_scope(then, scope);
                scope.close();
                scoped
            }
            _ => {
                let mut scoped = true;
                expr.each_part(&mut |part| scoped &= self.in_scope(part, scope));
                scoped
            }
        }
    }

    fn copies(&self, input: usize) -> Vec<bool> {
        let mut copy = vec![false; self.of_slot().len().max(input.saturating_add(1))];
        if let Some(place) = copy.get_mut(input) {
            *place = true;
        }
        let mut grew = true;
        while grew {
            grew = false;
            for (slot, all) in self.of_slot().iter().enumerate() {
                let from_copies = !all.is_empty()
                    && all.iter().all(|binder| match binder {
                        Binder::Bind { value, .. } => match value.plain() {
                            Expr::Local(source) => copy.get(*source).copied().unwrap_or(false),
                            _ => false,
                        },
                        Binder::Sweep => false,
                    });
                if let Some(place) = copy.get_mut(slot)
                    && from_copies
                    && !*place
                {
                    *place = true;
                    grew = true;
                }
            }
        }
        copy
    }

    fn fields_read(&self, exprs: &[&Expr], input: usize) -> Option<Vec<Sym>> {
        if self.bound(input) {
            return None;
        }
        let copy = self.copies(input);
        let holds_input = |slot: usize| copy.get(slot).copied().unwrap_or(false);
        let mut names = Vec::new();
        for expr in exprs {
            if !reads_fields_only(expr, &holds_input, &mut names) {
                return None;
            }
        }
        names.sort_unstable();
        names.dedup();
        Some(names)
    }
}

fn reads_fields_only(
    expr: &Expr,
    holds_input: &impl Fn(usize) -> bool,
    names: &mut Vec<Sym>,
) -> bool {
    match expr {
        Expr::Get(inner, name) if matches!(inner.plain(), Expr::Local(slot) if holds_input(*slot)) =>
        {
            names.push(*name);
            true
        }
        Expr::Bind { slot, body, .. } if holds_input(*slot) => {
            reads_fields_only(body, holds_input, names)
        }
        Expr::Local(slot) => !holds_input(*slot),
        _ => {
            let mut only = true;
            expr.each_part(&mut |part| only &= reads_fields_only(part, holds_input, names));
            only
        }
    }
}

fn first_of_each(values: &[Value], names: &[Sym]) -> Vec<bool> {
    let mut seen: HashSet<Vec<Result<Value, Fault>>> = HashSet::new();
    values
        .iter()
        .map(|value| seen.insert(names.iter().map(|name| field_of(value, *name)).collect()))
        .collect()
}
