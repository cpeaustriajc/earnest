use earnest_value::{Sym, Value};
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

pub(crate) fn listed(items: &BTreeSet<Value>, tuple_items: &[Sym]) -> Vec<Value> {
    Order { tuple_items }
        .sorted(items.iter())
        .into_iter()
        .cloned()
        .collect()
}

fn is_collection(value: &Value) -> bool {
    matches!(
        value,
        Value::Set(_) | Value::Map(_) | Value::Seq(_) | Value::Record(_)
    )
}

struct Order<'n> {
    tuple_items: &'n [Sym],
}

impl Order<'_> {
    fn sorted<'v>(&self, items: impl Iterator<Item = &'v Value>) -> Vec<&'v Value> {
        let mut all: Vec<&Value> = items.collect();
        if all.iter().any(|item| is_collection(item)) {
            all.sort_by(|left, right| self.compared(left, right));
        }
        all
    }

    fn each_of<'v>(
        &self,
        left: impl Iterator<Item = &'v Value>,
        right: impl Iterator<Item = &'v Value>,
    ) -> Ordering {
        left.zip(right)
            .map(|(one, other)| self.compared(one, other))
            .find(|order| order.is_ne())
            .unwrap_or(Ordering::Equal)
    }

    fn entries<'v>(&self, map: &'v BTreeMap<Value, Value>) -> Vec<(&'v Value, &'v Value)> {
        let mut all: Vec<(&Value, &Value)> = map.iter().collect();
        if all.iter().any(|(key, _)| is_collection(key)) {
            all.sort_by(|left, right| self.compared(left.0, right.0));
        }
        all
    }

    fn by_position<'v>(&self, fields: &'v [(Sym, Value)]) -> Option<Vec<(usize, &'v Value)>> {
        let mut items = fields
            .iter()
            .map(|(name, value)| {
                let position = self.tuple_items.iter().position(|item| item == name)?;
                Some((position, value))
            })
            .collect::<Option<Vec<_>>>()?;
        items.sort_by_key(|(position, _)| *position);
        Some(items)
    }

    fn records(&self, one: &[(Sym, Value)], other: &[(Sym, Value)]) -> Ordering {
        let sized = one.len().cmp(&other.len());
        if let (Some(one), Some(other)) = (self.by_position(one), self.by_position(other)) {
            return sized
                .then_with(|| {
                    one.iter()
                        .map(|(position, _)| position)
                        .cmp(other.iter().map(|(position, _)| position))
                })
                .then_with(|| {
                    self.each_of(
                        one.iter().map(|(_, value)| *value),
                        other.iter().map(|(_, value)| *value),
                    )
                });
        }
        sized
            .then_with(|| {
                one.iter()
                    .map(|(name, _)| name)
                    .cmp(other.iter().map(|(name, _)| name))
            })
            .then_with(|| {
                self.each_of(
                    one.iter().map(|(_, value)| value),
                    other.iter().map(|(_, value)| value),
                )
            })
    }

    fn compared(&self, left: &Value, right: &Value) -> Ordering {
        match (left, right) {
            (Value::Set(one), Value::Set(other)) => one.len().cmp(&other.len()).then_with(|| {
                self.each_of(
                    self.sorted(one.iter()).into_iter(),
                    self.sorted(other.iter()).into_iter(),
                )
            }),
            (Value::Seq(one), Value::Seq(other)) => one
                .len()
                .cmp(&other.len())
                .then_with(|| self.each_of(one.iter(), other.iter())),
            (Value::Map(one), Value::Map(other)) => one.len().cmp(&other.len()).then_with(|| {
                let (one, other) = (self.entries(one), self.entries(other));
                self.each_of(
                    one.iter().map(|(key, _)| *key),
                    other.iter().map(|(key, _)| *key),
                )
                .then_with(|| {
                    self.each_of(
                        one.iter().map(|(_, value)| *value),
                        other.iter().map(|(_, value)| *value),
                    )
                })
            }),
            (Value::Record(one), Value::Record(other)) => self.records(one, other),
            _ => left.cmp(right),
        }
    }
}
