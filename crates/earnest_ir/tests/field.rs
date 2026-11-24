//! A comparison of a field with a constant, an `and` of such comparisons and a field changed by a
//! constant, each in the prepared form that evaluates it in one step, must give what the
//! reference walk gives: the same truth, number or fault, and a note of numbers beyond 32 bits
//! exactly where the walk notes one.

use earnest_ir::{Code, Expr, Fault, Frame, Math, Op, Order, eval, truth};
use earnest_value::{IntType, MOST_SAFE_INTEGER, Value};

const SLOTS: usize = 2;
const PAST_32_BITS: i64 = 1 << 31;

fn state() -> Vec<Value> {
    vec![
        Value::Int(5),
        Value::Int(-7),
        Value::Int(0),
        Value::Int(PAST_32_BITS),
        Value::Int(i64::MAX),
        Value::Int(i64::MIN),
        Value::Int(MOST_SAFE_INTEGER),
        Value::Bool(true),
        Value::Null,
        Value::Int(9),
    ]
}

fn fields() -> impl Iterator<Item = usize> {
    0..=state().len()
}

fn constants() -> Vec<i64> {
    vec![
        0,
        1,
        -1,
        2,
        5,
        9,
        10,
        -7,
        -8,
        PAST_32_BITS - 1,
        PAST_32_BITS,
        -PAST_32_BITS - 1,
        MOST_SAFE_INTEGER,
        MOST_SAFE_INTEGER + 1,
        i64::MAX,
        i64::MAX - 1,
        i64::MIN,
        i64::MIN + 1,
    ]
}

const ORDERS: [Order; 4] = [
    Order::Less,
    Order::LessOrEqual,
    Order::Greater,
    Order::GreaterOrEqual,
];

const MATHS: [Math; 5] = [
    Math::Add,
    Math::Subtract,
    Math::Multiply,
    Math::Divide,
    Math::Remainder,
];

const TYPES: [IntType; 12] = [
    IntType::U8,
    IntType::U16,
    IntType::U32,
    IntType::U64,
    IntType::U128,
    IntType::Usize,
    IntType::I8,
    IntType::I16,
    IntType::I32,
    IntType::I64,
    IntType::I128,
    IntType::Isize,
];

fn int(number: i64) -> Expr {
    Expr::Const(Value::Int(number))
}

fn compared(order: Order, field: usize, number: i64) -> Expr {
    Expr::Apply(Op::Order(order), vec![Expr::Field(field), int(number)])
}

fn every_comparison() -> Vec<Expr> {
    let mut all = Vec::new();
    for order in ORDERS {
        for field in fields() {
            for number in constants() {
                all.push(compared(order, field, number));
            }
        }
    }
    all
}

fn and(parts: &[Expr]) -> Expr {
    parts
        .iter()
        .cloned()
        .reduce(|left, right| Expr::And(Box::new(left), Box::new(right)))
        .expect("at least one part")
}

struct Tested {
    answer: Result<bool, Fault>,
    beyond_32_bits: bool,
}

fn tested(test: impl FnOnce(&mut Frame<'_>) -> Result<bool, Fault>) -> Tested {
    let state = state();
    let mut frame = Frame::new(&state, SLOTS);
    let answer = test(&mut frame);
    Tested {
        answer,
        beyond_32_bits: frame.computed_beyond_32_bits(),
    }
}

fn agrees_as_truth(expr: &Expr) {
    let code = Code::new(expr.clone());
    let prepared = tested(|frame| code.truth(frame));
    let reference = tested(|frame| truth(expr, frame));
    assert_eq!(prepared.answer, reference.answer, "the truth of {expr:?}");
    assert!(
        !prepared.beyond_32_bits && !reference.beyond_32_bits,
        "a comparison notes nothing: {expr:?}"
    );
}

#[test]
fn a_comparison_of_a_field_with_a_constant_tells_what_the_walk_tells() {
    for expr in every_comparison() {
        agrees_as_truth(&expr);
    }
}

#[test]
fn an_and_of_comparisons_with_constants_stops_where_the_walk_stops() {
    let picked: Vec<Expr> = [
        (Order::Less, 0, 9),
        (Order::LessOrEqual, 9, 9),
        (Order::Greater, 1, -8),
        (Order::GreaterOrEqual, 2, 1),
        (Order::Less, 7, 3),
        (Order::LessOrEqual, 8, 3),
        (Order::Greater, 10, 3),
        (Order::Less, 4, i64::MAX),
        (Order::GreaterOrEqual, 5, i64::MIN),
        (Order::Less, 3, i64::MIN),
        (Order::Greater, 4, i64::MAX),
    ]
    .into_iter()
    .map(|(order, field, number)| compared(order, field, number))
    .collect();
    for first in &picked {
        for second in &picked {
            agrees_as_truth(&and(&[first.clone(), second.clone()]));
            for third in &picked {
                agrees_as_truth(&and(&[first.clone(), second.clone(), third.clone()]));
            }
        }
    }
}

#[test]
fn an_and_that_mixes_comparisons_with_other_tests_tells_what_the_walk_tells() {
    let other = Expr::Apply(
        Op::Equal,
        vec![Expr::Field(7), Expr::Const(Value::Bool(true))],
    );
    for comparison in [compared(Order::Less, 0, 9), compared(Order::Less, 0, 2)] {
        agrees_as_truth(&and(&[other.clone(), comparison.clone()]));
        agrees_as_truth(&and(&[comparison.clone(), other.clone(), comparison]));
    }
}

fn wrapped(expr: &Expr) -> Vec<Expr> {
    let mut all = vec![expr.clone(), Expr::Apply(Op::Safe, vec![expr.clone()])];
    all.extend(
        TYPES
            .iter()
            .map(|ty| Expr::Apply(Op::Fit(*ty), vec![expr.clone()])),
    );
    all
}

#[test]
fn a_field_changed_by_a_constant_gives_what_the_walk_gives() {
    let state = state();
    for math in MATHS {
        for field in fields() {
            for number in constants() {
                let changed = Expr::Apply(Op::Math(math), vec![Expr::Field(field), int(number)]);
                for expr in wrapped(&changed) {
                    let mut prepared = Frame::new(&state, SLOTS);
                    let answer = Code::new(expr.clone()).eval(&mut prepared);
                    let mut walked = Frame::new(&state, SLOTS);
                    let reference = eval(&expr, &mut walked);
                    assert_eq!(answer, reference, "the answer of {expr:?}");
                    assert_eq!(
                        prepared.computed_beyond_32_bits(),
                        walked.computed_beyond_32_bits(),
                        "the note of {expr:?}"
                    );
                }
            }
        }
    }
}
