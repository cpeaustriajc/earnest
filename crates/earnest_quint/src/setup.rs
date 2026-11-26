use crate::Runs;
use serde_json::{Map, Value, json};
use std::collections::{HashMap, HashSet};

const SEPARATOR: &str = "::";
const WILDCARD: &str = "_";
const TESTS_END_WITH: &str = "Test";
const MOST_ALIASES: usize = 64;
const FIELD_NAMES: &str = "fieldNames";

type Fill<T> = Option<T>;

fn text<'v>(value: &'v Value, key: &str) -> Option<&'v str> {
    value.get(key).and_then(Value::as_str)
}

fn number(value: &Value, key: &str) -> Option<u64> {
    value.get(key).and_then(Value::as_u64)
}

fn list<'v>(value: &'v Value, key: &str) -> &'v [Value] {
    value
        .get(key)
        .and_then(Value::as_array)
        .map_or(&[], Vec::as_slice)
}

fn namespaced(namespace: &str, name: &str) -> String {
    if name == WILDCARD || name.starts_with(&format!("{namespace}{SEPARATOR}")) {
        name.to_owned()
    } else {
        format!("{namespace}{SEPARATOR}{name}")
    }
}

fn most_id(value: &Value) -> u64 {
    match value {
        Value::Object(fields) => fields
            .iter()
            .map(|(key, field)| {
                let own = if key == "id" { field.as_u64() } else { None };
                own.unwrap_or(0).max(most_id(field))
            })
            .max()
            .unwrap_or(0),
        Value::Array(items) => items.iter().map(most_id).max().unwrap_or(0),
        _ => 0,
    }
}

/// What `quint typecheck --out` wrote for a `.qnt` file: every module of it, unflattened.
///
/// [`Typechecked::setup`] builds from it the module `quint compile --main <setup>` would give
/// for a setup of the plain shape `module Small { import Spec(K = e).* }`, so that a file of
/// many setups costs one Quint run.
#[derive(Debug)]
pub struct Typechecked {
    modules: Vec<Value>,
    table: Map<String, Value>,
    types: Map<String, Value>,
    next: u64,
}

/// One check of a setup module, as `earnest.quint.json` lists it.
#[derive(Clone, Copy, Debug)]
pub struct Setup<'c> {
    main: &'c str,
    init: &'c str,
    step: &'c str,
    invariants: &'c [String],
    temporal: &'c [String],
    runs: &'c Runs,
}

impl<'c> Setup<'c> {
    /// The check of the module `main`, with its `init` and `step`, the invariants and temporal
    /// properties it lists, and the runs it plays.
    #[must_use]
    pub const fn new(
        main: &'c str,
        (init, step): (&'c str, &'c str),
        (invariants, temporal): (&'c [String], &'c [String]),
        runs: &'c Runs,
    ) -> Self {
        Self {
            main,
            init,
            step,
            invariants,
            temporal,
            runs,
        }
    }
}

impl Typechecked {
    /// Reads what `quint typecheck --out` wrote. `None` when it is not that, or when Quint
    /// reported errors.
    #[must_use]
    pub fn read(source: &str) -> Option<Self> {
        let value: Value = serde_json::from_str(source).ok()?;
        if !list(&value, "errors").is_empty() || text(&value, "stage") != Some("typechecking") {
            return None;
        }
        let next = most_id(&value).checked_add(1)?;
        let Value::Object(mut fields) = value else {
            return None;
        };
        let Some(Value::Array(modules)) = fields.remove("modules") else {
            return None;
        };
        let Some(Value::Object(table)) = fields.remove("table") else {
            return None;
        };
        let types = match fields.remove("types") {
            Some(Value::Object(types)) => types,
            _ => Map::new(),
        };
        Some(Self {
            modules,
            table,
            types,
            next,
        })
    }

    fn module(&self, name: &str) -> Option<&Value> {
        self.modules
            .iter()
            .find(|module| text(module, "name") == Some(name))
    }

    /// The JSON `quint compile --target json` would write for `check`, with the runs of the
    /// imported module that `check` plays kept, or `None` when the setup is of a shape this
    /// does not build: anything but one `import Spec(...).*` of a module that imports nothing,
    /// with every constant given an expression of builtins only, and definitions of its own.
    #[must_use]
    pub fn setup(&self, check: &Setup<'_>) -> Option<String> {
        let filler = Filler::new(self, check.main)?;
        filler.filled(check)
    }
}

struct Filler<'t> {
    typechecked: &'t Typechecked,
    main: &'t Value,
    namespace: String,
    spec: HashMap<u64, &'t Value>,
    own: HashMap<u64, &'t Value>,
    constants: HashMap<u64, (&'t Value, &'t Value)>,
    typedefs: HashMap<&'t str, &'t Value>,
    renamed: HashMap<u64, String>,
    pushed: Vec<Value>,
    names: HashSet<String>,
    visiting: HashSet<u64>,
    entries: Map<String, Value>,
    types: Map<String, Value>,
    next: u64,
}

fn builtins_only(expr: &Value, table: &Map<String, Value>) -> bool {
    let own = number(expr, "id").is_none_or(|id| !table.contains_key(&id.to_string()));
    own && match expr {
        Value::Object(fields) => fields.values().all(|field| builtins_only(field, table)),
        Value::Array(items) => items.iter().all(|item| builtins_only(item, table)),
        _ => true,
    }
}

impl<'t> Filler<'t> {
    fn new(typechecked: &'t Typechecked, main_name: &str) -> Fill<Self> {
        let main = typechecked.module(main_name)?;
        let mut instance = None;
        let mut own = HashMap::new();
        for declaration in list(main, "declarations") {
            match text(declaration, "kind")? {
                "instance" if instance.is_none() && declaration.get("qualifiedName").is_none() => {
                    instance = Some(declaration);
                }
                "def" => {
                    own.insert(number(declaration, "id")?, declaration);
                }
                _ => return None,
            }
        }
        let instance = instance?;
        let proto = text(instance, "protoName")?;
        let spec_module = typechecked.module(proto)?;
        let mut spec = HashMap::new();
        let mut declared_constants = HashMap::new();
        let mut typedefs = HashMap::new();
        for declaration in list(spec_module, "declarations") {
            let id = number(declaration, "id");
            match text(declaration, "kind")? {
                "var" | "def" => {
                    spec.insert(id?, declaration);
                }
                "const" => {
                    declared_constants.insert(text(declaration, "name")?, (id?, declaration));
                }
                "typedef" => {
                    typedefs.insert(text(declaration, "name")?, declaration);
                }
                _ => return None,
            }
        }
        let mut constants = HashMap::new();
        for pair in list(instance, "overrides") {
            let [parameter, expr] = pair.as_array()?.as_slice() else {
                return None;
            };
            let (id, declaration) = declared_constants.get(text(parameter, "name")?)?;
            if !builtins_only(expr, &typechecked.table)
                || constants.insert(*id, (*declaration, expr)).is_some()
            {
                return None;
            }
        }
        if constants.len() != declared_constants.len() {
            return None;
        }
        Some(Self {
            typechecked,
            main,
            namespace: format!("{main_name}{SEPARATOR}{proto}"),
            spec,
            own,
            constants,
            typedefs,
            renamed: HashMap::new(),
            pushed: Vec::new(),
            names: HashSet::new(),
            visiting: HashSet::new(),
            entries: Map::new(),
            types: Map::new(),
            next: typechecked.next,
        })
    }

    fn entry(&self, id: u64) -> Option<&'t Value> {
        self.typechecked.table.get(&id.to_string())
    }

    fn push(&mut self, declaration: Value) {
        if let Some(name) = text(&declaration, "name")
            && self.names.insert(name.to_owned())
        {
            self.pushed.push(declaration);
        }
    }

    fn inlined(&self, annotation: &Value) -> Fill<Value> {
        let mut annotation = annotation.clone();
        for _ in 0..MOST_ALIASES {
            if text(&annotation, "kind") != Some("const") {
                return Some(annotation);
            }
            let alias = self.typedefs.get(text(&annotation, "name")?)?;
            annotation = alias.get("type")?.clone();
        }
        None
    }

    fn keep_type(&mut self, id: u64) -> Fill<()> {
        let key = id.to_string();
        if let Some(scheme) = self.typechecked.types.get(&key) {
            let mut scheme = scheme.clone();
            scheme["type"] = self.inlined(scheme.get("type")?)?;
            self.types.insert(key, scheme);
        }
        Some(())
    }

    fn reach(&mut self, target: u64) -> Fill<()> {
        if let Some((declaration, expr)) = self.constants.get(&target).copied() {
            let name = namespaced(&self.namespace, text(declaration, "name")?);
            let mut constant = json!({
                "kind": "def",
                "qualifier": "pureval",
                "expr": expr,
                "id": target,
                "name": name,
                "depth": 0,
            });
            if let Some(annotation) = declaration.get("typeAnnotation") {
                constant["typeAnnotation"] = annotation.clone();
            }
            self.push(constant);
            return Some(());
        }
        let (declaration, within) = match (self.spec.get(&target), self.own.get(&target)) {
            (Some(found), _) => (*found, Some(self.namespace.clone())),
            (None, Some(found)) => (*found, None),
            (None, None) => return None,
        };
        let name = text(declaration, "name")?;
        let shown = within
            .as_deref()
            .map_or_else(|| name.to_owned(), |space| namespaced(space, name));
        if self.names.contains(&shown) || !self.visiting.insert(target) {
            return Some(());
        }
        let mut copy = declaration.clone();
        copy["name"] = Value::String(shown);
        if text(declaration, "kind") == Some("var") {
            copy["typeAnnotation"] = self.inlined(declaration.get("typeAnnotation")?)?;
            self.keep_type(target)?;
        } else {
            copy["expr"] = self.expr(declaration.get("expr")?, within.as_deref())?;
        }
        if let Value::Object(fields) = &mut copy {
            fields.remove("importedFrom");
            fields.remove("hidden");
        }
        self.push(copy);
        Some(())
    }

    fn used(&mut self, id: u64, written: &str, within: Option<&str>) -> Fill<String> {
        let Some(entry) = self.entry(id) else {
            return Some(written.to_owned());
        };
        let target = number(entry, "id")?;
        let top = number(entry, "depth").unwrap_or(0) == 0 && text(entry, "kind") != Some("param");
        if top {
            self.reach(target)?;
        }
        let base = self
            .renamed
            .get(&target)
            .cloned()
            .unwrap_or_else(|| written.to_owned());
        let from_spec = self.spec.contains_key(&target) || self.constants.contains_key(&target);
        Some(match within {
            Some(space) => namespaced(space, &base),
            None if top && from_spec => namespaced(&self.namespace, text(entry, "name")?),
            None => base,
        })
    }

    fn expr(&mut self, expr: &Value, within: Option<&str>) -> Fill<Value> {
        let mut copy = expr.clone();
        match text(expr, "kind")? {
            "name" => {
                let id = number(expr, "id")?;
                copy["name"] = Value::String(self.used(id, text(expr, "name")?, within)?);
            }
            "app" => {
                let id = number(expr, "id")?;
                if text(expr, "opcode") == Some(FIELD_NAMES) {
                    for arg in list(expr, "args") {
                        if let Some(arg) = number(arg, "id") {
                            self.keep_type(arg)?;
                        }
                    }
                }
                copy["opcode"] = Value::String(self.used(id, text(expr, "opcode")?, within)?);
                copy["args"] = Value::Array(
                    list(expr, "args")
                        .iter()
                        .map(|arg| self.expr(arg, within))
                        .collect::<Fill<Vec<Value>>>()?,
                );
            }
            "lambda" => {
                let lambda = number(expr, "id")?;
                let mut params = Vec::new();
                for param in list(expr, "params") {
                    let unshadowed = format!("{}_{lambda}", text(param, "name")?);
                    self.renamed
                        .insert(number(param, "id")?, unshadowed.clone());
                    let mut renamed = param.clone();
                    renamed["name"] = Value::String(within.map_or_else(
                        || unshadowed.clone(),
                        |space| namespaced(space, &unshadowed),
                    ));
                    params.push(renamed);
                }
                copy["params"] = Value::Array(params);
                copy["expr"] = self.expr(expr.get("expr")?, within)?;
            }
            "let" => {
                let opdef = expr.get("opdef")?;
                let def = number(opdef, "id")?;
                let shadowing = self
                    .entry(def)
                    .and_then(|entry| entry.get("shadowing"))
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                let written = text(opdef, "name")?;
                let unshadowed = if shadowing {
                    format!("{written}_{}", number(expr, "id")?)
                } else {
                    written.to_owned()
                };
                if shadowing {
                    self.renamed.insert(def, unshadowed.clone());
                }
                let mut renamed = opdef.clone();
                renamed["name"] = Value::String(within.map_or_else(
                    || unshadowed.clone(),
                    |space| namespaced(space, &unshadowed),
                ));
                renamed["expr"] = self.expr(opdef.get("expr")?, within)?;
                copy["opdef"] = renamed;
                copy["expr"] = self.expr(expr.get("expr")?, within)?;
            }
            _ => {}
        }
        Some(copy)
    }

    fn fresh(&mut self) -> u64 {
        let id = self.next;
        self.next = self.next.saturating_add(1);
        id
    }

    fn named(&self, name: &str, qualifiers: &[&str]) -> Fill<(u64, String, &'static str)> {
        let fits = |declaration: &Value| {
            text(declaration, "name") == Some(name)
                && match text(declaration, "kind") {
                    Some("var") => qualifiers.contains(&"var"),
                    Some("def") => text(declaration, "qualifier")
                        .is_some_and(|qualifier| qualifiers.contains(&qualifier)),
                    _ => false,
                }
        };
        let own: Vec<&Value> = self
            .own
            .values()
            .copied()
            .filter(|each| fits(each))
            .collect();
        let spec: Vec<&Value> = self
            .spec
            .values()
            .copied()
            .filter(|each| fits(each))
            .collect();
        let lambda = |declaration: &Value| {
            declaration
                .get("expr")
                .is_some_and(|expr| text(expr, "kind") == Some("lambda"))
        };
        match (own.as_slice(), spec.as_slice()) {
            ([found], []) if !lambda(found) => {
                Some((number(found, "id")?, name.to_owned(), kind_of(found)))
            }
            ([], [found]) if !lambda(found) => Some((
                number(found, "id")?,
                namespaced(&self.namespace, name),
                kind_of(found),
            )),
            _ => None,
        }
    }

    fn special(
        &mut self,
        (name, qualifier): (&str, &str),
        listed: &[(u64, String, &'static str)],
    ) -> Fill<()> {
        let mut args = Vec::with_capacity(listed.len());
        for (target, shown, kind) in listed {
            self.reach(*target)?;
            let id = self.fresh();
            self.entries
                .insert(id.to_string(), json!({ "kind": kind, "id": target }));
            args.push(json!({ "id": id, "kind": "name", "name": shown }));
        }
        let expr = if qualifier == "action" {
            args.pop()?
        } else {
            json!({ "id": self.fresh(), "kind": "app", "opcode": "and", "args": args })
        };
        let id = self.fresh();
        self.push(
            json!({ "id": id, "kind": "def", "name": name, "qualifier": qualifier, "expr": expr }),
        );
        Some(())
    }

    fn played(&self, runs: &Runs) -> Vec<u64> {
        let mut found: Vec<(u64, &str)> = self
            .spec
            .iter()
            .filter(|(_, declaration)| text(declaration, "qualifier") == Some("run"))
            .filter_map(|(id, declaration)| Some((*id, text(declaration, "name")?)))
            .filter(|(_, name)| match runs {
                Runs::None => false,
                Runs::Tests => name.ends_with(TESTS_END_WITH),
                Runs::Named(names) => names.iter().any(|each| each == name),
            })
            .collect();
        found.sort_unstable();
        found.into_iter().map(|(id, _)| id).collect()
    }

    fn filled(mut self, check: &Setup<'_>) -> Fill<String> {
        let actions = ["action"];
        let init = self.named(check.init, &actions)?;
        let step = self.named(check.step, &actions)?;
        let invariants = check
            .invariants
            .iter()
            .map(|name| self.named(name, &["val", "pureval", "var"]))
            .collect::<Fill<Vec<_>>>()?;
        let temporal = check
            .temporal
            .iter()
            .map(|name| self.named(name, &["temporal"]))
            .collect::<Fill<Vec<_>>>()?;
        let mut own: Vec<u64> = list(self.main, "declarations")
            .iter()
            .filter(|declaration| text(declaration, "kind") == Some("def"))
            .filter_map(|declaration| number(declaration, "id"))
            .collect();
        own.retain(|id| self.own.contains_key(id));
        for id in own {
            self.reach(id)?;
        }
        self.special(("q::init", "action"), &[init])?;
        self.special(("q::step", "action"), &[step])?;
        if !invariants.is_empty() {
            self.special(("q::inv", "val"), &invariants)?;
        }
        if !temporal.is_empty() {
            self.special(("q::temporalProps", "temporal"), &temporal)?;
        }
        for run in self.played(check.runs) {
            self.reach(run)?;
        }
        let mut table = Map::new();
        for (key, entry) in &self.typechecked.table {
            let target = number(entry, "id")?;
            let kind = if self.constants.contains_key(&target) {
                "def"
            } else {
                text(entry, "kind")?
            };
            table.insert(key.clone(), json!({ "kind": kind, "id": target }));
        }
        table.extend(std::mem::take(&mut self.entries));
        let compiled = json!({
            "stage": "compiling",
            "warnings": [],
            "errors": [],
            "main": check.main,
            "modules": [{
                "id": self.main.get("id"),
                "name": check.main,
                "declarations": self.pushed,
            }],
            "table": table,
            "types": self.types,
        });
        serde_json::to_string(&compiled).ok()
    }
}

fn kind_of(declaration: &Value) -> &'static str {
    if text(declaration, "kind") == Some("var") {
        "var"
    } else {
        "def"
    }
}
