use crate::json::Json;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Value {
    Undefined,
    Null,
    Bool(bool),
    Number(String),
    Text(String),
    List(Vec<Value>),
    Object(Vec<(String, Value)>),
}

pub(crate) static UNDEFINED: Value = Value::Undefined;

impl Value {
    pub(crate) fn of(json: &Json) -> Self {
        match json {
            Json::Bool(value) => Self::Bool(*value),
            Json::Number(digits) => Self::Number(digits.clone()),
            Json::Text(text) => Self::Text(text.clone()),
            Json::List(items) => Self::List(items.iter().map(Self::of).collect()),
            Json::Object(fields) => Self::Object(
                fields
                    .iter()
                    .map(|(key, value)| (key.clone(), Self::of(value)))
                    .collect(),
            ),
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
                .map_or(&UNDEFINED, |(_, value)| value),
            _ => &UNDEFINED,
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
            match fields.iter_mut().find(|(name, _)| name == key) {
                Some((_, slot)) => *slot = value,
                None => fields.push((key.to_owned(), value)),
            }
        }
    }

    pub(crate) fn spread(&self, overrides: Vec<(&str, Value)>) -> Value {
        let mut copy = match self {
            Self::Object(_) => self.clone(),
            _ => Self::Object(Vec::new()),
        };
        for (key, value) in overrides {
            copy.set(key, value);
        }
        copy
    }

    pub(crate) fn same(&self, other: &Value) -> bool {
        match (self, other) {
            (Self::List(left), Self::List(right)) => {
                left.len() == right.len()
                    && left.iter().zip(right).all(|(left, right)| left.same(right))
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
