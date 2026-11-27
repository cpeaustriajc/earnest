use crate::inference::printing::{ErrorTree, Failure, expression_text, leaf};
use crate::inference::substitution::Constraint;
use crate::inference::value::Value;

pub(crate) type Typed<'a> = (&'a Value, Value);

fn shown(arguments: &[Typed<'_>]) -> String {
    arguments
        .iter()
        .map(|(expression, _)| expression_text(expression))
        .collect::<Vec<_>>()
        .join(",")
}

fn variable(name: &str) -> Value {
    Value::object(vec![
        ("kind", Value::text("var")),
        ("name", Value::text(name)),
    ])
}

fn field(name: &str, field_type: Value) -> Value {
    Value::object(vec![
        ("fieldName", Value::text(name)),
        ("fieldType", field_type),
    ])
}

fn row(fields: Vec<Value>, other: Value) -> Value {
    Value::object(vec![
        ("kind", Value::text("row")),
        ("fields", Value::list(fields)),
        ("other", other),
    ])
}

fn empty() -> Value {
    Value::object(vec![("kind", Value::text("empty"))])
}

fn typed(kind: &str, fields: Value) -> Value {
    Value::object(vec![("kind", Value::text(kind)), ("fields", fields)])
}

fn merged<T>(results: Vec<Result<T, ErrorTree>>) -> Result<Vec<T>, Failure> {
    let mut found = Vec::new();
    let mut failed = Vec::new();
    for result in results {
        match result {
            Ok(value) => found.push(value),
            Err(error) => failed.push(error),
        }
    }
    if failed.is_empty() {
        Ok(found)
    } else {
        Err(Failure::Trees(failed))
    }
}

fn text_of(expression: &Value) -> String {
    match expression.get("value") {
        Value::Text(text) => text.clone(),
        Value::Number(digits) => digits.clone(),
        _ => String::new(),
    }
}

pub(crate) fn record(
    id: u64,
    arguments: &[Typed<'_>],
    result: &Value,
) -> Result<Vec<Constraint>, Failure> {
    let fields = arguments
        .chunks(2)
        .map(|pair| {
            let (key, _) = &pair[0];
            let value_type = pair
                .get(1)
                .map_or(Value::Undefined, |(_, value)| value.clone());
            if key.kind() == "str" {
                Ok(field(&text_of(key), value_type))
            } else {
                Err(leaf(
                    &format!("Generating record constraints for {}", shown(arguments)),
                    &format!(
                        "Record field name must be a name expression but is {}: {}",
                        key.kind(),
                        expression_text(key)
                    ),
                ))
            }
        })
        .collect();
    let fields = merged(fields)?;
    let record = typed("rec", row(fields, empty()));
    Ok(vec![Constraint::Equal(record, result.clone(), id)])
}

fn field_name(arguments: &[Typed<'_>], at: usize) -> Result<String, Failure> {
    let (name, _) = &arguments[at];
    if name.kind() == "str" {
        Ok(text_of(name))
    } else {
        Err(Failure::Tree(leaf(
            &format!("Generating record constraints for {}", shown(arguments)),
            &format!(
                "Record field name must be a string expression but is {}: {}",
                name.kind(),
                expression_text(name)
            ),
        )))
    }
}

fn tail(result: &Value, prefix: &str) -> Value {
    variable(&format!("{prefix}{}", result.str("name")))
}

pub(crate) fn field_access(
    id: u64,
    arguments: &[Typed<'_>],
    result: &Value,
) -> Result<Vec<Constraint>, Failure> {
    let name = field_name(arguments, 1)?;
    let general = typed(
        "rec",
        row(vec![field(&name, result.clone())], tail(result, "tail_")),
    );
    Ok(vec![Constraint::Equal(arguments[0].1.clone(), general, id)])
}

pub(crate) fn field_names(id: u64, arguments: &[Typed<'_>], result: &Value) -> Vec<Constraint> {
    let general = typed("rec", tail(result, "rec_"));
    let strings = Value::object(vec![
        ("kind", Value::text("set")),
        ("elem", Value::object(vec![("kind", Value::text("str"))])),
    ]);
    vec![
        Constraint::Equal(result.clone(), strings, id),
        Constraint::Equal(arguments[0].1.clone(), general, id),
    ]
}

pub(crate) fn with(
    id: u64,
    arguments: &[Typed<'_>],
    result: &Value,
) -> Result<Vec<Constraint>, Failure> {
    let name = field_name(arguments, 1)?;
    let general = typed(
        "rec",
        row(
            vec![field(&name, arguments[2].1.clone())],
            tail(result, "tail_"),
        ),
    );
    Ok(vec![
        Constraint::Equal(arguments[0].1.clone(), general.clone(), id),
        Constraint::Equal(result.clone(), general, id),
    ])
}

pub(crate) fn tuple(id: u64, arguments: &[Typed<'_>], result: &Value) -> Vec<Constraint> {
    let fields = arguments
        .iter()
        .enumerate()
        .map(|(at, (_, field_type))| field(&at.to_string(), field_type.clone()))
        .collect();
    let tuple = typed("tup", row(fields, empty()));
    vec![Constraint::Equal(tuple, result.clone(), id)]
}

pub(crate) const LARGEST_ITEM: i128 = 100_000;

pub(crate) fn item(
    id: u64,
    arguments: &[Typed<'_>],
    result: &Value,
) -> Result<Vec<Constraint>, Failure> {
    let (index, _) = &arguments[1];
    if index.kind() != "int" {
        return Err(Failure::Tree(leaf(
            &format!("Generating record constraints for {}", shown(arguments)),
            &format!(
                "Tup field index must be an int expression but is {}: {}",
                index.kind(),
                expression_text(index)
            ),
        )));
    }
    let value: i128 = text_of(index).parse().unwrap_or(LARGEST_ITEM);
    let before = (value - 1).clamp(0, LARGEST_ITEM);
    let mut fields: Vec<Value> = (0..before)
        .map(|at| {
            field(
                &at.to_string(),
                variable(&format!("tup_{}_{at}", result.str("name"))),
            )
        })
        .collect();
    fields.push(field(&(value - 1).to_string(), result.clone()));
    let general = typed("tup", row(fields, tail(result, "tail_")));
    Ok(vec![Constraint::Equal(arguments[0].1.clone(), general, id)])
}

pub(crate) fn variant(
    id: u64,
    arguments: &[Typed<'_>],
    result: &Value,
) -> Result<Vec<Constraint>, Failure> {
    let (label, label_type) = &arguments[0];
    if label.kind() != "str" {
        return Err(Failure::Tree(leaf(
            &format!("Generating variant constraints for {}", shown(arguments)),
            &format!(
                "Variant label must be a string expression but is {}: {}",
                label_type.kind(),
                expression_text(label)
            ),
        )));
    }
    let general = typed(
        "sum",
        row(
            vec![field(&text_of(label), arguments[1].1.clone())],
            tail(result, "tail_"),
        ),
    );
    Ok(vec![
        Constraint::Defined(general.clone(), id),
        Constraint::Equal(result.clone(), general, id),
    ])
}

pub(crate) fn matching(
    id: u64,
    arguments: &[Typed<'_>],
    result: &Value,
) -> Result<Vec<Constraint>, Failure> {
    let variant_type = arguments[0].1.clone();
    let cases = &arguments[1..];
    let location = format!("Generating match constraints for {}", shown(cases));
    let mut validated: Vec<Result<(String, Value), ErrorTree>> = Vec::new();
    let mut wildcard = false;
    for pair in cases.chunks(2) {
        let (label, _) = &pair[0];
        let (eliminator, eliminator_type) = (pair[1].0, &pair[1].1);
        let arity = eliminator_type.items("args").len();
        let problem = if label.kind() != "str" {
            Some(format!(
                "Match variant name must be a string literal but it is a {}: {}",
                label.kind(),
                expression_text(label)
            ))
        } else if eliminator_type.kind() != "oper" {
            Some(format!(
                "Match case eliminator must be an operator expression but it is a {}: {}",
                eliminator_type.kind(),
                expression_text(eliminator)
            ))
        } else if arity != 1 {
            Some(format!(
                "Match case eliminator must be a unary operator but it is an operator of {arity} arguments: {}",
                expression_text(eliminator)
            ))
        } else if !wildcard && text_of(label) == "_" {
            wildcard = true;
            None
        } else if wildcard {
            Some(format!(
                "Invalid wildcard match ('_') in match expression: {}. Only one wildcard can appear and it must be the final case of a match.",
                expression_text(eliminator)
            ))
        } else {
            validated.push(Ok((
                text_of(label),
                eliminator_type.items("args")[0].clone(),
            )));
            None
        };
        if let Some(problem) = problem {
            validated.push(Err(leaf(&location, &problem)));
        }
    }
    let fields = merged(validated)?;
    let other = if wildcard {
        variable(&format!("{}_wildcard", result.str("name")))
    } else {
        empty()
    };
    let cases_type = Value::object(vec![
        ("kind", Value::text("sum")),
        (
            "fields",
            row(
                fields
                    .into_iter()
                    .map(|(name, field_type)| field(&name, field_type))
                    .collect(),
                other,
            ),
        ),
        ("id", Value::Undefined),
    ]);
    let mut constraints = vec![Constraint::Equal(variant_type, cases_type, id)];
    for pair in cases.chunks(2) {
        constraints.push(Constraint::Equal(
            result.clone(),
            pair[1].1.get("res").clone(),
            id,
        ));
    }
    Ok(constraints)
}
