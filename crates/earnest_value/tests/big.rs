//! A whole number past 64 bits prints as Quint's REPL writes it, is JSON as ITF writes it, and
//! sorts among the other numbers by its size.

use earnest_value::{Domain, Notation, Quint, Symbols, Value};
use serde_json::json;
use std::collections::BTreeSet;

fn number(text: &str) -> Value {
    Value::whole(text.parse().expect("digits"))
}

fn quint() -> Symbols {
    Symbols::default().written_in(Notation::Quint(Quint::default()))
}

#[test]
fn a_number_past_64_bits_prints_every_digit() {
    let symbols = quint();
    let set = Value::set(
        [
            number("-18446744073709551616"),
            Value::Int(3),
            number("18446744073709551616"),
        ]
        .into_iter()
        .collect(),
    );
    assert_eq!(
        symbols.show(&set),
        "Set(-18446744073709551616, 3, 18446744073709551616)"
    );
    assert_eq!(
        symbols.plain(&number("9223372036854775808")),
        "9223372036854775808"
    );
}

#[test]
fn a_number_past_64_bits_is_an_itf_bigint_in_json() {
    let symbols = quint();
    assert_eq!(
        symbols.json(&Value::seq(vec![
            Value::Int(1),
            number("-9223372036854775809")
        ])),
        json!([1, {"#bigint": "-9223372036854775809"}])
    );
}

#[test]
fn numbers_of_both_forms_sort_by_size_and_before_texts() {
    let mut symbols = Symbols::default();
    let text = Value::Str(symbols.intern("a"));
    let all: BTreeSet<Value> = [
        text.clone(),
        number("18446744073709551616"),
        Value::Int(i64::MAX),
        number("-18446744073709551616"),
        Value::Int(i64::MIN),
        Value::Bool(true),
        Value::Null,
    ]
    .into_iter()
    .collect();
    assert_eq!(
        all.into_iter().collect::<Vec<_>>(),
        [
            Value::Null,
            Value::Bool(true),
            number("-18446744073709551616"),
            Value::Int(i64::MIN),
            Value::Int(i64::MAX),
            number("18446744073709551616"),
            text,
        ]
    );
}

#[test]
fn only_the_domain_of_every_value_holds_a_number_past_64_bits() {
    let wide = number("18446744073709551616");
    assert!(Domain::Any.contains(&wide));
    assert!(
        !Domain::Int {
            lo: i64::MIN,
            hi: i64::MAX
        }
        .contains(&wide)
    );
}
