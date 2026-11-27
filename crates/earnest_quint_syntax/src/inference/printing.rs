use crate::inference::value::Value;

pub(crate) fn type_text(written: &Value) -> String {
    match written.kind() {
        kind @ ("bool" | "int" | "str") => kind.to_owned(),
        "const" | "var" => written.str("name").to_owned(),
        "set" => format!("Set[{}]", type_text(written.get("elem"))),
        "list" => format!("List[{}]", type_text(written.get("elem"))),
        "fun" => format!(
            "({} -> {})",
            type_text(written.get("arg")),
            type_text(written.get("res"))
        ),
        "oper" => format!(
            "({}) => {}",
            listed(written.items("args")),
            type_text(written.get("res"))
        ),
        "tup" => format!(
            "({})",
            row_fields_text(written.get("fields"), Labels::Hidden)
        ),
        "rec" => row_text(written.get("fields")),
        "sum" => format!("({})", sum_fields_text(written.get("fields"))),
        "app" => format!(
            "{}[{}]",
            type_text(written.get("ctor")),
            listed(written.items("args"))
        ),
        _ => String::new(),
    }
}

fn listed(types: &[Value]) -> String {
    types.iter().map(type_text).collect::<Vec<_>>().join(", ")
}

pub(crate) fn row_text(row: &Value) -> String {
    let fields = row_fields_text(row, Labels::Shown);
    if fields.is_empty() {
        "{}".to_owned()
    } else {
        format!("{{ {fields} }}")
    }
}

pub(crate) fn is_unit(written: &Value) -> bool {
    let fields = written.get("fields");
    written.kind() == "tup"
        && fields.kind() == "row"
        && fields.items("fields").is_empty()
        && fields.get("other").kind() == "empty"
}

fn sum_fields_text(row: &Value) -> String {
    let mut shown: Vec<String> = row
        .items("fields")
        .iter()
        .map(|field| {
            let name = field.str("fieldName");
            let field_type = field.get("fieldType");
            if is_unit(field_type) {
                name.to_owned()
            } else {
                format!("{name}({})", type_text(field_type))
            }
        })
        .collect();
    let other = row.get("other");
    if other.kind() == "row" {
        shown.push(sum_fields_text(other));
    }
    shown.join(" | ")
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Labels {
    Shown,
    Hidden,
}

fn row_fields_text(row: &Value, labels: Labels) -> String {
    match row.kind() {
        "var" => format!("| {}", row.str("name")),
        "row" => {
            let mut fields: Vec<String> = row
                .items("fields")
                .iter()
                .map(|field| {
                    let shown = type_text(field.get("fieldType"));
                    if labels == Labels::Shown {
                        format!("{}: {shown}", field.str("fieldName"))
                    } else {
                        shown
                    }
                })
                .collect();
            let other = row.get("other");
            let rest = row_fields_text(other, Labels::Shown);
            match other.kind() {
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

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Names {
    pub(crate) types: Vec<String>,
    pub(crate) rows: Vec<String>,
}

pub(crate) fn add(names: &mut Vec<String>, name: &str) {
    if !names.iter().any(|known| known == name) {
        names.push(name.to_owned());
    }
}

pub(crate) fn type_names(written: &Value) -> Names {
    let mut names = Names::default();
    collect_type(written, &mut names);
    names
}

fn collect_type(written: &Value, names: &mut Names) {
    match written.kind() {
        "var" => add(&mut names.types, written.str("name")),
        "set" | "list" => collect_type(written.get("elem"), names),
        "fun" => {
            collect_type(written.get("arg"), names);
            collect_type(written.get("res"), names);
        }
        "oper" => {
            for argument in written.items("args") {
                collect_type(argument, names);
            }
            collect_type(written.get("res"), names);
        }
        "tup" | "rec" | "sum" => collect_row(written.get("fields"), names),
        "app" => {
            collect_type(written.get("ctor"), names);
            for argument in written.items("args") {
                collect_type(argument, names);
            }
        }
        _ => {}
    }
}

fn collect_row(row: &Value, names: &mut Names) {
    match row.kind() {
        "row" => {
            for field in row.items("fields") {
                collect_type(field.get("fieldType"), names);
            }
            collect_row(row.get("other"), names);
        }
        "var" => add(&mut names.rows, row.str("name")),
        _ => {}
    }
}

pub(crate) fn row_names(row: &Value) -> Vec<String> {
    match row.kind() {
        "row" => row_names(row.get("other")),
        "var" => vec![row.str("name").to_owned()],
        _ => Vec::new(),
    }
}

pub(crate) fn expression_text(expression: &Value) -> String {
    crate::printing_ir::expression_text(&expression.to_json())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ErrorTree {
    pub(crate) message: Option<String>,
    pub(crate) location: String,
    pub(crate) children: Vec<ErrorTree>,
}

#[derive(Clone, Debug)]
pub(crate) enum Failure {
    Tree(ErrorTree),
    Trees(Vec<ErrorTree>),
}

pub(crate) fn leaf(location: &str, message: &str) -> ErrorTree {
    ErrorTree {
        message: Some(message.to_owned()),
        location: location.to_owned(),
        children: Vec::new(),
    }
}

pub(crate) fn tree(location: &str, errors: Failure) -> ErrorTree {
    match errors {
        Failure::Tree(error) if error.location == location => error,
        Failure::Tree(error) => ErrorTree {
            message: None,
            location: location.to_owned(),
            children: vec![error],
        },
        Failure::Trees(errors) => ErrorTree {
            message: None,
            location: location.to_owned(),
            children: errors,
        },
    }
}

pub(crate) fn error_text(error: &ErrorTree) -> String {
    let mut out = error
        .children
        .iter()
        .map(error_text)
        .collect::<Vec<_>>()
        .join("and\n");
    if let Some(message) = &error.message
        && !message.is_empty()
    {
        out.push_str(message);
        out.push('\n');
    }
    out.push_str(&error.location);
    out.push('\n');
    out
}

pub(crate) fn scheme_text(scheme: &crate::inference::signatures::Scheme) -> String {
    use crate::inference::solver::Context;
    use crate::inference::substitution::{Substitution, apply, compose};
    let mut names = Vec::new();
    let renamed = |prefix: &str, found: &[String], row: bool, names: &mut Vec<String>| {
        found
            .iter()
            .enumerate()
            .map(|(at, name)| {
                let shown = format!("{prefix}{at}");
                names.push(shown.clone());
                Substitution {
                    row,
                    name: name.clone(),
                    value: Value::object(vec![
                        ("kind", Value::text("var")),
                        ("name", Value::Text(shown)),
                    ]),
                }
            })
            .collect::<Vec<_>>()
    };
    let types = renamed("t", &scheme.types, false, &mut names);
    let rows = renamed("r", &scheme.rows, true, &mut names);
    let substitutions = compose(&Context::default(), &types, &rows);
    let shown = type_text(&apply(&substitutions, &scheme.value));
    if names.is_empty() {
        shown
    } else {
        format!("∀ {} . {shown}", names.join(", "))
    }
}
