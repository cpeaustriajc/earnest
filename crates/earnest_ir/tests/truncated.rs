//! Division and remainder as Rust and TypeScript have them: [`Math::TruncatedDivide`] drops the
//! fraction of the exact quotient and [`Math::TruncatedRemainder`] has the sign of the first
//! number. The prepared [`Code`] and the reference walk give the same answer and the same fault,
//! and Quint's [`Math::Divide`] still floors on the same numbers.

use earnest_ir::{Code, Expr, Fault, Frame, Math, Op, eval};
use earnest_value::{IntType, Notation, Quint, Value};
use proptest::prelude::*;

const SLOTS: usize = 2;
const DIVIDE: Math = Math::TruncatedDivide;
const REMAINDER: Math = Math::TruncatedRemainder(None);

fn int(number: i64) -> Expr {
    Expr::Const(Value::Int(number))
}

fn apply(op: Op, arguments: Vec<Expr>) -> Expr {
    Expr::Apply(op, arguments)
}

fn math(math: Math, left: Expr, right: Expr) -> Expr {
    apply(Op::Math(math), vec![left, right])
}

fn fit(ty: IntType, inner: Expr) -> Expr {
    apply(Op::Fit(ty), vec![inner])
}

#[derive(Clone, Debug, PartialEq)]
struct Gave(Result<Value, Fault>);

fn agreed(expr: &Expr, state: &[Value]) -> Gave {
    let reference = eval(expr, &mut Frame::new(state, SLOTS));
    let prepared = Code::new(expr.clone()).eval(&mut Frame::new(state, SLOTS));
    assert_eq!(
        reference, prepared,
        "the walk and the code agree on {expr:?}"
    );
    Gave(reference)
}

fn of_fields(op: Math, left: i64, right: i64) -> Gave {
    let state = [Value::Int(left), Value::Int(right)];
    agreed(&math(op, Expr::Field(0), Expr::Field(1)), &state)
}

fn every_form(op: Math, left: i64, right: i64) -> Gave {
    let state = [Value::Int(left), Value::Int(right)];
    let forms = [
        math(op, int(left), int(right)),
        math(op, Expr::Field(0), int(right)),
        math(op, int(left), Expr::Field(1)),
        math(op, Expr::Field(0), Expr::Field(1)),
        fit(IntType::I64, math(op, Expr::Field(0), Expr::Field(1))),
        apply(Op::Safe, vec![math(op, Expr::Field(0), int(right))]),
    ];
    let answers: Vec<Gave> = forms.iter().map(|form| agreed(form, &state)).collect();
    let first = answers[0].clone();
    for (form, answer) in forms.iter().zip(&answers).take(5) {
        assert_eq!(*answer, first, "{form:?} gives what the plain numbers give");
    }
    first
}

#[test]
fn division_drops_the_fraction_for_all_four_signs() {
    for (left, right, quotient) in [(7, 2, 3), (-7, 2, -3), (7, -2, -3), (-7, -2, 3)] {
        assert_eq!(
            every_form(DIVIDE, left, right).0,
            Ok(Value::Int(quotient)),
            "{left} / {right}"
        );
    }
    assert_eq!(every_form(DIVIDE, -1, 3).0, Ok(Value::Int(0)), "-1 / 3");
    assert_eq!(every_form(DIVIDE, 6, -3).0, Ok(Value::Int(-2)), "6 / -3");
}

#[test]
fn the_remainder_has_the_sign_of_the_first_number_for_all_four_signs() {
    for (left, right, remainder) in [(7, 2, 1), (-7, 2, -1), (7, -2, 1), (-7, -2, -1)] {
        assert_eq!(
            every_form(REMAINDER, left, right).0,
            Ok(Value::Int(remainder)),
            "{left} % {right}"
        );
    }
    assert_eq!(every_form(REMAINDER, -6, 3).0, Ok(Value::Int(0)), "-6 % 3");
}

#[test]
fn a_divisor_of_zero_is_the_fault_of_a_division_by_zero() {
    for op in [
        DIVIDE,
        REMAINDER,
        Math::TruncatedRemainder(Some(IntType::I8)),
    ] {
        for left in [-7, 0, 7] {
            assert_eq!(
                every_form(op, left, 0).0,
                Err(Fault::ByZero),
                "{op:?} {left} by 0"
            );
        }
    }
    assert_eq!(
        Fault::ByZero.describe(),
        "takes a division or a remainder by zero"
    );
}

#[test]
fn the_smallest_number_over_minus_one_is_the_fault_rust_panics_with() {
    assert_eq!(of_fields(DIVIDE, i64::MIN, -1).0, Err(Fault::Overflow));
    assert_eq!(of_fields(REMAINDER, i64::MIN, -1).0, Err(Fault::Overflow));
    assert_eq!(
        of_fields(Math::TruncatedRemainder(Some(IntType::I64)), i64::MIN, -1).0,
        Err(Fault::Overflow)
    );
    let state = [Value::Int(-128), Value::Int(-1)];
    let quotient = fit(IntType::I8, math(DIVIDE, Expr::Field(0), Expr::Field(1)));
    assert_eq!(
        agreed(&quotient, &state).0,
        Err(Fault::Outside(IntType::I8))
    );
    for ty in [IntType::I8, IntType::I16, IntType::I32] {
        let lowest = ty.lowest();
        let remainder = fit(
            ty,
            math(
                Math::TruncatedRemainder(Some(ty)),
                Expr::Field(0),
                Expr::Field(1),
            ),
        );
        let state = [Value::Int(lowest), Value::Int(-1)];
        assert_eq!(
            agreed(&remainder, &state).0,
            Err(Fault::Outside(ty)),
            "{ty:?}"
        );
        let state = [Value::Int(lowest + 1), Value::Int(-1)];
        assert_eq!(agreed(&remainder, &state).0, Ok(Value::Int(0)), "{ty:?}");
        let state = [Value::Int(lowest), Value::Int(1)];
        assert_eq!(agreed(&remainder, &state).0, Ok(Value::Int(0)), "{ty:?}");
    }
}

#[test]
fn one_division_floors_in_a_quint_frame_and_truncates_in_a_native_one() {
    let state = [Value::Int(-7), Value::Int(2)];
    let quint = Notation::Quint(Quint::default());
    let floored = math(Math::Divide, Expr::Field(0), Expr::Field(1));
    let walked = eval(&floored, &mut Frame::new(&state, SLOTS).numbers_of(quint));
    let prepared = Code::new(floored)
        .of_any_size()
        .eval(&mut Frame::new(&state, SLOTS));
    assert_eq!((walked, prepared), (Ok(Value::Int(-4)), Ok(Value::Int(-4))));
    let truncated = math(DIVIDE, Expr::Field(0), Expr::Field(1));
    assert_eq!(agreed(&truncated, &state).0, Ok(Value::Int(-3)));
    let prepared = Code::new(truncated)
        .of_any_size()
        .eval(&mut Frame::new(&state, SLOTS));
    assert_eq!(
        prepared,
        Ok(Value::Int(-3)),
        "numbers of any size do not floor it"
    );
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(2048))]

    #[test]
    fn the_code_and_the_walk_give_what_rust_gives(
        left in prop_oneof![Just(i64::MIN), Just(i64::MAX), -300_i64..300, any::<i64>()],
        right in prop_oneof![Just(0_i64), Just(-1), Just(1), -20_i64..20, any::<i64>()],
    ) {
        let quotient = left.checked_div(right).ok_or(if right == 0 {
            Fault::ByZero
        } else {
            Fault::Overflow
        });
        let remainder = left.checked_rem(right).ok_or(if right == 0 {
            Fault::ByZero
        } else {
            Fault::Overflow
        });
        prop_assert_eq!(every_form(DIVIDE, left, right).0, quotient.map(Value::Int));
        prop_assert_eq!(every_form(REMAINDER, left, right).0, remainder.map(Value::Int));
        if let (Ok(quotient), Ok(remainder)) = (quotient, remainder) {
            prop_assert_eq!(right * quotient + remainder, left);
        }
    }

    #[test]
    fn an_eight_bit_field_faults_where_rust_panics(left in -128_i64..128, right in -128_i64..128) {
        let state = [Value::Int(left), Value::Int(right)];
        let ty = IntType::I8;
        let narrow = |answer: Option<i8>| -> Result<Value, Fault> {
            match answer {
                Some(answer) => Ok(Value::Int(i64::from(answer))),
                None if right == 0 => Err(Fault::ByZero),
                None => Err(Fault::Outside(ty)),
            }
        };
        let (Ok(left8), Ok(right8)) = (i8::try_from(left), i8::try_from(right)) else {
            return Err(TestCaseError::fail("both fit in i8"));
        };
        let quotient = fit(ty, math(DIVIDE, Expr::Field(0), Expr::Field(1)));
        let remainder = fit(ty, math(Math::TruncatedRemainder(Some(ty)), Expr::Field(0), Expr::Field(1)));
        prop_assert_eq!(agreed(&quotient, &state).0, narrow(left8.checked_div(right8)));
        prop_assert_eq!(agreed(&remainder, &state).0, narrow(left8.checked_rem(right8)));
    }
}
