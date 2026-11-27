use std::fmt::{self, Write};

/// A JSON value as Quint writes it: numbers keep their digits, so an integer of any size
/// survives, and an object keeps its keys in the order they were written.
#[derive(Clone, Debug, PartialEq, Eq)]
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
}

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

    /// The value under `key`, when this is an object that has it.
    pub fn get(&self, key: &str) -> Option<&Json> {
        match self {
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

impl fmt::Display for Json {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Null => formatter.write_str("null"),
            Self::Bool(value) => write!(formatter, "{value}"),
            Self::Number(digits) => formatter.write_str(digits),
            Self::Text(value) => quoted(value, formatter),
            Self::List(values) => {
                formatter.write_char('[')?;
                for (index, value) in values.iter().enumerate() {
                    if index > 0 {
                        formatter.write_char(',')?;
                    }
                    write!(formatter, "{value}")?;
                }
                formatter.write_char(']')
            }
            Self::Object(fields) => {
                formatter.write_char('{')?;
                for (index, (key, value)) in fields.iter().enumerate() {
                    if index > 0 {
                        formatter.write_char(',')?;
                    }
                    quoted(key, formatter)?;
                    write!(formatter, ":{value}")?;
                }
                formatter.write_char('}')
            }
        }
    }
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
