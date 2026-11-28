use earnest_value::BigInt;
use serde::Deserialize;
use serde::de::IgnoredAny;
use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Deserialize)]
pub(crate) struct Probe {
    pub(crate) stage: String,
    #[serde(rename = "main")]
    _main: String,
    #[serde(rename = "modules")]
    _modules: IgnoredAny,
    #[serde(rename = "table")]
    _table: IgnoredAny,
}

#[derive(Debug, Deserialize)]
pub(crate) struct Compiled {
    pub(crate) main: String,
    pub(crate) modules: Vec<Module>,
    pub(crate) table: HashMap<String, Entry>,
    #[serde(default)]
    pub(crate) types: HashMap<String, Scheme>,
    #[serde(default)]
    pub(crate) errors: Vec<IgnoredAny>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct Scheme {
    #[serde(rename = "type")]
    pub(crate) shape: Shape,
}

#[derive(Debug, Deserialize)]
pub(crate) struct Shape {
    pub(crate) kind: String,
    #[serde(default)]
    pub(crate) fields: Option<Row>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct Row {
    pub(crate) kind: String,
    #[serde(default)]
    pub(crate) fields: Vec<RowField>,
    #[serde(default)]
    pub(crate) other: Option<Box<Row>>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct RowField {
    #[serde(rename = "fieldName")]
    pub(crate) name: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct Module {
    pub(crate) name: String,
    pub(crate) declarations: Vec<Declaration>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct Entry {
    pub(crate) kind: String,
    pub(crate) id: u64,
    #[serde(default)]
    pub(crate) name: Option<String>,
    #[serde(default)]
    pub(crate) qualifier: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct Type {
    pub(crate) kind: String,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub(crate) enum Declaration {
    Var {
        id: u64,
        name: String,
        #[serde(rename = "typeAnnotation")]
        type_annotation: Type,
    },
    Const {
        name: String,
    },
    Def(Def),
    Assume {
        name: String,
        assumption: Expr,
    },
    Typedef {},
    #[serde(other)]
    Other,
}

#[derive(Debug, Deserialize)]
pub(crate) struct Def {
    pub(crate) id: u64,
    pub(crate) name: String,
    pub(crate) qualifier: String,
    pub(crate) expr: Expr,
}

#[derive(Debug, Deserialize)]
pub(crate) struct Param {
    pub(crate) id: u64,
    pub(crate) name: String,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub(crate) enum Expr {
    Int {
        value: Literal,
    },
    Bool {
        value: bool,
    },
    Str {
        value: String,
    },
    Name {
        id: u64,
        name: String,
    },
    App {
        id: u64,
        opcode: String,
        args: Vec<Expr>,
    },
    Lambda {
        params: Vec<Param>,
        expr: Box<Expr>,
    },
    Let {
        opdef: Box<Def>,
        expr: Box<Expr>,
    },
}

/// An integer literal as Quint wrote it: a JSON number, or, past 64 bits, the digits that
/// [`exact`] put in quotes so that they are not rounded.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub(crate) enum Literal {
    Number(serde_json::Number),
    Digits(String),
}

impl Literal {
    pub(crate) fn as_u64(&self) -> Option<u64> {
        match self {
            Self::Number(number) => number.as_u64(),
            Self::Digits(_) => None,
        }
    }

    pub(crate) fn whole(&self) -> Option<BigInt> {
        match self {
            Self::Number(number) => number
                .as_i64()
                .map(BigInt::from)
                .or_else(|| number.as_u64().map(BigInt::from)),
            Self::Digits(digits) => digits.parse().ok(),
        }
    }
}

impl fmt::Display for Literal {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Number(number) => number.fmt(formatter),
            Self::Digits(digits) => formatter.write_str(digits),
        }
    }
}

/// `source` with every whole number that fits in neither `i64` nor `u64` put in quotes, so
/// that reading the JSON keeps its digits where `serde_json` would round it to a float.
/// Text inside strings is left alone.
pub(crate) fn exact(source: &str) -> Cow<'_, str> {
    let bytes = source.as_bytes();
    let mut out: Option<String> = None;
    let mut copied = 0;
    let mut at = 0;
    let mut in_string = false;
    while let Some(&byte) = bytes.get(at) {
        if in_string {
            match byte {
                b'\\' => at += 1,
                b'"' => in_string = false,
                _ => {}
            }
            at += 1;
            continue;
        }
        if byte == b'"' {
            in_string = true;
            at += 1;
            continue;
        }
        if byte != b'-' && !byte.is_ascii_digit() {
            at += 1;
            continue;
        }
        let start = at;
        at += 1;
        while bytes.get(at).is_some_and(|next| {
            next.is_ascii_digit() || matches!(next, b'.' | b'e' | b'E' | b'+' | b'-')
        }) {
            at += 1;
        }
        let Some(token) = source.get(start..at) else {
            continue;
        };
        let digits = token.strip_prefix('-').unwrap_or(token);
        if digits.is_empty()
            || !digits.bytes().all(|each| each.is_ascii_digit())
            || token.parse::<i64>().is_ok()
            || token.parse::<u64>().is_ok()
        {
            continue;
        }
        let written = out.get_or_insert_with(|| String::with_capacity(source.len() + 16));
        written.push_str(source.get(copied..start).unwrap_or_default());
        written.push('"');
        written.push_str(token);
        written.push('"');
        copied = at;
    }
    match out {
        None => Cow::Borrowed(source),
        Some(mut written) => {
            written.push_str(source.get(copied..).unwrap_or_default());
            Cow::Owned(written)
        }
    }
}
