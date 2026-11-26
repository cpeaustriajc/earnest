//! The forms that make a Quint model give the answers Quint's TLC run gives: a set's items in
//! TLC's order, a map built from pairs that must not give one key two values, and ranges too
//! wide to build. Each case is evaluated by the reference walk and by the prepared [`Code`].

use earnest_ir::{Code, Expr, Fault, Frame, MOST_RANGE_VALUES, MapOp, Op, eval};
use earnest_value::{Symbols, Value};

struct Agreed(Result<Value, Fault>);

fn agreed(expr: &Expr) -> Agreed {
    let reference = eval(expr, &mut Frame::new(&[], 0));
    let code = Code::new(expr.clone()).eval(&mut Frame::new(&[], 0));
    assert_eq!(reference, code, "the walk and the code agree on {expr:?}");
    Agreed(reference)
}

fn both(expr: &Expr) -> Result<Value, Fault> {
    agreed(expr).0
}

fn int(number: i64) -> Value {
    Value::Int(number)
}

fn set(items: &[Value]) -> Value {
    Value::set(items.iter().cloned().collect())
}

fn seq(items: &[Value]) -> Value {
    Value::seq(items.to_vec())
}

fn ints(numbers: &[i64]) -> Vec<Value> {
    numbers.iter().copied().map(Value::Int).collect()
}

fn map(pairs: &[(Value, Value)]) -> Value {
    Value::map(pairs.iter().cloned().collect())
}

fn in_tlc_order(items: &[Value]) -> Result<Value, Fault> {
    both(&Expr::Apply(Op::InTlcOrder, vec![Expr::Const(set(items))]))
}

fn apply(op: Op, arguments: &[Value]) -> Result<Value, Fault> {
    both(&Expr::Apply(
        op,
        arguments.iter().cloned().map(Expr::Const).collect(),
    ))
}

#[test]
fn numbers_keep_their_order() {
    assert_eq!(
        in_tlc_order(&ints(&[3, -1, 2])),
        Ok(seq(&ints(&[-1, 2, 3])))
    );
}

#[test]
fn smaller_sets_come_first_then_item_by_item() {
    let (small, large, other) = (
        set(&ints(&[2])),
        set(&ints(&[1, 2, 3])),
        set(&ints(&[1, 4])),
    );
    let ordered = in_tlc_order(&[large.clone(), small.clone(), other.clone(), set(&[])]);
    assert_eq!(ordered, Ok(seq(&[set(&[]), small, other, large])));
}

#[test]
fn shorter_lists_come_first_then_item_by_item() {
    let lists = [
        seq(&ints(&[1, 2, 3])),
        seq(&ints(&[2])),
        seq(&ints(&[1, 9])),
        seq(&ints(&[1, 2])),
    ];
    let expected = [
        seq(&ints(&[2])),
        seq(&ints(&[1, 2])),
        seq(&ints(&[1, 9])),
        seq(&ints(&[1, 2, 3])),
    ];
    assert_eq!(in_tlc_order(&lists), Ok(seq(&expected)));
}

#[test]
fn maps_compare_their_size_then_every_key_then_the_values() {
    let one = map(&[(int(5), int(0))]);
    let low_keys = map(&[(int(1), int(9)), (int(2), int(9))]);
    let high_keys = map(&[(int(1), int(0)), (int(3), int(0))]);
    let ordered = in_tlc_order(&[high_keys.clone(), low_keys.clone(), one.clone()]);
    assert_eq!(ordered, Ok(seq(&[one, low_keys, high_keys])));
}

#[test]
fn sets_inside_sets_are_put_in_the_same_order_before_they_are_compared() {
    let early = set(&[set(&ints(&[5])), set(&ints(&[1, 2]))]);
    let late = set(&[set(&ints(&[6])), set(&ints(&[0, 1]))]);
    assert_eq!(
        in_tlc_order(&[late.clone(), early.clone()]),
        Ok(seq(&[early, late])),
        "{{5}} is the first item of both before {{1, 2}} and {{6}} before {{0, 1}}, and 5 < 6"
    );
}

#[test]
fn records_compare_their_fields_by_name() {
    let mut symbols = Symbols::default();
    let (a, b) = (symbols.intern("a"), symbols.intern("b"));
    let record = |first: i64, second: i64| Value::record(vec![(a, int(first)), (b, int(second))]);
    assert_eq!(
        in_tlc_order(&[record(2, 0), record(1, 5), record(1, 3)]),
        Ok(seq(&[record(1, 3), record(1, 5), record(2, 0)]))
    );
}

#[test]
fn a_key_given_its_value_again_is_kept_and_another_value_is_a_fault() {
    let start = map(&[(int(1), int(2))]);
    assert_eq!(
        apply(Op::Map(MapOp::Once), &[start.clone(), int(1), int(2)]),
        Ok(start.clone())
    );
    assert_eq!(
        apply(Op::Map(MapOp::Once), &[start.clone(), int(2), int(3)]),
        Ok(map(&[(int(1), int(2)), (int(2), int(3))]))
    );
    assert_eq!(
        apply(Op::Map(MapOp::Once), &[start, int(1), int(3)]),
        Err(Fault::KeyTwice)
    );
    assert!(Fault::KeyTwice.describe().contains("`setToMap`"));
}

#[test]
fn a_range_of_the_most_numbers_is_built_and_a_wider_one_is_a_fault() {
    let most = i64::try_from(MOST_RANGE_VALUES).expect("the limit fits");
    let built = apply(Op::Interval, &[int(1), int(most)]);
    assert!(matches!(built, Ok(Value::Set(items)) if items.len() == 1_000_000));
    assert_eq!(
        apply(Op::Interval, &[int(0), int(most)]),
        Err(Fault::TooWide)
    );
    assert_eq!(
        apply(Op::Interval, &[int(0), int(i64::MAX)]),
        Err(Fault::TooWide)
    );
    assert_eq!(
        apply(Op::Interval, &[int(5), int(-5_000_000)]),
        Ok(set(&[]))
    );
    assert_eq!(
        apply(Op::Ascending, &[int(-1), int(most)]),
        Err(Fault::TooWide)
    );
    assert!(
        Fault::TooWide
            .describe()
            .contains("`a.to(b)` or `range(a, b)`) of more than the 1000000 numbers")
    );
}
