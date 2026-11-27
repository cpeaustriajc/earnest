//! Whole numbers of any size, as a Quint model has them: a prepared [`Code`] made with
//! [`Code::of_any_size`] and the reference walk in a frame of Quint's numbers give the number
//! exact arithmetic gives, where a plain code gives [`Fault::Overflow`].

use earnest_ir::{Code, Expr, Fault, Frame, Math, Op, Order, SeqOp, eval, truth};
use earnest_value::{BigInt, Notation, Quint, Value};
use proptest::prelude::*;
use std::collections::BTreeSet;

const SLOTS: usize = 2;

fn quint() -> Notation {
    Notation::Quint(Quint::default())
}

fn number(text: &str) -> Value {
    Value::whole(text.parse().expect("digits"))
}

fn apply(op: Op, arguments: Vec<Expr>) -> Expr {
    Expr::Apply(op, arguments)
}

fn constant(value: Value) -> Expr {
    Expr::Const(value)
}

#[derive(Debug)]
struct Gave(Result<Value, Fault>);

fn both(expr: &Expr, state: &[Value]) -> Gave {
    let reference = eval(expr, &mut Frame::new(state, SLOTS).numbers_of(quint()));
    let prepared = Code::new(expr.clone())
        .of_any_size()
        .eval(&mut Frame::new(state, SLOTS));
    assert_eq!(
        reference, prepared,
        "the walk and the code agree on {expr:?}"
    );
    let tested = Code::new(expr.clone())
        .of_any_size()
        .scalar(&mut Frame::new(state, SLOTS));
    assert_eq!(
        tested.is_err(),
        reference.is_err(),
        "`scalar` faults exactly when `eval` does on {expr:?}"
    );
    Gave(reference)
}

fn exact(math: Math, left: i64, right: i64) -> Option<BigInt> {
    let (left, right) = (BigInt::from(left), BigInt::from(right));
    let negative = left < BigInt::ZERO || right < BigInt::from(1);
    match math {
        Math::Add => Some(left + right),
        Math::Subtract => Some(left - right),
        Math::Multiply => Some(left * right),
        Math::Remainder | Math::Divide if negative => None,
        Math::Remainder => Some(left % right),
        Math::Divide => Some(left / right),
        _ => None,
    }
}

fn maths() -> impl Strategy<Value = Math> {
    prop_oneof![
        Just(Math::Add),
        Just(Math::Subtract),
        Just(Math::Multiply),
        Just(Math::Remainder),
        Just(Math::Divide),
    ]
}

fn edges() -> impl Strategy<Value = i64> {
    prop_oneof![
        any::<i64>(),
        Just(i64::MAX),
        Just(i64::MIN),
        Just(i64::MAX - 1),
        Just(-1_i64),
        Just(0_i64),
        Just(2_i64),
    ]
}

proptest! {
    #[test]
    fn arithmetic_on_fields_gives_the_exact_number(
        math in maths(),
        left in edges(),
        right in edges(),
    ) {
        let expr = apply(Op::Math(math), vec![Expr::Field(0), Expr::Field(1)]);
        let state = [Value::Int(left), Value::Int(right)];
        let found = both(&expr, &state).0;
        match exact(math, left, right) {
            Some(answer) => prop_assert_eq!(found, Ok(Value::whole(answer))),
            None => prop_assert!(found.is_err(), "{found:?}"),
        }
    }

    #[test]
    fn arithmetic_on_numbers_past_64_bits_gives_the_exact_number(
        math in maths(),
        left in edges(),
        right in edges(),
    ) {
        let wide = BigInt::from(i64::MAX) * 4;
        let expr = apply(Op::Math(math), vec![Expr::Field(0), Expr::Field(1)]);
        let state = [Value::whole(&wide + left), Value::whole(&wide + right)];
        let found = both(&expr, &state).0;
        let (first, second) = (&wide + left, &wide + right);
        let answer = match math {
            Math::Add => first + second,
            Math::Subtract => first - second,
            Math::Multiply => first * second,
            Math::Remainder => first % second,
            _ => first / second,
        };
        prop_assert_eq!(found, Ok(Value::whole(answer)));
    }

    #[test]
    fn a_plain_code_still_faults_past_64_bits(left in edges(), right in edges()) {
        let expr = apply(Op::Math(Math::Multiply), vec![Expr::Field(0), Expr::Field(1)]);
        let state = [Value::Int(left), Value::Int(right)];
        let found = Code::new(expr).eval(&mut Frame::new(&state, SLOTS));
        let expected = left.checked_mul(right).map(Value::Int).ok_or(Fault::Overflow);
        prop_assert_eq!(found, expected);
    }

    #[test]
    fn numbers_of_any_size_sort_as_numbers(left in edges(), right in edges(), wide in 0_u32..3) {
        let scale: BigInt = BigInt::from(i64::MAX) * BigInt::from(wide) + 1;
        let first: BigInt = BigInt::from(left) * &scale;
        let second: BigInt = BigInt::from(right) * &scale;
        prop_assert_eq!(
            Value::whole(first.clone()).cmp(&Value::whole(second.clone())),
            first.cmp(&second)
        );
        let expr = apply(
            Op::Order(Order::Less),
            vec![constant(Value::whole(first.clone())), Expr::Field(0)],
        );
        let state = [Value::whole(second.clone())];
        prop_assert_eq!(both(&expr, &state).0, Ok(Value::Bool(first < second)));
        prop_assert_eq!(
            Code::new(expr).of_any_size().truth(&mut Frame::new(&state, SLOTS)),
            Ok(first < second)
        );
    }
}

#[test]
fn a_number_has_one_value_whatever_built_it() {
    assert_eq!(Value::whole(BigInt::from(7)), Value::Int(7));
    assert_eq!(number("9223372036854775807"), Value::Int(i64::MAX));
    assert!(matches!(number("9223372036854775808"), Value::Big(_)));
    let sum = apply(
        Op::Math(Math::Add),
        vec![constant(Value::Int(i64::MAX)), constant(Value::Int(1))],
    );
    let back = apply(Op::Math(Math::Subtract), vec![sum, constant(Value::Int(1))]);
    assert_eq!(both(&back, &[]).0, Ok(Value::Int(i64::MAX)));
}

#[test]
fn negation_and_powers_leave_64_bits() {
    let negated = apply(Op::Negate, vec![constant(Value::Int(i64::MIN))]);
    assert_eq!(both(&negated, &[]).0, Ok(number("9223372036854775808")));
    let power = apply(
        Op::Power,
        vec![constant(Value::Int(2)), constant(Value::Int(70))],
    );
    assert_eq!(both(&power, &[]).0, Ok(number("1180591620717411303424")));
    let huge = apply(
        Op::Power,
        vec![constant(Value::Int(2)), constant(number("4294967296"))],
    );
    assert_eq!(both(&huge, &[]).0, Err(Fault::Overflow));
    let negative = apply(
        Op::Power,
        vec![constant(Value::Int(2)), constant(Value::Int(-1))],
    );
    assert_eq!(both(&negative, &[]).0, Err(Fault::Undefined));
}

#[test]
fn division_with_a_negative_number_past_64_bits_stays_a_fault() {
    for (math, fault) in [
        (Math::Divide, Fault::Division),
        (Math::Remainder, Fault::Remainder),
    ] {
        let expr = apply(
            Op::Math(math),
            vec![
                constant(number("-18446744073709551616")),
                constant(Value::Int(3)),
            ],
        );
        assert_eq!(both(&expr, &[]).0, Err(fault));
    }
}

#[test]
fn a_range_between_ends_past_64_bits_holds_its_numbers_and_stays_capped() {
    let lo = number("18446744073709551616");
    let hi = number("18446744073709551618");
    let range = apply(Op::Interval, vec![constant(lo.clone()), constant(hi)]);
    let expected: BTreeSet<Value> = [
        "18446744073709551616",
        "18446744073709551617",
        "18446744073709551618",
    ]
    .into_iter()
    .map(number)
    .collect();
    assert_eq!(both(&range, &[]).0, Ok(Value::set(expected)));
    let wide = apply(
        Op::Interval,
        vec![constant(lo), constant(number("18446744073709551616000"))],
    );
    assert_eq!(both(&wide, &[]).0, Err(Fault::TooWide));
    let across = apply(
        Op::Ascending,
        vec![
            constant(Value::Int(i64::MAX)),
            constant(number("9223372036854775809")),
        ],
    );
    assert_eq!(
        both(&across, &[]).0,
        Ok(Value::seq(vec![
            Value::Int(i64::MAX),
            number("9223372036854775808")
        ]))
    );
}

#[test]
fn a_position_past_64_bits_is_not_in_a_list() {
    let list = constant(Value::seq(vec![Value::Int(1)]));
    let nth = apply(
        Op::Seq(SeqOp::Nth),
        vec![list.clone(), constant(number("18446744073709551616"))],
    );
    assert_eq!(both(&nth, &[]).0, Err(Fault::Missing));
    let get = apply(
        Op::Seq(SeqOp::Get),
        vec![list, constant(number("-18446744073709551616"))],
    );
    assert_eq!(both(&get, &[]).0, Ok(Value::Null));
}

#[test]
fn a_number_past_64_bits_equals_only_itself_and_is_beyond_32_bits() {
    let big = number("18446744073709551616");
    let same = apply(Op::Equal, vec![Expr::Field(0), constant(big.clone())]);
    let small = apply(Op::Equal, vec![Expr::Field(0), constant(Value::Int(5))]);
    let state = [big];
    assert_eq!(truth(&same, &mut Frame::new(&state, SLOTS)), Ok(true));
    assert_eq!(
        Code::new(small)
            .of_any_size()
            .truth(&mut Frame::new(&state, SLOTS)),
        Ok(false)
    );
    let doubled = apply(
        Op::Math(Math::Multiply),
        vec![constant(Value::Int(i64::MAX)), constant(Value::Int(2))],
    );
    let mut frame = Frame::new(&[], SLOTS);
    let found = Code::new(doubled).of_any_size().eval(&mut frame);
    assert_eq!(found, Ok(number("18446744073709551614")));
    assert!(
        frame.computed_beyond_32_bits(),
        "TLC can't hold the number, so `confirm` is told"
    );
}
