//! An action expanded the way the search expands it, once trying every combination with the
//! reference walk and once skipping values not told apart with the prepared codes, must offer
//! the same successors from the same combinations, note the same first fault and mark the same
//! width.

use earnest_ir::{Action, Code, Expr, Fault, Frame, Input, Math, Op, Order, SetOp, Shelf, Sweep};
use earnest_value::{Domain, Sym, Symbols, Value};
use proptest::prelude::*;
use proptest::strategy::ValueTree;
use proptest::test_runner::TestRunner;

const SLOTS: usize = 5;
const NAMES: usize = 3;
const FIELDS: usize = 3;

fn any_name() -> impl Strategy<Value = Sym> {
    (0..NAMES).prop_map(|position| {
        let mut symbols = Symbols::default();
        let mut named = symbols.intern("a");
        for text in ["b", "c"].iter().take(position) {
            named = symbols.intern(text);
        }
        named
    })
}

fn small() -> impl Strategy<Value = Value> {
    prop_oneof![
        4 => (0..3_i64).prop_map(Value::Int),
        1 => Just(Value::Int(i64::MAX)),
        1 => Just(Value::Int(1 << 31)),
        1 => Just(Value::Null),
        1 => any::<bool>().prop_map(Value::Bool),
    ]
}

fn any_input_value() -> impl Strategy<Value = Value> {
    let leaf = small();
    leaf.prop_recursive(2, 8, 3, |inner| {
        prop_oneof![
            6 => proptest::collection::btree_map(any_name(), inner.clone(), 1..=NAMES)
                .prop_map(|fields| Value::record(fields.into_iter().collect())),
            1 => proptest::collection::btree_set(inner, 0..3).prop_map(Value::set),
        ]
    })
}

#[derive(Clone, Copy, Debug)]
enum Reading {
    Plain,
    Optional,
}

fn any_reading() -> impl Strategy<Value = Reading> {
    prop_oneof![Just(Reading::Plain), Just(Reading::Optional)]
}

fn local_read_as(slot: usize, reading: Reading) -> Expr {
    match reading {
        Reading::Plain => Expr::Local(slot),
        Reading::Optional => Expr::Apply(Op::SomeOf, vec![Expr::Local(slot)]),
    }
}

#[derive(Clone, Copy, Debug)]
enum Walk<'told> {
    EveryCombination,
    SkippingValuesNotToldApart(&'told [Vec<bool>]),
}

fn any_reading_expr() -> impl Strategy<Value = Expr> {
    let leaf = prop_oneof![
        2 => small().prop_map(Expr::Const),
        2 => (0..FIELDS).prop_map(Expr::Field),
        1 => (0..SLOTS).prop_map(Expr::Local),
        6 => (0..SLOTS, any_reading(), any_name())
            .prop_map(|(slot, reading, name)| Expr::Get(Box::new(local_read_as(slot, reading)), name)),
        2 => (0..SLOTS, any_name(), any_name()).prop_map(|(slot, outer, inner)| Expr::Get(
            Box::new(Expr::Get(Box::new(Expr::Local(slot)), outer)),
            inner
        )),
    ];
    leaf.prop_recursive(4, 28, 3, |inner| {
        let boxed = || inner.clone().prop_map(Box::new);
        let ops = prop_oneof![
            Just(Op::Math(Math::Add)),
            Just(Op::Math(Math::Divide)),
            Just(Op::Math(Math::Multiply)),
            Just(Op::Order(Order::Less)),
            Just(Op::Equal),
            Just(Op::Differ),
            Just(Op::Not),
            Just(Op::Unwrap),
            Just(Op::Set(SetOp::Has)),
            Just(Op::Set(SetOp::Add)),
            Just(Op::Set(SetOp::Size)),
        ];
        prop_oneof![
            4 => (ops, proptest::collection::vec(inner.clone(), 1..3))
                .prop_map(|(op, arguments)| Expr::Apply(op, arguments)),
            2 => (boxed(), boxed()).prop_map(|(left, right)| Expr::And(left, right)),
            2 => (boxed(), boxed()).prop_map(|(left, right)| Expr::Or(left, right)),
            1 => (boxed(), boxed(), boxed()).prop_map(|(test, yes, no)| Expr::Choose(test, yes, no)),
            3 => (2..SLOTS, 0..SLOTS, any_reading(), boxed()).prop_map(|(slot, source, reading, body)| Expr::Bind {
                slot,
                value: Box::new(local_read_as(source, reading)),
                body,
            }),
            2 => (2..SLOTS, boxed(), boxed())
                .prop_map(|(slot, value, body)| Expr::Bind { slot, value, body }),
            1 => (2..SLOTS, boxed(), boxed()).prop_map(|(slot, source, body)| Expr::Sweep {
                sweep: Sweep::Some,
                shelf: Shelf::Set,
                slot,
                source,
                body,
            }),
            1 => proptest::collection::vec(inner.clone(), 1..3).prop_map(Expr::SetOf),
            1 => proptest::collection::vec((any_name(), inner.clone()), 1..3).prop_map(Expr::Record),
            1 => (boxed(), proptest::collection::vec((any_name(), inner), 1..2))
                .prop_map(|(base, changes)| Expr::With(base, changes)),
        ]
    })
}

fn shared_unit() -> impl Strategy<Value = Vec<Expr>> {
    (
        (2..SLOTS, any_reading_expr()),
        proptest::collection::vec((any_reading_expr(), any::<bool>()), 1..4),
    )
        .prop_map(|((slot, value), parts)| {
            parts
                .into_iter()
                .map(|(body, wrapped)| {
                    if wrapped {
                        Expr::Bind {
                            slot,
                            value: Box::new(value.clone()),
                            body: Box::new(body),
                        }
                    } else {
                        body
                    }
                })
                .collect()
        })
}

fn action_of(values: [Vec<Value>; 2], exprs: Vec<Expr>) -> (Action, Vec<Expr>) {
    let mut codes = Code::together(exprs.clone()).into_iter();
    let guard = codes.next().expect("at least one expression");
    let action = Action {
        id: "act".to_owned(),
        label: "acts".to_owned(),
        line: 1,
        inputs: [0, 1]
            .into_iter()
            .zip(values)
            .enumerate()
            .map(|(position, (slot, values))| Input {
                name: format!("input{position}"),
                slot,
                domain: Domain::OneOf(values.clone()),
                values,
            })
            .collect(),
        guard,
        updates: codes.map(Some).collect(),
        slots: SLOTS,
    };
    (action, exprs)
}

#[derive(Debug, PartialEq)]
enum Faulted {
    Guard(Fault),
    Update(Fault),
}

#[derive(Debug, PartialEq)]
struct Expanded {
    offered: Vec<(usize, Vec<Value>)>,
    first_fault: Option<Faulted>,
    enabled: bool,
    unsure: bool,
    wide: bool,
}

fn digits(action: &Action, combination: usize) -> Vec<usize> {
    let mut rest = combination;
    action
        .inputs
        .iter()
        .map(|input| {
            let count = input.values.len();
            let digit = rest % count;
            rest /= count;
            digit
        })
        .collect()
}

fn expanded(
    action: &Action,
    exprs: &[Expr],
    state: &[Value],
    left_over: &[Value],
    walk: Walk<'_>,
) -> Expanded {
    let mut frame = Frame::new(state, SLOTS);
    for (slot, value) in left_over.iter().enumerate() {
        frame.put(slot, value.clone());
    }
    let mut found = Expanded {
        offered: Vec::new(),
        first_fault: None,
        enabled: false,
        unsure: false,
        wide: false,
    };
    let total: usize = action
        .inputs
        .iter()
        .map(|input| input.values.len())
        .product();
    let codes: Vec<&Code> = std::iter::once(&action.guard)
        .chain(action.updates.iter().flatten())
        .collect();
    let truth_of = |at: usize, frame: &mut Frame<'_>| match walk {
        Walk::EveryCombination => earnest_ir::truth(&exprs[at], frame),
        Walk::SkippingValuesNotToldApart(_) => codes[at].truth(frame),
    };
    let value_of = |at: usize, frame: &mut Frame<'_>| match walk {
        Walk::EveryCombination => earnest_ir::eval(&exprs[at], frame),
        Walk::SkippingValuesNotToldApart(_) => codes[at].eval(frame),
    };
    for combination in 0..total {
        let digits = digits(action, combination);
        if let Walk::SkippingValuesNotToldApart(told) = walk
            && !told.iter().zip(&digits).all(|(apart, digit)| apart[*digit])
        {
            continue;
        }
        for (input, digit) in action.inputs.iter().zip(&digits) {
            frame.put(input.slot, input.values[*digit].clone());
        }
        match truth_of(0, &mut frame) {
            Ok(true) => {}
            Ok(false) => continue,
            Err(fault) => {
                found.first_fault.get_or_insert(Faulted::Guard(fault));
                found.unsure = true;
                continue;
            }
        }
        found.enabled = true;
        let mut updates = Vec::new();
        let mut faulted = None;
        for at in 1..codes.len() {
            match value_of(at, &mut frame) {
                Ok(value) => updates.push(value),
                Err(fault) => {
                    faulted = Some(fault);
                    break;
                }
            }
        }
        match faulted {
            Some(fault) => {
                found.first_fault.get_or_insert(Faulted::Update(fault));
            }
            None => {
                if !found.offered.iter().any(|(_, seen)| *seen == updates) {
                    found.offered.push((combination, updates));
                }
            }
        }
    }
    found.wide = frame.computed_beyond_32_bits();
    found
}

const CASES: u32 = 800;
const LEAST_SHARE_SKIPPING: usize = 10;

#[derive(Debug)]
struct Case {
    action: Action,
    exprs: Vec<Expr>,
    state: Vec<Value>,
    left_over: Vec<Value>,
}

fn any_case() -> impl Strategy<Value = Case> {
    (
        proptest::collection::vec(any_input_value(), 1..6),
        proptest::collection::vec(any_input_value(), 1..4),
        shared_unit(),
        proptest::collection::vec(small(), FIELDS),
        proptest::collection::vec(small(), SLOTS),
    )
        .prop_map(|(first_values, second_values, exprs, state, left_over)| {
            let (action, exprs) = action_of([first_values, second_values], exprs);
            Case {
                action,
                exprs,
                state,
                left_over,
            }
        })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(CASES))]

    #[test]
    fn an_expansion_that_skips_values_not_told_apart_matches_the_walk_of_every_combination(
        case in any_case()
    ) {
        let Case { action, exprs, state, left_over } = case;
        let told = action.told_apart();
        let reference = expanded(&action, &exprs, &state, &left_over, Walk::EveryCombination);
        let skipping = expanded(
            &action,
            &exprs,
            &state,
            &left_over,
            Walk::SkippingValuesNotToldApart(&told),
        );
        prop_assert_eq!(reference, skipping, "told apart: {:?}", told);
    }
}

#[test]
fn a_fair_share_of_the_generated_actions_skip_a_value() {
    let mut runner = TestRunner::deterministic();
    let strategy = any_case();
    let skipping = (0..CASES)
        .filter(|_| {
            let case = strategy
                .new_tree(&mut runner)
                .expect("a case is generated")
                .current();
            case.action.told_apart().iter().flatten().any(|new| !new)
        })
        .count();
    assert!(
        skipping * 100
            >= usize::try_from(CASES).expect("the cases fit a count") * LEAST_SHARE_SKIPPING,
        "only {skipping} of {CASES} generated actions skip a value"
    );
}
