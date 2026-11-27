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

fn text_field<'a>(written: &'a Json, key: &str) -> &'a str {
    match written.get(key) {
        Some(Json::Text(text)) => text,
        _ => "",
    }
}

fn listed(written: Option<&Json>) -> Vec<String> {
    match written {
        Some(Json::List(items)) => items.iter().map(type_text).collect(),
        _ => Vec::new(),
    }
}

pub(crate) fn type_text(written: &Json) -> String {
    let inner = |key: &str| written.get(key).map_or_else(String::new, type_text);
    match text_field(written, "kind") {
        kind @ ("bool" | "int" | "str") => kind.to_owned(),
        "const" | "var" => text_field(written, "name").to_owned(),
        "set" => format!("Set[{}]", inner("elem")),
        "list" => format!("List[{}]", inner("elem")),
        "fun" => format!("({} -> {})", inner("arg"), inner("res")),
        "oper" => format!(
            "({}) => {}",
            listed(written.get("args")).join(", "),
            inner("res")
        ),
        "tup" => format!(
            "({})",
            row_text(written.get("fields").unwrap_or(&Json::Null), Labels::Hidden)
        ),
        "rec" => {
            let fields = row_text(written.get("fields").unwrap_or(&Json::Null), Labels::Shown);
            if fields.is_empty() {
                "{}".to_owned()
            } else {
                format!("{{ {fields} }}")
            }
        }
        "app" => format!(
            "{}[{}]",
            written.get("ctor").map_or_else(String::new, type_text),
            listed(written.get("args")).join(", ")
        ),
        _ => String::new(),
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Labels {
    Shown,
    Hidden,
}

fn row_text(written: &Json, labels: Labels) -> String {
    match text_field(written, "kind") {
        "var" => format!("| {}", text_field(written, "name")),
        "row" => {
            let mut fields: Vec<String> = match written.get("fields") {
                Some(Json::List(fields)) => fields
                    .iter()
                    .map(|field| {
                        let shown = field.get("fieldType").map_or_else(String::new, type_text);
                        if labels == Labels::Shown {
                            format!("{}: {shown}", text_field(field, "fieldName"))
                        } else {
                            shown
                        }
                    })
                    .collect(),
                _ => Vec::new(),
            };
            let other = written.get("other").unwrap_or(&Json::Null);
            let rest = row_text(other, Labels::Shown);
            match text_field(other, "kind") {
                "row" => {
                    fields.push(rest);
                    fields.join(", ")
                }
                "var" => format!("{} {rest}", fields.join(", ")),
                _ => fields.join(", "),
            }
        }
        _ => String::new(),
    }
}
