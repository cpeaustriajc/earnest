//! An update of a field by a constant gives its answer through `FieldStep` without a frame. It
//! must give an answer only where the reference walk gives that number without a fault and
//! without a note of numbers beyond 32 bits.

use earnest_ir::{Code, Expr, Frame, Math, Op, Order, eval};
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
fn a_field_step_gives_only_answers_the_walk_gives_without_a_fault_or_a_note() {
    let state = state();
    let (mut given, mut left_to_the_walk) = (0, 0);
    for math in MATHS {
        for field in fields() {
            for number in constants() {
                let changed = Expr::Apply(Op::Math(math), vec![Expr::Field(field), int(number)]);
                for expr in wrapped(&changed) {
                    let code = Code::new(expr.clone());
                    let step = code
                        .field_step()
                        .unwrap_or_else(|| panic!("a step for {expr:?}"));
                    let mut frame = Frame::new(&state, SLOTS);
                    let reference = eval(&expr, &mut frame);
                    let noted = frame.computed_beyond_32_bits();
                    if let Some(answer) = step.apply(&state) {
                        given += 1;
                        assert_eq!(reference, Ok(Value::Int(answer)), "the answer of {expr:?}");
                        assert!(!noted, "an answer the walk notes for {expr:?}");
                    } else {
                        left_to_the_walk += 1;
                        assert!(
                            reference.is_err() || noted,
                            "{expr:?} gives {reference:?} without a note, and the step gave none"
                        );
                    }
                }
            }
        }
    }
    assert!(
        given > 0 && left_to_the_walk > 0,
        "both kinds of case are met"
    );
}

#[test]
fn only_a_field_changed_by_a_constant_is_a_field_step() {
    let not_steps = [
        Expr::Field(0),
        int(3),
        Expr::Apply(Op::Math(Math::Add), vec![int(3), Expr::Field(0)]),
        Expr::Apply(Op::Math(Math::Add), vec![Expr::Field(0), Expr::Field(1)]),
        Expr::Apply(Op::Math(Math::Add), vec![Expr::Field(0), Expr::Local(0)]),
        Expr::Apply(
            Op::Math(Math::Add),
            vec![
                Expr::Apply(Op::Math(Math::Add), vec![Expr::Field(0), int(1)]),
                int(1),
            ],
        ),
        Expr::Apply(Op::Negate, vec![Expr::Field(0)]),
        compared(Order::Less, 0, 9),
    ];
    for expr in not_steps {
        assert!(
            Code::new(expr.clone()).field_step().is_none(),
            "no step for {expr:?}"
        );
    }
}
