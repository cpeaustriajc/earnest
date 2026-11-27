use crate::json::Json;
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

const BUILT_IN: [&str; 103] = [
    "Bool",
    "Int",
    "List",
    "Map",
    "Nat",
    "Rec",
    "Set",
    "Tup",
    "actionAll",
    "actionAny",
    "allLists",
    "allListsUpTo",
    "always",
    "and",
    "apalache::generate",
    "append",
    "applyTo",
    "assert",
    "assign",
    "chooseSome",
    "concat",
    "contains",
    "cross",
    "difference",
    "enabled",
    "eq",
    "eventually",
    "exclude",
    "exists",
    "expect",
    "fail",
    "field",
    "fieldNames",
    "fields",
    "filter",
    "flatten",
    "fold",
    "foldl",
    "foldr",
    "forall",
    "get",
    "getOnlyElement",
    "head",
    "iadd",
    "idiv",
    "iff",
    "igt",
    "igte",
    "ilt",
    "ilte",
    "imod",
    "implies",
    "imul",
    "in",
    "indices",
    "intersect",
    "ipow",
    "isFinite",
    "isub",
    "item",
    "ite",
    "iuminus",
    "keys",
    "leadsTo",
    "length",
    "map",
    "mapBy",
    "matchVariant",
    "mustChange",
    "neq",
    "next",
    "not",
    "nth",
    "of",
    "oneOf",
    "or",
    "orKeep",
    "powerset",
    "put",
    "q::debug",
    "q::lastTrace",
    "q::test",
    "q::testOnce",
    "range",
    "replaceAt",
    "reps",
    "select",
    "set",
    "setBy",
    "setOfMaps",
    "setToMap",
    "size",
    "slice",
    "strongFair",
    "subseteq",
    "tail",
    "then",
    "to",
    "tuples",
    "union",
    "variant",
    "weakFair",
    "with",
];

pub(crate) fn built_in(name: &str) -> bool {
    BUILT_IN.contains(&name)
}

#[derive(Clone, Debug, Default)]
pub(crate) struct Entry {
    fields: Vec<(String, Option<Arc<Json>>)>,
}

impl Entry {
    pub(crate) fn of(written: &Json) -> Self {
        match written {
            Json::Object(fields) => Self {
                fields: fields
                    .iter()
                    .map(|(key, value)| {
                        let kept = (*value != Json::Null).then(|| Arc::new(value.clone()));
                        (key.clone(), kept)
                    })
                    .collect(),
            },
            _ => Self::default(),
        }
    }

    pub(crate) fn get(&self, key: &str) -> Option<&Json> {
        self.fields
            .iter()
            .find(|(name, _)| name == key)
            .and_then(|(_, value)| value.as_deref())
    }

    pub(crate) fn set(&mut self, key: &str, value: Option<Json>) {
        let value = value.map(Arc::new);
        match self.fields.iter_mut().find(|(name, _)| name == key) {
            Some((_, slot)) => *slot = value,
            None => self.fields.push((key.to_owned(), value)),
        }
    }

    pub(crate) fn with(&self, key: &str, value: Option<Json>) -> Self {
        let mut copy = self.clone();
        copy.set(key, value);
        copy
    }

    pub(crate) fn id(&self) -> Option<u64> {
        number(self.get("id"))
    }

    pub(crate) fn text(&self, key: &str) -> &str {
        match self.get(key) {
            Some(Json::Text(text)) => text,
            _ => "",
        }
    }

    pub(crate) fn to_json(&self) -> Json {
        Json::Object(
            self.fields
                .iter()
                .filter_map(|(key, value)| {
                    value
                        .as_ref()
                        .map(|value| (key.clone(), Json::clone(value)))
                })
                .collect(),
        )
    }
}

pub(crate) fn number(value: Option<&Json>) -> Option<u64> {
    match value {
        Some(Json::Number(digits)) => digits.parse().ok(),
        _ => None,
    }
}

fn text<'a>(value: &'a Json, key: &str) -> &'a str {
    match value.get(key) {
        Some(Json::Text(text)) => text,
        _ => "",
    }
}

fn items<'a>(value: &'a Json, key: &str) -> &'a [Json] {
    match value.get(key) {
        Some(Json::List(items)) => items,
        _ => &[],
    }
}

#[derive(Clone, Debug, Default)]
struct Names {
    order: Vec<String>,
    entries: HashMap<String, Vec<Entry>>,
}

impl Names {
    fn get(&self, name: &str) -> Option<&Vec<Entry>> {
        self.entries.get(name)
    }

    fn set(&mut self, name: &str, entries: Vec<Entry>) {
        if self.entries.insert(name.to_owned(), entries).is_none() {
            self.order.push(name.to_owned());
        }
    }

    fn first(&self, name: &str) -> Option<&Entry> {
        self.entries.get(name).and_then(|entries| entries.first())
    }

    fn last(&self, name: &str) -> Option<&Entry> {
        self.entries.get(name).and_then(|entries| entries.last())
    }

    fn iter(&self) -> impl Iterator<Item = (&String, &Vec<Entry>)> {
        self.order
            .iter()
            .filter_map(|name| self.entries.get(name).map(|entries| (name, entries)))
    }

    fn copied(&self, namespace: Option<&str>, copy_as: Copy) -> Self {
        let mut copy = Self::default();
        for (name, entries) in self.iter() {
            let named = match namespace {
                Some(namespace) if !namespace.is_empty() => format!("{namespace}::{name}"),
                _ => name.clone(),
            };
            let kept = entries
                .iter()
                .filter(|entry| {
                    entry.get("hidden") != Some(&Json::Bool(true))
                        || entry.get("kind") == Some(&Json::text("const"))
                })
                .map(|entry| {
                    if copy_as == Copy::Hidden {
                        entry.with("hidden", Some(Json::Bool(true)))
                    } else {
                        entry.clone()
                    }
                })
                .collect();
            copy.set(&named, kept);
        }
        copy
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Copy {
    Hidden,
    AsTheyAre,
}

#[derive(Clone, Debug)]
pub(crate) struct Raised {
    pub(crate) code: &'static str,
    pub(crate) message: String,
    pub(crate) reference: Option<u64>,
}

fn raised(code: &'static str, message: String, reference: Option<u64>) -> Raised {
    Raised {
        code,
        message,
        reference,
    }
}

fn with_namespaces(entry: &Entry, namespaces: &[String]) -> Entry {
    let mut current = entry.clone();
    for namespace in namespaces {
        let known = match current.get("namespaces") {
            Some(Json::List(known)) => Some(known.clone()),
            _ => None,
        };
        if known
            .as_ref()
            .and_then(|known| known.last())
            .is_some_and(|last| *last == Json::text(namespace))
        {
            continue;
        }
        if current.text("name").starts_with(&format!("{namespace}::")) {
            continue;
        }
        let added = if namespace.is_empty() {
            Vec::new()
        } else {
            let mut known = known.unwrap_or_default();
            known.push(Json::text(namespace));
            known
        };
        current.set("namespaces", Some(Json::List(added)));
    }
    current
}

#[derive(Debug, Default)]
pub(crate) struct Resolution {
    pub(crate) table: BTreeMap<u64, Entry>,
    pub(crate) inserted: Vec<u64>,
    pub(crate) errors: Vec<Raised>,
}

impl Resolution {
    fn insert(&mut self, id: u64, entry: Entry) {
        if self.table.insert(id, entry).is_none() {
            self.inserted.push(id);
        }
    }
}

#[derive(Debug, Default)]
struct Resolver {
    by_module: HashMap<String, Names>,
    names: Names,
    module: String,
    depth: i64,
    resolution: Resolution,
}

pub(crate) fn resolve(modules: &mut [Json]) -> Resolution {
    let mut resolver = Resolver::default();
    for module in modules.iter_mut() {
        resolver.module(module);
    }
    resolver.resolution
}

fn declarations_mut(module: &mut Json) -> Option<&mut Vec<Json>> {
    match module {
        Json::Object(fields) => fields
            .iter_mut()
            .find(|(key, _)| key == "declarations")
            .and_then(|(_, value)| match value {
                Json::List(items) => Some(items),
                _ => None,
            }),
        _ => None,
    }
}

fn append(value: &mut Json, key: &str, inner: Json) {
    if let Json::Object(fields) = value {
        fields.push((key.to_owned(), inner));
    }
}

impl Resolver {
    fn raise(&mut self, code: &'static str, message: String, reference: Option<u64>) {
        self.resolution
            .errors
            .push(raised(code, message, reference));
    }

    fn module(&mut self, module: &mut Json) {
        let name = text(module, "name").to_owned();
        self.module.clone_from(&name);
        self.names = Names::default();
        if self.by_module.contains_key(&name) {
            self.raise(
                "QNT102",
                format!("Module with name '{name}' was already defined"),
                number(module.get("id")),
            );
        }
        if let Some(declarations) = declarations_mut(module) {
            for declaration in declarations.iter_mut() {
                self.collect_declaration(declaration);
            }
        }
        for declaration in items(module, "declarations") {
            self.declaration(declaration);
        }
        self.by_module.insert(name, std::mem::take(&mut self.names));
    }

    fn collect_declaration(&mut self, declaration: &mut Json) {
        match text(declaration, "kind") {
            "const" | "var" | "typedef" | "assume" => {
                let mut entry = Entry::of(declaration);
                self.collect(&mut entry, None);
                if declaration.get("depth").is_none()
                    && let Some(depth) = entry.get("depth")
                {
                    let depth = depth.clone();
                    append(declaration, "depth", depth);
                }
            }
            "def" => {
                let mut entry = Entry::of(declaration)
                    .with("typeAnnotation", None)
                    .with("depth", Some(Json::number(0)));
                self.collect(&mut entry, None);
            }
            "instance" => self.collect_instance(declaration),
            "import" => self.collect_import(declaration),
            "export" => self.collect_export(declaration),
            _ => {}
        }
    }

    fn self_reference(&mut self, declaration: &Json) -> bool {
        let proto = text(declaration, "protoName");
        if proto != self.module {
            return false;
        }
        let verb = match text(declaration, "kind") {
            "instance" => "instantiate",
            other => other,
        };
        self.raise(
            "QNT407",
            format!("Cannot {verb} '{proto}' inside '{proto}'"),
            number(declaration.get("id")),
        );
        true
    }

    fn proto_names(&mut self, declaration: &Json) -> Option<Names> {
        let proto = text(declaration, "protoName");
        if let Some(names) = self.by_module.get(proto) {
            return Some(names.clone());
        }
        self.raise(
            "QNT405",
            format!("Module '{proto}' not found"),
            number(declaration.get("id")),
        );
        None
    }

    fn collect_instance(&mut self, declaration: &Json) {
        if self.self_reference(declaration) {
            return;
        }
        let Some(mut instance) = self.proto_names(declaration) else {
            return;
        };
        let proto = text(declaration, "protoName");
        let reference = number(declaration.get("id"));
        for pair in items(declaration, "overrides") {
            let Json::List(pair) = pair else {
                continue;
            };
            let Some(param) = pair.first() else {
                continue;
            };
            let param = text(param, "name");
            let Some(constant) = instance
                .entries
                .get_mut(param)
                .and_then(|entries| entries.first_mut())
            else {
                self.raise(
                    "QNT406",
                    format!("Instantiation error: '{param}' not found in '{proto}'"),
                    reference,
                );
                continue;
            };
            if constant.get("kind") != Some(&Json::text("const")) {
                self.raise(
                    "QNT406",
                    format!("Instantiation error: '{param}' is not a constant in '{proto}'"),
                    reference,
                );
                continue;
            }
            constant.set("hidden", Some(Json::Bool(false)));
        }
        let qualified = match declaration.get("qualifiedName") {
            Some(Json::Text(qualified)) => Some(qualified.clone()),
            _ => None,
        };
        let copied = instance.copied(qualified.as_deref(), Copy::Hidden);
        if let Some(qualified) = &qualified {
            self.by_module.insert(qualified.clone(), instance);
        }
        self.collect_all(&copied, declaration);
    }

    fn qualifier(declaration: &Json) -> Option<String> {
        if text(declaration, "kind") == "instance" {
            return match declaration.get("qualifiedName") {
                Some(Json::Text(qualified)) => Some(qualified.clone()),
                _ => None,
            };
        }
        if declaration.get("defName").is_some() {
            return None;
        }
        match declaration.get("qualifiedName") {
            Some(Json::Text(qualified)) => Some(qualified.clone()),
            _ => Some(text(declaration, "protoName").to_owned()),
        }
    }

    fn collect_import(&mut self, declaration: &Json) {
        if self.self_reference(declaration) {
            return;
        }
        let Some(imported) = self.proto_names(declaration) else {
            return;
        };
        if let Some(Json::Text(qualified)) = declaration.get("qualifiedName") {
            self.by_module.insert(qualified.clone(), imported.clone());
        }
        let copied = imported.copied(Self::qualifier(declaration).as_deref(), Copy::Hidden);
        self.collect_named(&copied, declaration);
    }

    fn collect_export(&mut self, declaration: &Json) {
        if self.self_reference(declaration) {
            return;
        }
        let Some(exported) = self.proto_names(declaration) else {
            return;
        };
        let copied = exported.copied(Self::qualifier(declaration).as_deref(), Copy::AsTheyAre);
        self.collect_named(&copied, declaration);
    }

    fn collect_named(&mut self, copied: &Names, declaration: &Json) {
        let wanted = match declaration.get("defName") {
            Some(Json::Text(wanted)) if !wanted.is_empty() && wanted != "*" => wanted.clone(),
            _ => {
                self.collect_all(copied, declaration);
                return;
            }
        };
        match copied.first(&wanted) {
            Some(entry) => {
                let mut entry = entry.clone();
                self.collect(&mut entry, Some(declaration));
            }
            None => self.raise(
                "QNT404",
                format!(
                    "Name '{}::{wanted}' not found",
                    text(declaration, "protoName")
                ),
                number(declaration.get("id")),
            ),
        }
    }

    fn namespaces(&self, declaration: &Json) -> Vec<String> {
        if text(declaration, "kind") == "instance" {
            let first = match declaration.get("qualifiedName") {
                Some(Json::Text(qualified)) => qualified.clone(),
                _ => text(declaration, "protoName").to_owned(),
            };
            return [first, self.module.clone()]
                .into_iter()
                .filter(|name| !name.is_empty())
                .collect();
        }
        match Self::qualifier(declaration) {
            Some(namespace) if !namespace.is_empty() => vec![namespace],
            _ => Vec::new(),
        }
    }

    fn collect_all(&mut self, copied: &Names, declaration: &Json) {
        let namespaces = self.namespaces(declaration);
        let from = Some(Json::clone(declaration));
        let mut added: Vec<(String, Vec<Entry>)> = Vec::new();
        for (name, entries) in copied.iter() {
            let Some(entry) = entries.first() else {
                continue;
            };
            let new = with_namespaces(entry, &namespaces).with("importedFrom", from.clone());
            match self.names.get(name) {
                Some(existing) => {
                    let conflicting: Vec<Entry> = existing
                        .iter()
                        .filter(|known| known.get("id") != entry.get("id"))
                        .cloned()
                        .collect();
                    for known in &conflicting {
                        self.conflict(name, known.id(), entry.id());
                    }
                    let mut kept = conflicting;
                    kept.push(new);
                    added.push((name.clone(), kept));
                }
                None => added.push((name.clone(), vec![new])),
            }
        }
        for (name, entries) in added {
            self.names.set(&name, entries);
        }
    }

    fn conflict(&mut self, name: &str, existing: Option<u64>, new: Option<u64>) {
        let module = self.module.clone();
        match existing {
            Some(existing) if existing != 0 => {
                let message =
                    format!("Conflicting definitions found for name '{name}' in module '{module}'");
                self.raise("QNT101", message.clone(), Some(existing));
                self.raise("QNT101", message, new);
            }
            _ => self.raise(
                "QNT101",
                format!("Built-in name '{name}' is redefined in module '{module}'"),
                new,
            ),
        }
    }

    fn collect(&mut self, entry: &mut Entry, from: Option<&Json>) -> Entry {
        let name = match from.and_then(|from| from.get("defName")) {
            Some(Json::Text(name)) => name.clone(),
            _ => entry.text("name").to_owned(),
        };
        let source = match from {
            Some(from) => number(from.get("id")),
            None => entry.id(),
        };
        if name == "_" {
            return entry.clone();
        }
        if built_in(&name) {
            self.conflict(&name, None, source);
            return entry.clone();
        }
        if entry.get("depth").is_none() {
            entry.set("depth", Some(Json::number(0)));
        }
        let namespaces = from.map_or_else(Vec::new, |from| self.namespaces(from));
        let imported = from.cloned();
        let Some(existing) = self.names.get(&name).cloned() else {
            self.names.set(
                &name,
                vec![with_namespaces(entry, &namespaces).with("importedFrom", imported)],
            );
            return entry.clone();
        };
        for known in &existing {
            if known.get("id") != entry.get("id") && known.get("depth") == entry.get("depth") {
                self.conflict(&name, known.id(), source);
            }
        }
        let new = with_namespaces(entry, &namespaces)
            .with("importedFrom", imported)
            .with("shadowing", Some(Json::Bool(!existing.is_empty())));
        let mut kept: Vec<Entry> = existing
            .into_iter()
            .filter(|known| known.get("id") != entry.get("id"))
            .collect();
        kept.push(new.clone());
        self.names.set(&name, kept);
        new
    }

    fn delete(&mut self, name: &str) {
        if let Some(entries) = self.names.entries.get_mut(name) {
            entries.pop();
        }
    }

    fn signed_depth(&self) -> Json {
        Json::Number(self.depth.to_string())
    }

    fn declaration(&mut self, declaration: &Json) {
        match text(declaration, "kind") {
            "def" => {
                self.depth = -1;
                self.definition(declaration);
            }
            "const" | "var" | "typedef" | "assume" => {
                self.depth = 0;
                self.definition(declaration);
            }
            "instance" => {
                self.depth = 0;
                let qualified = match declaration.get("qualifiedName") {
                    Some(Json::Text(qualified)) => Some(qualified.clone()),
                    _ => None,
                };
                for pair in items(declaration, "overrides") {
                    if let Json::List(pair) = pair
                        && let Some(param) = pair.first()
                    {
                        let name = text(param, "name");
                        let name = match &qualified {
                            Some(qualified) => format!("{qualified}::{name}"),
                            None => name.to_owned(),
                        };
                        self.resolve_name(&name, number(param.get("id")));
                    }
                }
                for pair in items(declaration, "overrides") {
                    if let Json::List(pair) = pair
                        && let Some(value) = pair.get(1)
                    {
                        self.expression(value);
                    }
                }
            }
            _ => {}
        }
    }

    fn definition(&mut self, definition: &Json) {
        self.depth += 1;
        if let Some(annotation) = definition.get("typeAnnotation") {
            self.kind_of(annotation);
        } else if text(definition, "kind") == "typedef"
            && let Some(written) = definition.get("type")
        {
            self.kind_of(written);
        }
        match text(definition, "kind") {
            "def" => {
                self.operator(definition);
                if let Some(body) = definition.get("expr") {
                    self.expression(body);
                }
            }
            "assume" => {
                if let Some(assumption) = definition.get("assumption") {
                    self.expression(assumption);
                }
            }
            _ => {}
        }
        self.depth -= 1;
    }

    fn operator(&mut self, definition: &Json) {
        let Some(id) = number(definition.get("id")) else {
            return;
        };
        let depth = Some(self.signed_depth());
        let entry = if self.depth > 0 {
            let mut scoped = Entry::of(definition).with("depth", depth.clone());
            self.collect(&mut scoped, None).with("depth", depth)
        } else {
            Entry::of(definition).with("depth", depth)
        };
        self.resolution.insert(id, entry);
    }

    fn expression(&mut self, expression: &Json) {
        match text(expression, "kind") {
            "name" => self.resolve_name(text(expression, "name"), number(expression.get("id"))),
            "app" => {
                self.resolve_name(text(expression, "opcode"), number(expression.get("id")));
                for argument in items(expression, "args") {
                    self.expression(argument);
                }
            }
            "lambda" => {
                self.depth += 1;
                let params = items(expression, "params");
                for param in params {
                    let mut entry = Entry::of(param)
                        .with("kind", Some(Json::text("param")))
                        .with("depth", Some(self.signed_depth()));
                    self.collect(&mut entry, None);
                }
                for param in params {
                    if let Some(annotation) = param.get("typeAnnotation") {
                        self.kind_of(annotation);
                    }
                }
                if let Some(body) = expression.get("expr") {
                    self.expression(body);
                }
                for param in params {
                    self.delete(text(param, "name"));
                }
                self.depth -= 1;
            }
            "let" => {
                self.depth += 1;
                if let Some(definition) = expression.get("opdef") {
                    self.definition(definition);
                }
                if let Some(body) = expression.get("expr") {
                    self.expression(body);
                }
                if let Some(definition) = expression.get("opdef") {
                    self.delete(text(definition, "name"));
                }
                self.depth -= 1;
            }
            _ => {}
        }
    }

    fn resolve_name(&mut self, name: &str, id: Option<u64>) {
        if built_in(name) {
            return;
        }
        match self.names.last(name) {
            Some(entry) if entry.get("kind") != Some(&Json::text("typedef")) => {
                let entry = entry.clone();
                if let Some(id) = id {
                    self.resolution.insert(id, entry);
                }
            }
            _ => self.raise("QNT404", format!("Name '{name}' not found"), id),
        }
    }

    fn kind_of(&mut self, written: &Json) {
        match text(written, "kind") {
            "const" => {
                let name = text(written, "name");
                let id = number(written.get("id"));
                match self.names.last(name) {
                    Some(entry) if entry.get("kind") == Some(&Json::text("typedef")) => {
                        let entry = entry.clone();
                        if let Some(id) = id {
                            self.resolution.insert(id, entry);
                        }
                    }
                    _ => self.raise("QNT404", format!("Type alias '{name}' not found"), id),
                }
            }
            "set" | "list" => {
                if let Some(element) = written.get("elem") {
                    self.kind_of(element);
                }
            }
            "fun" => {
                for key in ["arg", "res"] {
                    if let Some(inner) = written.get(key) {
                        self.kind_of(inner);
                    }
                }
            }
            "oper" => {
                for argument in items(written, "args") {
                    self.kind_of(argument);
                }
                if let Some(result) = written.get("res") {
                    self.kind_of(result);
                }
            }
            "tup" | "rec" | "sum" => {
                if let Some(row) = written.get("fields") {
                    self.row(row);
                }
            }
            "app" => {
                if let Some(constructor) = written.get("ctor") {
                    self.kind_of(constructor);
                }
                for argument in items(written, "args") {
                    self.kind_of(argument);
                }
            }
            _ => {}
        }
    }

    fn row(&mut self, row: &Json) {
        if text(row, "kind") != "row" {
            return;
        }
        for field in items(row, "fields") {
            if let Some(inner) = field.get("fieldType") {
                self.kind_of(inner);
            }
        }
        if let Some(other) = row.get("other") {
            self.row(other);
        }
    }
}
