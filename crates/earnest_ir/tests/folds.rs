//! The forms added for Quint models: the fold, the must-get and the other list primitives that
//! fault, powers and runs of numbers. Each case is evaluated by the reference walk and by the
//! prepared [`Code`], which must agree, and is checked against values worked out by hand.

use earnest_ir::{Code, Expr, Fault, Frame, MapOp, Math, Op, SeqOp, SetOp, Shelf, eval};
use earnest_value::Value;

const SLOTS: usize = 4;
const TOTAL: usize = 0;
const ITEM: usize = 1;

struct Agreed(Result<Value, Fault>);

fn agreed(expr: &Expr) -> Agreed {
    let mut walked = Frame::new(&[], SLOTS);
    let reference = eval(expr, &mut walked);
    let mut prepared = Frame::new(&[], SLOTS);
    let code = Code::new(expr.clone()).eval(&mut prepared);
    assert_eq!(reference, code, "the walk and the code agree on {expr:?}");
    Agreed(reference)
}

fn both(expr: &Expr) -> Result<Value, Fault> {
    agreed(expr).0
}

fn int(number: i64) -> Expr {
    Expr::Const(Value::Int(number))
}

fn apply(op: Op, arguments: &[Expr]) -> Expr {
    Expr::Apply(op, arguments.to_vec())
}

fn ints(numbers: &[i64]) -> Vec<Value> {
    numbers.iter().copied().map(Value::Int).collect()
}

fn set_value(numbers: &[i64]) -> Value {
    Value::set(ints(numbers).into_iter().collect())
}

fn seq_value(numbers: &[i64]) -> Value {
    Value::seq(ints(numbers))
}

fn map_value(pairs: &[(i64, i64)]) -> Value {
    Value::map(
        pairs
            .iter()
            .map(|(key, value)| (Value::Int(*key), Value::Int(*value)))
            .collect(),
    )
}

fn fold(source: Value, start: Expr, body: Expr) -> Expr {
    let shelf = match source {
        Value::Seq(_) => Shelf::Seq,
        _ => Shelf::Set,
    };
    Expr::Fold {
        shelf,
        source: Box::new(Expr::Const(source)),
        start: Box::new(start),
        total: TOTAL,
        item: ITEM,
        body: Box::new(body),
    }
}

fn digits_after(total: Expr) -> Expr {
    apply(
        Op::Math(Math::Add),
        &[
            apply(Op::Math(Math::Multiply), &[total, int(10)]),
            Expr::Local(ITEM),
        ],
    )
}

#[test]
fn a_fold_over_a_sequence_takes_the_items_in_order() {
    let answer = both(&fold(
        seq_value(&[1, 2, 3]),
        int(0),
        digits_after(Expr::Local(TOTAL)),
    ));
    assert_eq!(answer, Ok(Value::Int(123)), "1, then 2, then 3");
}

#[test]
fn a_fold_over_a_set_takes_the_items_from_the_smallest() {
    let answer = both(&fold(
        set_value(&[3, 1, 2]),
        int(0),
        digits_after(Expr::Local(TOTAL)),
    ));
    assert_eq!(answer, Ok(Value::Int(123)), "the set's order is ascending");
}

#[test]
fn a_fold_over_nothing_gives_its_start() {
    let answer = both(&fold(set_value(&[]), int(7), int(0)));
    assert_eq!(answer, Ok(Value::Int(7)), "no item, no step");
}

#[test]
fn a_fold_that_adds_each_image_to_a_set_is_a_set_map() {
    let image = apply(Op::Math(Math::Remainder), &[Expr::Local(ITEM), int(2)]);
    let answer = both(&fold(
        set_value(&[1, 2, 3]),
        Expr::Const(set_value(&[])),
        apply(Op::Set(SetOp::Add), &[Expr::Local(TOTAL), image]),
    ));
    assert_eq!(answer, Ok(set_value(&[0, 1])), "1 % 2, 2 % 2 and 3 % 2");
}

#[test]
fn a_fault_in_a_step_of_a_fold_is_the_folds_fault() {
    let answer = both(&fold(
        set_value(&[1, 0]),
        int(0),
        apply(Op::Math(Math::Divide), &[int(6), Expr::Local(ITEM)]),
    ));
    assert_eq!(answer, Err(Fault::Division), "6 / 0 is taken first");
}

#[test]
fn a_fold_over_a_set_declared_a_sequence_is_of_the_wrong_kind() {
    let declared = Expr::Fold {
        shelf: Shelf::Seq,
        source: Box::new(Expr::Const(set_value(&[1]))),
        start: Box::new(int(0)),
        total: TOTAL,
        item: ITEM,
        body: Box::new(int(0)),
    };
    assert_eq!(
        both(&declared),
        Err(Fault::WrongKind),
        "a set is not a sequence"
    );
}

#[test]
fn a_fold_over_a_number_is_of_the_wrong_kind() {
    let answer = both(&fold(Value::Int(3), int(0), int(0)));
    assert_eq!(answer, Err(Fault::WrongKind), "a number has no items");
}

#[test]
fn a_must_get_gives_the_value_or_faults_on_a_missing_key() {
    let map = Expr::Const(map_value(&[(1, 10)]));
    let get = |key| both(&apply(Op::Map(MapOp::MustGet), &[map.clone(), int(key)]));
    assert_eq!(get(1), Ok(Value::Int(10)), "the key's value");
    assert_eq!(get(2), Err(Fault::Missing), "no value for 2");
}

#[test]
fn a_replace_changes_a_present_key_and_faults_on_a_missing_one() {
    let map = Expr::Const(map_value(&[(1, 10)]));
    let replace = |key| {
        both(&apply(
            Op::Map(MapOp::Replace),
            &[map.clone(), int(key), int(5)],
        ))
    };
    assert_eq!(replace(1), Ok(map_value(&[(1, 5)])), "1 now maps to 5");
    assert_eq!(replace(2), Err(Fault::Missing), "2 is not a key");
}

fn on_seq(op: SeqOp, arguments: &[Expr]) -> Result<Value, Fault> {
    both(&apply(Op::Seq(op), arguments))
}

fn seq(numbers: &[i64]) -> Expr {
    Expr::Const(seq_value(numbers))
}

#[test]
fn nth_counts_from_zero_and_faults_outside_the_sequence() {
    let at = |position| on_seq(SeqOp::Nth, &[seq(&[5, 6]), int(position)]);
    assert_eq!(at(0), Ok(Value::Int(5)), "the first item");
    assert_eq!(at(1), Ok(Value::Int(6)), "the second item");
    assert_eq!(at(2), Err(Fault::Missing), "past the end");
    assert_eq!(at(-1), Err(Fault::Missing), "before the start");
}

#[test]
fn head_and_tail_fault_on_an_empty_sequence() {
    assert_eq!(
        on_seq(SeqOp::Head, &[seq(&[5, 6])]),
        Ok(Value::Int(5)),
        "the first item"
    );
    assert_eq!(
        on_seq(SeqOp::Tail, &[seq(&[5, 6])]),
        Ok(seq_value(&[6])),
        "all but the first"
    );
    assert_eq!(
        on_seq(SeqOp::Head, &[seq(&[])]),
        Err(Fault::Missing),
        "no first item"
    );
    assert_eq!(
        on_seq(SeqOp::Tail, &[seq(&[])]),
        Err(Fault::Missing),
        "nothing to drop"
    );
}

#[test]
fn concat_joins_two_sequences_in_order() {
    assert_eq!(
        on_seq(SeqOp::Concat, &[seq(&[1, 2]), seq(&[3])]),
        Ok(seq_value(&[1, 2, 3])),
        "the first, then the second"
    );
}

#[test]
fn slice_takes_a_part_and_faults_on_positions_out_of_order_or_outside() {
    let slice = |from, to| on_seq(SeqOp::Slice, &[seq(&[1, 2, 3]), int(from), int(to)]);
    assert_eq!(slice(1, 3), Ok(seq_value(&[2, 3])), "from 1 up to 3");
    assert_eq!(slice(3, 3), Ok(seq_value(&[])), "an empty part at the end");
    assert_eq!(slice(2, 1), Err(Fault::Missing), "out of order");
    assert_eq!(slice(0, 4), Err(Fault::Missing), "past the end");
    assert_eq!(slice(-1, 2), Err(Fault::Missing), "before the start");
}

#[test]
fn replace_at_changes_one_item_and_faults_outside_the_sequence() {
    let replace = |position| on_seq(SeqOp::ReplaceAt, &[seq(&[1, 2, 3]), int(position), int(9)]);
    assert_eq!(replace(1), Ok(seq_value(&[1, 9, 3])), "the second item");
    assert_eq!(replace(3), Err(Fault::Missing), "past the end");
}

#[test]
fn a_power_is_defined_for_powers_from_zero_except_zero_to_zero() {
    let power = |base, exponent| both(&apply(Op::Power, &[int(base), int(exponent)]));
    assert_eq!(power(2, 3), Ok(Value::Int(8)), "2 cubed");
    assert_eq!(
        power(-2, 3),
        Ok(Value::Int(-8)),
        "an odd power keeps the sign"
    );
    assert_eq!(power(5, 0), Ok(Value::Int(1)), "anything else to the zero");
    assert_eq!(power(2, -1), Err(Fault::Undefined), "a negative power");
    assert_eq!(power(0, 0), Err(Fault::Undefined), "zero to the zero");
    assert_eq!(power(2, 63), Err(Fault::Overflow), "past 64 bits");
}

#[test]
fn a_power_beyond_32_bits_is_noted() {
    let mut frame = Frame::new(&[], 0);
    let answer = eval(&apply(Op::Power, &[int(2), int(40)]), &mut frame);
    assert_eq!(answer, Ok(Value::Int(1 << 40)), "2 to the 40");
    assert!(frame.computed_beyond_32_bits(), "TLC could not hold it");
}

#[test]
fn an_interval_holds_both_ends_and_is_empty_when_it_counts_down() {
    let interval = |lo, hi| both(&apply(Op::Interval, &[int(lo), int(hi)]));
    assert_eq!(interval(1, 3), Ok(set_value(&[1, 2, 3])), "1, 2 and 3");
    assert_eq!(interval(3, 1), Ok(set_value(&[])), "nothing from 3 up to 1");
}

#[test]
fn an_ascending_run_leaves_out_its_end_and_faults_when_it_counts_down() {
    let run = |lo, hi| both(&apply(Op::Ascending, &[int(lo), int(hi)]));
    assert_eq!(run(-2, 1), Ok(seq_value(&[-2, -1, 0])), "up to 1, not 1");
    assert_eq!(run(3, 3), Ok(seq_value(&[])), "empty");
    assert_eq!(run(5, 1), Err(Fault::Undefined), "it counts down");
}

#[test]
fn the_new_faults_say_what_went_wrong() {
    assert_eq!(
        Fault::Missing.describe(),
        "reads a key, a position or a case that is not there",
        "a missing key"
    );
    assert_eq!(
        Fault::Undefined.describe(),
        "computes a power or a run of numbers that is not defined",
        "an undefined power"
    );
}

#[test]
fn a_fold_inside_a_binding_of_its_own_slots_reads_the_fold_values() {
    let summed = Expr::Bind {
        slot: TOTAL,
        value: Box::new(int(100)),
        body: Box::new(fold(
            seq_value(&[1, 2]),
            Expr::Local(TOTAL),
            apply(
                Op::Math(Math::Add),
                &[Expr::Local(TOTAL), Expr::Local(ITEM)],
            ),
        )),
    };
    assert_eq!(both(&summed), Ok(Value::Int(103)), "100 + 1 + 2");
}
