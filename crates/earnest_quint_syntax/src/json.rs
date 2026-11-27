use std::fmt::{self, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};

/// Set by a test: every object the types table and the alias inliner write is shared, not only
/// those held twice, so that any reading that does not see through a [`Json::Shared`] shows as
/// a difference in the output. `EARNEST_QUINT_SHARE_EVERY_OBJECT=1` sets it for a whole run.
static SHARE_EVERY_OBJECT: AtomicBool = AtomicBool::new(false);

#[cfg(test)]
pub(crate) fn share_every_object(on: bool) {
    SHARE_EVERY_OBJECT.store(on, Ordering::Relaxed);
}

pub(crate) fn sharing_every_object() -> bool {
    static FROM_ENVIRONMENT: OnceLock<bool> = OnceLock::new();
    SHARE_EVERY_OBJECT.load(Ordering::Relaxed)
        || *FROM_ENVIRONMENT
            .get_or_init(|| std::env::var_os("EARNEST_QUINT_SHARE_EVERY_OBJECT").is_some())
}

/// A JSON value as Quint writes it: numbers keep their digits, so an integer of any size
/// survives, and an object keeps its keys in the order they were written.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum Json {
    /// `null`.
    Null,
    /// `true` or `false`.
    Bool(bool),
    /// A number, as its digits.
    Number(String),
    /// A string.
    Text(String),
    /// An array.
    List(Vec<Json>),
    /// An object, its keys in order.
    Object(Vec<(String, Json)>),
    /// A value held in more than one place, as the types table holds a type that many ids
    /// have. It is written, compared and read as the value it holds. It holds an object, or
    /// another `Shared`: a list or a scalar is never shared.
    Shared(Arc<Json>),
}

impl PartialEq for Json {
    fn eq(&self, other: &Self) -> bool {
        if let (Self::Shared(left), Self::Shared(right)) = (self, other)
            && Arc::ptr_eq(left, right)
        {
            return true;
        }
        match (self.unshared(), other.unshared()) {
            (Self::Null, Self::Null) => true,
            (Self::Bool(left), Self::Bool(right)) => left == right,
            (Self::Number(left), Self::Number(right)) | (Self::Text(left), Self::Text(right)) => {
                left == right
            }
            (Self::List(left), Self::List(right)) => left == right,
            (Self::Object(left), Self::Object(right)) => left == right,
            _ => false,
        }
    }
}

impl Eq for Json {}

impl Json {
    pub(crate) fn object(fields: Vec<(&str, Json)>) -> Self {
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

    pub(crate) fn number(value: u64) -> Self {
        Self::Number(value.to_string())
    }

    pub(crate) fn signed(value: isize) -> Self {
        Self::Number(value.to_string())
    }

    /// The value this one holds: itself, or what a [`Json::Shared`] holds.
    pub fn unshared(&self) -> &Json {
        let mut value = self;
        while let Self::Shared(inner) = value {
            value = inner;
        }
        value
    }

    /// This value with every object in it shared, for [`sharing_every_object`].
    pub(crate) fn with_every_object_shared(&self) -> Json {
        match self.unshared() {
            Self::List(items) => {
                Self::List(items.iter().map(Self::with_every_object_shared).collect())
            }
            Self::Object(fields) => Self::Shared(Arc::new(Self::Object(
                fields
                    .iter()
                    .map(|(key, inner)| (key.clone(), inner.with_every_object_shared()))
                    .collect(),
            ))),
            other => other.clone(),
        }
    }

    /// This value to change in place: a [`Json::Shared`] is copied first where another holder
    /// has it too, so a change is never seen through another holder.
    pub(crate) fn owned_mut(&mut self) -> &mut Json {
        let mut value = self;
        while let Self::Shared(inner) = value {
            value = Arc::make_mut(inner);
        }
        value
    }

    /// The value under `key`, when this is an object that has it.
    pub fn get(&self, key: &str) -> Option<&Json> {
        match self.unshared() {
            Self::Object(fields) => fields
                .iter()
                .find(|(name, _)| name == key)
                .map(|(_, value)| value),
            _ => None,
        }
    }

    /// Reads JSON text.
    ///
    /// # Errors
    ///
    /// The character offset where the text stops being JSON.
    pub fn parse(text: &str) -> Result<Json, usize> {
        let chars: Vec<char> = text.chars().collect();
        let mut reader = Reader {
            chars: &chars,
            at: 0,
        };
        let value = reader.value()?;
        reader.space();
        if reader.at == chars.len() {
            Ok(value)
        } else {
            Err(reader.at)
        }
    }
}

impl Drop for Json {
    fn drop(&mut self) {
        let mut pending = Vec::new();
        self.take_children(&mut pending);
        while let Some(mut value) = pending.pop() {
            value.take_children(&mut pending);
        }
    }
}

impl Json {
    /// Moves what this value holds to `pending`, so that dropping it drops nothing nested: a
    /// shared value gives up what it holds only when this is its last holder.
    fn take_children(&mut self, pending: &mut Vec<Json>) {
        match self {
            Self::List(items) => pending.append(items),
            Self::Object(fields) => {
                pending.extend(std::mem::take(fields).into_iter().map(|(_, inner)| inner));
            }
            Self::Shared(shared) => {
                if let Some(inner) = Arc::get_mut(shared) {
                    pending.push(std::mem::replace(inner, Self::Null));
                }
            }
            _ => {}
        }
    }
}

impl fmt::Display for Json {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut pending = vec![Step::Value(self)];
        while let Some(step) = pending.pop() {
            match step {
                Step::Mark(mark) => formatter.write_char(mark)?,
                Step::Key(key) => {
                    quoted(key, formatter)?;
                    formatter.write_char(':')?;
                }
                Step::Value(value) => match value {
                    Self::Null => formatter.write_str("null")?,
                    Self::Bool(value) => write!(formatter, "{value}")?,
                    Self::Number(digits) => formatter.write_str(digits)?,
                    Self::Text(value) => quoted(value, formatter)?,
                    Self::Shared(inner) => pending.push(Step::Value(inner)),
                    Self::List(values) => {
                        formatter.write_char('[')?;
                        pending.push(Step::Mark(']'));
                        for (index, value) in values.iter().enumerate().rev() {
                            pending.push(Step::Value(value));
                            if index > 0 {
                                pending.push(Step::Mark(','));
                            }
                        }
                    }
                    Self::Object(fields) => {
                        formatter.write_char('{')?;
                        pending.push(Step::Mark('}'));
                        for (index, (key, value)) in fields.iter().enumerate().rev() {
                            pending.push(Step::Value(value));
                            pending.push(Step::Key(key));
                            if index > 0 {
                                pending.push(Step::Mark(','));
                            }
                        }
                    }
                },
            }
        }
        Ok(())
    }
}

enum Step<'a> {
    Value(&'a Json),
    Key(&'a str),
    Mark(char),
}

fn quoted(value: &str, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    formatter.write_char('"')?;
    for char in value.chars() {
        match char {
            '"' => formatter.write_str("\\\"")?,
            '\\' => formatter.write_str("\\\\")?,
            '\n' => formatter.write_str("\\n")?,
            '\r' => formatter.write_str("\\r")?,
            '\t' => formatter.write_str("\\t")?,
            '\u{8}' => formatter.write_str("\\b")?,
            '\u{c}' => formatter.write_str("\\f")?,
            control if u32::from(control) < 0x20 => {
                write!(formatter, "\\u{:04x}", u32::from(control))?;
            }
            other => formatter.write_char(other)?,
        }
    }
    formatter.write_char('"')
}

struct Reader<'a> {
    chars: &'a [char],
    at: usize,
}

impl Reader<'_> {
    fn space(&mut self) {
        while self
            .chars
            .get(self.at)
            .is_some_and(|char| matches!(char, ' ' | '\t' | '\r' | '\n'))
        {
            self.at += 1;
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.at).copied()
    }

    fn expect(&mut self, char: char) -> Result<(), usize> {
        self.space();
        if self.peek() == Some(char) {
            self.at += 1;
            Ok(())
        } else {
            Err(self.at)
        }
    }

    fn word(&mut self, word: &str, value: Json) -> Result<Json, usize> {
        for expected in word.chars() {
            if self.peek() != Some(expected) {
                return Err(self.at);
            }
            self.at += 1;
        }
        Ok(value)
    }

    fn value(&mut self) -> Result<Json, usize> {
        self.space();
        match self.peek() {
            Some('n') => self.word("null", Json::Null),
            Some('t') => self.word("true", Json::Bool(true)),
            Some('f') => self.word("false", Json::Bool(false)),
            Some('"') => self.string().map(Json::Text),
            Some('[') => {
                self.at += 1;
                let mut values = Vec::new();
                self.space();
                if self.peek() == Some(']') {
                    self.at += 1;
                    return Ok(Json::List(values));
                }
                loop {
                    values.push(self.value()?);
                    self.space();
                    match self.peek() {
                        Some(',') => self.at += 1,
                        Some(']') => {
                            self.at += 1;
                            return Ok(Json::List(values));
                        }
                        _ => return Err(self.at),
                    }
                }
            }
            Some('{') => {
                self.at += 1;
                let mut fields = Vec::new();
                self.space();
                if self.peek() == Some('}') {
                    self.at += 1;
                    return Ok(Json::Object(fields));
                }
                loop {
                    self.space();
                    let key = self.string()?;
                    self.expect(':')?;
                    fields.push((key, self.value()?));
                    self.space();
                    match self.peek() {
                        Some(',') => self.at += 1,
                        Some('}') => {
                            self.at += 1;
                            return Ok(Json::Object(fields));
                        }
                        _ => return Err(self.at),
                    }
                }
            }
            Some(char) if char == '-' || char.is_ascii_digit() => {
                let start = self.at;
                while self.peek().is_some_and(|char| {
                    char == '-'
                        || char == '+'
                        || char == '.'
                        || char == 'e'
                        || char == 'E'
                        || char.is_ascii_digit()
                }) {
                    self.at += 1;
                }
                Ok(Json::Number(self.chars[start..self.at].iter().collect()))
            }
            _ => Err(self.at),
        }
    }

    fn string(&mut self) -> Result<String, usize> {
        if self.peek() != Some('"') {
            return Err(self.at);
        }
        self.at += 1;
        let mut value = String::new();
        loop {
            let char = self.peek().ok_or(self.at)?;
            self.at += 1;
            match char {
                '"' => return Ok(value),
                '\\' => {
                    let escaped = self.peek().ok_or(self.at)?;
                    self.at += 1;
                    match escaped {
                        'n' => value.push('\n'),
                        'r' => value.push('\r'),
                        't' => value.push('\t'),
                        'b' => value.push('\u{8}'),
                        'f' => value.push('\u{c}'),
                        'u' => {
                            let unit = self.unit()?;
                            if (0xd800..0xdc00).contains(&unit)
                                && self.chars.get(self.at) == Some(&'\\')
                                && self.chars.get(self.at + 1) == Some(&'u')
                            {
                                self.at += 2;
                                let low = self.unit()?;
                                let joined = 0x10000 + ((unit - 0xd800) << 10) + (low - 0xdc00);
                                value.push(char::from_u32(joined).unwrap_or('\u{fffd}'));
                            } else {
                                value.push(char::from_u32(unit).unwrap_or('\u{fffd}'));
                            }
                        }
                        other => value.push(other),
                    }
                }
                other => value.push(other),
            }
        }
    }

    fn unit(&mut self) -> Result<u32, usize> {
        let digits: String = self
            .chars
            .get(self.at..self.at + 4)
            .ok_or(self.at)?
            .iter()
            .collect();
        self.at += 4;
        u32::from_str_radix(&digits, 16).map_err(|_| self.at)
    }
}
