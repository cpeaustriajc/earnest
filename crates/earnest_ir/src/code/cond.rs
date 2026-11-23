use super::num::{Operand, Stop, int_at, pair};
use super::shared::Shared;
use super::val::{Binding, Place, Val};
use crate::eval::{as_bool, compare};
use crate::{Expr, Fault, Frame, Op, Order};
use earnest_value::Value;
use std::mem::{Discriminant, discriminant};

#[derive(Clone, Copy, Debug)]
pub(super) enum Wanted {
    Equal,
    Different,
}

impl Wanted {
    fn judge<T: PartialEq + ?Sized>(self, first: &T, second: &T) -> bool {
        match self {
            Self::Equal => first == second,
            Self::Different => first != second,
        }
    }

    fn compares(self, found: &Value, constant: &Value) -> bool {
        match self {
            Self::Equal => is(found, constant),
            Self::Different => !is(found, constant),
        }
    }

    fn of_other_kinds(self) -> bool {
        matches!(self, Self::Different)
    }
}

#[derive(Clone, Copy, Debug)]
pub(super) struct Within {
    field: usize,
    lowest: i64,
    span: u64,
}

impl Within {
    fn of(order: Order, field: usize, number: i64) -> Option<Self> {
        let (lowest, highest) = match order {
            Order::Less => (i64::MIN, number.checked_sub(1)?),
            Order::LessOrEqual => (i64::MIN, number),
            Order::Greater => (number.checked_add(1)?, i64::MAX),
            Order::GreaterOrEqual => (number, i64::MAX),
        };
        Some(Self {
            field,
            lowest,
            span: highest.wrapping_sub(lowest).cast_unsigned(),
        })
    }

    #[inline]
    fn holds(self, state: &[Value]) -> Result<bool, Fault> {
        let number = int_at(state, self.field).map_err(Stop::fault)?;
        Ok(number.wrapping_sub(self.lowest).cast_unsigned() <= self.span)
    }
}

#[derive(Clone, Debug)]
pub(super) enum Cond {
    Const(bool),
    Within(Within),
    AllWithin(Box<[Within]>),
    Order {
        order: Order,
        left: Operand,
        right: Operand,
    },
    FieldOrder {
        order: Order,
        field: usize,
        number: i64,
    },
    Is {
        operand: Operand,
        number: i64,
        wanted: Wanted,
    },
    Matches {
        place: Place,
        value: Value,
        wanted: Wanted,
    },
    Alike(Box<(Place, Place)>, Wanted),
    Same(Box<(Val, Val)>, Wanted),
    Read(Place),
    Not(Box<Self>),
    All(Box<[Self]>),
    Any(Box<[Self]>),
    Choose(Box<(Self, Self, Self)>),
    Bind(Box<Binding<Self>>),
    Value(Box<Val>),
}

impl Cond {
    pub(super) fn of(expr: &Expr, shared: &Shared) -> Self {
        if let Some(cond) = Self::shaped(expr, shared) {
            return cond;
        }
        match expr {
            Expr::Const(Value::Bool(flag)) => Self::Const(*flag),
            Expr::Choose(test, yes, no) => Self::Choose(Box::new((
                Self::of(test, shared),
                Self::of(yes, shared),
                Self::of(no, shared),
            ))),
            _ => match Place::of(expr) {
                Some(place) => Self::Read(place),
                None => Self::Value(Box::new(Val::of(expr, shared))),
            },
        }
    }

    pub(super) fn shaped(expr: &Expr, shared: &Shared) -> Option<Self> {
        match expr {
            Expr::And(..) => Some(Self::all(joined_parts(expr, shared))),
            Expr::Or(..) => Some(Self::Any(joined_parts(expr, shared))),
            Expr::Bind { slot, value, body } => {
                let binding = Binding::of(*slot, value, Self::of(body, shared), shared);
                Some(Self::Bind(Box::new(binding)))
            }
            Expr::Apply(op, arguments) => match (op, arguments.as_slice()) {
                (Op::Order(order), [Expr::Field(field), Expr::Const(Value::Int(number))]) => {
                    Some(Within::of(*order, *field, *number).map_or(
                        Self::FieldOrder {
                            order: *order,
                            field: *field,
                            number: *number,
                        },
                        Self::Within,
                    ))
                }
                (Op::Order(order), [left, right]) => Some(Self::Order {
                    order: *order,
                    left: Operand::of(left, shared),
                    right: Operand::of(right, shared),
                }),
                (Op::Equal, [left, right]) => {
                    Some(Self::equality(left, right, Wanted::Equal, shared))
                }
                (Op::Differ, [left, right]) => {
                    Some(Self::equality(left, right, Wanted::Different, shared))
                }
                (Op::Not, [inner]) => Some(Self::Not(Box::new(Self::of(inner, shared)))),
                _ => None,
            },
            _ => None,
        }
    }

    fn all(parts: Box<[Self]>) -> Self {
        let within = parts
            .iter()
            .map(|part| match part {
                Self::Within(within) => Some(*within),
                _ => None,
            })
            .collect::<Option<Box<[Within]>>>();
        match within {
            Some(tests) if tests.len() > 1 => Self::AllWithin(tests),
            _ => Self::All(parts),
        }
    }

    fn equality(left: &Expr, right: &Expr, wanted: Wanted, shared: &Shared) -> Self {
        if let (other, Expr::Const(Value::Int(number))) | (Expr::Const(Value::Int(number)), other) =
            (left, right)
        {
            return Self::Is {
                operand: Operand::of(other, shared),
                number: *number,
                wanted,
            };
        }
        match (
            Place::of(left),
            Place::of(right),
            left.plain(),
            right.plain(),
        ) {
            (Some(place), None, _, Expr::Const(value))
            | (None, Some(place), Expr::Const(value), _) => Self::Matches {
                place,
                value: value.clone(),
                wanted,
            },
            (Some(first), Some(second), _, _) => Self::Alike(Box::new((first, second)), wanted),
            _ => Self::Same(
                Box::new((Val::of(left, shared), Val::of(right, shared))),
                wanted,
            ),
        }
    }

    #[inline]
    pub(super) fn eval(&self, frame: &mut Frame<'_>) -> Result<bool, Fault> {
        match self {
            Self::Within(within) => within.holds(frame.state),
            Self::AllWithin(tests) => {
                for test in tests {
                    if !test.holds(frame.state)? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            Self::All(parts) => {
                for part in parts {
                    if !part.leaf(frame)? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            _ => self.leaf(frame),
        }
    }

    #[inline]
    fn leaf(&self, frame: &mut Frame<'_>) -> Result<bool, Fault> {
        match self {
            Self::Within(within) => within.holds(frame.state),
            Self::FieldOrder {
                order,
                field,
                number,
            } => Ok(compare(
                *order,
                int_at(frame.state, *field).map_err(Stop::fault)?,
                *number,
            )),
            Self::Const(flag) => Ok(*flag),
            Self::Matches {
                place,
                value,
                wanted,
            } => Ok(wanted.compares(place.find(frame)?, value)),
            _ => self.rest(frame),
        }
    }

    #[inline(never)]
    fn rest(&self, frame: &mut Frame<'_>) -> Result<bool, Fault> {
        match self {
            Self::Order { order, left, right } => {
                let (first, second) = pair(left, right, frame)?;
                Ok(compare(*order, first, second))
            }
            Self::Is {
                operand,
                number,
                wanted,
            } => match operand.get(frame) {
                Ok(found) => Ok(wanted.judge(&found, number)),
                Err(Stop::Kind(_)) => Ok(wanted.of_other_kinds()),
                Err(Stop::Fault(fault)) => Err(fault),
            },
            Self::Alike(places, wanted) => {
                let first = places.0.find(frame)?;
                let second = places.1.find(frame)?;
                Ok(wanted.judge(first, second))
            }
            Self::Same(sides, wanted) => {
                let first = sides.0.eval(frame)?;
                let second = sides.1.eval(frame)?;
                Ok(wanted.judge(&first, &second))
            }
            Self::Read(place) => as_bool(place.find(frame)?),
            Self::Not(inner) => Ok(!inner.eval(frame)?),
            Self::Within(_)
            | Self::AllWithin(_)
            | Self::FieldOrder { .. }
            | Self::Matches { .. }
            | Self::All(_)
            | Self::Const(_) => self.eval(frame),
            Self::Any(parts) => {
                for part in parts {
                    if part.eval(frame)? {
                        return Ok(true);
                    }
                }
                Ok(false)
            }
            Self::Choose(branches) => {
                if branches.0.eval(frame)? {
                    branches.1.eval(frame)
                } else {
                    branches.2.eval(frame)
                }
            }
            Self::Bind(binding) => {
                binding.enter(frame)?;
                binding.body.eval(frame)
            }
            Self::Value(inner) => as_bool(&inner.eval(frame)?),
        }
    }
}

fn is(found: &Value, constant: &Value) -> bool {
    match constant {
        Value::Null => matches!(found, Value::Null),
        Value::Str(name) => matches!(found, Value::Str(other) if other == name),
        Value::Bool(flag) => matches!(found, Value::Bool(other) if other == flag),
        _ => found == constant,
    }
}

fn joined_parts(expr: &Expr, shared: &Shared) -> Box<[Cond]> {
    let mut parts = Vec::new();
    gather_joined(expr, discriminant(expr), &mut parts, shared);
    parts.into_boxed_slice()
}

fn gather_joined(expr: &Expr, join: Discriminant<Expr>, parts: &mut Vec<Cond>, shared: &Shared) {
    match expr {
        Expr::And(left, right) | Expr::Or(left, right) if discriminant(expr) == join => {
            gather_joined(left, join, parts, shared);
            gather_joined(right, join, parts, shared);
        }
        _ => parts.push(Cond::of(expr, shared)),
    }
}
