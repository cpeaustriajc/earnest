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

#[cfg(test)]
mod tests {
    use super::expression_text;
    use crate::files::read;
    use crate::json::Json;
    use std::fs;
    use std::path::Path;

    const SAMPLE: &str = "crates/earnest_quint_syntax/tests/samples/printing.qnt";

    fn expressions<'a>(value: &'a Json, found: &mut Vec<&'a Json>) {
        match value {
            Json::Object(fields) => {
                let kind = match value.get("kind") {
                    Some(Json::Text(kind)) => kind.as_str(),
                    _ => "",
                };
                let expression = match kind {
                    "name" | "lambda" | "let" => true,
                    "bool" | "int" | "str" => value.get("value").is_some(),
                    "app" => value.get("opcode").is_some(),
                    _ => false,
                };
                if expression && value.get("id").is_some() {
                    found.push(value);
                }
                for (_, inner) in fields {
                    expressions(inner, found);
                }
            }
            Json::List(items) => {
                for inner in items {
                    expressions(inner, found);
                }
            }
            _ => {}
        }
    }

    #[test]
    fn every_expression_of_the_sample_is_printed_as_quint_prints_it() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let recorded = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/quint")
            .join(format!("{SAMPLE}.printing.json"));
        let printed = Json::parse(&fs::read_to_string(recorded).expect("the recording is read"))
            .expect("the recording is JSON");
        let parsed = read(SAMPLE, |path| fs::read_to_string(root.join(path)).ok())
            .expect("the sample is read");
        let mut found = Vec::new();
        for module in parsed.modules() {
            expressions(module, &mut found);
        }
        let Json::Object(entries) = &printed else {
            panic!("the recording is an object");
        };
        assert!(entries.len() > 90, "the recording has every expression");
        for expression in found {
            let id = expression
                .get("id")
                .map_or_else(String::new, ToString::to_string);
            let Some(Json::Text(theirs)) = printed.get(&id) else {
                panic!("id {id} is printed by Quint");
            };
            assert_eq!(&expression_text(expression), theirs, "id {id}");
        }
    }
}
