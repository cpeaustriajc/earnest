use crate::json::Json;

pub(crate) fn unbound_type_variables(written: &Json) -> Vec<String> {
    let mut types = Vec::new();
    let mut rows = Vec::new();
    collect(written, &mut types, &mut rows);
    types.extend(rows);
    types
}

fn add(names: &mut Vec<String>, name: Option<&Json>) {
    if let Some(Json::Text(name)) = name
        && !names.contains(name)
    {
        names.push(name.clone());
    }
}

fn collect(written: &Json, types: &mut Vec<String>, rows: &mut Vec<String>) {
    let kind = match written.get("kind") {
        Some(Json::Text(kind)) => kind.as_str(),
        _ => return,
    };
    match kind {
        "var" => add(types, written.get("name")),
        "set" | "list" => {
            if let Some(element) = written.get("elem") {
                collect(element, types, rows);
            }
        }
        "fun" => {
            for key in ["arg", "res"] {
                if let Some(inner) = written.get(key) {
                    collect(inner, types, rows);
                }
            }
        }
        "oper" | "app" => {
            if let Some(Json::List(args)) = written.get("args") {
                for arg in args {
                    collect(arg, types, rows);
                }
            }
            if let Some(result) = written.get("res") {
                collect(result, types, rows);
            }
        }
        "tup" | "rec" | "sum" => {
            if let Some(fields) = written.get("fields") {
                collect_row(fields, types, rows);
            }
        }
        _ => {}
    }
}

fn collect_row(written: &Json, types: &mut Vec<String>, rows: &mut Vec<String>) {
    match written.get("kind") {
        Some(Json::Text(kind)) if kind == "row" => {
            if let Some(Json::List(fields)) = written.get("fields") {
                for field in fields {
                    if let Some(field_type) = field.get("fieldType") {
                        collect(field_type, types, rows);
                    }
                }
            }
            if let Some(other) = written.get("other") {
                collect_row(other, types, rows);
            }
        }
        Some(Json::Text(kind)) if kind == "var" => add(rows, written.get("name")),
        _ => {}
    }
}

pub(crate) fn type_text(written: &Json) -> String {
    crate::inference::printed_type(written)
}
