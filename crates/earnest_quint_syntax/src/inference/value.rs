use crate::json::Json;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

/// The values already converted, by the address of what they share, so that a subtree held in
/// more than one place is converted once and stays shared: [`Value::of_shared`] keeps a
/// [`Json::Shared`] as one `Rc`, [`Value::to_shared_json`] an `Rc` held twice as one
/// [`Json::Shared`]. A memo is only valid while the values it was filled from live.
#[derive(Default)]
pub(crate) struct Memo {
    values: HashMap<usize, Value>,
    json: HashMap<usize, Json>,
}

/// A value's content with each part named by the number the [`Interner`] gave it.
#[derive(Hash, PartialEq, Eq)]
enum Node {
    Undefined,
    Null,
    Bool(bool),
    Number(String),
    Text(String),
    List(Vec<usize>),
    Object(Vec<(String, usize)>),
}

/// Values stored once by their content: [`Interner::intern`] gives, for a value, the one value
/// equal to it that the interner holds, every part of it shared with every other value interned
/// that has an equal part. A type that many ids have is then held once.
#[derive(Default)]
pub(crate) struct Interner {
    /// The number of each list and object held, by its address; the value is kept, so the
    /// address stays its own.
    by_address: HashMap<usize, (usize, Value)>,
    by_content: HashMap<Node, usize>,
    held: Vec<Value>,
}

fn address<T: ?Sized>(shared: &Rc<T>) -> usize {
    Rc::as_ptr(shared).cast::<()>().addr()
}

impl Interner {
    pub(crate) fn intern(&mut self, value: &Value) -> Value {
        self.numbered(value).1
    }

    fn numbered(&mut self, value: &Value) -> (usize, Value) {
        let at = match value {
            Value::List(items) => Some(address(items)),
            Value::Object(fields) => Some(address(fields)),
            _ => None,
        };
        if let Some(known) = at.and_then(|at| self.by_address.get(&at)) {
            return known.clone();
        }
        let (node, built) = match value {
            Value::Undefined => (Node::Undefined, None),
            Value::Null => (Node::Null, None),
            Value::Bool(value) => (Node::Bool(*value), None),
            Value::Number(digits) => (Node::Number(digits.clone()), None),
            Value::Text(text) => (Node::Text(text.clone()), None),
            Value::List(items) => {
                let (numbers, items): (Vec<usize>, Vec<Value>) =
                    items.iter().map(|item| self.numbered(item)).unzip();
                (Node::List(numbers), Some(Value::List(items.into())))
            }
            Value::Object(fields) => {
                let mut numbers = Vec::with_capacity(fields.len());
                let mut kept = Vec::with_capacity(fields.len());
                for (key, inner) in fields.iter() {
                    let (number, inner) = self.numbered(inner);
                    numbers.push((key.clone(), number));
                    kept.push((key.clone(), inner));
                }
                (Node::Object(numbers), Some(Value::Object(kept.into())))
            }
        };
        if let Some(&number) = self.by_content.get(&node) {
            return (number, self.held[number].clone());
        }
        let number = self.held.len();
        let held = built.unwrap_or_else(|| value.clone());
        self.by_content.insert(node, number);
        self.held.push(held.clone());
        match &held {
            Value::List(items) => {
                self.by_address
                    .insert(address(items), (number, held.clone()));
            }
            Value::Object(fields) => {
                self.by_address
                    .insert(address(fields), (number, held.clone()));
            }
            _ => {}
        }
        (number, held)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Value {
    Undefined,
    Null,
    Bool(bool),
    Number(String),
    Text(String),
    List(Rc<[Value]>),
    Object(Rc<[(String, Value)]>),
}

pub(crate) const UNDEFINED: &Value = &Value::Undefined;

impl Value {
    pub(crate) fn of(json: &Json) -> Self {
        Self::of_shared(json, &mut Memo::default())
    }

    /// [`Value::of`], each [`Json::Shared`] read once and held as one `Rc`.
    pub(crate) fn of_shared(json: &Json, memo: &mut Memo) -> Self {
        match json {
            Json::Bool(value) => Self::Bool(*value),
            Json::Number(digits) => Self::Number(digits.clone()),
            Json::Text(text) => Self::Text(text.clone()),
            Json::List(items) => Self::List(
                items
                    .iter()
                    .map(|item| Self::of_shared(item, memo))
                    .collect(),
            ),
            Json::Object(fields) => Self::Object(
                fields
                    .iter()
                    .map(|(key, value)| (key.clone(), Self::of_shared(value, memo)))
                    .collect(),
            ),
            Json::Shared(inner) => {
                let address = Arc::as_ptr(inner).addr();
                if let Some(known) = memo.values.get(&address) {
                    return known.clone();
                }
                let value = Self::of_shared(inner, memo);
                memo.values.insert(address, value.clone());
                value
            }
            _ => Self::Null,
        }
    }

    pub(crate) fn to_json(&self) -> Json {
        match self {
            Self::Undefined | Self::Null => Json::Null,
            Self::Bool(value) => Json::Bool(*value),
            Self::Number(digits) => Json::Number(digits.clone()),
            Self::Text(text) => Json::Text(text.clone()),
            Self::List(items) => Json::List(items.iter().map(Self::to_json).collect()),
            Self::Object(fields) => Json::Object(
                fields
                    .iter()
                    .filter(|(_, value)| *value != Self::Undefined)
                    .map(|(key, value)| (key.clone(), value.to_json()))
                    .collect(),
            ),
        }
    }

    /// [`Value::to_json`], each object held in more than one place converted once and written
    /// as one [`Json::Shared`]: the same text, a type that many ids have stored once.
    pub(crate) fn to_shared_json(&self, memo: &mut Memo) -> Json {
        match self {
            Self::List(items) => {
                Json::List(items.iter().map(|item| item.to_shared_json(memo)).collect())
            }
            Self::Object(fields) => {
                let shared = Rc::strong_count(fields) > 1 || crate::json::sharing_every_object();
                let address = address(fields);
                if shared && let Some(known) = memo.json.get(&address) {
                    return known.clone();
                }
                let json = Json::Object(
                    fields
                        .iter()
                        .filter(|(_, value)| *value != Self::Undefined)
                        .map(|(key, value)| (key.clone(), value.to_shared_json(memo)))
                        .collect(),
                );
                if !shared {
                    return json;
                }
                let json = Json::Shared(Arc::new(json));
                memo.json.insert(address, json.clone());
                json
            }
            other => other.to_json(),
        }
    }

    pub(crate) fn list(items: Vec<Value>) -> Self {
        Self::List(items.into())
    }

    pub(crate) fn object(fields: Vec<(&str, Value)>) -> Self {
        Self::Object(
            fields
                .into_iter()
                .map(|(key, value)| (key.to_owned(), value))
                .collect(),
        )
    }

    pub(crate) fn text(value: &str) -> Self {
        Self::Text(value.to_owned())
    }

    pub(crate) fn get(&self, key: &str) -> &Value {
        match self {
            Self::Object(fields) => fields
                .iter()
                .find(|(name, _)| name == key)
                .map_or(UNDEFINED, |(_, value)| value),
            _ => UNDEFINED,
        }
    }

    pub(crate) fn is_defined(&self) -> bool {
        *self != Self::Undefined
    }

    pub(crate) fn truthy(&self) -> bool {
        match self {
            Self::Undefined | Self::Null => false,
            Self::Bool(value) => *value,
            Self::Number(digits) => !digits.trim_start_matches(['0', '-']).is_empty(),
            Self::Text(text) => !text.is_empty(),
            Self::List(_) | Self::Object(_) => true,
        }
    }

    pub(crate) fn str(&self, key: &str) -> &str {
        match self.get(key) {
            Self::Text(text) => text,
            _ => "",
        }
    }

    pub(crate) fn kind(&self) -> &str {
        self.str("kind")
    }

    pub(crate) fn items(&self, key: &str) -> &[Value] {
        match self.get(key) {
            Self::List(items) => items,
            _ => &[],
        }
    }

    pub(crate) fn as_items(&self) -> &[Value] {
        match self {
            Self::List(items) => items,
            _ => &[],
        }
    }

    pub(crate) fn id(&self) -> Option<u64> {
        match self.get("id") {
            Self::Number(digits) => digits.parse().ok(),
            _ => None,
        }
    }

    pub(crate) fn set(&mut self, key: &str, value: Value) {
        if let Self::Object(fields) = self {
            let mut owned = fields.to_vec();
            match owned.iter_mut().find(|(name, _)| name == key) {
                Some((_, slot)) => *slot = value,
                None => owned.push((key.to_owned(), value)),
            }
            *fields = owned.into();
        }
    }

    pub(crate) fn spread(&self, overrides: Vec<(&str, Value)>) -> Value {
        let mut copy = match self {
            Self::Object(_) => self.clone(),
            _ => Self::Object(Rc::from(Vec::new())),
        };
        for (key, value) in overrides {
            copy.set(key, value);
        }
        copy
    }

    pub(crate) fn same(&self, other: &Value) -> bool {
        match (self, other) {
            (Self::List(left), Self::List(right)) if Rc::ptr_eq(left, right) => true,
            (Self::Object(left), Self::Object(right)) if Rc::ptr_eq(left, right) => true,
            (Self::List(left), Self::List(right)) => {
                left.len() == right.len()
                    && left
                        .iter()
                        .zip(right.iter())
                        .all(|(left, right)| left.same(right))
            }
            (Self::Object(left), Self::Object(right)) => {
                left.len() == right.len()
                    && left.iter().all(|(key, value)| {
                        right
                            .iter()
                            .find(|(name, _)| name == key)
                            .is_some_and(|(_, inner)| value.same(inner))
                    })
            }
            _ => self == other,
        }
    }
}
