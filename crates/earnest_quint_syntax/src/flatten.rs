use crate::ids::Ids;
use crate::json::Json;
use crate::names::{Entry, Raised, Resolution, built_in, resolve};
use std::collections::{BTreeMap, HashMap, HashSet};

#[derive(Clone, Debug, Default)]
pub(crate) struct Table {
    pub(crate) entries: BTreeMap<u64, Entry>,
    pub(crate) order: Vec<u64>,
}

impl Table {
    pub(crate) fn of(resolution: Resolution) -> Self {
        Self {
            entries: resolution.table,
            order: resolution.order,
        }
    }

    pub(crate) fn get(&self, id: u64) -> Option<&Entry> {
        self.entries.get(&id)
    }

    fn replace(&mut self, id: u64, entry: Entry) {
        if self.entries.insert(id, entry).is_none() {
            self.order.push(id);
        }
    }

    fn in_order(&self) -> impl Iterator<Item = &Entry> {
        self.order.iter().filter_map(|id| self.entries.get(id))
    }

    pub(crate) fn to_json(&self) -> Json {
        Json::Object(
            self.entries
                .iter()
                .map(|(id, entry)| (id.to_string(), entry.to_json()))
                .collect(),
        )
    }
}

pub(crate) fn text<'a>(value: &'a Json, key: &str) -> &'a str {
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

pub(crate) fn id_of(value: &Json) -> Option<u64> {
    match value.get("id") {
        Some(Json::Number(digits)) => digits.parse().ok(),
        _ => None,
    }
}

pub(crate) fn put(value: &mut Json, key: &str, inner: Json) {
    if let Json::Object(fields) = value {
        match fields.iter_mut().find(|(name, _)| name == key) {
            Some((_, slot)) => *slot = inner,
            None => fields.push((key.to_owned(), inner)),
        }
    }
}

fn remove(value: &mut Json, key: &str) {
    if let Json::Object(fields) = value {
        fields.retain(|(name, _)| name != key);
    }
}

pub(crate) fn field_mut<'a>(value: &'a mut Json, key: &str) -> Option<&'a mut Json> {
    match value {
        Json::Object(fields) => fields
            .iter_mut()
            .find(|(name, _)| name == key)
            .map(|(_, inner)| inner),
        _ => None,
    }
}

fn list_mut<'a>(value: &'a mut Json, key: &str) -> &'a mut [Json] {
    match field_mut(value, key) {
        Some(Json::List(items)) => items,
        _ => &mut [],
    }
}

fn is_definition(declaration: &Json) -> bool {
    !matches!(text(declaration, "kind"), "instance" | "import" | "export")
}

fn imported_from_instance(entry: &Entry) -> bool {
    entry
        .get("importedFrom")
        .is_some_and(|from| text(from, "kind") == "instance")
}

fn namespace_of(entry: &Entry) -> Option<String> {
    let Some(Json::List(namespaces)) = entry.get("namespaces") else {
        return None;
    };
    let mut parts = namespaces.iter().map(|part| match part {
        Json::Text(part) => part.clone(),
        _ => String::new(),
    });
    let first = parts.next()?;
    Some(parts.fold(first, |inner, outer| format!("{outer}::{inner}")))
}

fn present(namespace: Option<&str>) -> Option<&str> {
    namespace.filter(|namespace| !namespace.is_empty())
}

fn needs(namespace: Option<&str>, name: &str) -> Option<String> {
    present(namespace)
        .filter(|namespace| !name.starts_with(&format!("{namespace}::")))
        .map(str::to_owned)
}

fn namespaced_name(namespace: &str, name: &str) -> String {
    if name == "_" || namespace.is_empty() || name.starts_with(&format!("{namespace}::")) {
        name.to_owned()
    } else {
        format!("{namespace}::{name}")
    }
}

struct Unshadower<'a> {
    table: &'a mut Table,
    nested: HashMap<u64, String>,
}

impl Unshadower<'_> {
    fn declaration(&mut self, declaration: &mut Json) {
        match text(declaration, "kind") {
            "instance" => {
                for pair in list_mut(declaration, "overrides") {
                    if let Json::List(pair) = pair
                        && let Some(value) = pair.get_mut(1)
                    {
                        self.expression(value);
                    }
                }
            }
            _ => self.definition(declaration),
        }
    }

    fn definition(&mut self, definition: &mut Json) {
        let key = match text(definition, "kind") {
            "def" => "expr",
            "assume" => "assumption",
            _ => return,
        };
        if let Some(body) = field_mut(definition, key) {
            self.expression(body);
        }
    }

    fn rename(&mut self, expression: &mut Json, key: &str) {
        let Some(id) = id_of(expression) else {
            return;
        };
        let Some(entry) = self.table.get(id) else {
            return;
        };
        let Some(renamed) = entry.id().and_then(|defined| self.nested.get(&defined)) else {
            return;
        };
        let renamed = renamed.clone();
        let changed = entry.with("name", Some(Json::text(&renamed)));
        self.table.replace(id, changed);
        put(expression, key, Json::Text(renamed));
    }

    fn expression(&mut self, expression: &mut Json) {
        match text(expression, "kind") {
            "name" => self.rename(expression, "name"),
            "app" => {
                self.rename(expression, "opcode");
                for argument in list_mut(expression, "args") {
                    self.expression(argument);
                }
            }
            "lambda" => {
                let lambda = id_of(expression).unwrap_or(0);
                for param in list_mut(expression, "params") {
                    let renamed = format!("{}_{lambda}", text(param, "name"));
                    if let Some(id) = id_of(param) {
                        self.nested.insert(id, renamed.clone());
                    }
                    put(param, "name", Json::Text(renamed));
                }
                if let Some(body) = field_mut(expression, "expr") {
                    self.expression(body);
                }
            }
            "let" => {
                let at = id_of(expression).unwrap_or(0);
                if let Some(definition) = field_mut(expression, "opdef") {
                    let shadowing = id_of(definition)
                        .and_then(|id| self.table.get(id))
                        .and_then(|entry| entry.get("shadowing"))
                        == Some(&Json::Bool(true));
                    if shadowing {
                        let renamed = format!("{}_{at}", text(definition, "name"));
                        if let Some(id) = id_of(definition) {
                            self.nested.insert(id, renamed.clone());
                        }
                        put(definition, "name", Json::Text(renamed));
                    }
                    self.definition(definition);
                }
                if let Some(body) = field_mut(expression, "expr") {
                    self.expression(body);
                }
            }
            _ => {}
        }
    }
}

fn unshadow(module: &Json, table: &mut Table) -> Json {
    let mut module = module.clone();
    let mut unshadower = Unshadower {
        table,
        nested: HashMap::new(),
    };
    for declaration in list_mut(&mut module, "declarations") {
        unshadower.declaration(declaration);
    }
    module
}

fn resolve_alias<'a>(table: &'a Table, written: &'a Json) -> &'a Json {
    if text(written, "kind") == "const"
        && let Some(entry) = id_of(written).and_then(|id| table.get(id))
        && entry.text("kind") == "typedef"
        && let Some(aliased) = entry.get("type")
    {
        return resolve_alias(table, aliased);
    }
    written
}

fn inline_type(table: &Table, written: &Json) -> Json {
    let mut inlined = resolve_alias(table, written).clone();
    match text(&inlined, "kind") {
        "set" | "list" => inline_at(table, &mut inlined, "elem"),
        "fun" => {
            inline_at(table, &mut inlined, "arg");
            inline_at(table, &mut inlined, "res");
        }
        "oper" => {
            for argument in list_mut(&mut inlined, "args") {
                *argument = inline_type(table, argument);
            }
            inline_at(table, &mut inlined, "res");
        }
        "tup" | "rec" | "sum" => {
            if let Some(row) = field_mut(&mut inlined, "fields") {
                inline_row(table, row);
            }
        }
        "app" => {
            for argument in list_mut(&mut inlined, "args") {
                *argument = inline_type(table, argument);
            }
        }
        _ => {}
    }
    inlined
}

fn inline_at(table: &Table, value: &mut Json, key: &str) {
    if let Some(inner) = field_mut(value, key) {
        *inner = inline_type(table, inner);
    }
}

fn inline_row(table: &Table, row: &mut Json) {
    if text(row, "kind") != "row" {
        return;
    }
    for field in list_mut(row, "fields") {
        inline_at(table, field, "fieldType");
    }
    if let Some(other) = field_mut(row, "other") {
        inline_row(table, other);
    }
}

fn inline_definition(table: &Table, definition: &mut Json) {
    if definition.get("typeAnnotation").is_some() {
        inline_at(table, definition, "typeAnnotation");
    } else if text(definition, "kind") == "typedef" {
        inline_at(table, definition, "type");
    }
    match text(definition, "kind") {
        "def" => {
            if let Some(body) = field_mut(definition, "expr") {
                inline_expression(table, body);
            }
        }
        "assume" => {
            if let Some(body) = field_mut(definition, "assumption") {
                inline_expression(table, body);
            }
        }
        _ => {}
    }
}

fn inline_expression(table: &Table, expression: &mut Json) {
    match text(expression, "kind") {
        "app" => {
            for argument in list_mut(expression, "args") {
                inline_expression(table, argument);
            }
        }
        "lambda" => {
            for param in list_mut(expression, "params") {
                inline_at(table, param, "typeAnnotation");
            }
            if let Some(body) = field_mut(expression, "expr") {
                inline_expression(table, body);
            }
        }
        "let" => {
            if let Some(definition) = field_mut(expression, "opdef") {
                inline_definition(table, definition);
            }
            if let Some(body) = field_mut(expression, "expr") {
                inline_expression(table, body);
            }
        }
        _ => {}
    }
}

fn inline_declaration(table: &Table, declaration: &mut Json) {
    if text(declaration, "kind") == "instance" {
        for pair in list_mut(declaration, "overrides") {
            if let Json::List(pair) = pair
                && let Some(value) = pair.get_mut(1)
            {
                inline_expression(table, value);
            }
        }
    } else if is_definition(declaration) {
        inline_definition(table, declaration);
    }
}

fn inline_entry(table: &Table, entry: &Entry) -> Entry {
    if entry.text("kind") == "param" {
        let inlined = entry
            .get("typeAnnotation")
            .map(|written| inline_type(table, written));
        return entry.with("typeAnnotation", inlined);
    }
    let mut inlined = entry.clone();
    if let Some(written) = entry.get("typeAnnotation") {
        inlined.set("typeAnnotation", Some(inline_type(table, written)));
    } else if entry.text("kind") == "typedef"
        && let Some(written) = entry.get("type")
    {
        inlined.set("type", Some(inline_type(table, written)));
    }
    let key = match entry.text("kind") {
        "def" => "expr",
        "assume" => "assumption",
        _ => return inlined,
    };
    if let Some(body) = entry.get(key) {
        let mut body = body.clone();
        inline_expression(table, &mut body);
        inlined.set(key, Some(body));
    }
    inlined
}

fn inline_aliases(modules: &[Json], table: &Table) -> (Vec<Json>, Table) {
    let modules = modules
        .iter()
        .map(|module| {
            let mut module = module.clone();
            for declaration in list_mut(&mut module, "declarations") {
                inline_declaration(table, declaration);
            }
            module
        })
        .collect();
    let entries = table
        .entries
        .iter()
        .map(|(id, entry)| (*id, inline_entry(table, entry)))
        .collect();
    (
        modules,
        Table {
            entries,
            order: table.order.clone(),
        },
    )
}

struct Namespacer<'a> {
    namespace: &'a str,
}

impl Namespacer<'_> {
    fn definition(&self, definition: &mut Json) {
        if definition.get("typeAnnotation").is_some() {
            if let Some(written) = field_mut(definition, "typeAnnotation") {
                self.kind(written);
            }
        } else if text(definition, "kind") == "typedef"
            && let Some(written) = field_mut(definition, "type")
        {
            self.kind(written);
        }
        let key = match text(definition, "kind") {
            "def" => Some("expr"),
            "assume" => Some("assumption"),
            _ => None,
        };
        if let Some(body) = key.and_then(|key| field_mut(definition, key)) {
            self.expression(body);
        }
        let renamed = namespaced_name(self.namespace, text(definition, "name"));
        put(definition, "name", Json::Text(renamed));
    }

    fn preserved(&self, value: &mut Json, key: &str) {
        let name = text(value, key);
        if !built_in(name) {
            let renamed = namespaced_name(self.namespace, name);
            put(value, key, Json::Text(renamed));
        }
    }

    fn expression(&self, expression: &mut Json) {
        match text(expression, "kind") {
            "name" => self.preserved(expression, "name"),
            "app" => {
                for argument in list_mut(expression, "args") {
                    self.expression(argument);
                }
                self.preserved(expression, "opcode");
            }
            "lambda" => {
                for param in list_mut(expression, "params") {
                    if let Some(written) = field_mut(param, "typeAnnotation") {
                        self.kind(written);
                    }
                }
                if let Some(body) = field_mut(expression, "expr") {
                    self.expression(body);
                }
                for param in list_mut(expression, "params") {
                    let renamed = namespaced_name(self.namespace, text(param, "name"));
                    put(param, "name", Json::Text(renamed));
                }
            }
            "let" => {
                if let Some(definition) = field_mut(expression, "opdef") {
                    self.definition(definition);
                }
                if let Some(body) = field_mut(expression, "expr") {
                    self.expression(body);
                }
            }
            _ => {}
        }
    }

    fn kind(&self, written: &mut Json) {
        match text(written, "kind") {
            "const" => self.preserved(written, "name"),
            "set" | "list" => {
                if let Some(inner) = field_mut(written, "elem") {
                    self.kind(inner);
                }
            }
            "fun" => {
                for key in ["arg", "res"] {
                    if let Some(inner) = field_mut(written, key) {
                        self.kind(inner);
                    }
                }
            }
            "oper" | "app" => {
                for argument in list_mut(written, "args") {
                    self.kind(argument);
                }
                if text(written, "kind") == "oper"
                    && let Some(inner) = field_mut(written, "res")
                {
                    self.kind(inner);
                }
            }
            "tup" | "rec" | "sum" => {
                if let Some(row) = field_mut(written, "fields") {
                    self.row(row);
                }
            }
            _ => {}
        }
    }

    fn row(&self, row: &mut Json) {
        if text(row, "kind") != "row" {
            return;
        }
        for field in list_mut(row, "fields") {
            if let Some(inner) = field_mut(field, "fieldType") {
                self.kind(inner);
            }
        }
        if let Some(other) = field_mut(row, "other") {
            self.row(other);
        }
    }
}

fn with_namespace(definition: &Json, namespace: &str) -> Json {
    let mut renamed = definition.clone();
    Namespacer { namespace }.definition(&mut renamed);
    renamed
}

struct Refresher<'a> {
    ids: &'a mut Ids,
    copies: &'a mut Vec<(u64, u64)>,
}

impl Refresher<'_> {
    fn fresh(&mut self, value: &mut Json) {
        let Some(old) = id_of(value) else {
            return;
        };
        let new = match self.ids.place(old).copied() {
            Some(place) => self.ids.placed(place),
            None => self.ids.unplaced(),
        };
        self.copies.push((new, old));
        put(value, "id", Json::number(new));
    }

    fn definition(&mut self, definition: &mut Json) {
        self.fresh(definition);
        if definition.get("typeAnnotation").is_some() {
            if let Some(written) = field_mut(definition, "typeAnnotation") {
                self.kind(written);
            }
        } else if text(definition, "kind") == "typedef"
            && let Some(written) = field_mut(definition, "type")
        {
            self.kind(written);
        }
        let key = match text(definition, "kind") {
            "def" => Some("expr"),
            "assume" => Some("assumption"),
            _ => None,
        };
        if let Some(body) = key.and_then(|key| field_mut(definition, key)) {
            self.expression(body);
        }
    }

    fn expression(&mut self, expression: &mut Json) {
        self.fresh(expression);
        match text(expression, "kind") {
            "app" => {
                for argument in list_mut(expression, "args") {
                    self.expression(argument);
                }
            }
            "lambda" => {
                for param in list_mut(expression, "params") {
                    self.fresh(param);
                }
                for param in list_mut(expression, "params") {
                    if let Some(written) = field_mut(param, "typeAnnotation") {
                        self.kind(written);
                    }
                }
                if let Some(body) = field_mut(expression, "expr") {
                    self.expression(body);
                }
            }
            "let" => {
                if let Some(definition) = field_mut(expression, "opdef") {
                    self.definition(definition);
                }
                if let Some(body) = field_mut(expression, "expr") {
                    self.expression(body);
                }
            }
            _ => {}
        }
    }

    fn kind(&mut self, written: &mut Json) {
        if written.get("id").is_some() {
            self.fresh(written);
        }
        match text(written, "kind") {
            "set" | "list" => {
                if let Some(inner) = field_mut(written, "elem") {
                    self.kind(inner);
                }
            }
            "fun" => {
                for key in ["arg", "res"] {
                    if let Some(inner) = field_mut(written, key) {
                        self.kind(inner);
                    }
                }
            }
            "oper" | "app" => {
                for argument in list_mut(written, "args") {
                    self.kind(argument);
                }
                if text(written, "kind") == "oper"
                    && let Some(inner) = field_mut(written, "res")
                {
                    self.kind(inner);
                }
            }
            "tup" | "rec" | "sum" => {
                if let Some(row) = field_mut(written, "fields") {
                    self.row(row);
                }
            }
            _ => {}
        }
    }

    fn row(&mut self, row: &mut Json) {
        if text(row, "kind") != "row" {
            return;
        }
        for field in list_mut(row, "fields") {
            if let Some(inner) = field_mut(field, "fieldType") {
                self.kind(inner);
            }
        }
        if let Some(other) = field_mut(row, "other") {
            self.row(other);
        }
    }
}

struct Modules {
    aliases: HashMap<String, String>,
    flattened: Vec<Json>,
    waiting: Vec<Json>,
    originals: HashMap<String, Json>,
}

impl Modules {
    fn target(&self, name: &str) -> Option<String> {
        self.aliases.get(name).cloned()
    }

    fn module(&self, name: &str) -> Option<&Json> {
        let target = self.aliases.get(name)?;
        self.flattened
            .iter()
            .rev()
            .chain(self.waiting.iter())
            .find(|module| text(module, "name") == target)
            .or_else(|| self.originals.get(target))
    }

    fn register(&mut self, declaration: &Json) {
        if let Some(Json::Text(qualified)) = declaration.get("qualifiedName")
            && let Some(target) = self.target(text(declaration, "protoName"))
        {
            self.aliases.insert(qualified.clone(), target);
        }
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
enum Unique {
    Name(String),
    Id(Option<u64>),
}

struct Flattener<'a> {
    modules: Option<&'a mut Modules>,
    table: &'a Table,
    added: Vec<Json>,
    names: HashSet<String>,
    nested: Option<String>,
    walked: HashSet<(u64, Option<String>)>,
}

impl<'a> Flattener<'a> {
    fn new(modules: Option<&'a mut Modules>, table: &'a Table) -> Self {
        Self {
            modules,
            table,
            added: Vec::new(),
            names: HashSet::new(),
            nested: None,
            walked: HashSet::new(),
        }
    }

    fn push(&mut self, declaration: Json) {
        if is_definition(&declaration) {
            self.names.insert(text(&declaration, "name").to_owned());
        }
        self.added.push(declaration);
    }

    fn declaration(&mut self, declaration: &Json) -> Result<(), String> {
        match text(declaration, "kind") {
            "def" | "assume" => self.definition(declaration),
            "instance" => {
                if let Some(modules) = self.modules.as_deref_mut() {
                    modules.register(declaration);
                }
                for pair in items(declaration, "overrides") {
                    if let Json::List(pair) = pair
                        && let Some(value) = pair.get(1)
                    {
                        self.expression(value);
                    }
                }
            }
            "import" => {
                if let Some(modules) = self.modules.as_deref_mut() {
                    modules.register(declaration);
                }
            }
            "export" => self.export(declaration)?,
            _ => {}
        }
        self.push(declaration.clone());
        Ok(())
    }

    fn definition(&mut self, definition: &Json) {
        let key = match text(definition, "kind") {
            "def" => "expr",
            "assume" => "assumption",
            _ => return,
        };
        if let Some(body) = definition.get(key) {
            self.expression(body);
        }
    }

    fn expression(&mut self, expression: &Json) {
        match text(expression, "kind") {
            "name" => self.name(id_of(expression)),
            "app" => {
                self.name(id_of(expression));
                for argument in items(expression, "args") {
                    self.expression(argument);
                }
            }
            "lambda" => {
                if let Some(body) = expression.get("expr") {
                    self.expression(body);
                }
            }
            "let" => {
                if let Some(definition) = expression.get("opdef") {
                    self.definition(definition);
                }
                if let Some(body) = expression.get("expr") {
                    self.expression(body);
                }
            }
            _ => {}
        }
    }

    fn name(&mut self, id: Option<u64>) {
        let Some(entry) = id.and_then(|id| self.table.get(id)) else {
            return;
        };
        let kind = entry.text("kind");
        let depth = match entry.get("depth") {
            Some(Json::Number(digits)) => digits.parse::<i64>().unwrap_or(0),
            _ => 0,
        };
        if (kind == "def" && depth > 0) || kind == "param" || imported_from_instance(entry) {
            return;
        }
        let namespace = self.nested.clone().or_else(|| namespace_of(entry));
        let mut definition = entry.to_json();
        if let Some(namespace) = needs(namespace.as_deref(), entry.text("name")) {
            definition = with_namespace(&definition, &namespace);
        }
        self.nested_walk(namespace, entry.id(), &definition);
        if self.names.contains(text(&definition, "name")) {
            return;
        }
        self.push(definition);
    }

    fn nested_walk(&mut self, namespace: Option<String>, id: Option<u64>, definition: &Json) {
        if let Some(id) = id
            && !self.walked.insert((id, namespace.clone()))
        {
            return;
        }
        let before = std::mem::replace(&mut self.nested, namespace);
        self.definition(definition);
        self.nested = before;
    }

    fn export(&mut self, declaration: &Json) -> Result<(), String> {
        let proto = text(declaration, "protoName");
        let Some(module) = self
            .modules
            .as_deref()
            .and_then(|modules| modules.module(proto))
        else {
            return Err(format!(
                "Cannot read properties of undefined (reading 'declarations'): export of {proto}"
            ));
        };
        let ids: HashSet<u64> = items(module, "declarations")
            .iter()
            .filter_map(id_of)
            .collect();
        let table = self.table;
        let qualifier = if declaration.get("defName").is_some() {
            None
        } else {
            match declaration.get("qualifiedName") {
                Some(Json::Text(qualified)) => Some(qualified.clone()),
                _ => Some(proto.to_owned()),
            }
        };
        for entry in table.in_order() {
            if !entry.id().is_some_and(|id| ids.contains(&id)) {
                continue;
            }
            if entry.text("kind") == "param" {
                return Err(
                    "Impossible: intersection of top-level declarations with lookup table entries should never be a param"
                        .to_owned(),
                );
            }
            let namespace = self.nested.clone().or_else(|| qualifier.clone());
            let mut definition = entry.to_json();
            if let Some(namespace) = needs(namespace.as_deref(), entry.text("name")) {
                definition = with_namespace(&definition, &namespace);
            }
            if self.names.contains(text(&definition, "name")) {
                continue;
            }
            let mut bare = definition.clone();
            put(&mut bare, "hidden", Json::Bool(false));
            put(&mut bare, "namespaces", Json::List(Vec::new()));
            remove(&mut bare, "importedFrom");
            self.nested_walk(namespace, entry.id(), &definition);
            self.push(bare);
        }
        Ok(())
    }

    fn finished(self) -> Vec<Json> {
        let mut seen: HashSet<Unique> = HashSet::new();
        self.added
            .into_iter()
            .filter(|declaration| !matches!(text(declaration, "kind"), "import" | "export"))
            .filter(|declaration| {
                let key = if is_definition(declaration) {
                    Unique::Name(text(declaration, "name").to_owned())
                } else {
                    Unique::Id(id_of(declaration))
                };
                seen.insert(key)
            })
            .collect()
    }
}

fn flatten_module(module: &Json, modules: &mut Modules, table: &Table) -> Result<Json, String> {
    let mut flattener = Flattener::new(Some(modules), table);
    for declaration in items(module, "declarations") {
        flattener.declaration(declaration)?;
    }
    let mut flat = module.clone();
    put(&mut flat, "declarations", Json::List(flattener.finished()));
    Ok(flat)
}

fn dependent_definitions(expression: &Json, table: &Table) -> Vec<Json> {
    let mut flattener = Flattener::new(None, table);
    flattener.expression(expression);
    flattener.added
}

struct Instances<'a> {
    modules: &'a Modules,
    table: &'a Table,
    ids: &'a mut Ids,
    copies: &'a mut Vec<(u64, u64)>,
    current: String,
    created: Vec<Json>,
}

impl Instances<'_> {
    fn rename(&self, expression: &mut Json, key: &str) {
        let Some(entry) = id_of(expression).and_then(|id| self.table.get(id)) else {
            return;
        };
        if !imported_from_instance(entry) {
            return;
        }
        let renamed = match namespace_of(entry) {
            Some(namespace) if !namespace.is_empty() => {
                format!("{namespace}::{}", entry.text("name"))
            }
            _ => entry.text("name").to_owned(),
        };
        put(expression, key, Json::Text(renamed));
    }

    fn expression(&self, expression: &mut Json) {
        match text(expression, "kind") {
            "name" => self.rename(expression, "name"),
            "app" => {
                self.rename(expression, "opcode");
                for argument in list_mut(expression, "args") {
                    self.expression(argument);
                }
            }
            "lambda" => {
                if let Some(body) = field_mut(expression, "expr") {
                    self.expression(body);
                }
            }
            "let" => {
                if let Some(definition) = field_mut(expression, "opdef") {
                    self.definition(definition);
                }
                if let Some(body) = field_mut(expression, "expr") {
                    self.expression(body);
                }
            }
            _ => {}
        }
    }

    fn definition(&self, definition: &mut Json) {
        let key = match text(definition, "kind") {
            "def" => "expr",
            "assume" => "assumption",
            _ => return,
        };
        if let Some(body) = field_mut(definition, key) {
            self.expression(body);
        }
    }

    fn declaration(&mut self, declaration: &mut Json) -> Result<(), String> {
        if text(declaration, "kind") != "instance" {
            self.definition(declaration);
            return Ok(());
        }
        for pair in list_mut(declaration, "overrides") {
            if let Json::List(pair) = pair
                && let Some(value) = pair.get_mut(1)
            {
                self.expression(value);
            }
        }
        let proto_name = text(declaration, "protoName");
        let Some(proto) = self.modules.module(proto_name) else {
            return Err(format!(
                "Cannot read properties of undefined (reading 'name'): instance of {proto_name}"
            ));
        };
        let proto = proto.clone();
        let named = match declaration.get("qualifiedName") {
            Some(Json::Text(qualified)) => qualified.clone(),
            _ => text(&proto, "name").to_owned(),
        };
        let new_name = format!("{}::{named}", self.current);
        let mut created = Vec::new();
        for pair in items(declaration, "overrides") {
            let Json::List(pair) = pair else {
                continue;
            };
            let (Some(param), Some(value)) = (pair.first(), pair.get(1)) else {
                continue;
            };
            created.extend(dependent_definitions(value, self.table));
            let mut fields = vec![
                ("kind", Json::text("def")),
                ("qualifier", Json::text("pureval")),
                ("expr", value.clone()),
                ("id", param.get("id").cloned().unwrap_or(Json::Null)),
                (
                    "name",
                    Json::Text(format!("{new_name}::{}", text(param, "name"))),
                ),
            ];
            if let Some(written) = id_of(param)
                .and_then(|id| self.table.get(id))
                .and_then(|entry| entry.get("typeAnnotation"))
            {
                fields.push(("typeAnnotation", written.clone()));
            }
            created.push(Json::object(fields));
        }
        for inner in items(&proto, "declarations") {
            if text(inner, "kind") == "const" {
                continue;
            }
            if !is_definition(inner) {
                created.push(inner.clone());
                continue;
            }
            let mut copy = with_namespace(inner, &new_name);
            Refresher {
                ids: self.ids,
                copies: self.copies,
            }
            .definition(&mut copy);
            created.push(copy);
        }
        let mut module = proto;
        put(&mut module, "declarations", Json::List(created));
        put(&mut module, "name", Json::Text(new_name.clone()));
        self.created.push(module);
        *declaration = Json::object(vec![
            ("kind", Json::text("import")),
            ("id", declaration.get("id").cloned().unwrap_or(Json::Null)),
            ("protoName", Json::Text(new_name)),
            ("defName", Json::text("*")),
        ]);
        Ok(())
    }
}

fn flatten_instances(
    module: &Json,
    modules: &Modules,
    table: &Table,
    ids: &mut Ids,
    copies: &mut Vec<(u64, u64)>,
) -> Result<Vec<Json>, String> {
    let mut module = module.clone();
    let mut instances = Instances {
        modules,
        table,
        ids,
        copies,
        current: text(&module, "name").to_owned(),
        created: Vec::new(),
    };
    for declaration in list_mut(&mut module, "declarations") {
        instances.declaration(declaration)?;
    }
    let mut all = instances.created;
    all.push(module);
    Ok(all)
}

pub(crate) fn error_text(errors: &[Raised]) -> String {
    errors
        .iter()
        .map(|raised| format!("[{}] {}", raised.code, raised.message))
        .collect::<Vec<_>>()
        .join(",")
}

fn resolved(modules: &mut [Json]) -> Result<Table, String> {
    let resolution = resolve(modules);
    if resolution.errors.is_empty() {
        Ok(Table::of(resolution))
    } else {
        Err(format!(
            "Internal error while flattening {}",
            error_text(&resolution.errors)
        ))
    }
}

pub(crate) struct Flat {
    pub(crate) modules: Vec<Json>,
    pub(crate) table: Table,
    pub(crate) copies: Vec<(u64, u64)>,
}

pub(crate) fn flatten_modules(
    modules: &[Json],
    table: &Table,
    ids: &mut Ids,
) -> Result<Flat, String> {
    let mut copies = Vec::new();
    let mut table = table.clone();
    let unique: Vec<Json> = modules
        .iter()
        .map(|module| unshadow(module, &mut table))
        .collect();
    let (inlined, mut flat_table) = inline_aliases(&unique, &table);
    let mut state = Modules {
        aliases: inlined
            .iter()
            .map(|module| {
                let name = text(module, "name").to_owned();
                (name.clone(), name)
            })
            .collect(),
        flattened: Vec::new(),
        waiting: inlined.clone(),
        originals: inlined
            .iter()
            .map(|module| (text(module, "name").to_owned(), module.clone()))
            .collect(),
    };
    while !state.waiting.is_empty() {
        let module = state.waiting.remove(0);
        let first = flatten_module(&module, &mut state, &flat_table)?;
        let instanced = flatten_instances(&first, &state, &flat_table, ids, &mut copies)?;
        let done = state.flattened.len();
        let mut together: Vec<Json> = state.flattened.iter().cloned().chain(instanced).collect();
        let intermediate = resolved(&mut together)?;
        let instanced = together.split_off(done);
        state.flattened = together;
        for each in instanced {
            let flat = flatten_module(&each, &mut state, &intermediate)?;
            let name = text(&flat, "name").to_owned();
            state.flattened.push(flat);
            state.aliases.insert(name.clone(), name);
        }
        let done = state.flattened.len();
        let mut together: Vec<Json> = state
            .flattened
            .drain(..)
            .chain(state.waiting.drain(..))
            .collect();
        flat_table = resolved(&mut together)?;
        state.waiting = together.split_off(done);
        state.flattened = together;
    }
    Ok(Flat {
        modules: state.flattened,
        table: flat_table,
        copies,
    })
}
