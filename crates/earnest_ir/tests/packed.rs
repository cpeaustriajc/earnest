//! A guard, a rule, an end or an update prepared for packed rows must give what the reference
//! walk gives for the state the row holds: a test gives the walk's truth exactly when the walk
//! gives it without a fault and without a note of numbers beyond 32 bits, and an update writes
//! the code of the walk's answer exactly when the walk gives it so and it is a value of the
//! field. The rows are packed here by hand, and the fields sit at the start, the middle and the
//! end of a word and in a second word.

use earnest_ir::{
    Code, Coding, Expr, Frame, Math, Op, Order, SeqOp, Shelf, Slot, Sweep, eval, truth,
};
use earnest_value::{IntType, Symbols, Value};

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

const BOUNDS: [Option<IntType>; 4] = [
    None,
    Some(IntType::I8),
    Some(IntType::U8),
    Some(IntType::I64),
];

const PAST_32_BITS: i64 = 1 << 31;
const WIDE: i64 = 1 << 40;
const LOCAL_SLOTS: usize = 8;

#[derive(Clone, Debug)]
struct Field {
    coding: Coding,
    samples: Vec<Value>,
}

fn whole(lowest: i64, highest: i64) -> Field {
    let span = highest.abs_diff(lowest);
    let samples = if span < 16 {
        (0..=span)
            .map(|step| Value::Int(lowest.wrapping_add_unsigned(step)))
            .collect()
    } else {
        let middle = lowest.wrapping_add_unsigned(span / 2);
        [lowest, lowest + 1, middle, highest - 1, highest]
            .into_iter()
            .map(Value::Int)
            .collect()
    };
    Field {
        coding: Coding::Whole { lowest, highest },
        samples,
    }
}

fn truth_field() -> Field {
    Field {
        coding: Coding::Truth,
        samples: vec![Value::Bool(false), Value::Bool(true)],
    }
}

fn listed(values: Vec<Value>) -> Field {
    Field {
        coding: Coding::Listed(values.clone()),
        samples: values,
    }
}

fn lights(symbols: &mut Symbols) -> Vec<Value> {
    ["Red", "Green", "Amber"]
        .iter()
        .map(|name| Value::Str(symbols.intern(name)))
        .collect()
}

fn bits_for(field: &Field) -> u32 {
    let count = match &field.coding {
        Coding::Whole { lowest, highest } => u128::from(highest.abs_diff(*lowest)) + 1,
        Coding::Truth => 2,
        Coding::Listed(values) => u128::try_from(values.len()).expect("a count"),
        _ => unreachable!("only these codings are tried"),
    };
    if count <= 1 {
        0
    } else {
        (count - 1).bit_width()
    }
}

fn laid_out(fields: &[Field], first_shift: u32) -> Vec<Option<Slot>> {
    let mut word = 0;
    let mut used = first_shift;
    fields
        .iter()
        .map(|field| {
            let width = bits_for(field);
            if used > 0 && used + width > u64::BITS {
                word += 1;
                used = 0;
            }
            let slot = Slot {
                word,
                shift: used,
                width,
                coding: field.coding.clone(),
            };
            used += width;
            Some(slot)
        })
        .collect()
}

fn code_of(slot: &Slot, value: &Value) -> u64 {
    match (&slot.coding, value) {
        (Coding::Whole { lowest, .. }, Value::Int(number)) => number.abs_diff(*lowest),
        (Coding::Truth, Value::Bool(flag)) => u64::from(*flag),
        (Coding::Listed(values), found) => u64::try_from(
            values
                .iter()
                .position(|value| value == found)
                .expect("listed"),
        )
        .expect("a code"),
        _ => unreachable!("a value of the field"),
    }
}

fn packed(slots: &[Option<Slot>], state: &[Value], garbage: u64) -> Vec<u64> {
    let words = slots
        .iter()
        .flatten()
        .map(|slot| slot.word + 1)
        .max()
        .unwrap_or(0);
    let mut row = vec![garbage; words];
    for (slot, value) in slots.iter().flatten().zip(state) {
        let mask = if slot.width == 0 {
            0
        } else {
            u64::MAX >> (u64::BITS - slot.width)
        };
        let placed = mask.checked_shl(slot.shift).unwrap_or(0);
        let code = code_of(slot, value).checked_shl(slot.shift).unwrap_or(0);
        row[slot.word] = (row[slot.word] & !placed) | (code & placed);
    }
    row
}

fn unpacked(slot: &Slot, row: &[u64]) -> Value {
    let mask = if slot.width == 0 {
        0
    } else {
        u64::MAX >> (u64::BITS - slot.width)
    };
    let code = row[slot.word].checked_shr(slot.shift).unwrap_or(0) & mask;
    match &slot.coding {
        Coding::Whole { lowest, .. } => Value::Int(lowest.wrapping_add_unsigned(code)),
        Coding::Truth => Value::Bool(code == 1),
        Coding::Listed(values) => usize::try_from(code)
            .ok()
            .and_then(|at| values.get(at))
            .cloned()
            .unwrap_or(Value::Null),
        _ => unreachable!("only these codings are tried"),
    }
}

struct Seed(u64);

impl Seed {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn below(&mut self, count: usize) -> usize {
        usize::try_from(self.next() % u64::try_from(count).expect("a count")).expect("an index")
    }
}

fn states(fields: &[Field], seed: &mut Seed) -> Vec<Vec<Value>> {
    let total: usize = fields.iter().map(|field| field.samples.len()).product();
    if total <= 400 {
        let mut all = vec![Vec::new()];
        for field in fields {
            all = all
                .into_iter()
                .flat_map(|state| {
                    field.samples.iter().map(move |value| {
                        let mut longer = state.clone();
                        longer.push(value.clone());
                        longer
                    })
                })
                .collect();
        }
        return all;
    }
    (0..400)
        .map(|_| {
            fields
                .iter()
                .map(|field| field.samples[seed.below(field.samples.len())].clone())
                .collect()
        })
        .collect()
}

#[derive(Default)]
struct Tally {
    answered: usize,
    left: usize,
    not_prepared: usize,
}

fn walked_truth(expr: &Expr, state: &[Value]) -> (Result<bool, earnest_ir::Fault>, bool) {
    let mut frame = Frame::new(state, LOCAL_SLOTS);
    let answer = truth(expr, &mut frame);
    (answer, frame.computed_beyond_32_bits())
}

fn walked_value(expr: &Expr, state: &[Value]) -> (Result<Value, earnest_ir::Fault>, bool) {
    let mut frame = Frame::new(state, LOCAL_SLOTS);
    let answer = eval(expr, &mut frame);
    (answer, frame.computed_beyond_32_bits())
}

fn layouts(fields: &[Field]) -> Vec<Vec<Option<Slot>>> {
    let total: u32 = fields.iter().map(bits_for).sum();
    let mut shifts = vec![0, 1, 17];
    if total < u64::BITS {
        shifts.push(u64::BITS - total);
    }
    shifts.push(u64::BITS - 1);
    shifts
        .into_iter()
        .map(|shift| laid_out(fields, shift))
        .collect()
}

fn check_tests(fields: &[Field], exprs: &[Expr], tally: &mut Tally) {
    let mut seed = Seed(0x9E37_79B9_7F4A_7C15);
    let all = states(fields, &mut seed);
    for slots in layouts(fields) {
        for expr in exprs {
            let test = Code::new(expr.clone())
                .packed_test(&slots, &[])
                .unwrap_or_else(|| panic!("a packed test for {expr:?}"));
            for state in &all {
                let row = packed(&slots, state, seed.next());
                let (answer, noted) = walked_truth(expr, state);
                if let Some(found) = test.holds(&row) {
                    tally.answered += 1;
                    assert_eq!(answer, Ok(found), "{expr:?} in {state:?}");
                    assert!(
                        !noted,
                        "{expr:?} in {state:?} notes a number beyond 32 bits"
                    );
                } else {
                    tally.left += 1;
                    assert!(
                        answer.is_err() || noted,
                        "{expr:?} in {state:?} gives {answer:?} plainly, and the row gave nothing"
                    );
                }
            }
        }
    }
}

fn check_updates(fields: &[Field], updates: &[(usize, Expr)], tally: &mut Tally) {
    let mut seed = Seed(0x2545_F491_4F6C_DD1D);
    let all = states(fields, &mut seed);
    for slots in layouts(fields) {
        for (into, expr) in updates {
            let slot = slots[*into].as_ref().expect("a slot");
            let Some(update) = Code::new(expr.clone()).packed_update(&slots, *into, &[]) else {
                let Expr::Const(constant) = expr else {
                    panic!("a packed update of field {into} for {expr:?}");
                };
                assert!(
                    !in_domain(slot, constant),
                    "a constant of the field is prepared"
                );
                tally.not_prepared += 1;
                continue;
            };
            for state in &all {
                let row = packed(&slots, state, seed.next());
                let mut next = row.clone();
                let (answer, noted) = walked_value(expr, state);
                let fits = !noted && answer.as_ref().is_ok_and(|value| in_domain(slot, value));
                let written = update.write(&row, &mut next);
                if let [word] = row.as_slice() {
                    assert_eq!(
                        update.write_word(&row, *word),
                        (written && slot.word == 0).then(|| next[0]),
                        "a row of one word: {expr:?} in {state:?}"
                    );
                }
                if written {
                    tally.answered += 1;
                    assert!(
                        fits,
                        "{expr:?} in {state:?} gives {answer:?}, and the row took it"
                    );
                    assert_eq!(
                        Ok(unpacked(slot, &next)),
                        answer,
                        "{expr:?} into field {into} in {state:?}"
                    );
                    let mut expected = row;
                    let mask = if slot.width == 0 {
                        0
                    } else {
                        u64::MAX >> (u64::BITS - slot.width)
                    };
                    let placed = mask.checked_shl(slot.shift).unwrap_or(0);
                    expected[slot.word] =
                        (expected[slot.word] & !placed) | (next[slot.word] & placed);
                    assert_eq!(next, expected, "only the slot of field {into} changes");
                } else {
                    tally.left += 1;
                    assert!(
                        !fits,
                        "{expr:?} in {state:?} gives {answer:?} plainly, and the row gave nothing"
                    );
                }
            }
        }
    }
}

fn in_domain(slot: &Slot, value: &Value) -> bool {
    match (&slot.coding, value) {
        (Coding::Whole { lowest, highest }, Value::Int(number)) => {
            lowest <= number && number <= highest
        }
        (Coding::Truth, Value::Bool(_)) => true,
        (Coding::Listed(values), found) => values.contains(found),
        _ => false,
    }
}

fn int(number: i64) -> Expr {
    Expr::Const(Value::Int(number))
}

fn apply(op: Op, arguments: Vec<Expr>) -> Expr {
    Expr::Apply(op, arguments)
}

fn math(how: Math, left: Expr, right: Expr) -> Expr {
    apply(Op::Math(how), vec![left, right])
}

fn bounded(bound: Option<IntType>, inner: Expr) -> Expr {
    match bound {
        None => apply(Op::Safe, vec![inner]),
        Some(ty) => apply(Op::Fit(ty), vec![inner]),
    }
}

fn constants_near(field: &Field) -> Vec<i64> {
    let mut all = vec![
        0,
        1,
        -1,
        PAST_32_BITS,
        -PAST_32_BITS - 1,
        i64::MAX,
        i64::MIN,
        WIDE,
    ];
    if let Coding::Whole { lowest, highest } = field.coding {
        for edge in [lowest, highest] {
            all.extend([edge, edge.saturating_add(1), edge.saturating_sub(1)]);
        }
    }
    all.sort_unstable();
    all.dedup();
    all
}

fn whole_fields() -> Vec<Field> {
    vec![
        whole(0, 9),
        whole(-5, 5),
        whole(WIDE, WIDE + 20),
        whole(i64::MIN, i64::MIN + 6),
        whole(i64::MAX - 6, i64::MAX),
        whole(-PAST_32_BITS - 2, -PAST_32_BITS + 3),
        whole(PAST_32_BITS - 3, PAST_32_BITS + 2),
        whole(7, 7),
        whole(-1_000, 1 << 33),
    ]
}

fn comparisons(field: usize, of: &Field) -> Vec<Expr> {
    let mut all = Vec::new();
    for number in constants_near(of) {
        for order in ORDERS {
            all.push(apply(
                Op::Order(order),
                vec![Expr::Field(field), int(number)],
            ));
            all.push(apply(
                Op::Order(order),
                vec![int(number), Expr::Field(field)],
            ));
        }
        for op in [Op::Equal, Op::Differ] {
            all.push(apply(op, vec![Expr::Field(field), int(number)]));
            all.push(apply(op, vec![int(number), Expr::Field(field)]));
        }
    }
    all
}

#[test]
fn a_comparison_of_a_whole_number_field_with_a_constant_tells_what_the_walk_tells() {
    let mut tally = Tally::default();
    for of in whole_fields() {
        let fields = vec![whole(0, 3), of.clone(), truth_field()];
        check_tests(&fields, &comparisons(1, &of), &mut tally);
    }
    assert!(tally.answered > 0, "the row answers");
    assert_eq!(
        tally.left, 0,
        "a comparison with a constant is always answered"
    );
}

#[test]
fn an_and_an_or_a_not_and_a_choice_of_comparisons_tell_what_the_walk_tells() {
    let fields = vec![whole(0, 9), whole(-5, 5), whole(WIDE, WIDE + 20)];
    let parts = [
        apply(Op::Order(Order::Less), vec![Expr::Field(0), int(9)]),
        apply(
            Op::Order(Order::GreaterOrEqual),
            vec![Expr::Field(1), int(0)],
        ),
        apply(Op::Equal, vec![Expr::Field(2), int(WIDE + 3)]),
        apply(Op::Differ, vec![int(4), Expr::Field(0)]),
    ];
    let mut exprs = Vec::new();
    for first in &parts {
        for second in &parts {
            let pair = (Box::new(first.clone()), Box::new(second.clone()));
            exprs.push(Expr::And(pair.0.clone(), pair.1.clone()));
            exprs.push(Expr::Or(pair.0.clone(), pair.1.clone()));
            exprs.push(apply(Op::Not, vec![Expr::And(pair.0.clone(), pair.1)]));
            for third in &parts {
                exprs.push(Expr::Choose(
                    Box::new(first.clone()),
                    Box::new(second.clone()),
                    Box::new(third.clone()),
                ));
                exprs.push(Expr::And(
                    Box::new(Expr::Or(Box::new(first.clone()), Box::new(second.clone()))),
                    Box::new(third.clone()),
                ));
            }
        }
    }
    exprs.push(Expr::Const(Value::Bool(true)));
    exprs.push(Expr::Const(Value::Bool(false)));
    let mut tally = Tally::default();
    check_tests(&fields, &exprs, &mut tally);
    assert_eq!(tally.left, 0, "none of these needs the walk");
}

#[test]
fn a_constant_in_an_and_an_or_a_not_or_a_choice_stops_only_where_the_walk_stops() {
    let fields = vec![whole(0, 3), whole(0, 9)];
    let faulting = apply(
        Op::Order(Order::Less),
        vec![math(Math::Divide, int(10), Expr::Field(0)), int(5)],
    );
    let plain = apply(Op::Order(Order::Less), vec![Expr::Field(1), int(5)]);
    let mut exprs = Vec::new();
    for flag in [true, false] {
        let constant = Expr::Const(Value::Bool(flag));
        for (first, second) in [
            (faulting.clone(), constant.clone()),
            (constant.clone(), faulting.clone()),
            (plain.clone(), constant.clone()),
            (constant.clone(), plain.clone()),
        ] {
            let (first, second) = (Box::new(first), Box::new(second));
            exprs.push(Expr::And(first.clone(), second.clone()));
            exprs.push(Expr::Or(first.clone(), second.clone()));
            exprs.push(Expr::And(
                Box::new(Expr::And(first.clone(), second.clone())),
                Box::new(faulting.clone()),
            ));
            exprs.push(Expr::Or(
                Box::new(Expr::Or(first.clone(), second.clone())),
                Box::new(plain.clone()),
            ));
            exprs.push(Expr::Choose(first, second, Box::new(faulting.clone())));
        }
        exprs.push(apply(Op::Not, vec![constant.clone()]));
        exprs.push(Expr::Choose(
            Box::new(constant),
            Box::new(faulting.clone()),
            Box::new(plain.clone()),
        ));
    }
    let mut tally = Tally::default();
    check_tests(&fields, &exprs, &mut tally);
    assert!(
        tally.answered > 0 && tally.left > 0,
        "both kinds of case are met"
    );
}

#[test]
fn a_comparison_of_two_fields_and_of_arithmetic_tells_what_the_walk_tells() {
    let fields = vec![
        whole(0, 9),
        whole(-5, 5),
        whole(PAST_32_BITS - 3, PAST_32_BITS + 2),
        whole(i64::MAX - 6, i64::MAX),
    ];
    let mut exprs = Vec::new();
    for first in 0..fields.len() {
        for second in 0..fields.len() {
            for order in ORDERS {
                exprs.push(apply(
                    Op::Order(order),
                    vec![Expr::Field(first), Expr::Field(second)],
                ));
            }
            for op in [Op::Equal, Op::Differ] {
                exprs.push(apply(op, vec![Expr::Field(first), Expr::Field(second)]));
            }
            for how in MATHS {
                for number in [0, 1, 3, -2, PAST_32_BITS] {
                    exprs.push(apply(
                        Op::Order(Order::Less),
                        vec![
                            math(how, Expr::Field(first), int(number)),
                            Expr::Field(second),
                        ],
                    ));
                    exprs.push(apply(
                        Op::Equal,
                        vec![math(how, int(number), Expr::Field(first)), int(4)],
                    ));
                }
                exprs.push(apply(
                    Op::Order(Order::GreaterOrEqual),
                    vec![
                        bounded(
                            Some(IntType::I8),
                            math(how, Expr::Field(first), Expr::Field(second)),
                        ),
                        int(0),
                    ],
                ));
            }
        }
    }
    let mut tally = Tally::default();
    check_tests(&fields, &exprs, &mut tally);
    assert!(
        tally.answered > 0 && tally.left > 0,
        "both kinds of case are met"
    );
}

#[test]
fn a_field_of_true_or_false_and_a_field_of_listed_values_tell_what_the_walk_tells() {
    let mut symbols = Symbols::default();
    let colours = lights(&mut symbols);
    let other = Value::Str(symbols.intern("Blue"));
    let fields = vec![
        truth_field(),
        listed(colours.clone()),
        listed(colours.clone()),
        truth_field(),
        listed(vec![Value::Null, Value::Int(3), Value::Bool(true)]),
    ];
    let mut exprs = vec![
        Expr::Field(0),
        apply(Op::Not, vec![Expr::Field(3)]),
        apply(Op::Equal, vec![Expr::Field(1), Expr::Field(2)]),
        apply(Op::Differ, vec![Expr::Field(2), Expr::Field(1)]),
        apply(Op::Equal, vec![Expr::Field(0), Expr::Field(3)]),
    ];
    let mut constants: Vec<Value> = colours;
    constants.extend([
        other,
        Value::Bool(true),
        Value::Bool(false),
        Value::Null,
        Value::Int(3),
        Value::Int(0),
    ]);
    for field in 0..fields.len() {
        for constant in &constants {
            for op in [Op::Equal, Op::Differ] {
                exprs.push(apply(
                    op,
                    vec![Expr::Field(field), Expr::Const(constant.clone())],
                ));
                exprs.push(apply(
                    op,
                    vec![Expr::Const(constant.clone()), Expr::Field(field)],
                ));
            }
        }
    }
    let mut tally = Tally::default();
    check_tests(&fields, &exprs, &mut tally);
    assert_eq!(tally.left, 0, "none of these needs the walk");
}

#[test]
fn a_test_the_row_cannot_judge_is_not_prepared() {
    let fields = vec![
        whole(0, 9),
        truth_field(),
        listed(vec![Value::Int(1), Value::Int(2)]),
    ];
    let slots = laid_out(&fields, 0);
    let not_packed = [
        Expr::Field(0),
        Expr::Field(2),
        Expr::Local(0),
        apply(Op::Order(Order::Less), vec![Expr::Field(1), int(1)]),
        apply(Op::Order(Order::Less), vec![Expr::Field(0), Expr::Local(0)]),
        apply(Op::Equal, vec![Expr::Field(0), Expr::Field(1)]),
        apply(Op::Equal, vec![Expr::Field(5), int(1)]),
        apply(Op::Negate, vec![Expr::Field(0)]),
        Expr::Const(Value::Int(1)),
    ];
    for expr in not_packed {
        assert!(
            Code::new(expr.clone()).packed_test(&slots, &[]).is_none(),
            "no packed test for {expr:?}"
        );
    }
}

fn whole_updates(into: usize, from: usize, of: &Field) -> Vec<(usize, Expr)> {
    let mut all = Vec::new();
    for number in constants_near(of) {
        all.push((into, int(number)));
        all.push((into, math(Math::Add, Expr::Field(from), int(number))));
        all.push((into, math(Math::Add, int(number), Expr::Field(from))));
        all.push((into, math(Math::Subtract, Expr::Field(from), int(number))));
        all.push((into, math(Math::Subtract, int(number), Expr::Field(from))));
        for bound in BOUNDS {
            all.push((
                into,
                bounded(bound, math(Math::Add, Expr::Field(from), int(number))),
            ));
        }
    }
    for how in MATHS {
        for number in [0, 1, 2, 3, -1] {
            all.push((into, math(how, Expr::Field(from), int(number))));
        }
        all.push((into, math(how, Expr::Field(from), Expr::Field(into))));
    }
    all.push((into, Expr::Field(from)));
    all.push((into, bounded(None, Expr::Field(from))));
    all.push((
        into,
        Expr::Choose(
            Box::new(apply(
                Op::Order(Order::Less),
                vec![Expr::Field(from), int(2)],
            )),
            Box::new(int(0)),
            Box::new(math(Math::Subtract, Expr::Field(from), int(1))),
        ),
    ));
    all
}

#[test]
fn an_update_of_a_whole_number_field_writes_what_the_walk_gives() {
    let mut tally = Tally::default();
    for into in whole_fields() {
        for from in [
            whole(0, 9),
            whole(-5, 5),
            into.clone(),
            whole(WIDE, WIDE + 20),
        ] {
            let fields = vec![from.clone(), into.clone(), whole(0, 1)];
            check_updates(&fields, &whole_updates(1, 0, &into), &mut tally);
            check_updates(&fields, &whole_updates(0, 1, &from), &mut tally);
        }
    }
    assert!(
        tally.answered > 0 && tally.left > 0,
        "both kinds of case are met"
    );
}

#[test]
fn an_update_that_reads_the_field_it_writes_and_another_writes_what_the_walk_gives() {
    let fields = vec![whole(0, 9), whole(0, 9)];
    let updates = [
        (0, math(Math::Add, Expr::Field(1), int(1))),
        (1, math(Math::Add, Expr::Field(0), int(1))),
        (0, math(Math::Subtract, Expr::Field(0), int(1))),
        (1, Expr::Field(0)),
        (0, Expr::Field(1)),
    ];
    let mut tally = Tally::default();
    check_updates(&fields, &updates, &mut tally);
    let slots = laid_out(&fields, 60);
    let swap = [
        Code::new(Expr::Field(1))
            .packed_update(&slots, 0, &[])
            .expect("a copy"),
        Code::new(Expr::Field(0))
            .packed_update(&slots, 1, &[])
            .expect("a copy"),
    ];
    for state in states(&fields, &mut Seed(7)) {
        let row = packed(&slots, &state, 0);
        let mut next = row.clone();
        for update in &swap {
            assert!(update.write(&row, &mut next), "a copy within the domain");
        }
        let swapped = [state[1].clone(), state[0].clone()];
        assert_eq!(next, packed(&slots, &swapped, 0), "each reads the old row");
    }
}

#[test]
fn an_update_of_a_field_of_true_or_false_or_of_listed_values_writes_what_the_walk_gives() {
    let mut symbols = Symbols::default();
    let colours = lights(&mut symbols);
    let other = Value::Str(symbols.intern("Blue"));
    let fields = vec![
        truth_field(),
        listed(colours.clone()),
        listed(colours.clone()),
        whole(0, 9),
    ];
    let is = |field: usize, value: &Value| {
        apply(
            Op::Equal,
            vec![Expr::Field(field), Expr::Const(value.clone())],
        )
    };
    let mut updates = vec![
        (0, Expr::Const(Value::Bool(true))),
        (0, Expr::Const(Value::Bool(false))),
        (0, Expr::Field(0)),
        (0, apply(Op::Not, vec![Expr::Field(0)])),
        (
            0,
            apply(Op::Order(Order::Less), vec![Expr::Field(3), int(5)]),
        ),
        (0, is(1, &colours[1])),
        (1, Expr::Field(2)),
        (2, Expr::Field(1)),
        (
            1,
            Expr::Choose(
                Box::new(is(1, &colours[0])),
                Box::new(Expr::Const(colours[1].clone())),
                Box::new(Expr::Choose(
                    Box::new(is(1, &colours[1])),
                    Box::new(Expr::Const(colours[2].clone())),
                    Box::new(Expr::Const(colours[0].clone())),
                )),
            ),
        ),
        (
            3,
            Expr::Choose(
                Box::new(Expr::Field(0)),
                Box::new(math(Math::Add, Expr::Field(3), int(1))),
                Box::new(int(0)),
            ),
        ),
    ];
    for colour in &colours {
        updates.push((2, Expr::Const(colour.clone())));
    }
    let mut tally = Tally::default();
    check_updates(&fields, &updates, &mut tally);
    let slots = laid_out(&fields, 0);
    for (into, expr) in [
        (1, Expr::Const(other)),
        (1, Expr::Field(0)),
        (0, Expr::Field(1)),
        (1, int(1)),
        (3, Expr::Const(Value::Bool(true))),
        (3, int(10)),
        (0, int(1)),
        (5, int(1)),
        (3, Expr::Local(0)),
    ] {
        assert!(
            Code::new(expr.clone())
                .packed_update(&slots, into, &[])
                .is_none(),
            "no packed update of field {into} for {expr:?}"
        );
    }
}

#[test]
fn random_tests_and_updates_over_mixed_fields_give_what_the_walk_gives() {
    let mut symbols = Symbols::default();
    let colours = lights(&mut symbols);
    let mut seed = Seed(0x0123_4567_89AB_CDEF);
    let mut tally = Tally::default();
    for _ in 0..40 {
        let mut fields: Vec<Field> = (0..4)
            .map(|_| match seed.below(5) {
                0 => truth_field(),
                1 => listed(colours.clone()),
                _ => {
                    let all = whole_fields();
                    all[seed.below(all.len())].clone()
                }
            })
            .collect();
        fields.push(whole(-3, 3));
        let exprs: Vec<Expr> = (0..12)
            .map(|_| random_test(&fields, &mut seed, 3))
            .collect();
        check_tests(&fields, &exprs, &mut tally);
        let updates: Vec<(usize, Expr)> = (0..12)
            .filter_map(|_| {
                let into = seed.below(fields.len());
                random_update(&fields, into, &mut seed, 2).map(|expr| (into, expr))
            })
            .collect();
        check_updates(&fields, &updates, &mut tally);
    }
    assert!(
        tally.answered > 0 && tally.left > 0,
        "both kinds of case are met"
    );
}

fn fields_with(fields: &[Field], wanted: impl Fn(&Coding) -> bool) -> Vec<usize> {
    (0..fields.len())
        .filter(|at| wanted(&fields[*at].coding))
        .collect()
}

fn pick<T: Clone>(all: &[T], seed: &mut Seed) -> Option<T> {
    (!all.is_empty()).then(|| all[seed.below(all.len())].clone())
}

fn random_number(fields: &[Field], seed: &mut Seed, depth: usize) -> Expr {
    let wholes = fields_with(fields, |coding| matches!(coding, Coding::Whole { .. }));
    let field = pick(&wholes, seed).expect("a whole-number field");
    match (depth, seed.below(4)) {
        (0, _) | (_, 0) => Expr::Field(field),
        (_, 1) => {
            let near = constants_near(&fields[field]);
            int(near[seed.below(near.len())])
        }
        (_, 2) => math(
            MATHS[seed.below(MATHS.len())],
            random_number(fields, seed, depth - 1),
            random_number(fields, seed, depth - 1),
        ),
        _ => bounded(
            BOUNDS[seed.below(BOUNDS.len())],
            random_number(fields, seed, depth - 1),
        ),
    }
}

fn random_test(fields: &[Field], seed: &mut Seed, depth: usize) -> Expr {
    let truths = fields_with(fields, |coding| matches!(coding, Coding::Truth));
    let lists = fields_with(fields, |coding| matches!(coding, Coding::Listed(_)));
    let choice = if depth == 0 {
        seed.below(4)
    } else {
        seed.below(8)
    };
    match choice {
        0 => apply(
            Op::Order(ORDERS[seed.below(ORDERS.len())]),
            vec![
                random_number(fields, seed, 1),
                random_number(fields, seed, 1),
            ],
        ),
        1 => apply(
            [Op::Equal, Op::Differ][seed.below(2)],
            vec![
                random_number(fields, seed, 1),
                random_number(fields, seed, 1),
            ],
        ),
        2 => match pick(&truths, seed) {
            Some(field) => Expr::Field(field),
            None => Expr::Const(Value::Bool(seed.below(2) == 0)),
        },
        3 => match pick(&lists, seed) {
            Some(field) => {
                let Coding::Listed(values) = &fields[field].coding else {
                    unreachable!("a listed field");
                };
                let value = values[seed.below(values.len())].clone();
                apply(
                    [Op::Equal, Op::Differ][seed.below(2)],
                    vec![Expr::Field(field), Expr::Const(value)],
                )
            }
            None => Expr::Const(Value::Bool(false)),
        },
        4 => Expr::And(
            Box::new(random_test(fields, seed, depth - 1)),
            Box::new(random_test(fields, seed, depth - 1)),
        ),
        5 => Expr::Or(
            Box::new(random_test(fields, seed, depth - 1)),
            Box::new(random_test(fields, seed, depth - 1)),
        ),
        6 => apply(Op::Not, vec![random_test(fields, seed, depth - 1)]),
        _ => Expr::Choose(
            Box::new(random_test(fields, seed, depth - 1)),
            Box::new(random_test(fields, seed, depth - 1)),
            Box::new(random_test(fields, seed, depth - 1)),
        ),
    }
}

fn random_update(fields: &[Field], into: usize, seed: &mut Seed, depth: usize) -> Option<Expr> {
    if depth > 0 && seed.below(4) == 0 {
        return Some(Expr::Choose(
            Box::new(random_test(fields, seed, 1)),
            Box::new(random_update(fields, into, seed, depth - 1)?),
            Box::new(random_update(fields, into, seed, depth - 1)?),
        ));
    }
    match &fields[into].coding {
        Coding::Whole { .. } => Some(random_number(fields, seed, 2)),
        Coding::Truth => Some(random_test(fields, seed, 1)),
        Coding::Listed(values) => Some(Expr::Const(values[seed.below(values.len())].clone())),
        _ => None,
    }
}

fn queues(letters: &[Value], longest: usize) -> Vec<Value> {
    let mut all = vec![Vec::new()];
    let mut grown = vec![Vec::new()];
    for _ in 0..longest {
        grown = grown
            .iter()
            .flat_map(|queue: &Vec<Value>| {
                letters.iter().map(move |letter| {
                    let mut longer = queue.clone();
                    longer.push(letter.clone());
                    longer
                })
            })
            .collect();
        all.extend(grown.iter().cloned());
    }
    all.into_iter().map(Value::seq).collect()
}

fn tallies(symbols: &mut Symbols) -> (Vec<Value>, earnest_value::Sym, earnest_value::Sym) {
    let (first, second) = (symbols.intern("a"), symbols.intern("b"));
    let all = (0..4)
        .flat_map(|a| {
            (0..3)
                .map(move |b| Value::record(vec![(first, Value::Int(a)), (second, Value::Int(b))]))
        })
        .collect();
    (all, first, second)
}

fn bind(slot: usize, value: Expr, body: Expr) -> Expr {
    Expr::Bind {
        slot,
        value: Box::new(value),
        body: Box::new(body),
    }
}

struct Tabled {
    fields: Vec<Field>,
    letters: Vec<Value>,
    first: earnest_value::Sym,
    second: earnest_value::Sym,
}

fn tabled_fields() -> Tabled {
    let mut symbols = Symbols::default();
    let letters = lights(&mut symbols);
    let (records, first, second) = tallies(&mut symbols);
    let fields = vec![
        listed(queues(&letters, 3)),
        listed(records),
        whole(0, 5),
        listed(vec![Value::Int(1), Value::Int(2), Value::Int(7)]),
        whole(-2, 9),
    ];
    Tabled {
        fields,
        letters,
        first,
        second,
    }
}

fn raised() -> Expr {
    bind(
        0,
        apply(Op::Safe, vec![math(Math::Add, Expr::Field(2), int(1))]),
        Expr::Choose(
            Box::new(apply(
                Op::Order(Order::Greater),
                vec![Expr::Local(0), int(4)],
            )),
            Box::new(int(4)),
            Box::new(Expr::Local(0)),
        ),
    )
}

#[test]
fn a_test_that_reads_one_field_of_few_values_is_answered_from_a_table_of_its_codes() {
    let Tabled {
        fields,
        letters,
        first,
        second,
    } = tabled_fields();
    let size = apply(Op::Seq(SeqOp::Size), vec![Expr::Field(0)]);
    let get = |name| Expr::Get(Box::new(Expr::Field(1)), name);
    let faulting = apply(
        Op::Order(Order::Less),
        vec![
            math(
                Math::Divide,
                int(10),
                math(Math::Subtract, Expr::Field(2), int(2)),
            ),
            int(4),
        ],
    );
    let every_letter_red = Expr::Sweep {
        sweep: Sweep::Every,
        shelf: Shelf::Seq,
        slot: 1,
        source: Box::new(Expr::Field(0)),
        body: Box::new(apply(
            Op::Equal,
            vec![Expr::Local(1), Expr::Const(letters[0].clone())],
        )),
    };
    let tests = vec![
        apply(Op::Equal, vec![size.clone(), int(0)]),
        apply(Op::Order(Order::Less), vec![size.clone(), int(3)]),
        apply(Op::Not, vec![apply(Op::Equal, vec![size.clone(), int(2)])]),
        apply(Op::Order(Order::Less), vec![get(first), int(3)]),
        Expr::And(
            Box::new(apply(Op::Order(Order::Less), vec![get(second), int(2)])),
            Box::new(apply(Op::Equal, vec![size, int(1)])),
        ),
        apply(Op::Order(Order::Less), vec![Expr::Field(3), int(2)]),
        bind(
            2,
            Expr::Field(2),
            apply(
                Op::Order(Order::GreaterOrEqual),
                vec![Expr::Local(2), int(2)],
            ),
        ),
        apply(Op::Order(Order::Less), vec![raised(), int(4)]),
        faulting,
        every_letter_red,
        Expr::Sweep {
            sweep: Sweep::Some,
            shelf: Shelf::Seq,
            slot: 3,
            source: Box::new(Expr::Const(Value::seq(letters))),
            body: Box::new(apply(
                Op::Seq(SeqOp::Includes),
                vec![Expr::Field(0), Expr::Local(3)],
            )),
        },
        Expr::Or(
            Box::new(apply(Op::Order(Order::Less), vec![Expr::Field(4), int(0)])),
            Box::new(apply(
                Op::Order(Order::Less),
                vec![math(Math::Remainder, Expr::Field(4), int(3)), int(1)],
            )),
        ),
    ];
    let mut tally = Tally::default();
    check_tests(&fields, &tests, &mut tally);
    assert!(
        tally.answered > 0 && tally.left > 0,
        "both kinds of case are met"
    );
}

#[test]
fn an_update_that_reads_one_field_of_few_values_is_written_from_a_table_of_its_codes() {
    let Tabled {
        fields,
        letters,
        first,
        second,
    } = tabled_fields();
    let size = apply(Op::Seq(SeqOp::Size), vec![Expr::Field(0)]);
    let get = |name| Expr::Get(Box::new(Expr::Field(1)), name);
    let mut tally = Tally::default();
    let with = |name, change: Expr| {
        (
            1,
            Expr::With(Box::new(Expr::Field(1)), vec![(name, change)]),
        )
    };
    let updates = vec![
        (0, apply(Op::Seq(SeqOp::Shift), vec![Expr::Field(0)])),
        (
            0,
            apply(
                Op::Seq(SeqOp::Push),
                vec![Expr::Field(0), Expr::Const(letters[1].clone())],
            ),
        ),
        with(
            first,
            apply(Op::Safe, vec![math(Math::Add, get(first), int(1))]),
        ),
        with(second, math(Math::Subtract, get(second), int(1))),
        (2, raised()),
        (2, size),
        (4, Expr::Field(3)),
        (0, Expr::SeqOf(Vec::new())),
        (0, Expr::SeqOf(vec![Expr::Const(letters[2].clone())])),
        (4, math(Math::Add, int(2), int(3))),
        (
            2,
            bind(
                3,
                math(Math::Multiply, Expr::Field(2), int(2)),
                Expr::Local(3),
            ),
        ),
    ];
    check_updates(&fields, &updates, &mut tally);
}

#[test]
fn an_expression_of_no_field_of_two_fields_or_of_a_constant_sweep_is_not_tabled() {
    let mut symbols = Symbols::default();
    let letters = lights(&mut symbols);
    let fields = vec![
        listed(queues(&letters, 2)),
        listed(queues(&letters, 2)),
        whole(0, (1 << 20) - 1),
    ];
    let slots = laid_out(&fields, 0);
    let size = |of: Expr| apply(Op::Seq(SeqOp::Size), vec![of]);
    let below = |left: Expr, right: Expr| apply(Op::Order(Order::Less), vec![left, right]);
    let constant = Expr::Const(Value::seq(vec![Value::Int(1), Value::Int(2)]));
    let many = Expr::Const(Value::seq((0..17).map(Value::Int).collect()));
    let not_tabled = [
        below(size(Expr::Field(0)), size(Expr::Field(1))),
        bind(0, Expr::Field(2), below(Expr::Local(0), int(3))),
        below(size(constant.clone()), int(2)),
        below(size(Expr::Field(0)), Expr::Local(5)),
        Expr::Sweep {
            sweep: Sweep::Every,
            shelf: Shelf::Seq,
            slot: 0,
            source: Box::new(many),
            body: Box::new(below(size(Expr::Field(0)), Expr::Local(0))),
        },
        Expr::Sweep {
            sweep: Sweep::Every,
            shelf: Shelf::Seq,
            slot: 0,
            source: Box::new(constant.clone()),
            body: Box::new(Expr::Sweep {
                sweep: Sweep::Some,
                shelf: Shelf::Seq,
                slot: 1,
                source: Box::new(constant),
                body: Box::new(below(size(Expr::Field(0)), Expr::Local(1))),
            }),
        },
    ];
    for expr in not_tabled {
        assert!(
            Code::new(expr.clone()).packed_test(&slots, &[]).is_none(),
            "no packed test for {expr:?}"
        );
    }
}
