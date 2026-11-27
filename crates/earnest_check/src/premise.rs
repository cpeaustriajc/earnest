use crate::limits::asked;
use earnest_ir::{Code, Expr, Frame, Model, Op};
use earnest_value::Value;
use std::borrow::Cow;

/// What a [`Premise`] marks of its rule.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum PremiseKind {
    /// The left side of an always rule whose condition is an implication: `!a || b`,
    /// `a ? b : true`, Rust's `if a { b } else { true }` or Quint's `a implies b`, also
    /// inside a helper. With several negations before the last `||` it is all of them:
    /// `a && b` for `!a || !b || c`. The rule says something only of the states where this is
    /// true.
    Implies,
    /// The states an eventually rule starts from: where its `after` condition is true.
    After,
    /// The states where an eventually rule is met: where its `reaches` condition is true.
    Reaches,
}

/// A condition a chart routes to after the states that break a rule, so that a chart of a
/// model where every rule holds still shows the states the rules are about.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Premise {
    /// The id of the rule, as [`crate::RuleOutcome::id`] gives it.
    pub rule: String,
    /// Which condition of the rule it is.
    pub kind: PremiseKind,
}

pub(crate) struct Premises<'m> {
    pub(crate) list: Vec<Premise>,
    codes: Vec<(Cow<'m, Code>, usize)>,
}

fn or_operands<'e>(expr: &'e Expr, operands: &mut Vec<&'e Expr>) {
    if let Expr::Or(first, second) = expr {
        or_operands(first, operands);
        or_operands(second, operands);
    } else {
        operands.push(expr);
    }
}

fn negated(expr: &Expr) -> Option<&Expr> {
    match expr {
        Expr::Apply(Op::Not, arguments) => match arguments.as_slice() {
            [inner] => Some(inner),
            _ => None,
        },
        _ => None,
    }
}

pub(crate) fn implies(expr: &Expr) -> Option<Expr> {
    match expr {
        Expr::Bind { slot, value, body } => implies(body).map(|inner| Expr::Bind {
            slot: *slot,
            value: value.clone(),
            body: Box::new(inner),
        }),
        Expr::Choose(test, _, otherwise)
            if matches!(**otherwise, Expr::Const(Value::Bool(true))) =>
        {
            Some((**test).clone())
        }
        Expr::Or(..) => {
            let mut operands = Vec::new();
            or_operands(expr, &mut operands);
            let conclusion = operands.len().saturating_sub(1);
            let given: Vec<&Expr> = operands
                .iter()
                .take(conclusion)
                .map_while(|operand| negated(operand))
                .collect();
            let mut given = given.into_iter().rev();
            let last = given.next()?.clone();
            Some(given.fold(last, |rest, before| {
                Expr::And(Box::new(before.clone()), Box::new(rest))
            }))
        }
        _ => None,
    }
}

impl<'m> Premises<'m> {
    pub(crate) fn of(model: &'m Model, only_rule: Option<&str>) -> Self {
        let mut premises = Self {
            list: Vec::new(),
            codes: Vec::new(),
        };
        for rule in model.rules.iter().filter(|rule| asked(only_rule, &rule.id)) {
            if let Some(given) = implies(rule.holds.expr()) {
                premises.add(
                    &rule.id,
                    PremiseKind::Implies,
                    Cow::Owned(rule.holds.part(given)),
                    rule.slots,
                );
            }
        }
        for rule in model
            .runs
            .eventually
            .iter()
            .filter(|rule| asked(only_rule, &rule.id))
        {
            if let Some(after) = &rule.after {
                premises.add(
                    &rule.id,
                    PremiseKind::After,
                    Cow::Borrowed(after),
                    rule.slots,
                );
            }
            premises.add(
                &rule.id,
                PremiseKind::Reaches,
                Cow::Borrowed(&rule.reaches),
                rule.slots,
            );
        }
        premises
    }

    fn add(&mut self, rule: &str, kind: PremiseKind, code: Cow<'m, Code>, slots: usize) {
        self.list.push(Premise {
            rule: rule.to_owned(),
            kind,
        });
        self.codes.push((code, slots));
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.list.is_empty()
    }

    pub(crate) fn holding(&self, state: &[Value]) -> impl Iterator<Item = usize> {
        self.codes
            .iter()
            .enumerate()
            .filter(move |(_, (code, slots))| {
                code.truth(&mut Frame::new(state, *slots)) == Ok(true)
            })
            .map(|(position, _)| position)
    }
}

#[cfg(test)]
mod tests {
    use super::implies;
    use earnest_ir::{Expr, Op};
    use earnest_value::Value;

    fn field(position: usize) -> Expr {
        Expr::Field(position)
    }

    fn not(expr: Expr) -> Expr {
        Expr::Apply(Op::Not, vec![expr])
    }

    fn or(first: Expr, second: Expr) -> Expr {
        Expr::Or(Box::new(first), Box::new(second))
    }

    fn and(first: Expr, second: Expr) -> Expr {
        Expr::And(Box::new(first), Box::new(second))
    }

    #[test]
    fn a_negation_before_or_is_the_left_side() {
        assert_eq!(implies(&or(not(field(0)), field(1))), Some(field(0)));
    }

    #[test]
    fn every_leading_negation_of_a_chain_of_or_is_one_left_side() {
        let chained = or(or(not(field(0)), not(field(1))), field(2));
        assert_eq!(implies(&chained), Some(and(field(0), field(1))));
        let nested = or(not(field(0)), or(not(field(1)), field(2)));
        assert_eq!(implies(&nested), Some(and(field(0), field(1))));
    }

    #[test]
    fn the_last_operand_is_the_conclusion_even_when_negated() {
        assert_eq!(implies(&or(not(field(0)), not(field(1)))), Some(field(0)));
    }

    #[test]
    fn a_choice_that_is_true_otherwise_is_an_implication() {
        let chosen = Expr::Choose(
            Box::new(field(0)),
            Box::new(field(1)),
            Box::new(Expr::Const(Value::Bool(true))),
        );
        assert_eq!(implies(&chosen), Some(field(0)));
    }

    #[test]
    fn a_helper_keeps_its_bindings_around_the_left_side() {
        let helper = Expr::Bind {
            slot: 0,
            value: Box::new(field(0)),
            body: Box::new(or(not(Expr::Local(0)), field(1))),
        };
        let expected = Expr::Bind {
            slot: 0,
            value: Box::new(field(0)),
            body: Box::new(Expr::Local(0)),
        };
        assert_eq!(implies(&helper), Some(expected));
    }

    #[test]
    fn other_shapes_have_no_left_side() {
        for shape in [
            field(0),
            not(field(0)),
            or(field(0), not(field(1))),
            not(and(field(0), field(1))),
            and(or(not(field(0)), field(1)), field(2)),
        ] {
            assert!(implies(&shape).is_none(), "{shape:?}");
        }
    }
}
