//! A number expression whose operations each have a constant, a field or a local as an operand
//! is evaluated as a run of steps over one number. It must give what the reference walk gives:
//! the same value or the same fault and the same note of numbers beyond 32 bits, through
//! `eval`, `scalar` and `truth`, and inside a comparison or an equality.

use earnest_ir::{Code, Expr, Fault, Frame, Math, Op, Order, Scalar, eval, truth};
use earnest_value::{IntType, MOST_SAFE_INTEGER, Value};

const SLOTS: usize = 4;
const BEYOND_SAFE: i64 = MOST_SAFE_INTEGER + 1;
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
        Value::Int(100),
    ]
}

fn locals() -> Vec<Value> {
    vec![
        Value::Int(3),
        Value::Int(1 << 40),
        Value::Null,
        Value::Bool(false),
    ]
}

fn leaves() -> Vec<Expr> {
    let mut all: Vec<Expr> = [
        0,
        1,
        -1,
        2,
        7,
        100,
        PAST_32_BITS - 1,
        PAST_32_BITS,
        -PAST_32_BITS - 1,
        MOST_SAFE_INTEGER,
        BEYOND_SAFE,
        i64::MAX,
        i64::MIN,
    ]
    .into_iter()
    .map(int)
    .collect();
    all.extend((0..=state().len()).map(Expr::Field));
    all.extend((0..=SLOTS).map(Expr::Local));
    all.push(Expr::Const(Value::Bool(true)));
    all.push(Expr::Const(Value::Null));
    all
}

fn int(number: i64) -> Expr {
    Expr::Const(Value::Int(number))
}

fn apply(op: Op, arguments: Vec<Expr>) -> Expr {
    Expr::Apply(op, arguments)
}

const MATHS: [Math; 5] = [
    Math::Add,
    Math::Subtract,
    Math::Multiply,
    Math::Divide,
    Math::Remainder,
];

fn unary_ops() -> [Op; 5] {
    [
        Op::Safe,
        Op::Fit(IntType::U8),
        Op::Fit(IntType::I32),
        Op::Fit(IntType::U64),
        Op::Negate,
    ]
}

struct Ran<T> {
    answer: Result<T, Fault>,
    beyond_32_bits: bool,
}

fn ran<T>(evaluate: impl FnOnce(&mut Frame<'_>) -> Result<T, Fault>) -> Ran<T> {
    let state = state();
    let mut frame = Frame::new(&state, SLOTS);
    for (slot, value) in locals().into_iter().enumerate() {
        frame.put(slot, value);
    }
    let answer = evaluate(&mut frame);
    Ran {
        answer,
        beyond_32_bits: frame.computed_beyond_32_bits(),
    }
}

fn scalar_of(value: &Value) -> Scalar {
    match value {
        Value::Int(number) => Scalar::Int(*number),
        Value::Bool(flag) => Scalar::Bool(*flag),
        _ => Scalar::Other,
    }
}

fn agrees(expr: &Expr) {
    let code = Code::new(expr.clone());
    let reference = ran(|frame| eval(expr, frame));
    let prepared = ran(|frame| code.eval(frame));
    assert_eq!(prepared.answer, reference.answer, "the value of {expr:?}");
    assert_eq!(
        prepared.beyond_32_bits, reference.beyond_32_bits,
        "the note of {expr:?}"
    );
    let scalar = ran(|frame| code.scalar(frame));
    assert_eq!(
        scalar.answer,
        reference
            .answer
            .as_ref()
            .map(scalar_of)
            .map_err(|fault| *fault),
        "the scalar of {expr:?}"
    );
    assert_eq!(
        scalar.beyond_32_bits, reference.beyond_32_bits,
        "the note of the scalar of {expr:?}"
    );
}

fn agrees_as_truth(expr: &Expr) {
    let code = Code::new(expr.clone());
    let reference = ran(|frame| truth(expr, frame));
    let prepared = ran(|frame| code.truth(frame));
    assert_eq!(prepared.answer, reference.answer, "the truth of {expr:?}");
    assert_eq!(
        prepared.beyond_32_bits, reference.beyond_32_bits,
        "the note of the truth of {expr:?}"
    );
    agrees(expr);
}

fn agrees_inside_conditions(expr: &Expr) {
    agrees(expr);
    for other in [int(3), Expr::Local(0), Expr::Field(7)] {
        agrees_as_truth(&apply(Op::Equal, vec![expr.clone(), other.clone()]));
        agrees_as_truth(&apply(Op::Differ, vec![other.clone(), expr.clone()]));
        agrees_as_truth(&apply(
            Op::Order(Order::Less),
            vec![expr.clone(), other.clone()],
        ));
        agrees_as_truth(&apply(
            Op::Order(Order::GreaterOrEqual),
            vec![other, expr.clone()],
        ));
    }
}

struct Shuffle(u64);

impl Shuffle {
    fn below(&mut self, count: usize) -> usize {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        usize::try_from(self.0 % u64::try_from(count).expect("a small count"))
            .expect("below the count")
    }

    fn pick<T: Clone>(&mut self, all: &[T]) -> T {
        all[self.below(all.len())].clone()
    }
}

fn grown(shuffle: &mut Shuffle, depth: usize, leaves: &[Expr]) -> Expr {
    if depth == 0 {
        return shuffle.pick(leaves);
    }
    let inner = grown(shuffle, depth - 1, leaves);
    match shuffle.below(4) {
        0 => apply(shuffle.pick(&unary_ops()), vec![inner]),
        1 => apply(
            Op::Math(shuffle.pick(&MATHS)),
            vec![shuffle.pick(leaves), inner],
        ),
        2 => apply(
            Op::Math(shuffle.pick(&MATHS)),
            vec![inner, grown(shuffle, depth / 2, leaves)],
        ),
        _ => apply(
            Op::Math(shuffle.pick(&MATHS)),
            vec![inner, shuffle.pick(leaves)],
        ),
    }
}

#[test]
fn every_operation_of_two_leaves_gives_what_the_walk_gives() {
    let leaves = leaves();
    for math in MATHS {
        for left in &leaves {
            for right in &leaves {
                agrees(&apply(Op::Math(math), vec![left.clone(), right.clone()]));
            }
        }
    }
}

#[test]
fn every_bound_and_negation_of_a_leaf_or_an_operation_gives_what_the_walk_gives() {
    let leaves = leaves();
    for op in unary_ops() {
        for leaf in &leaves {
            agrees(&apply(op, vec![leaf.clone()]));
            for math in MATHS {
                for other in [int(1), int(2), Expr::Field(3), Expr::Local(1)] {
                    let inner = apply(Op::Math(math), vec![leaf.clone(), other.clone()]);
                    agrees(&apply(op, vec![inner.clone()]));
                    agrees(&apply(op, vec![apply(op, vec![inner])]));
                    let reversed = apply(Op::Math(math), vec![other, leaf.clone()]);
                    agrees(&apply(op, vec![reversed]));
                }
            }
        }
    }
}

#[test]
fn a_remainder_or_a_quotient_of_a_sum_by_a_constant_gives_what_the_walk_gives() {
    let divisors = [
        -3,
        -1,
        0,
        1,
        2,
        3,
        7,
        64,
        100,
        PAST_32_BITS + 1,
        MOST_SAFE_INTEGER,
        i64::MAX,
    ];
    for field in 0..=state().len() {
        for slot in 0..=SLOTS {
            let sum = apply(
                Op::Safe,
                vec![apply(
                    Op::Math(Math::Add),
                    vec![Expr::Field(field), Expr::Local(slot)],
                )],
            );
            for divisor in divisors {
                for math in [Math::Remainder, Math::Divide] {
                    let divided = apply(Op::Math(math), vec![sum.clone(), int(divisor)]);
                    agrees(&apply(Op::Safe, vec![divided.clone()]));
                    agrees(&apply(Op::Fit(IntType::U8), vec![divided]));
                    agrees(&apply(Op::Math(math), vec![int(divisor), sum.clone()]));
                }
            }
        }
    }
}

#[test]
fn operations_nested_three_and_four_deep_give_what_the_walk_gives() {
    let leaves = leaves();
    let mut shuffle = Shuffle(0x9E37_79B9_7F4A_7C15);
    for _ in 0..4_000 {
        let depth = 2 + shuffle.below(3);
        agrees_inside_conditions(&grown(&mut shuffle, depth, &leaves));
    }
}

#[test]
fn a_wheel_that_turns_by_a_sum_and_a_remainder_gives_what_the_walk_gives() {
    let sum = |left: Expr, right: Expr| {
        apply(
            Op::Safe,
            vec![apply(Op::Math(Math::Add), vec![left, right])],
        )
    };
    let hundred = |inner: Expr| {
        apply(
            Op::Safe,
            vec![apply(Op::Math(Math::Remainder), vec![inner, int(100)])],
        )
    };
    for field in 0..=state().len() {
        for slot in 0..=SLOTS {
            agrees_inside_conditions(&hundred(sum(Expr::Field(field), Expr::Local(slot))));
            agrees_inside_conditions(&hundred(sum(
                sum(Expr::Field(field), Expr::Local(slot)),
                Expr::Local(0),
            )));
        }
    }
}
