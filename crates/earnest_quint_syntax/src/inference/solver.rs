use crate::inference::printing::{
    ErrorTree, Failure, leaf, row_names, row_text, tree, type_names, type_text,
};
use crate::inference::simplification::simplify_row;
use crate::inference::substitution::{
    Constraint, Substitution, Substitutions, apply, apply_to_constraint, compose,
};
use crate::inference::value::Value;
use std::cell::RefCell;
use std::collections::HashMap;

#[derive(Debug, Default)]
pub(crate) struct Table {
    order: Vec<u64>,
    entries: HashMap<u64, Value>,
}

impl Table {
    pub(crate) fn get(&self, id: u64) -> Option<&Value> {
        self.entries.get(&id)
    }

    pub(crate) fn set(&mut self, id: u64, value: Value) {
        if self.entries.insert(id, value).is_none() {
            self.order.push(id);
        }
    }

    pub(crate) fn ids(&self) -> Vec<u64> {
        self.order.clone()
    }

    pub(crate) fn values(&self) -> impl Iterator<Item = &Value> {
        self.order.iter().filter_map(|id| self.entries.get(id))
    }
}

#[derive(Debug, Default)]
pub(crate) struct Context {
    pub(crate) table: Table,
    crashed: RefCell<Option<String>>,
}

impl Context {
    pub(crate) fn new(table: Table) -> Self {
        Self {
            table,
            crashed: RefCell::new(None),
        }
    }

    pub(crate) fn crash(&self, message: String) {
        let mut crashed = self.crashed.borrow_mut();
        if crashed.is_none() {
            *crashed = Some(message);
        }
    }

    pub(crate) fn crashed(&self) -> Option<String> {
        self.crashed.borrow().clone()
    }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct Errors {
    pub(crate) entries: Vec<(u64, ErrorTree)>,
}

impl Errors {
    pub(crate) fn set(&mut self, id: u64, error: ErrorTree) {
        match self.entries.iter_mut().find(|(known, _)| *known == id) {
            Some((_, slot)) => *slot = error,
            None => self.entries.push((id, error)),
        }
    }

    pub(crate) fn get(&self, id: u64) -> Option<&ErrorTree> {
        self.entries
            .iter()
            .find(|(known, _)| *known == id)
            .map(|(_, error)| error)
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

pub(crate) fn solve(context: &Context, constraint: &Constraint) -> Result<Substitutions, Errors> {
    let mut errors = Errors::default();
    match constraint {
        Constraint::Equal(left, right, source) => unify(context, left, right).map_err(|error| {
            errors.set(*source, error);
            errors
        }),
        Constraint::Conjunction(constraints) => {
            let mut sorted = constraints.clone();
            sorted.sort_by_key(Constraint::rank);
            let mut result: Result<Substitutions, ()> = Ok(Vec::new());
            for inner in &sorted {
                let applied = match &result {
                    Ok(substitutions) => apply_to_constraint(substitutions, inner),
                    Err(()) => inner.clone(),
                };
                match solve(context, &applied) {
                    Err(inner_errors) => {
                        for (id, error) in inner_errors.entries {
                            errors.set(id, error);
                        }
                        result = Err(());
                    }
                    Ok(found) => {
                        if let Ok(substitutions) = &result {
                            result = Ok(compose(context, &found, substitutions));
                        }
                    }
                }
            }
            result.map_err(|()| errors)
        }
        Constraint::Defined(written, source) => {
            for definition in context.table.values() {
                let defined = definition.get("type");
                if definition.kind() == "typedef"
                    && defined.truthy()
                    && let Ok(substitutions) = unify(context, defined, written)
                {
                    return Ok(substitutions);
                }
            }
            errors.set(
                *source,
                leaf(
                    &format!(
                        "Looking for defined type unifying with {}",
                        type_text(written)
                    ),
                    "Expected type is not defined",
                ),
            );
            Err(errors)
        }
    }
}

pub(crate) fn unify(
    context: &Context,
    first: &Value,
    second: &Value,
) -> Result<Substitutions, ErrorTree> {
    let first_text = type_text(first);
    let second_text = type_text(second);
    let location = format!("Trying to unify {first_text} and {second_text}");
    let (first_kind, second_kind) = (first.kind(), second.kind());
    if first_text == second_text {
        Ok(Vec::new())
    } else if first_kind == "var" {
        bind_type(first.str("name"), second).map_err(|message| leaf(&location, &message))
    } else if second_kind == "var" {
        bind_type(second.str("name"), first).map_err(|message| leaf(&location, &message))
    } else if first_kind == "oper" && second_kind == "oper" {
        let (first_args, second_args) = (first.items("args"), second.items("args"));
        if first_args.len() != second_args.len() {
            return Err(leaf(
                &location,
                &format!(
                    "Expected {} arguments, got {}",
                    first_args.len(),
                    second_args.len()
                ),
            ));
        }
        let mut firsts = first_args.to_vec();
        firsts.push(first.get("res").clone());
        let mut seconds = second_args.to_vec();
        seconds.push(second.get("res").clone());
        chain(context, &firsts, &seconds).map_err(|error| tree(&location, Failure::Tree(error)))
    } else if (first_kind == "set" && second_kind == "set")
        || (first_kind == "list" && second_kind == "list")
    {
        unify(context, first.get("elem"), second.get("elem"))
    } else if first_kind == "fun" && second_kind == "fun" {
        let arguments = unify(context, first.get("arg"), second.get("arg"))?;
        let results = unify(
            context,
            &apply(&arguments, first.get("res")),
            &apply(&arguments, second.get("res")),
        )?;
        Ok(compose(context, &arguments, &results))
    } else if first_kind == "tup" && second_kind == "tup" {
        unify_rows(context, first.get("fields"), second.get("fields"))
            .map_err(|error| tree(&location, Failure::Tree(error)))
    } else if first_kind == "const" {
        unify_with_alias(context, first, second)
    } else if second_kind == "const" {
        unify_with_alias(context, second, first)
    } else if (first_kind == "rec" && second_kind == "rec")
        || (first_kind == "sum" && second_kind == "sum")
    {
        unify_rows(context, first.get("fields"), second.get("fields"))
            .map_err(|error| tree(&location, Failure::Tree(error)))
    } else {
        Err(leaf(
            &location,
            &format!("Couldn't unify {first_kind} and {second_kind}"),
        ))
    }
}

pub(crate) fn unify_rows(
    context: &Context,
    first: &Value,
    second: &Value,
) -> Result<Substitutions, ErrorTree> {
    let first = simplify_row(context, first);
    let second = simplify_row(context, second);
    let first_text = row_text(&first);
    let second_text = row_text(&second);
    let location = format!("Trying to unify {first_text} and {second_text}");
    if first_text == second_text {
        return Ok(Vec::new());
    }
    if first.kind() == "var" {
        return bind_row(first.str("name"), &second).map_err(|message| leaf(&location, &message));
    }
    if second.kind() == "var" {
        return bind_row(second.str("name"), &first).map_err(|message| leaf(&location, &message));
    }
    if first.kind() != "row" || second.kind() != "row" {
        return Err(leaf(
            &location,
            &format!("Couldn't unify {} and {}", first.kind(), second.kind()),
        ));
    }
    let first_fields = first.items("fields");
    let second_fields = second.items("fields");
    let shared: Vec<&str> = first_fields
        .iter()
        .map(|field| field.str("fieldName"))
        .filter(|name| {
            second_fields
                .iter()
                .any(|field| field.str("fieldName") == *name)
        })
        .collect();
    let (first_other, second_other) = (first.get("other"), second.get("other"));
    if shared.is_empty() {
        if first_other.kind() == "var"
            && second_other.kind() == "var"
            && first_other.str("name") != second_other.str("name")
        {
            let tail = Value::object(vec![
                ("kind", Value::text("var")),
                (
                    "name",
                    Value::Text(format!(
                        "${}${}",
                        first_other.str("name"),
                        second_other.str("name")
                    )),
                ),
            ]);
            let bound_first = bind_row(
                first_other.str("name"),
                &second.spread(vec![("other", tail.clone())]),
            );
            let bound_second = bind_row(
                second_other.str("name"),
                &first.spread(vec![("other", tail)]),
            );
            return match (bound_first, bound_second) {
                (Ok(first_bound), Ok(second_bound)) => {
                    Ok(compose(context, &first_bound, &second_bound))
                }
                (Err(message), _) | (_, Err(message)) => Err(leaf(&location, &message)),
            };
        }
        return Err(leaf(
            &location,
            &format!(
                "Incompatible tails for rows with disjoint fields: {} and {}",
                row_text(first_other),
                row_text(second_other)
            ),
        ));
    }
    let unique = |fields: &[Value]| {
        Value::List(
            fields
                .iter()
                .filter(|field| !shared.contains(&field.str("fieldName")))
                .cloned()
                .collect(),
        )
    };
    let tails = unify_rows(
        context,
        &first.spread(vec![("fields", unique(first_fields))]),
        &second.spread(vec![("fields", unique(second_fields))]),
    );
    let field_type = |fields: &[Value], name: &str| {
        fields
            .iter()
            .find(|field| field.str("fieldName") == name)
            .map_or(Value::Undefined, |field| field.get("fieldType").clone())
    };
    let firsts: Vec<Value> = shared
        .iter()
        .map(|name| field_type(first_fields, name))
        .collect();
    let seconds: Vec<Value> = shared
        .iter()
        .map(|name| field_type(second_fields, name))
        .collect();
    let fields = chain(context, &firsts, &seconds);
    match (tails, fields) {
        (Ok(tails), Ok(fields)) => Ok(compose(context, &tails, &fields)),
        (Err(error), _) | (_, Err(error)) => Err(tree(&location, Failure::Tree(error))),
    }
}

fn unify_with_alias(
    context: &Context,
    alias: &Value,
    other: &Value,
) -> Result<Substitutions, ErrorTree> {
    let name = alias.str("name");
    let location = format!("Trying to unify {name} and {}", type_text(other));
    let value = alias.id().and_then(|id| context.table.get(id));
    let Some(value) = value.filter(|value| value.kind() == "typedef") else {
        return Err(leaf(&location, &format!("Couldn't find type alias {name}")));
    };
    let defined = value.get("type");
    if !defined.truthy() {
        return Err(leaf(
            &location,
            &format!("Couldn't unify uninterpreted type {name} with different type"),
        ));
    }
    unify(context, defined, other)
}

fn bind_type(name: &str, written: &Value) -> Result<Substitutions, String> {
    if type_names(written).types.iter().any(|known| known == name) {
        Err(format!(
            "Can't bind {name} to {}: cyclical binding",
            type_text(written)
        ))
    } else {
        Ok(vec![Substitution {
            row: false,
            name: name.to_owned(),
            value: written.clone(),
        }])
    }
}

fn bind_row(name: &str, row: &Value) -> Result<Substitutions, String> {
    if row_names(row).iter().any(|known| known == name) {
        Err(format!(
            "Can't bind {name} to {}: cyclical binding",
            row_text(row)
        ))
    } else {
        Ok(vec![Substitution {
            row: true,
            name: name.to_owned(),
            value: row.clone(),
        }])
    }
}

fn chain(
    context: &Context,
    firsts: &[Value],
    seconds: &[Value],
) -> Result<Substitutions, ErrorTree> {
    let mut substitutions = Vec::new();
    for (first, second) in firsts.iter().zip(seconds) {
        let found = unify(
            context,
            &apply(&substitutions, first),
            &apply(&substitutions, second),
        )?;
        substitutions = compose(context, &found, &substitutions);
    }
    Ok(substitutions)
}
