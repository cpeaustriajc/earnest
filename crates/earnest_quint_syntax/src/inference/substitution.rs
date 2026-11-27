use crate::inference::solver::{Context, unify, unify_rows};
use crate::inference::value::Value;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Substitution {
    pub(crate) row: bool,
    pub(crate) name: String,
    pub(crate) value: Value,
}

pub(crate) type Substitutions = Vec<Substitution>;

#[derive(Clone, Debug)]
pub(crate) enum Constraint {
    Equal(Value, Value, u64),
    Conjunction(Vec<Constraint>),
    Defined(Value, u64),
}

impl Constraint {
    pub(crate) fn rank(&self) -> u8 {
        match self {
            Self::Equal(..) => 1,
            Self::Conjunction(_) => 2,
            Self::Defined(..) => 3,
        }
    }
}

pub(crate) fn compose(
    context: &Context,
    first: &[Substitution],
    second: &[Substitution],
) -> Substitutions {
    let mut composed = applied_to_substitutions(context, first, second);
    composed.extend(first.iter().cloned());
    composed
}

fn applied_to_substitutions(
    context: &Context,
    first: &[Substitution],
    second: &[Substitution],
) -> Substitutions {
    let mut result = Vec::new();
    for substitution in second {
        if let Some(found) = first.iter().find(|found| found.name == substitution.name) {
            if found.row != substitution.row {
                context.crash(format!(
                    "Substitutions with same name ({}) but incompatible kinds",
                    substitution.name
                ));
                continue;
            }
            let unified = if found.row {
                unify_rows(context, &substitution.value, &found.value)
            } else {
                unify(context, &substitution.value, &found.value)
            };
            match unified {
                Ok(substitutions) => result.extend(substitutions),
                Err(_) => context.crash(format!(
                    "Unifying substitutions with same name: {}",
                    substitution.name
                )),
            }
            continue;
        }
        let value = if substitution.row {
            apply_to_row(first, &substitution.value)
        } else {
            apply(first, &substitution.value)
        };
        result.push(Substitution {
            row: substitution.row,
            name: substitution.name.clone(),
            value,
        });
    }
    result
}

pub(crate) fn apply(substitutions: &[Substitution], written: &Value) -> Value {
    let mut current = written.clone();
    loop {
        let next = once(substitutions, &current);
        if next.same(&current) {
            return current;
        }
        current = next;
    }
}

fn once(substitutions: &[Substitution], written: &Value) -> Value {
    let id = written.get("id").clone();
    let kind = written.get("kind").clone();
    match written.kind() {
        "var" => match substitutions
            .iter()
            .find(|found| found.name == written.str("name"))
        {
            Some(found) if !found.row => found.value.clone(),
            _ => written.clone(),
        },
        "oper" => Value::object(vec![
            ("kind", kind),
            (
                "args",
                Value::List(
                    written
                        .items("args")
                        .iter()
                        .map(|argument| apply(substitutions, argument))
                        .collect(),
                ),
            ),
            ("res", apply(substitutions, written.get("res"))),
            ("id", id),
        ]),
        "list" | "set" => Value::object(vec![
            ("kind", kind),
            ("elem", apply(substitutions, written.get("elem"))),
            ("id", id),
        ]),
        "fun" => Value::object(vec![
            ("kind", kind),
            ("arg", apply(substitutions, written.get("arg"))),
            ("res", apply(substitutions, written.get("res"))),
            ("id", id),
        ]),
        "tup" | "rec" => Value::object(vec![
            ("kind", kind),
            ("fields", apply_to_row(substitutions, written.get("fields"))),
            ("id", id),
        ]),
        "sum" => written.spread(vec![(
            "fields",
            apply_to_row(substitutions, written.get("fields")),
        )]),
        "app" => written.spread(vec![(
            "args",
            Value::List(
                written
                    .items("args")
                    .iter()
                    .map(|argument| apply(substitutions, argument))
                    .collect(),
            ),
        )]),
        _ => written.clone(),
    }
}

pub(crate) fn apply_to_row(substitutions: &[Substitution], row: &Value) -> Value {
    match row.kind() {
        "row" => Value::object(vec![
            ("kind", Value::text("row")),
            (
                "fields",
                Value::List(
                    row.items("fields")
                        .iter()
                        .map(|field| {
                            Value::object(vec![
                                ("fieldName", field.get("fieldName").clone()),
                                ("fieldType", apply(substitutions, field.get("fieldType"))),
                            ])
                        })
                        .collect(),
                ),
            ),
            ("other", apply_to_row(substitutions, row.get("other"))),
        ]),
        "var" => substitutions
            .iter()
            .find(|found| found.row && found.name == row.str("name"))
            .map_or_else(|| row.clone(), |found| found.value.clone()),
        _ => row.clone(),
    }
}

pub(crate) fn apply_to_constraint(
    substitutions: &[Substitution],
    constraint: &Constraint,
) -> Constraint {
    match constraint {
        Constraint::Equal(left, right, source) => Constraint::Equal(
            apply(substitutions, left),
            apply(substitutions, right),
            *source,
        ),
        Constraint::Defined(written, source) => {
            Constraint::Defined(apply(substitutions, written), *source)
        }
        Constraint::Conjunction(constraints) => Constraint::Conjunction(
            constraints
                .iter()
                .map(|inner| apply_to_constraint(substitutions, inner))
                .collect(),
        ),
    }
}
