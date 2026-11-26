use rustc_hash::FxHashMap;
use serde_json::Value as Json;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

/// An interned name: a text value, or the name of a record field.
///
/// Only the [`Symbols`] that handed it out can turn it back into text. Two names from the same
/// `Symbols` are equal exactly when their text is equal. They sort in the order they were first
/// interned, so a `Symbols` made by [`Symbols::alphabetical`] sorts them by their text, in the
/// order of the code points, which is how a set, a map and a record are ordered everywhere.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub struct Sym(u32);

/// What one field of a model holds in one state.
///
/// An optional value has no wrapper: it is [`Value::Null`] when absent and the plain value when
/// present. A queue is a [`Value::Seq`], an owned lock is its owner or `Null`, and a timer or a
/// request is the [`Value::Str`] naming its stage. Collections are shared, so a clone is cheap.
#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
#[non_exhaustive]
pub enum Value {
    /// An absent optional value.
    Null,
    /// True or false.
    Bool(bool),
    /// A whole number.
    Int(i64),
    /// A text value or an enum value.
    Str(Sym),
    /// Named fields, sorted by name. Build one with [`Value::record`] to keep them sorted.
    Record(Arc<Vec<(Sym, Value)>>),
    /// Distinct items in sorted order.
    Set(Arc<BTreeSet<Value>>),
    /// Keys in sorted order, each with one value.
    Map(Arc<BTreeMap<Value, Value>>),
    /// Items in the order they were added.
    Seq(Arc<Vec<Value>>),
}

impl Value {
    /// A record of `fields`, in any order. Two records with the same fields are equal however
    /// they were listed.
    #[must_use]
    pub fn record(mut fields: Vec<(Sym, Self)>) -> Self {
        fields.sort_by_key(|(name, _)| *name);
        Self::Record(Arc::new(fields))
    }

    /// A set holding `items`.
    #[must_use]
    pub fn set(items: BTreeSet<Self>) -> Self {
        Self::Set(Arc::new(items))
    }

    /// A map holding `entries`.
    #[must_use]
    pub fn map(entries: BTreeMap<Self, Self>) -> Self {
        Self::Map(Arc::new(entries))
    }

    /// A sequence or a queue holding `items`, first item first.
    #[must_use]
    pub fn seq(items: Vec<Self>) -> Self {
        Self::Seq(Arc::new(items))
    }

    /// The value of the record field `name`. `None` when this is not a record or the record has
    /// no such field.
    #[must_use]
    pub fn field(&self, name: Sym) -> Option<&Self> {
        match self {
            Self::Record(fields) => fields
                .iter()
                .find(|(key, _)| *key == name)
                .map(|(_, value)| value),
            Self::Null
            | Self::Bool(_)
            | Self::Int(_)
            | Self::Str(_)
            | Self::Set(_)
            | Self::Map(_)
            | Self::Seq(_) => None,
        }
    }
}

/// Every name one model uses, and the way its values are printed.
///
/// A [`Sym`] means something only to the `Symbols` that made it. Keep one per model and pass it
/// along with the model's values.
#[derive(Clone, Debug, Default)]
pub struct Symbols {
    names: Vec<String>,
    ids: FxHashMap<String, Sym>,
    notation: Notation,
}

/// The way [`Symbols::show`], [`Symbols::plain`] and [`Symbols::json`] write a value: earnest's
/// own, or the language the model was read from.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum Notation {
    /// `{a, b}` for a set, `{k -> v}` for a map and `{name: value}` for a record.
    #[default]
    Earnest,
    /// Quint's, as its REPL writes values: `Set(a, b)`, `Map(k -> v)` and `{ name: value }`,
    /// with the records [`Quint`] names written as tuples and variants.
    Quint(Quint),
}

/// Which records a model read from Quint holds for a Quint tuple or variant, so
/// [`Notation::Quint`] writes them as Quint does.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Quint {
    /// A record of the fields `_1` to `_n` is the tuple `(a, b)`, `()` when it has none. In JSON
    /// it is `{"#tup": [a, b]}`, as Quint's ITF traces write it.
    pub tuples: bool,
    /// A record of the fields `tag`, a text, and `value` is the variant `Tag(value)`, or `Tag`
    /// when its value is the empty tuple. In JSON it is `{"tag": "Tag", "value": value}`.
    pub variants: bool,
}

enum Shape<'v> {
    Tuple(Vec<&'v Value>),
    Variant(Sym, &'v Value),
}

const TUPLE_ITEMS_START_WITH: char = '_';
const VARIANT_TAG: &str = "tag";
const VARIANT_VALUE: &str = "value";
const ITF_TUPLE: &str = "#tup";

impl Symbols {
    /// The name for `text`. Interning the same text again gives the same name back.
    pub fn intern(&mut self, text: &str) -> Sym {
        if let Some(found) = self.ids.get(text) {
            return *found;
        }
        let sym = Sym(u32::try_from(self.names.len()).unwrap_or(u32::MAX));
        self.names.push(text.to_owned());
        self.ids.insert(text.to_owned(), sym);
        sym
    }

    /// A `Symbols` that holds the same texts with the names handed out in the order of their
    /// code points, which is the order of Rust's `str` and of the UTF-8 bytes.
    ///
    /// Intern every text into the result in any order and all the names sort by their text, so
    /// the order of a set, of a map's keys and of a record's fields does not depend on the order
    /// the texts were first met in. Names of the old `Symbols` mean something else in the new
    /// one; intern the texts again.
    #[must_use]
    pub fn alphabetical(&self) -> Self {
        let mut texts: Vec<&String> = self.names.iter().collect();
        texts.sort();
        let mut sorted = Self {
            notation: self.notation,
            ..Self::default()
        };
        for text in texts {
            sorted.intern(text);
        }
        sorted
    }

    /// The same names, with values printed in `notation`. A model keeps
    /// [`Notation::Earnest`] unless its reader says otherwise.
    #[must_use]
    pub fn written_in(self, notation: Notation) -> Self {
        Self { notation, ..self }
    }

    /// The way values are printed.
    #[must_use]
    pub fn notation(&self) -> Notation {
        self.notation
    }

    fn shape<'v>(&self, fields: &'v [(Sym, Value)]) -> Option<Shape<'v>> {
        let Notation::Quint(quint) = self.notation else {
            return None;
        };
        if quint.variants
            && let [(first, Value::Str(tag)), (second, inner)] = fields
            && self.name(*first) == VARIANT_TAG
            && self.name(*second) == VARIANT_VALUE
        {
            return Some(Shape::Variant(*tag, inner));
        }
        if !quint.tuples {
            return None;
        }
        let mut items: Vec<Option<&Value>> = vec![None; fields.len()];
        for (name, inner) in fields {
            let position: usize = self
                .name(*name)
                .strip_prefix(TUPLE_ITEMS_START_WITH)
                .filter(|digits| !digits.starts_with('0'))?
                .parse()
                .ok()?;
            let slot = items.get_mut(position.checked_sub(1)?)?;
            *slot = Some(inner);
        }
        items
            .into_iter()
            .collect::<Option<Vec<_>>>()
            .map(Shape::Tuple)
    }

    /// Every text interned so far, in the order the names were handed out.
    pub fn texts(&self) -> impl Iterator<Item = &str> {
        self.names.iter().map(String::as_str)
    }

    /// The name for `text` if it was interned before. Unlike [`Symbols::intern`], it adds
    /// nothing.
    #[must_use]
    pub fn find(&self, text: &str) -> Option<Sym> {
        self.ids.get(text).copied()
    }

    /// The text behind `sym`. Empty when `sym` came from a different `Symbols` that holds more
    /// names than this one.
    #[must_use]
    pub fn name(&self, sym: Sym) -> &str {
        usize::try_from(sym.0)
            .ok()
            .and_then(|index| self.names.get(index))
            .map_or("", String::as_str)
    }

    /// `value` as the text report prints it: `"text"`, `12`, `true`, `null`, `{a, b}` for a set,
    /// `[a, b]` for a sequence or queue, `{k -> v}` for a map and `{name: value}` for a record,
    /// or in Quint's forms when the [`Notation`] is [`Notation::Quint`].
    ///
    /// A text value stays on one line and can be read back: inside the quotes `"` is `\"`, `\`
    /// is `\\`, a line feed is `\n`, a carriage return `\r`, a tab `\t`, and any other
    /// control character or line separator is `\u{...}` with its code in hexadecimal.
    #[must_use]
    pub fn show(&self, value: &Value) -> String {
        let mut out = String::new();
        self.write(value, &mut out);
        out
    }

    /// `value` as a chart label prints it: a text value loses its quotes, and everything else is
    /// as [`Symbols::show`] prints it.
    #[must_use]
    pub fn plain(&self, value: &Value) -> String {
        match value {
            Value::Str(sym) => self.name(*sym).to_owned(),
            Value::Null
            | Value::Bool(_)
            | Value::Int(_)
            | Value::Record(_)
            | Value::Set(_)
            | Value::Map(_)
            | Value::Seq(_) => self.show(value),
        }
    }

    fn write(&self, value: &Value, out: &mut String) {
        match value {
            Value::Null => out.push_str("null"),
            Value::Bool(flag) => out.push_str(if *flag { "true" } else { "false" }),
            Value::Int(number) => out.push_str(&number.to_string()),
            Value::Str(sym) => write_quoted(self.name(*sym), out),
            Value::Record(fields) => match self.shape(fields) {
                Some(Shape::Tuple(items)) => {
                    write_listed(out, "(", items.into_iter().map(|item| self.show(item)), ")");
                }
                Some(Shape::Variant(tag, inner)) => {
                    out.push_str(self.name(tag));
                    if !matches!(self.shape_of(inner), Some(Shape::Tuple(items)) if items.is_empty())
                    {
                        write_listed(out, "(", std::iter::once(self.show(inner)), ")");
                    }
                }
                None if fields.is_empty() => out.push_str("{}"),
                None => {
                    let shown = fields
                        .iter()
                        .map(|(name, inner)| format!("{}: {}", self.name(*name), self.show(inner)));
                    let (open, close) = self.braces();
                    write_listed(out, open, shown, close);
                }
            },
            Value::Set(items) => {
                let open = if self.is_quint() { "Set(" } else { "{" };
                let close = if self.is_quint() { ")" } else { "}" };
                write_listed(out, open, items.iter().map(|item| self.show(item)), close);
            }
            Value::Map(entries) => {
                let shown = entries
                    .iter()
                    .map(|(key, inner)| format!("{} -> {}", self.show(key), self.show(inner)));
                let open = if self.is_quint() { "Map(" } else { "{" };
                let close = if self.is_quint() { ")" } else { "}" };
                write_listed(out, open, shown, close);
            }
            Value::Seq(items) => {
                write_listed(out, "[", items.iter().map(|item| self.show(item)), "]");
            }
        }
    }

    /// `value` as the JSON output holds it: a set and a sequence are arrays, a map is an array
    /// of `[key, value]` pairs and a record is an object. In [`Notation::Quint`] a tuple is
    /// `{"#tup": [...]}`, as Quint's ITF traces write it.
    #[must_use]
    pub fn json(&self, value: &Value) -> Json {
        match value {
            Value::Null => Json::Null,
            Value::Bool(flag) => Json::Bool(*flag),
            Value::Int(number) => Json::from(*number),
            Value::Str(sym) => Json::String(self.name(*sym).to_owned()),
            Value::Record(fields) => match self.shape(fields) {
                Some(Shape::Tuple(items)) => Json::Object(
                    [(
                        ITF_TUPLE.to_owned(),
                        Json::Array(items.into_iter().map(|item| self.json(item)).collect()),
                    )]
                    .into_iter()
                    .collect(),
                ),
                Some(Shape::Variant(_, _)) | None => Json::Object(
                    fields
                        .iter()
                        .map(|(name, inner)| (self.name(*name).to_owned(), self.json(inner)))
                        .collect(),
                ),
            },
            Value::Set(items) => Json::Array(items.iter().map(|item| self.json(item)).collect()),
            Value::Map(entries) => Json::Array(
                entries
                    .iter()
                    .map(|(key, inner)| Json::Array(vec![self.json(key), self.json(inner)]))
                    .collect(),
            ),
            Value::Seq(items) => Json::Array(items.iter().map(|item| self.json(item)).collect()),
        }
    }
}

impl Symbols {
    fn is_quint(&self) -> bool {
        matches!(self.notation, Notation::Quint(_))
    }

    fn braces(&self) -> (&'static str, &'static str) {
        if self.is_quint() {
            ("{ ", " }")
        } else {
            ("{", "}")
        }
    }

    fn shape_of<'v>(&self, value: &'v Value) -> Option<Shape<'v>> {
        match value {
            Value::Record(fields) => self.shape(fields),
            Value::Null
            | Value::Bool(_)
            | Value::Int(_)
            | Value::Str(_)
            | Value::Set(_)
            | Value::Map(_)
            | Value::Seq(_) => None,
        }
    }
}

fn write_quoted(text: &str, out: &mut String) {
    out.push('"');
    for letter in text.chars() {
        match letter {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            other if other.is_control() || matches!(other, '\u{2028}' | '\u{2029}') => {
                out.extend(other.escape_unicode());
            }
            other => out.push(other),
        }
    }
    out.push('"');
}

fn write_listed(out: &mut String, open: &str, items: impl Iterator<Item = String>, close: &str) {
    out.push_str(open);
    for (index, item) in items.enumerate() {
        if index > 0 {
            out.push_str(", ");
        }
        out.push_str(&item);
    }
    out.push_str(close);
}
