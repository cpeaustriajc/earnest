mod chain;
mod cond;
mod num;
mod packed;
mod shared;
mod val;

use crate::{Expr, Fault, Frame};
use cond::Cond;
use earnest_value::Value;
pub use num::FieldStep;
use num::Stop;
pub use packed::{Coding, PackedTest, PackedUpdate, Slot};
use shared::Shared;
use std::fmt;
use val::Val;

/// The value of a [`Code`] for a caller that keeps only numbers and booleans, from
/// [`Code::scalar`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Scalar {
    /// The value is this whole number.
    Int(i64),
    /// The value is this boolean.
    Bool(bool),
    /// The value is of another kind; [`Code::eval`] gives it.
    Other,
}

/// An expression together with the form the search evaluates it in.
///
/// Build one with [`Code::new`] once, before the search, and evaluate it in every state with
/// [`Code::eval`] or [`Code::truth`]. Both give the value or the [`Fault`] that [`crate::eval`] and
/// [`crate::truth`] give for [`Code::expr`], with the same effects on the frame; they are faster
/// because numbers and booleans are computed, and fields and records are read where they are,
/// without building a [`Value`] for each step.
#[derive(Clone)]
pub struct Code {
    expr: Expr,
    value: Val,
    test: Cond,
}

impl Code {
    /// Prepares `expr` for evaluation. The cost is one walk over the expression.
    #[must_use]
    pub fn new(expr: Expr) -> Self {
        let shared = Shared::of(std::slice::from_ref(&expr));
        Self::prepared(expr, &shared)
    }

    /// Prepares expressions that are evaluated one after another in one frame, such as an
    /// action's guard and updates, and gives their codes in the order given.
    ///
    /// A binding that several of them repeat, with the same slot and the same value, is
    /// computed once and then read from its slot, until [`Frame::put`] or a code prepared apart
    /// writes to the frame. Each code still gives what [`crate::eval`] gives for its expression, with
    /// the same effects on the frame, whatever order the codes run in.
    #[must_use]
    pub fn together(exprs: impl IntoIterator<Item = Expr>) -> Vec<Self> {
        let exprs: Vec<Expr> = exprs.into_iter().collect();
        let shared = Shared::of(&exprs);
        exprs
            .into_iter()
            .map(|expr| Self::prepared(expr, &shared))
            .collect()
    }

    fn prepared(expr: Expr, shared: &Shared) -> Self {
        let value = Val::of(&expr, shared);
        let test = Cond::of(&expr, shared);
        Self { expr, value, test }
    }

    /// The expression as it was given, for printing it or translating it.
    #[must_use]
    pub fn expr(&self) -> &Expr {
        &self.expr
    }

    /// The local slots the expression reads, when it binds no slot and tests no items of a
    /// collection; `None` when it does.
    ///
    /// Such an expression gives the same value or the same [`Fault`] in every frame with the
    /// same state and the same values in these slots, and evaluating it again in that frame
    /// changes nothing in it, so its earlier answer may be used in place of evaluating it.
    #[must_use]
    pub fn slots_read(&self) -> Option<Vec<usize>> {
        let mut slots = Vec::new();
        self.expr.reads_plainly(&mut slots).then(|| {
            slots.sort_unstable();
            slots.dedup();
            slots
        })
    }

    /// The value of the expression in the state and locals of `frame`.
    ///
    /// # Errors
    ///
    /// A [`Fault`] when the expression has no value in this state.
    pub fn eval(&self, frame: &mut Frame<'_>) -> Result<Value, Fault> {
        match &self.value {
            Val::Num(num) => num.eval(frame).map(Value::Int).map_err(Stop::fault),
            other => other.eval(frame),
        }
    }

    /// The value of the expression when it is a number or a boolean, without building a
    /// [`Value`]. It has the same effects on the frame as [`Code::eval`], and gives a fault
    /// exactly when [`Code::eval`] does.
    ///
    /// # Errors
    ///
    /// The [`Fault`] that [`Code::eval`] gives.
    #[inline]
    pub fn scalar(&self, frame: &mut Frame<'_>) -> Result<Scalar, Fault> {
        match &self.value {
            Val::Num(num) => num.eval(frame).map(Scalar::Int).map_err(Stop::fault),
            other => Self::other_scalar(other, frame),
        }
    }

    #[inline(never)]
    fn other_scalar(value: &Val, frame: &mut Frame<'_>) -> Result<Scalar, Fault> {
        match value {
            Val::Cond(cond) => cond.eval(frame).map(Scalar::Bool),
            other => Ok(match other.eval(frame)? {
                Value::Int(number) => Scalar::Int(number),
                Value::Bool(flag) => Scalar::Bool(flag),
                _ => Scalar::Other,
            }),
        }
    }

    /// The expression as an update of one whole-number field by a constant, such as `s.a + 1`,
    /// when it is one.
    #[must_use]
    pub fn field_step(&self) -> Option<FieldStep> {
        match &self.value {
            Val::Num(num) => num.field_step(),
            _ => None,
        }
    }

    /// The expression prepared as a test of a packed row, for a guard, a rule or an end, when it
    /// can be judged from the codes of the fields in `slots` alone: comparisons of whole-number
    /// fields with constants or with each other, possibly through arithmetic, a field of true
    /// or false read or compared, a field of listed values compared with a constant or with a
    /// field of the same list, any expression that reads one field of at most 4096 values and
    /// no local slot it does not bind, and `and`, `or`, `not` and `?:` of those. `None` for
    /// every other expression.
    ///
    /// `slots` gives the slot of each field by position, `None` for a field the row does not
    /// hold by a [`Coding`]. `inputs` gives the value of each local slot the expression reads
    /// as an input, by slot; the test is then prepared for those values only, and is right only
    /// if no part of the expression binds another value to those slots.
    #[must_use]
    pub fn packed_test(
        &self,
        slots: &[Option<Slot>],
        inputs: &[(usize, Value)],
    ) -> Option<PackedTest> {
        PackedTest::of(&self.expr, slots, inputs)
    }

    /// The expression prepared as an update of field `field` that writes its answer from one
    /// packed row into another, when the answer can be worked out from the codes of the fields
    /// in `slots` alone: a whole number worked out as [`Code::packed_test`] works out numbers,
    /// a test that [`Code::packed_test`] takes for a field of true or false, a field with the
    /// same coding copied, any expression that reads one field of at most 4096 values or no
    /// field and no loop, and `?:` of those. `None` otherwise, for an answer that is never a
    /// value of the field, and when `slots` has no slot for `field`. `inputs` is as for
    /// [`Code::packed_test`].
    #[must_use]
    pub fn packed_update(
        &self,
        slots: &[Option<Slot>],
        field: usize,
        inputs: &[(usize, Value)],
    ) -> Option<PackedUpdate> {
        PackedUpdate::of(&self.expr, slots, field, inputs)
    }

    /// Whether the expression is true in the state and locals of `frame`, for a guard or a rule.
    ///
    /// # Errors
    ///
    /// A [`Fault`] when the expression has no value in this state, and [`Fault::NullRead`] or
    /// [`Fault::WrongKind`] when its value is not a boolean.
    #[inline]
    pub fn truth(&self, frame: &mut Frame<'_>) -> Result<bool, Fault> {
        self.test.eval(frame)
    }
}

impl From<Expr> for Code {
    fn from(expr: Expr) -> Self {
        Self::new(expr)
    }
}

impl PartialEq for Code {
    fn eq(&self, other: &Self) -> bool {
        self.expr == other.expr
    }
}

impl Eq for Code {}

impl fmt::Debug for Code {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.expr.fmt(formatter)
    }
}
