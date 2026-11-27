use crate::inference::solver::Context;
use crate::inference::value::Value;

pub(crate) fn simplify_type(context: &Context, written: &Value) -> Value {
    let mut copy = written.clone();
    match written.kind() {
        "set" | "list" => copy.set("elem", simplify_type(context, written.get("elem"))),
        "fun" => {
            copy.set("arg", simplify_type(context, written.get("arg")));
            copy.set("res", simplify_type(context, written.get("res")));
        }
        "oper" => {
            copy.set(
                "args",
                Value::List(
                    written
                        .items("args")
                        .iter()
                        .map(|argument| simplify_type(context, argument))
                        .collect(),
                ),
            );
            copy.set("res", simplify_type(context, written.get("res")));
        }
        "tup" | "rec" => copy.set("fields", simplify_row(context, written.get("fields"))),
        "sum" => {
            let fields = simplify_row(context, written.get("fields"));
            if fields.kind() != "row" {
                context.crash("Impossible: sum type fields transformed into non-row".to_owned());
            }
            copy.set("fields", fields);
        }
        "app" => copy.set(
            "args",
            Value::List(
                written
                    .items("args")
                    .iter()
                    .map(|argument| simplify_type(context, argument))
                    .collect(),
            ),
        ),
        _ => {}
    }
    copy
}

pub(crate) fn simplify_row(context: &Context, row: &Value) -> Value {
    let mut copy = row.clone();
    if row.kind() == "row" {
        copy.set(
            "fields",
            Value::List(
                row.items("fields")
                    .iter()
                    .map(|field| {
                        field.spread(vec![(
                            "fieldType",
                            simplify_type(context, field.get("fieldType")),
                        )])
                    })
                    .collect(),
            ),
        );
        copy.set("other", simplify_row(context, row.get("other")));
    }
    flattened(copy)
}

fn flattened(row: Value) -> Value {
    if row.kind() != "row" {
        return row;
    }
    if row.items("fields").is_empty() {
        return row.get("other").clone();
    }
    let other = row.get("other");
    if other.kind() != "row" {
        return row;
    }
    let mut fields = row.items("fields").to_vec();
    fields.extend(other.items("fields").iter().cloned());
    let rest = other.get("other").clone();
    row.spread(vec![("fields", Value::List(fields)), ("other", rest)])
}
