use crate::Expr;
use crate::eval::NO_UNIT;
use crate::parts::{Binder, Binders, Scope};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_UNIT: AtomicU64 = AtomicU64::new(NO_UNIT + 1);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Unbound,
    Steady,
    Varied,
}

#[derive(Debug)]
pub(super) struct Shared {
    unit: u64,
    kinds: Vec<Kind>,
    repeated: Vec<bool>,
}

impl Shared {
    pub(super) fn of(exprs: &[Expr]) -> Self {
        let binders = Binders::of(exprs);
        let mut shared = Self {
            unit: NEXT_UNIT.fetch_add(1, Ordering::Relaxed),
            kinds: binders.of_slot().iter().map(|all| kind(all)).collect(),
            repeated: binders.of_slot().iter().map(|all| repeated(all)).collect(),
        };
        let mut scope = Scope::default();
        while exprs.iter().any(|expr| shared.narrowed(expr, &mut scope)) {}
        shared
    }

    pub(super) fn unit(&self) -> u64 {
        self.unit
    }

    pub(super) fn keeps(&self, slot: usize) -> bool {
        self.steady(slot) && self.repeated.get(slot).copied().unwrap_or(false)
    }

    fn steady(&self, slot: usize) -> bool {
        self.kinds.get(slot) == Some(&Kind::Steady)
    }

    fn bound(&self, slot: usize) -> bool {
        self.kinds
            .get(slot)
            .is_some_and(|kind| *kind != Kind::Unbound)
    }

    fn narrowed(&mut self, expr: &Expr, scope: &mut Scope) -> bool {
        match expr {
            Expr::Bind { slot, value, body } => {
                let mut changed = false;
                if self.steady(*slot) && !self.settles(value, scope) {
                    if let Some(kind) = self.kinds.get_mut(*slot) {
                        *kind = Kind::Varied;
                    }
                    changed = true;
                }
                changed |= self.narrowed(value, scope);
                scope.open(*slot);
                changed |= self.narrowed(body, scope);
                scope.close();
                changed
            }
            Expr::Sweep {
                slot, source, body, ..
            } => {
                let mut changed = self.narrowed(source, scope);
                scope.open(*slot);
                changed |= self.narrowed(body, scope);
                scope.close();
                changed
            }
            _ => {
                let mut changed = false;
                expr.each_part(&mut |part| changed |= self.narrowed(part, scope));
                changed
            }
        }
    }

    fn settles(&self, expr: &Expr, scope: &mut Scope) -> bool {
        match expr {
            Expr::Local(slot) => !self.bound(*slot) || (self.steady(*slot) && scope.holds(*slot)),
            Expr::Bind { slot, value, body } => {
                if !self.steady(*slot) || !self.settles(value, scope) {
                    return false;
                }
                scope.open(*slot);
                let settles = self.settles(body, scope);
                scope.close();
                settles
            }
            Expr::Sweep { .. } => false,
            _ => {
                let mut settles = true;
                expr.each_part(&mut |part| settles &= self.settles(part, scope));
                settles
            }
        }
    }
}

fn kind(all: &[Binder<'_>]) -> Kind {
    match all {
        [] => Kind::Unbound,
        [Binder::Bind { value: first, .. }, ..]
            if !holds_a_sweep(first)
                && all.iter().all(
                    |binder| matches!(binder, Binder::Bind { value, .. } if value == first),
                ) =>
        {
            Kind::Steady
        }
        _ => Kind::Varied,
    }
}

fn repeated(all: &[Binder<'_>]) -> bool {
    match all {
        [] => false,
        [Binder::Bind { in_a_sweep, .. }] => *in_a_sweep,
        _ => true,
    }
}

fn holds_a_sweep(expr: &Expr) -> bool {
    if matches!(expr, Expr::Sweep { .. }) {
        return true;
    }
    let mut found = false;
    expr.each_part(&mut |part| found |= holds_a_sweep(part));
    found
}
