use crate::build::{Sketch, apply, int, steps, texts, yes};
use earnest_check::{FindingKind, Limits, Outcome, Status, Verdict, explore};
use earnest_ir::{Code, Expr, Math, Model, Op, Order, Pick, SetOp};
use earnest_value::{Domain, Value};
use std::collections::BTreeSet;

const WHO: usize = 0;
const BY: usize = 1;
const GROWN_TO: i64 = 40;

fn set_of(numbers: &[i64]) -> Value {
    Value::set(
        numbers
            .iter()
            .copied()
            .map(Value::Int)
            .collect::<BTreeSet<_>>(),
    )
}

fn set_op(op: SetOp, set: &Expr, item: Expr) -> Expr {
    Expr::Apply(Op::Set(op), vec![set.clone(), item])
}

fn picked(mut model: Model, picks: &[(usize, usize, Expr)]) -> Model {
    for (action, input, from) in picks {
        model.actions[*action].inputs[*input].values.clear();
        model.actions[*action].inputs[*input].domain = Domain::Any;
        model.runs.picks.push(Pick {
            action: *action,
            input: *input,
            from: Code::new(from.clone()),
        });
    }
    model
}

fn serving(ends: bool) -> Model {
    let mut sketch = Sketch::new();
    let pending = sketch.field("pending", Domain::Any, vec![set_of(&[1, 2, 3])]);
    sketch.action_with(
        "serve",
        &[("who", Vec::new())],
        yes(),
        &[(&pending, set_op(SetOp::Delete, &pending, Expr::Local(WHO)))],
    );
    sketch.rule(
        "twoIsNotServedFirst",
        apply(Op::Differ, &pending, Expr::Const(set_of(&[1, 3]))),
    );
    if ends {
        sketch.end("anywhere", yes());
    }
    let model = sketch.model();
    picked(model, &[(0, WHO, pending)])
}

fn grows(jobs: usize, max_states: usize) -> Outcome {
    let mut sketch = Sketch::new();
    let seen = sketch.field("seen", Domain::Any, vec![set_of(&[0])]);
    let last = sketch.field("last", Domain::Any, vec![Value::Int(0)]);
    let next = apply(Op::Math(Math::Add), &Expr::Local(WHO), int(1));
    sketch.action_with(
        "extend",
        &[("x", Vec::new())],
        apply(Op::Order(Order::Less), &Expr::Local(WHO), int(GROWN_TO - 1)),
        &[
            (&seen, set_op(SetOp::Add, &seen, next)),
            (&last, Expr::Local(WHO)),
        ],
    );
    sketch.rule(
        "belowForty",
        apply(
            Op::Order(Order::Less),
            &Expr::Apply(Op::Set(SetOp::Size), vec![seen.clone()]),
            int(GROWN_TO),
        ),
    );
    sketch.end("anywhere", yes());
    let model = picked(sketch.model(), &[(0, WHO, seen)]);
    explore(
        &model,
        &Limits {
            jobs,
            max_states,
            ..Limits::default()
        },
    )
}

#[test]
fn every_item_of_a_set_read_from_the_state_is_tried_in_every_state() {
    let outcome = explore(&serving(true), &Limits::default());
    assert_eq!(
        outcome.states, 8,
        "every subset of {{1, 2, 3}} is left pending"
    );
    assert!(outcome.findings.is_empty(), "{:?}", outcome.findings);
}

#[test]
fn a_path_prints_the_item_picked_not_its_position() {
    let outcome = explore(&serving(true), &Limits::default());
    let rule = &outcome.rules[0];
    assert_eq!(rule.verdict, Verdict::Broken);
    let path = rule.path.as_ref().expect("a path to {1, 3}");
    assert_eq!(texts(&path.start), ["pending = {1, 2, 3}"]);
    assert_eq!(
        steps(path),
        ["serve  who = 2  pending: {1, 2, 3} -> {1, 3}"]
    );
    assert_eq!(path.steps[0].input[0].value.json, serde_json::json!(2));
}

#[test]
fn an_empty_set_leaves_the_action_unable_to_happen() {
    let outcome = explore(&serving(false), &Limits::default());
    assert_eq!(outcome.states, 8);
    let stuck = outcome
        .findings
        .iter()
        .find(|finding| finding.kind == FindingKind::Stuck)
        .expect("nothing is pending at the end, and no end marks it");
    assert_eq!(
        stuck.path.steps.last().map(|step| texts(&step.state)),
        Some(vec!["pending = {}".to_owned()])
    );
}

#[test]
fn a_set_may_read_an_earlier_input_of_the_same_action() {
    let mut sketch = Sketch::new();
    let total = sketch.field("total", Domain::Any, vec![Value::Int(0)]);
    let limit = sketch.field("limit", Domain::Any, vec![Value::Int(2)]);
    sketch.action_with(
        "add",
        &[("who", Vec::new()), ("by", Vec::new())],
        apply(Op::Order(Order::Less), &total, int(100)),
        &[(
            &total,
            apply(
                Op::Math(Math::Add),
                &apply(Op::Math(Math::Multiply), &Expr::Local(WHO), int(10)),
                Expr::Local(BY),
            ),
        )],
    );
    sketch.end("anywhere", yes());
    let up_to_limit = Expr::Apply(Op::Interval, vec![int(1), limit.clone()]);
    let up_to_who = Expr::Apply(Op::Interval, vec![int(0), Expr::Local(WHO)]);
    let model = picked(sketch.model(), &[(0, WHO, up_to_limit), (0, BY, up_to_who)]);
    let outcome = explore(&model, &Limits::default());
    assert_eq!(
        outcome.states, 6,
        "the start, then 10 and 11 for who = 1, 20, 21 and 22 for who = 2"
    );
}

#[test]
fn a_set_that_grows_to_forty_items_is_searched_alike_by_one_worker_and_by_four() {
    let one = grows(1, Limits::default().max_states);
    let four = grows(4, Limits::default().max_states);
    assert_eq!(
        one.states, 781,
        "1 + (1 + 2 + ... + 39): seen = {{0..k}}, last below k"
    );
    assert_eq!(four.states, one.states);
    assert_eq!(one.rules[0].verdict, Verdict::Broken);
    assert_eq!(four.rules[0].verdict, Verdict::Broken);
    let path = one.rules[0].path.as_ref().expect("a path to forty items");
    assert_eq!(path.steps.len(), 39, "each step adds the next number");
    assert_eq!(
        four.rules[0].path.as_ref().map(steps),
        Some(steps(path)),
        "the same path with four workers"
    );
}

#[test]
fn the_state_limit_met_while_an_action_tries_its_items_leaves_the_search_unfinished() {
    let outcome = grows(1, 50);
    assert_ne!(outcome.status, Status::Pass);
    assert_eq!(outcome.status, Status::Unfinished, "{:?}", outcome.rules);
    assert_eq!(outcome.reason.as_deref(), Some("state limit of 50"));
}

#[test]
fn a_set_that_is_not_a_set_is_a_fault_of_the_action() {
    let mut sketch = Sketch::new();
    let n = sketch.field("n", Domain::Any, vec![Value::Int(0)]);
    sketch.action_with(
        "go",
        &[("who", Vec::new())],
        yes(),
        &[(&n, Expr::Local(WHO))],
    );
    sketch.end("anywhere", yes());
    let model = picked(sketch.model(), &[(0, WHO, n)]);
    let outcome = explore(&model, &Limits::default());
    assert_eq!(outcome.status, Status::Fail);
    let fault = outcome
        .findings
        .iter()
        .find(|finding| finding.kind == FindingKind::Fault)
        .expect("a number has no items to pick");
    assert!(fault.message.contains("go"), "{}", fault.message);
}
