//! Quint's notation writes values as its REPL and its ITF traces do, and earnest's stays the default.

use earnest_value::{Notation, Quint, Symbols, Value};
use std::collections::{BTreeMap, BTreeSet};

const BOTH: Quint = Quint {
    tuples: true,
    variants: true,
};

fn tuple(symbols: &mut Symbols, items: Vec<Value>) -> Value {
    let fields = items
        .into_iter()
        .enumerate()
        .map(|(position, item)| (symbols.intern(&format!("_{}", position + 1)), item))
        .collect();
    Value::record(fields)
}

fn variant(symbols: &mut Symbols, tag: &str, value: Value) -> Value {
    let tag = Value::Str(symbols.intern(tag));
    Value::record(vec![
        (symbols.intern("tag"), tag),
        (symbols.intern("value"), value),
    ])
}

#[test]
fn quint_s_notation_writes_tuples_variants_sets_maps_and_records_as_its_repl_does() {
    let mut symbols = Symbols::default();
    let unit = tuple(&mut symbols, Vec::new());
    let pair = tuple(&mut symbols, vec![Value::Int(1), Value::Int(5)]);
    let empty = variant(&mut symbols, "Aa", unit.clone());
    let held = variant(&mut symbols, "Some", Value::Int(3));
    let of_pair = variant(&mut symbols, "Cc", pair.clone());
    let a = symbols.intern("a");
    let b = symbols.intern("b");
    let named = Value::record(vec![
        (b, Value::Str(symbols.intern("x"))),
        (a, Value::Int(1)),
    ]);
    let set = Value::set(BTreeSet::from([Value::Int(2), Value::Int(1)]));
    let map = Value::map(BTreeMap::from([(pair.clone(), held.clone())]));
    let list = Value::seq(vec![Value::Int(1), Value::Int(2)]);
    let all = [unit, pair, empty, held, of_pair, named, set, map, list];
    let quint = symbols.clone().written_in(Notation::Quint(BOTH));
    let shown: Vec<String> = all.iter().map(|value| quint.show(value)).collect();
    assert_eq!(
        shown,
        [
            "()",
            "(1, 5)",
            "Aa",
            "Some(3)",
            "Cc((1, 5))",
            "{ a: 1, b: \"x\" }",
            "Set(1, 2)",
            "Map((1, 5) -> Some(3))",
            "[1, 2]",
        ],
        "as `quint -r` prints each"
    );
    let earnest: Vec<String> = all.iter().map(|value| symbols.show(value)).collect();
    assert_eq!(
        earnest,
        [
            "{}",
            "{_1: 1, _2: 5}",
            "{tag: \"Aa\", value: {}}",
            "{tag: \"Some\", value: 3}",
            "{tag: \"Cc\", value: {_1: 1, _2: 5}}",
            "{a: 1, b: \"x\"}",
            "{1, 2}",
            "{{_1: 1, _2: 5} -> {tag: \"Some\", value: 3}}",
            "[1, 2]",
        ],
        "earnest's own notation is the default and is unchanged"
    );
}

#[test]
fn a_tuple_of_ten_items_is_written_in_the_order_of_its_positions() {
    let mut symbols = Symbols::default();
    for position in 1..=10 {
        symbols.intern(&format!("_{position}"));
    }
    let quint = symbols.written_in(Notation::Quint(BOTH)).alphabetical();
    let mut again = quint.clone();
    let ten_again = tuple(&mut again, (1..=10).map(Value::Int).collect());
    assert_eq!(
        again.show(&ten_again),
        "(1, 2, 3, 4, 5, 6, 7, 8, 9, 10)",
        "`_10` sorts before `_2` as a text, not as a position"
    );
    assert_eq!(
        quint.notation(),
        Notation::Quint(BOTH),
        "alphabetical keeps the notation"
    );
}

#[test]
fn a_record_that_only_looks_like_a_tuple_or_a_variant_is_a_record() {
    let mut symbols = Symbols::default();
    let gap = Value::record(vec![
        (symbols.intern("_1"), Value::Int(1)),
        (symbols.intern("_3"), Value::Int(3)),
    ]);
    let leading_zero = Value::record(vec![(symbols.intern("_01"), Value::Int(1))]);
    let number_tag = Value::record(vec![
        (symbols.intern("tag"), Value::Int(1)),
        (symbols.intern("value"), Value::Int(2)),
    ]);
    let text_tag = variant(&mut symbols, "a", Value::Int(0));
    let quint = symbols.clone().written_in(Notation::Quint(BOTH));
    assert_eq!(
        quint.show(&gap),
        "{ _1: 1, _3: 3 }",
        "a position is missing"
    );
    assert_eq!(quint.show(&leading_zero), "{ _01: 1 }", "not a position");
    assert_eq!(
        quint.show(&number_tag),
        "{ tag: 1, value: 2 }",
        "the tag is not a text"
    );
    let records_only = symbols.written_in(Notation::Quint(Quint {
        tuples: true,
        variants: false,
    }));
    assert_eq!(
        records_only.show(&text_tag),
        "{ tag: \"a\", value: 0 }",
        "a module that writes such a record has its variants written as records"
    );
}

#[test]
fn quint_s_json_holds_a_tuple_and_a_variant_as_its_itf_traces_do() {
    let mut symbols = Symbols::default();
    let unit = tuple(&mut symbols, Vec::new());
    let pair = tuple(&mut symbols, vec![Value::Int(1), Value::Int(5)]);
    let empty = variant(&mut symbols, "Aa", unit);
    let quint = symbols.clone().written_in(Notation::Quint(BOTH));
    assert_eq!(
        quint.json(&pair),
        serde_json::json!({ "#tup": [1, 5] }),
        "a tuple"
    );
    assert_eq!(
        quint.json(&empty),
        serde_json::json!({ "tag": "Aa", "value": { "#tup": [] } }),
        "a variant without a value"
    );
    assert_eq!(
        symbols.json(&pair),
        serde_json::json!({ "_1": 1, "_2": 5 }),
        "earnest's own JSON is unchanged"
    );
}
