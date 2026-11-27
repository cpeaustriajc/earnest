use crate::json::Json;
use crate::names::{Entry, Raised, number};
use std::collections::hash_map::Entry as Slot;
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
    unsettled: bool,
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
            self.unsettled |= self.order.len() > LARGEST_KEPT_IN_ORDER;
        }
    }

    fn settled(&mut self) -> &[u64] {
        if self.unsettled {
            settle(&mut self.order);
            self.unsettled = false;
        }
        &self.order
    }

    fn add_all(&mut self, other: &mut Self) {
        for id in other.settled() {
            self.add(*id);
        }
    }
}

#[derive(Debug, Default)]
struct Graph {
    order: Ids,
    uses: HashMap<u64, Ids>,
}

impl Graph {
    fn set(&mut self, id: u64, uses: Ids) {
        if self.uses.insert(id, uses).is_none() {
            self.order.add(id);
        }
    }

    fn callees(&mut self, id: u64) -> &mut Ids {
        if !self.uses.contains_key(&id) {
            self.set(id, Ids::default());
        }
        self.uses.entry(id).or_default()
    }

    fn add_one(&mut self, id: u64, callee: u64) {
        self.callees(id).add(callee);
    }

    fn add_all(&mut self, id: u64, added: &Ids) {
        let mut added = added.clone();
        self.callees(id).add_all(&mut added);
    }
}

struct Buckets {
    by_hash: BTreeMap<[u32; 7], Vec<u64>>,
    known: HashSet<u64>,
}

impl Buckets {
    fn of(settled: &[u64]) -> Self {
        let mut by_hash: BTreeMap<[u32; 7], Vec<u64>> = BTreeMap::new();
        for id in settled {
            by_hash.entry(chunks(*id)).or_default().push(*id);
        }
        Self {
            by_hash,
            known: settled.iter().copied().collect(),
        }
    }

    fn put_first(&mut self, settled: &[u64]) {
        let mut touched: BTreeMap<[u32; 7], Vec<u64>> = BTreeMap::new();
        for id in settled {
            touched.entry(chunks(*id)).or_default().push(*id);
        }
        for (hash, first) in touched {
            let firsts: HashSet<u64> = first.iter().copied().collect();
            let bucket = self.by_hash.entry(hash).or_default();
            let rest: Vec<u64> = bucket
                .iter()
                .copied()
                .filter(|id| !firsts.contains(id))
                .collect();
            *bucket = first;
            bucket.extend(rest);
        }
        self.known.extend(settled.iter().copied());
    }

    fn order(&self) -> Vec<u64> {
        self.by_hash.values().flatten().copied().collect()
    }
}

fn every_used(keys: &[u64], uses: &mut HashMap<u64, Ids>, unsorted: &Ids) -> Vec<u64> {
    let mut small = unsorted.clone();
    let mut keys = keys.iter();
    loop {
        if small.order.len() > LARGEST_KEPT_IN_ORDER {
            break;
        }
        let Some(key) = keys.next() else {
            return small.settled().to_vec();
        };
        let first = uses
            .get_mut(key)
            .map_or_else(Vec::new, |found| found.settled().to_vec());
        small = Ids::of(first.into_iter().chain(small.settled().iter().copied()));
    }
    let mut large = Buckets::of(small.settled());
    for key in keys {
        if let Some(used) = uses.get_mut(key) {
            large.put_first(used.settled());
        }
    }
    large.order()
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
    let every = every_used(keys.settled(), &mut uses, &unsorted);
    for id in every {
        if let Slot::Vacant(slot) = uses.entry(id) {
            slot.insert(Ids::default());
            keys.add(id);
        }
    }
    let keys = keys.settled().to_vec();
    let mut waiting: HashMap<u64, usize> = HashMap::new();
    let mut users: HashMap<u64, Vec<u64>> = HashMap::new();
    for key in &keys {
        let used = &uses[key];
        waiting.insert(*key, used.order.len());
        for id in &used.order {
            users.entry(*id).or_default().push(*key);
        }
    }
    let mut ready: Vec<u64> = keys
        .iter()
        .copied()
        .filter(|key| waiting[key] == 0)
        .collect();
    let mut alive: HashSet<u64> = keys.iter().copied().collect();
    let mut done = Vec::new();
    while !ready.is_empty() {
        let mut layer: Vec<u64> = ready
            .iter()
            .copied()
            .filter(|id| unsorted.known.contains(id))
            .collect();
        layer.sort_by_key(u64::to_string);
        done.extend(layer);
        let mut next = Vec::new();
        for sink in &ready {
            alive.remove(sink);
            for user in users.get(sink).map_or(&[][..], Vec::as_slice) {
                if let Some(count) = waiting.get_mut(user) {
                    *count -= 1;
                    if *count == 0 {
                        next.push(*user);
                    }
                }
            }
        }
        ready = next;
    }
    let cycles = keys.into_iter().filter(|key| alive.contains(key)).collect();
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
