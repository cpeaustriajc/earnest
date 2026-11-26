use earnest_value::Value;
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

pub(crate) fn listed(items: &BTreeSet<Value>) -> Vec<Value> {
    sorted(items.iter()).into_iter().cloned().collect()
}

fn is_collection(value: &Value) -> bool {
    matches!(
        value,
        Value::Set(_) | Value::Map(_) | Value::Seq(_) | Value::Record(_)
    )
}

fn sorted<'v>(items: impl Iterator<Item = &'v Value>) -> Vec<&'v Value> {
    let mut all: Vec<&Value> = items.collect();
    if all.iter().any(|item| is_collection(item)) {
        all.sort_by(|left, right| compared(left, right));
    }
    all
}

fn each_of<'v>(
    left: impl Iterator<Item = &'v Value>,
    right: impl Iterator<Item = &'v Value>,
) -> Ordering {
    left.zip(right)
        .map(|(one, other)| compared(one, other))
        .find(|order| order.is_ne())
        .unwrap_or(Ordering::Equal)
}

fn entries(map: &BTreeMap<Value, Value>) -> Vec<(&Value, &Value)> {
    let mut all: Vec<(&Value, &Value)> = map.iter().collect();
    if all.iter().any(|(key, _)| is_collection(key)) {
        all.sort_by(|left, right| compared(left.0, right.0));
    }
    all
}

fn compared(left: &Value, right: &Value) -> Ordering {
    match (left, right) {
        (Value::Set(one), Value::Set(other)) => one.len().cmp(&other.len()).then_with(|| {
            each_of(
                sorted(one.iter()).into_iter(),
                sorted(other.iter()).into_iter(),
            )
        }),
        (Value::Seq(one), Value::Seq(other)) => one
            .len()
            .cmp(&other.len())
            .then_with(|| each_of(one.iter(), other.iter())),
        (Value::Map(one), Value::Map(other)) => one.len().cmp(&other.len()).then_with(|| {
            let (one, other) = (entries(one), entries(other));
            each_of(
                one.iter().map(|(key, _)| *key),
                other.iter().map(|(key, _)| *key),
            )
            .then_with(|| {
                each_of(
                    one.iter().map(|(_, value)| *value),
                    other.iter().map(|(_, value)| *value),
                )
            })
        }),
        (Value::Record(one), Value::Record(other)) => one
            .len()
            .cmp(&other.len())
            .then_with(|| {
                one.iter()
                    .map(|(name, _)| name)
                    .cmp(other.iter().map(|(name, _)| name))
            })
            .then_with(|| {
                each_of(
                    one.iter().map(|(_, value)| value),
                    other.iter().map(|(_, value)| value),
                )
            }),
        _ => left.cmp(right),
    }
}
