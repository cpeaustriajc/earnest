use crate::json::Json;
use crate::types::type_text;

fn text<'a>(value: &'a Json, key: &str) -> &'a str {
    match value.get(key) {
        Some(Json::Text(text)) => text,
        _ => "",
    }
}

fn items<'a>(value: &'a Json, key: &str) -> &'a [Json] {
    match value.get(key) {
        Some(Json::List(items)) => items,
        _ => &[],
    }
}

pub(crate) fn qualifier_text(qualifier: &str) -> &str {
    match qualifier {
        "puredef" => "pure def",
        "pureval" => "pure val",
        other => other,
    }
}

pub(crate) fn expression_text(expression: &Json) -> String {
    let mut out = String::new();
    write_expression(expression, &mut out);
    out
}

fn write_expression(expression: &Json, out: &mut String) {
    match text(expression, "kind") {
        "name" => out.push_str(text(expression, "name")),
        "bool" | "int" => {
            if let Some(value) = expression.get("value") {
                out.push_str(&value.to_string());
            }
        }
        "str" => {
            out.push('"');
            out.push_str(text(expression, "value"));
            out.push('"');
        }
        "app" => {
            out.push_str(text(expression, "opcode"));
            out.push('(');
            for (index, argument) in items(expression, "args").iter().enumerate() {
                if index > 0 {
                    out.push_str(", ");
                }
                write_expression(argument, out);
            }
            out.push(')');
        }
        "lambda" => {
            out.push_str("((");
            let names: Vec<&str> = items(expression, "params")
                .iter()
                .map(|param| text(param, "name"))
                .collect();
            out.push_str(&names.join(", "));
            out.push_str(") => ");
            if let Some(body) = expression.get("expr") {
                write_expression(body, out);
            }
            out.push(')');
        }
        "let" => {
            if let Some(definition) = expression.get("opdef") {
                out.push_str(qualifier_text(text(definition, "qualifier")));
                out.push(' ');
                out.push_str(text(definition, "name"));
                if let Some(annotation) = definition.get("typeAnnotation") {
                    out.push_str(": ");
                    out.push_str(&type_text(annotation));
                }
                out.push_str(" = ");
                if let Some(value) = definition.get("expr") {
                    write_expression(value, out);
                }
            }
            out.push_str(" { ");
            if let Some(body) = expression.get("expr") {
                write_expression(body, out);
            }
            out.push_str(" }");
        }
        _ => {}
    }
}
