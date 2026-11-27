use crate::json::Json;
use crate::names::{Entry, Raised, number};
use std::collections::{BTreeMap, HashMap, HashSet};

const LARGEST_KEPT_IN_ORDER: usize = 8;

fn chunks(id: u64) -> [u32; 7] {
    let mut hash: i32 = 0;
    for digit in id.to_string().bytes() {
        hash = hash.wrapping_mul(31).wrapping_add(i32::from(digit));
    }
    let bits = u32::from_ne_bytes(hash.to_ne_bytes());
    let small = ((bits >> 1) & 0x4000_0000) | (bits & 0xbfff_ffff);
    let mut chunks = [0; 7];
    for (index, chunk) in chunks.iter_mut().enumerate() {
        *chunk = (small >> (5 * index)) & 31;
    }
    chunks
}

fn settle(ids: &mut [u64]) {
    if ids.len() > LARGEST_KEPT_IN_ORDER {
        ids.sort_by_key(|id| chunks(*id));
    }
}

#[derive(Clone, Debug, Default)]
struct Ids {
    order: Vec<u64>,
    known: HashSet<u64>,
}

impl Ids {
    fn of(ids: impl IntoIterator<Item = u64>) -> Self {
        let mut built = Self::default();
        for id in ids {
            if built.known.insert(id) {
                built.order.push(id);
            }
        }
        settle(&mut built.order);
        built
    }

    fn add(&mut self, id: u64) {
        if self.known.insert(id) {
            self.order.push(id);
            settle(&mut self.order);
        }
    }

    fn union(&self, other: &Self) -> Self {
        if other.order.is_empty() {
            return self.clone();
        }
        if self.order.is_empty() {
            return other.clone();
        }
        Self::of(self.order.iter().chain(&other.order).copied())
    }

    fn is_empty(&self) -> bool {
        self.order.is_empty()
    }
}

#[derive(Debug, Default)]
struct Graph {
    order: Vec<u64>,
    uses: HashMap<u64, Ids>,
}

impl Graph {
    fn set(&mut self, id: u64, uses: Ids) {
        if self.uses.insert(id, uses).is_none() {
            self.order.push(id);
            settle(&mut self.order);
        }
    }

    fn add_one(&mut self, id: u64, callee: u64) {
        let mut callees = self.uses.get(&id).cloned().unwrap_or_default();
        callees.add(callee);
        self.set(id, callees);
    }

    fn add_all(&mut self, id: u64, added: &Ids) {
        let callees = self.uses.get(&id).cloned().unwrap_or_default().union(added);
        self.set(id, callees);
    }
}

struct Context {
    defined_at: HashMap<u64, usize>,
    imports: HashMap<(usize, String), Ids>,
    by_name: HashMap<String, usize>,
    ids: Vec<u64>,
    names: Vec<String>,
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

fn context(modules: &[Json]) -> Context {
    let mut found = Context {
        defined_at: HashMap::new(),
        imports: HashMap::new(),
        by_name: HashMap::new(),
        ids: Vec::new(),
        names: Vec::new(),
    };
    for (index, module) in modules.iter().enumerate() {
        found.ids.push(number(module.get("id")).unwrap_or(0));
        found.names.push(text(module, "name").to_owned());
        found.by_name.insert(text(module, "name").to_owned(), index);
        for declaration in items(module, "declarations") {
            let Some(id) = number(declaration.get("id")) else {
                continue;
            };
            found.defined_at.insert(id, index);
            if matches!(text(declaration, "kind"), "import" | "instance") {
                let mut keys = vec![text(declaration, "protoName").to_owned()];
                if let Some(Json::Text(qualified)) = declaration.get("qualifiedName") {
                    keys.push(qualified.clone());
                }
                for key in keys {
                    found.imports.entry((index, key)).or_default().add(id);
                }
            }
        }
    }
    found
}

struct Walk<'a> {
    table: &'a BTreeMap<u64, Entry>,
    context: &'a Context,
    module: usize,
    stack: Vec<u64>,
    graph: Graph,
}

impl Walk<'_> {
    fn imports_of(&self, module: usize) -> Option<&Ids> {
        self.context
            .imports
            .get(&(self.module, self.context.names[module].clone()))
    }

    fn declaration(&mut self, declaration: &Json) {
        let id = number(declaration.get("id")).unwrap_or(0);
        match text(declaration, "kind") {
            "const" | "var" | "typedef" | "assume" | "def" => self.definition(declaration),
            "import" => {
                if let Some(module) = self.context.by_name.get(text(declaration, "protoName")) {
                    self.graph
                        .add_all(id, &Ids::of([self.context.ids[*module]]));
                }
            }
            "instance" => {
                self.stack.push(id);
                if let Some(module) = self.context.by_name.get(text(declaration, "protoName")) {
                    self.graph
                        .add_all(id, &Ids::of([self.context.ids[*module]]));
                }
                for pair in items(declaration, "overrides") {
                    if let Json::List(pair) = pair
                        && let Some(value) = pair.get(1)
                    {
                        self.expression(value);
                    }
                }
                self.stack.pop();
            }
            "export" => {
                let imports = self
                    .context
                    .imports
                    .get(&(self.module, text(declaration, "protoName").to_owned()))
                    .cloned()
                    .unwrap_or_default();
                self.graph.add_all(id, &imports);
            }
            _ => {}
        }
    }

    fn definition(&mut self, definition: &Json) {
        let id = number(definition.get("id")).unwrap_or(0);
        self.stack.push(id);
        if let Some(&host) = self.context.defined_at.get(&id)
            && host != self.module
        {
            let imports = self.imports_of(host).cloned().unwrap_or_default();
            self.graph.add_all(id, &imports);
        }
        if let Some(annotation) = definition.get("typeAnnotation") {
            self.kind_of(annotation);
        } else if text(definition, "kind") == "typedef"
            && let Some(written) = definition.get("type")
        {
            self.kind_of(written);
        }
        match text(definition, "kind") {
            "def" => {
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
        self.stack.pop();
    }

    fn used(&mut self, id: Option<u64>) {
        let Some(entry) = id.and_then(|id| self.table.get(&id)) else {
            return;
        };
        let Some(origin) = entry.id() else {
            return;
        };
        for &caller in &self.stack.clone() {
            self.graph.add_one(caller, origin);
        }
        if let Some(&host) = self.context.defined_at.get(&origin)
            && host != self.module
            && let Some(imports) = self.imports_of(host).cloned()
        {
            for &caller in &self.stack.clone() {
                self.graph.add_all(caller, &imports);
            }
        }
    }

    fn expression(&mut self, expression: &Json) {
        match text(expression, "kind") {
            "name" => self.used(number(expression.get("id"))),
            "app" => {
                for argument in items(expression, "args") {
                    self.expression(argument);
                }
                self.used(number(expression.get("id")));
            }
            "lambda" => {
                for param in items(expression, "params") {
                    if let Some(annotation) = param.get("typeAnnotation") {
                        self.kind_of(annotation);
                    }
                }
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

    fn kind_of(&mut self, written: &Json) {
        match text(written, "kind") {
            "const" => {
                let id = number(written.get("id")).filter(|id| *id != 0);
                self.used(id);
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

fn sorted(graph: Graph, declarations: &[Json]) -> (Vec<u64>, Vec<u64>) {
    let unsorted = Ids::of(
        declarations
            .iter()
            .filter_map(|declaration| number(declaration.get("id"))),
    );
    let Graph {
        order: mut keys,
        mut uses,
    } = graph;
    let mut every = unsorted.clone();
    for key in &keys {
        every = uses[key].union(&every);
    }
    for id in &every.order {
        if !uses.contains_key(id) {
            uses.insert(*id, Ids::default());
            keys.push(*id);
            settle(&mut keys);
        }
    }
    let mut done = Vec::new();
    loop {
        let (sinks, rest): (Vec<u64>, Vec<u64>) = keys.iter().partition(|id| uses[*id].is_empty());
        keys = rest;
        settle(&mut keys);
        if sinks.is_empty() {
            break;
        }
        let mut layer: Vec<u64> = sinks
            .iter()
            .copied()
            .filter(|id| unsorted.known.contains(id))
            .collect();
        layer.sort_by_key(u64::to_string);
        done.extend(layer);
        let gone: HashSet<u64> = sinks.iter().copied().collect();
        for sink in &sinks {
            uses.remove(sink);
        }
        for key in &keys {
            if let Some(used) = uses.get_mut(key) {
                used.order.retain(|id| !gone.contains(id));
                used.known.retain(|id| !gone.contains(id));
            }
        }
    }
    let mut cycles = keys;
    settle(&mut cycles);
    (done, cycles)
}

pub(crate) fn sort(modules: &mut [Json], table: &BTreeMap<u64, Entry>, errors: &mut Vec<Raised>) {
    let context = context(modules);
    for (index, module) in modules.iter_mut().enumerate() {
        let mut walk = Walk {
            table,
            context: &context,
            module: index,
            stack: Vec::new(),
            graph: Graph::default(),
        };
        let declarations = items(module, "declarations");
        for declaration in declarations {
            walk.declaration(declaration);
        }
        let (done, cycles) = sorted(walk.graph, declarations);
        for id in cycles {
            errors.push(Raised {
                code: "QNT099",
                message: "Found cyclic declarations. Use fold and foldl instead of recursion"
                    .to_owned(),
                reference: Some(id),
            });
        }
        let mut by_id: HashMap<u64, Json> = declarations
            .iter()
            .filter_map(|declaration| {
                number(declaration.get("id")).map(|id| (id, declaration.clone()))
            })
            .collect();
        let reordered: Vec<Json> = done.iter().filter_map(|id| by_id.remove(id)).collect();
        if let Json::Object(fields) = module
            && let Some((_, value)) = fields.iter_mut().find(|(key, _)| key == "declarations")
        {
            *value = Json::List(reordered);
        }
    }
}
