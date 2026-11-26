use crate::json::{Compiled, Declaration, Def};
use crate::reader::Reader;
use earnest_ir::{Fault, Frame, eval};
use earnest_value::{Symbols, Value};
use std::collections::HashMap;

const TABLE: &str = include_str!("../fixtures/table.json");
const TAG: &str = "tag";
const VALUE: &str = "value";

enum Quint {
    Value(&'static str),
    Reordered(&'static str, &'static str),
    Error(&'static str, Fault),
}

const CASES: [(&str, Quint); 84] = [
    ("rec_build", Quint::Value(r#"{ a: 1, b: "x" }"#)),
    ("rec_field", Quint::Value("2")),
    ("rec_with", Quint::Value("{ a: 5, b: 2 }")),
    ("rec_spread", Quint::Value("{ a: 1, b: 7 }")),
    ("tup_build", Quint::Value(r#"(1, "a", true)"#)),
    ("tup_item", Quint::Value(r#""a""#)),
    ("tup_unit", Quint::Value("()")),
    ("tuples_two", Quint::Value(r#"Set((1, "a"), (2, "a"))"#)),
    ("tuples_none", Quint::Value("Set()")),
    ("variant_bare", Quint::Value("V1")),
    ("variant_value", Quint::Value("V2(3)")),
    ("match_value", Quint::Value("6")),
    ("match_bare", Quint::Value("10")),
    ("match_default", Quint::Value("99")),
    ("set_build", Quint::Value("Set(1, 2, 3)")),
    ("set_empty", Quint::Value("Set(1)")),
    ("set_contains", Quint::Value("true")),
    ("set_in", Quint::Value("false")),
    ("set_union", Quint::Value("Set(1, 2, 3)")),
    ("set_intersect", Quint::Value("Set(2)")),
    ("set_exclude", Quint::Value("Set(1, 3)")),
    ("set_subseteq", Quint::Value("true")),
    ("set_not_subseteq", Quint::Value("false")),
    ("set_size", Quint::Value("3")),
    ("set_filter", Quint::Value("Set(2, 4)")),
    ("set_exists", Quint::Value("true")),
    ("set_exists_empty", Quint::Value("false")),
    ("set_forall", Quint::Value("false")),
    ("set_forall_empty", Quint::Value("true")),
    ("set_map", Quint::Value("Set(0, 1)")),
    ("set_map_records", Quint::Value("Set({ n: 1 }, { n: 2 })")),
    ("set_fold_sum", Quint::Value("6")),
    ("set_fold_empty", Quint::Value("7")),
    ("set_flatten", Quint::Value("Set(1, 2, 3)")),
    (
        "set_powerset",
        Quint::Reordered(
            "Set(Set(), Set(1), Set(2), Set(1, 2))",
            "Set(Set(), Set(1), Set(1, 2), Set(2))",
        ),
    ),
    ("set_powerset_empty", Quint::Value("Set(Set())")),
    ("set_is_finite", Quint::Value("true")),
    ("set_to", Quint::Value("Set(1, 2, 3, 4)")),
    ("set_to_empty", Quint::Value("Set()")),
    ("map_build", Quint::Value(r#"Map(1 -> "a", 2 -> "b")"#)),
    ("map_empty", Quint::Value("Map()")),
    ("map_get", Quint::Value("20")),
    ("map_get_missing", Quint::Error("QNT507", Fault::Missing)),
    ("map_keys", Quint::Value("Set(1, 2)")),
    ("map_put_new", Quint::Value("Map(1 -> 10, 2 -> 20)")),
    ("map_put_old", Quint::Value("Map(1 -> 5)")),
    ("map_set", Quint::Value("Map(1 -> 5)")),
    ("map_set_missing", Quint::Error("QNT507", Fault::Missing)),
    ("map_set_by", Quint::Value("Map(1 -> 11)")),
    ("map_set_by_missing", Quint::Error("QNT507", Fault::Missing)),
    ("map_map_by", Quint::Value("Map(1 -> 10, 2 -> 20)")),
    ("map_set_to_map", Quint::Value(r#"Map(1 -> "a", 2 -> "b")"#)),
    ("list_build", Quint::Value("[3, 1, 2]")),
    ("list_append", Quint::Value("[1, 2, 3]")),
    ("list_concat", Quint::Value("[1, 2, 3, 1]")),
    ("list_head", Quint::Value("5")),
    ("list_head_empty", Quint::Error("QNT505", Fault::Missing)),
    ("list_tail", Quint::Value("[6, 7]")),
    ("list_tail_empty", Quint::Error("QNT505", Fault::Missing)),
    ("list_length", Quint::Value("3")),
    ("list_nth", Quint::Value("6")),
    ("list_nth_out", Quint::Error("QNT510", Fault::Missing)),
    ("list_nth_negative", Quint::Error("QNT510", Fault::Missing)),
    ("list_indices", Quint::Value("Set(0, 1, 2)")),
    ("list_indices_empty", Quint::Value("Set()")),
    ("list_replace_at", Quint::Value("[1, 9, 3]")),
    (
        "list_replace_at_out",
        Quint::Error("QNT510", Fault::Missing),
    ),
    ("list_slice", Quint::Value("[2, 3]")),
    ("list_slice_empty", Quint::Value("[]")),
    ("list_slice_out", Quint::Error("QNT506", Fault::Missing)),
    (
        "list_slice_backwards",
        Quint::Error("QNT506", Fault::Missing),
    ),
    ("list_range", Quint::Value("[1, 2, 3]")),
    ("list_range_empty", Quint::Value("[]")),
    ("list_select", Quint::Value("[3, 4]")),
    ("list_foldl", Quint::Value("123")),
    ("list_foldl_empty", Quint::Value("5")),
    ("int_pow", Quint::Value("1024")),
    ("int_pow_zero", Quint::Value("1")),
    ("int_pow_negative_base", Quint::Value("-27")),
    ("int_pow_negative", Quint::Error("QNT503", Fault::Undefined)),
    (
        "int_pow_zero_zero",
        Quint::Error("QNT503", Fault::Undefined),
    ),
    ("text_equal", Quint::Value("true")),
    ("text_set", Quint::Value(r#"Set("a", "b", "c")"#)),
    (
        "nested_order",
        Quint::Value(r#"Set({ k: 1, v: "b" }, { k: 2, v: "a" })"#),
    ),
];

fn read_all(compiled: &Compiled, symbols: Symbols) -> (Vec<Result<Value, Fault>>, Symbols) {
    let defs: HashMap<u64, &Def> = compiled
        .modules
        .iter()
        .flat_map(|module| &module.declarations)
        .filter_map(|declaration| match declaration {
            Declaration::Def(def) => Some((def.id, def)),
            _ => None,
        })
        .collect();
    let mut reader = Reader::new(compiled, defs, HashMap::new(), symbols);
    let mut values = Vec::with_capacity(CASES.len());
    for (name, _) in &CASES {
        let def = reader
            .top(name)
            .unwrap_or_else(|| panic!("the table module defines {name}"));
        (*name).clone_into(&mut reader.within);
        let expr = reader
            .pure(&def.expr)
            .unwrap_or_else(|refusal| panic!("{name} is read: {}", refusal.why));
        values.push(eval(&expr, &mut Frame::new(&[], reader.slots)));
    }
    (values, std::mem::take(&mut reader.symbols))
}

fn listed(open: &str, items: impl Iterator<Item = String>, close: &str) -> String {
    format!("{open}{}{close}", items.collect::<Vec<String>>().join(", "))
}

fn as_quint_prints(value: &Value, symbols: &Symbols) -> String {
    let show = |inner: &Value| as_quint_prints(inner, symbols);
    match value {
        Value::Bool(flag) => flag.to_string(),
        Value::Int(number) => number.to_string(),
        Value::Str(text) => format!("\"{}\"", symbols.name(*text)),
        Value::Set(items) => listed("Set(", items.iter().map(show), ")"),
        Value::Seq(items) => listed("[", items.iter().map(show), "]"),
        Value::Map(entries) => listed(
            "Map(",
            entries
                .iter()
                .map(|(key, inner)| format!("{} -> {}", show(key), show(inner))),
            ")",
        ),
        Value::Record(fields) => {
            let names: Vec<&str> = fields.iter().map(|(name, _)| symbols.name(*name)).collect();
            if let ([TAG, VALUE], [(_, Value::Str(tag)), (_, inner)]) =
                (names.as_slice(), fields.as_slice())
            {
                return match inner {
                    Value::Record(empty) if empty.is_empty() => symbols.name(*tag).to_owned(),
                    other => format!("{}({})", symbols.name(*tag), show(other)),
                };
            }
            let tuple = names
                .iter()
                .enumerate()
                .all(|(index, name)| *name == format!("_{}", index + 1));
            if tuple {
                return listed("(", fields.iter().map(|(_, inner)| show(inner)), ")");
            }
            let shown = fields
                .iter()
                .map(|(name, inner)| format!("{}: {}", symbols.name(*name), show(inner)));
            listed("{ ", shown, " }")
        }
        _ => "null".to_owned(),
    }
}

#[test]
fn every_operator_of_the_table_gives_the_value_or_the_error_quints_own_evaluator_gives() {
    let compiled: Compiled = serde_json::from_str(TABLE).expect("the table is compiled Quint");
    let (_, met) = read_all(&compiled, Symbols::default());
    let (values, symbols) = read_all(&compiled, met.alphabetical());
    for ((name, quint), value) in CASES.iter().zip(values) {
        match (quint, value) {
            (Quint::Value(printed), Ok(found)) => assert_eq!(
                as_quint_prints(&found, &symbols),
                *printed,
                "{name} has the value quint repl prints"
            ),
            (Quint::Reordered(printed, in_order), Ok(found)) => assert_eq!(
                as_quint_prints(&found, &symbols),
                *in_order,
                "{name} has the items quint repl prints as {printed}, in earnest's order"
            ),
            (Quint::Error(code, fault), found) => assert_eq!(
                found,
                Err(*fault),
                "{name} is an error in Quint ({code}) and a fault in earnest"
            ),
            (Quint::Value(printed) | Quint::Reordered(printed, _), Err(fault)) => {
                panic!("{name} faults with {fault:?}, but Quint gives {printed}")
            }
        }
    }
}
