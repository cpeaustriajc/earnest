use serde::Deserialize;
use serde::de::IgnoredAny;
use std::collections::HashMap;

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
        value: serde_json::Number,
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
