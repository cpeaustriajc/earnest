use crate::inference::solver::{Context, unify, unify_rows};
use crate::inference::value::Value;
use std::cell::Cell;

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
                    "Substitutions with same name ({}) but incompatible kinds: [ {}, {} ]",
                    substitution.name,
                    shown(found),
                    shown(substitution)
                ));
                return Vec::new();
            }
            let unified = if found.row {
                unify_rows(context, &substitution.value, &found.value)
            } else {
                unify(context, &substitution.value, &found.value)
            };
            match unified {
                Ok(substitutions) => result.extend(substitutions),
                Err(error) => {
                    context.crash(format!(
                        "Unifying substitutions with same name: {}, {}",
                        substitution.name,
                        crate::inference::printing::error_text(&error)
                    ));
                    return Vec::new();
                }
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

pub(crate) const ROUND_LIMIT: usize = 100;

pub(crate) const DEPTH_LIMIT: usize = 2_100;

pub(crate) const UNIFY_LIMIT: usize = 5_000;

thread_local! {
    static MOST_ROUNDS: Cell<usize> = const { Cell::new(0) };
    static DEPTH: Cell<usize> = const { Cell::new(0) };
    static DEEPEST: Cell<usize> = const { Cell::new(0) };
    static STOPPED: Cell<bool> = const { Cell::new(false) };
    static UNIFYING: Cell<usize> = const { Cell::new(0) };
    static TOO_DEEP_TO_UNIFY: Cell<bool> = const { Cell::new(false) };
}

pub(crate) fn start_counting_rounds() {
    MOST_ROUNDS.with(|most| most.set(0));
    DEEPEST.with(|deepest| deepest.set(0));
    STOPPED.with(|stopped| stopped.set(false));
    TOO_DEEP_TO_UNIFY.with(|deep| deep.set(false));
}

pub(crate) fn most_rounds() -> usize {
    MOST_ROUNDS.with(Cell::get)
}

pub(crate) fn deepest() -> usize {
    DEEPEST.with(Cell::get)
}

pub(crate) fn apply(substitutions: &[Substitution], written: &Value) -> Value {
    applied(substitutions, written).unwrap_or_else(|| written.clone())
}

fn shown(substitution: &Substitution) -> String {
    let value = if substitution.row {
        crate::inference::printing::row_text(&substitution.value)
    } else {
        crate::inference::printing::type_text(&substitution.value)
    };
    format!("{} |-> {value}", substitution.name)
}

pub(crate) fn stop() {
    STOPPED.with(|stopped| stopped.set(true));
}

/// Runs `unify` one level deeper, or stops all type work where that is deeper than
/// [`UNIFY_LIMIT`], as Quint's stack runs out on a long chain of aliases.
pub(crate) fn deeper<T>(stopped: T, unify: impl FnOnce() -> T) -> T {
    let depth = UNIFYING.with(|depth| {
        depth.set(depth.get() + 1);
        depth.get()
    });
    let result = if depth > UNIFY_LIMIT {
        TOO_DEEP_TO_UNIFY.with(|deep| deep.set(true));
        stop();
        stopped
    } else {
        unify()
    };
    UNIFYING.with(|depth| depth.set(depth.get() - 1));
    result
}

pub(crate) fn too_deep_to_unify() -> bool {
    TOO_DEEP_TO_UNIFY.with(Cell::get)
}

pub(crate) fn stopped() -> bool {
    STOPPED.with(Cell::get)
}

fn applied(substitutions: &[Substitution], written: &Value) -> Option<Value> {
    if stopped() {
        return None;
    }
    let depth = DEPTH.with(|depth| {
        depth.set(depth.get() + 1);
        depth.get()
    });
    DEEPEST.with(|deepest| deepest.set(deepest.get().max(depth)));
    let result = if depth > DEPTH_LIMIT {
        stop();
        None
    } else {
        settled(substitutions, written)
    };
    DEPTH.with(|depth| depth.set(depth.get() - 1));
    result
}

fn settled(substitutions: &[Substitution], written: &Value) -> Option<Value> {
    let mut current: Option<Value> = None;
    for round in 1..=ROUND_LIMIT {
        let next = once(substitutions, current.as_ref().unwrap_or(written));
        match next {
            None => {
                MOST_ROUNDS.with(|most| most.set(most.get().max(round)));
                return current;
            }
            Some(next) => current = Some(next),
        }
    }
    MOST_ROUNDS.with(|most| most.set(ROUND_LIMIT + 1));
    current
}

fn has_exactly(written: &Value, keys: &[&str]) -> bool {
    match written {
        Value::Object(fields) => {
            fields.len() == keys.len()
                && keys
                    .iter()
                    .all(|key| fields.iter().any(|(name, _)| name == key))
        }
        _ => false,
    }
}

fn kept(changed: Option<Value>, original: &Value, any: &mut bool) -> Value {
    match changed {
        Some(value) => {
            *any = true;
            value
        }
        None => original.clone(),
    }
}

fn applied_list(substitutions: &[Substitution], items: &Value, any: &mut bool) -> Value {
    let mut built: Option<Vec<Value>> = None;
    for (at, item) in items.as_items().iter().enumerate() {
        let changed = applied(substitutions, item);
        match (&mut built, changed) {
            (Some(built), changed) => built.push(changed.unwrap_or_else(|| item.clone())),
            (None, Some(changed)) => {
                let mut started = items.as_items()[..at].to_vec();
                started.push(changed);
                built = Some(started);
            }
            (None, None) => {}
        }
    }
    match (built, items) {
        (Some(built), _) => {
            *any = true;
            Value::list(built)
        }
        (None, Value::List(_)) => items.clone(),
        (None, _) => Value::list(Vec::new()),
    }
}

fn once(substitutions: &[Substitution], written: &Value) -> Option<Value> {
    let id = || written.get("id").clone();
    let kind = || written.get("kind").clone();
    let mut any = false;
    match written.kind() {
        "var" => match substitutions
            .iter()
            .find(|found| found.name == written.str("name"))
        {
            Some(found) if !found.row && !found.value.same(written) => Some(found.value.clone()),
            _ => None,
        },
        "oper" => {
            let arguments = applied_list(substitutions, written.get("args"), &mut any);
            let result = kept(
                applied(substitutions, written.get("res")),
                written.get("res"),
                &mut any,
            );
            (any || !has_exactly(written, &["kind", "args", "res", "id"])).then(|| {
                Value::object(vec![
                    ("kind", kind()),
                    ("args", arguments),
                    ("res", result),
                    ("id", id()),
                ])
            })
        }
        "list" | "set" => {
            let element = kept(
                applied(substitutions, written.get("elem")),
                written.get("elem"),
                &mut any,
            );
            (any || !has_exactly(written, &["kind", "elem", "id"]))
                .then(|| Value::object(vec![("kind", kind()), ("elem", element), ("id", id())]))
        }
        "fun" => {
            let argument = kept(
                applied(substitutions, written.get("arg")),
                written.get("arg"),
                &mut any,
            );
            let result = kept(
                applied(substitutions, written.get("res")),
                written.get("res"),
                &mut any,
            );
            (any || !has_exactly(written, &["kind", "arg", "res", "id"])).then(|| {
                Value::object(vec![
                    ("kind", kind()),
                    ("arg", argument),
                    ("res", result),
                    ("id", id()),
                ])
            })
        }
        "tup" | "rec" => {
            let fields = kept(
                row_applied(substitutions, written.get("fields")),
                written.get("fields"),
                &mut any,
            );
            (any || !has_exactly(written, &["kind", "fields", "id"]))
                .then(|| Value::object(vec![("kind", kind()), ("fields", fields), ("id", id())]))
        }
        "sum" => {
            let fields = kept(
                row_applied(substitutions, written.get("fields")),
                written.get("fields"),
                &mut any,
            );
            any.then(|| written.spread(vec![("fields", fields)]))
        }
        "app" => {
            let arguments = applied_list(substitutions, written.get("args"), &mut any);
            any.then(|| written.spread(vec![("args", arguments)]))
        }
        _ => None,
    }
}

pub(crate) fn apply_to_row(substitutions: &[Substitution], row: &Value) -> Value {
    row_applied(substitutions, row).unwrap_or_else(|| row.clone())
}

fn row_applied(substitutions: &[Substitution], row: &Value) -> Option<Value> {
    match row.kind() {
        "row" => {
            let mut any = !has_exactly(row, &["kind", "fields", "other"]);
            let changed: Vec<Option<Value>> = row
                .items("fields")
                .iter()
                .map(|field| {
                    if !has_exactly(field, &["fieldName", "fieldType"]) {
                        any = true;
                    }
                    let changed = applied(substitutions, field.get("fieldType"));
                    any |= changed.is_some();
                    changed
                })
                .collect();
            let other = kept(
                row_applied(substitutions, row.get("other")),
                row.get("other"),
                &mut any,
            );
            any.then(|| {
                let fields = row
                    .items("fields")
                    .iter()
                    .zip(changed)
                    .map(|(field, changed)| {
                        Value::object(vec![
                            ("fieldName", field.get("fieldName").clone()),
                            (
                                "fieldType",
                                changed.unwrap_or_else(|| field.get("fieldType").clone()),
                            ),
                        ])
                    })
                    .collect();
                Value::object(vec![
                    ("kind", Value::text("row")),
                    ("fields", Value::list(fields)),
                    ("other", other),
                ])
            })
        }
        "var" => substitutions
            .iter()
            .find(|found| found.row && found.name == row.str("name"))
            .filter(|found| !found.value.same(row))
            .map(|found| found.value.clone()),
        _ => None,
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
