use crate::{Expr, Op};

impl Expr {
    pub(crate) fn plain(&self) -> &Self {
        match self {
            Self::Apply(Op::SomeOf, arguments) => match arguments.as_slice() {
                [inner] => inner.plain(),
                _ => self,
            },
            _ => self,
        }
    }

    pub(crate) fn reads_plainly(&self, slots: &mut Vec<usize>) -> bool {
        match self {
            Self::Bind { .. } | Self::Sweep { .. } | Self::Fold { .. } => false,
            Self::Local(slot) => {
                slots.push(*slot);
                true
            }
            _ => {
                let mut plain = true;
                self.each_part(&mut |part| plain &= part.reads_plainly(slots));
                plain
            }
        }
    }

    pub(crate) fn each_part<'e>(&'e self, visit: &mut impl FnMut(&'e Self)) {
        match self {
            Self::Const(_) | Self::Field(_) | Self::Local(_) => {}
            Self::Apply(_, items) | Self::SetOf(items) | Self::SeqOf(items) => {
                items.iter().for_each(visit);
            }
            Self::And(left, right) | Self::Or(left, right) => {
                visit(left);
                visit(right);
            }
            Self::Choose(test, yes, no) => {
                visit(test);
                visit(yes);
                visit(no);
            }
            Self::Bind { value, body, .. } => {
                visit(value);
                visit(body);
            }
            Self::Record(fields) => fields.iter().for_each(|(_, inner)| visit(inner)),
            Self::Get(inner, _) => visit(inner),
            Self::With(base, changes) => {
                visit(base);
                for (_, inner) in changes {
                    visit(inner);
                }
            }
            Self::MapOf(entries) => entries.iter().for_each(|(key, value)| {
                visit(key);
                visit(value);
            }),
            Self::Sweep { source, body, .. } => {
                visit(source);
                visit(body);
            }
            Self::Fold {
                source,
                start,
                body,
                ..
            } => {
                visit(source);
                visit(start);
                visit(body);
            }
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) enum Binder<'e> {
    Bind { value: &'e Expr, in_a_sweep: bool },
    Sweep,
}

pub(crate) struct Binders<'e> {
    of_slot: Vec<Vec<Binder<'e>>>,
    sweeping: usize,
}

impl<'e> Binders<'e> {
    pub(crate) fn of(exprs: impl IntoIterator<Item = &'e Expr>) -> Self {
        let mut binders = Self {
            of_slot: Vec::new(),
            sweeping: 0,
        };
        for expr in exprs {
            binders.gather(expr);
        }
        binders
    }

    fn gather(&mut self, expr: &'e Expr) {
        match expr {
            Expr::Bind { slot, value, .. } => {
                let in_a_sweep = self.sweeping > 0;
                self.add(*slot, Binder::Bind { value, in_a_sweep });
            }
            Expr::Sweep {
                slot, source, body, ..
            } => {
                self.add(*slot, Binder::Sweep);
                self.gather(source);
                self.sweeping = self.sweeping.saturating_add(1);
                self.gather(body);
                self.sweeping = self.sweeping.saturating_sub(1);
                return;
            }
            Expr::Fold {
                source,
                start,
                total,
                item,
                body,
            } => {
                self.add(*total, Binder::Sweep);
                self.add(*item, Binder::Sweep);
                self.gather(source);
                self.gather(start);
                self.sweeping = self.sweeping.saturating_add(1);
                self.gather(body);
                self.sweeping = self.sweeping.saturating_sub(1);
                return;
            }
            _ => {}
        }
        expr.each_part(&mut |part| self.gather(part));
    }

    fn add(&mut self, slot: usize, binder: Binder<'e>) {
        if self.of_slot.len() <= slot {
            self.of_slot.resize(slot.saturating_add(1), Vec::new());
        }
        if let Some(all) = self.of_slot.get_mut(slot) {
            all.push(binder);
        }
    }

    pub(crate) fn of_slot(&self) -> &[Vec<Binder<'e>>] {
        &self.of_slot
    }

    pub(crate) fn bound(&self, slot: usize) -> bool {
        self.of_slot.get(slot).is_some_and(|all| !all.is_empty())
    }
}

#[derive(Default)]
pub(crate) struct Scope {
    open: Vec<usize>,
}

impl Scope {
    pub(crate) fn open(&mut self, slot: usize) {
        self.open.push(slot);
    }

    pub(crate) fn close(&mut self) {
        self.open.pop();
    }

    pub(crate) fn holds(&self, slot: usize) -> bool {
        self.open.contains(&slot)
    }
}
