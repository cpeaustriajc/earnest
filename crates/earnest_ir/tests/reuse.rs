//! The search keeps the answer of a guard or an update while the inputs it reads stay the same.
//! It may do so only for an expression that binds nothing and sweeps nothing, so that its answer
//! depends on the state and the slots it reads alone, and it must put every input again when an
//! expression may overwrite the slot of one. These cases pin both answers.

use earnest_ir::{Action, Code, Expr, Input, Math, Op, Order, Shelf, Sweep};
use earnest_value::{Domain, Value};

fn code(expr: Expr) -> Code {
    Code::new(expr)
}

fn input(name: &str, slot: usize) -> Input {
    let values = vec![Value::Int(0), Value::Int(1)];
    Input {
        name: name.to_owned(),
        slot,
        domain: Domain::OneOf(values.clone()),
        values,
    }
}

fn sum(left: Expr, right: Expr) -> Expr {
    Expr::Apply(Op::Math(Math::Add), vec![left, right])
}

fn action(inputs: Vec<Input>, guard: Expr, updates: Vec<Expr>) -> Action {
    let mut codes = Code::together(std::iter::once(guard).chain(updates)).into_iter();
    let guard = codes.next().expect("the guard comes first");
    Action {
        id: "act".to_owned(),
        label: "acts".to_owned(),
        line: 1,
        inputs,
        guard,
        updates: codes.map(Some).collect(),
        slots: 4,
    }
}

#[test]
fn an_expression_that_binds_a_slot_reads_no_slots_that_could_stand_for_it() {
    let binds = Expr::Bind {
        slot: 2,
        value: Box::new(Expr::Local(0)),
        body: Box::new(Expr::Local(2)),
    };
    assert_eq!(code(binds).slots_read(), None);
}

#[test]
fn an_expression_that_sweeps_a_collection_reads_no_slots_that_could_stand_for_it() {
    let sweeps = Expr::Sweep {
        sweep: Sweep::Some,
        shelf: Shelf::Set,
        slot: 2,
        source: Box::new(Expr::Field(0)),
        body: Box::new(Expr::Local(2)),
    };
    assert_eq!(code(sweeps).slots_read(), None);
}

#[test]
fn an_expression_that_reads_slots_names_each_once_in_order() {
    let reads = sum(
        sum(Expr::Local(3), Expr::Field(0)),
        sum(Expr::Local(1), Expr::Local(3)),
    );
    assert_eq!(code(reads).slots_read(), Some(vec![1, 3]));
}

#[test]
fn an_expression_that_reads_only_fields_reads_no_slot() {
    let fields = Expr::Apply(
        Op::Order(Order::Less),
        vec![
            Expr::Field(0),
            sum(Expr::Field(1), Expr::Const(Value::Int(2))),
        ],
    );
    assert_eq!(code(fields).slots_read(), Some(Vec::new()));
}

#[test]
fn a_guard_that_binds_the_slot_of_an_input_may_overwrite_it() {
    let guard = Expr::Bind {
        slot: 1,
        value: Box::new(Expr::Const(Value::Int(7))),
        body: Box::new(Expr::Const(Value::Bool(true))),
    };
    let rebinding = action(
        vec![input("a", 0), input("b", 1)],
        guard,
        vec![Expr::Local(1)],
    );
    assert!(rebinding.inputs_may_be_overwritten());
}

#[test]
fn two_inputs_on_one_slot_may_overwrite_each_other() {
    let shared = action(
        vec![input("a", 0), input("b", 0)],
        Expr::Const(Value::Bool(true)),
        vec![Expr::Local(0)],
    );
    assert!(shared.inputs_may_be_overwritten());
}

#[test]
fn an_action_that_only_reads_its_inputs_overwrites_none() {
    let plain = action(
        vec![input("a", 0), input("b", 1)],
        Expr::Apply(
            Op::Order(Order::Less),
            vec![Expr::Local(1), Expr::Const(Value::Int(1))],
        ),
        vec![sum(Expr::Field(0), Expr::Local(0))],
    );
    assert!(!plain.inputs_may_be_overwritten());
}
