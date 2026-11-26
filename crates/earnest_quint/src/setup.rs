use crate::Runs;
use serde_json::{Map, Value, json};
use std::collections::{HashMap, HashSet};

const SEPARATOR: &str = "::";
const WILDCARD: &str = "_";
const TESTS_END_WITH: &str = "Test";
const MOST_ALIASES: usize = 64;

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
/// for a setup such as `module Small { import Spec(K = e).* }`, so that a file of many setups
/// costs one Quint run.
#[derive(Debug)]
pub struct Typechecked {
    modules: Vec<Value>,
    table: Map<String, Value>,
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
        Some(Self {
            modules,
            table,
            next,
        })
    }

    fn module(&self, name: &str) -> Option<&Value> {
        self.modules
            .iter()
            .find(|module| text(module, "name") == Some(name))
    }

    /// The JSON `quint compile --target json` would write for `check`, with the runs of the
    /// instantiated modules that `check` plays kept, or `None` when the setup is of a shape this
    /// does not build. It builds a module made of instances (`import Spec(K = e).*` or
    /// `import Spec(K = e) as C`) of modules that import nothing, `import Lib.*` of modules of
    /// definitions only, `export C.*` of its own instances, and definitions of its own.
    #[must_use]
    pub fn setup(&self, check: &Setup<'_>) -> Option<String> {
        let filler = Filler::new(self, check.main)?;
        filler.filled(check)
    }
}

struct Instance<'t> {
    alias: Option<&'t str>,
    namespace: String,
    visible: bool,
    declarations: HashMap<u64, &'t Value>,
    constants: HashMap<u64, (&'t Value, &'t Value)>,
    typedefs: HashMap<&'t str, &'t Value>,
    fresh_ids: bool,
}

type Target = (Option<usize>, u64, String, &'static str);

struct Filler<'t> {
    typechecked: &'t Typechecked,
    main: &'t Value,
    instances: Vec<Instance<'t>>,
    by_declaration: HashMap<u64, usize>,
    by_instance: HashMap<u64, usize>,
    own: HashMap<u64, &'t Value>,
    renamed: HashMap<(Option<usize>, u64), String>,
    refreshed: HashMap<(usize, u64), u64>,
    pushed: Vec<Value>,
    names: HashSet<String>,
    visiting: HashSet<(Option<usize>, u64)>,
    entries: Map<String, Value>,
    next: u64,
}

fn plain_import<'t>(typechecked: &'t Typechecked, declaration: &Value) -> Fill<Vec<&'t Value>> {
    if text(declaration, "defName") != Some("*") || declaration.get("qualifiedName").is_some() {
        return None;
    }
    let module = typechecked.module(text(declaration, "protoName")?)?;
    let mut definitions = Vec::new();
    for each in list(module, "declarations") {
        match text(each, "kind")? {
            "def" => definitions.push(each),
            "typedef" => {}
            _ => return None,
        }
    }
    Some(definitions)
}

fn instance<'t>(
    typechecked: &'t Typechecked,
    main_name: &str,
    declaration: &'t Value,
) -> Fill<Instance<'t>> {
    let alias = text(declaration, "qualifiedName");
    let proto = text(declaration, "protoName")?;
    let spec_module = typechecked.module(proto)?;
    let mut declarations = HashMap::new();
    let mut declared_constants = HashMap::new();
    let mut typedefs = HashMap::new();
    for each in list(spec_module, "declarations") {
        let id = number(each, "id");
        match text(each, "kind")? {
            "var" | "def" => {
                declarations.insert(id?, each);
            }
            "const" => {
                declared_constants.insert(text(each, "name")?, (id?, each));
            }
            "typedef" => {
                typedefs.insert(text(each, "name")?, each);
            }
            _ => return None,
        }
    }
    let mut constants = HashMap::new();
    for pair in list(declaration, "overrides") {
        let [parameter, expr] = pair.as_array()?.as_slice() else {
            return None;
        };
        let (id, constant) = declared_constants.get(text(parameter, "name")?)?;
        if constants.insert(*id, (*constant, expr)).is_some() {
            return None;
        }
    }
    if constants.len() != declared_constants.len() {
        return None;
    }
    Some(Instance {
        alias,
        namespace: format!("{main_name}{SEPARATOR}{}", alias.unwrap_or(proto)),
        visible: alias.is_none(),
        declarations,
        constants,
        typedefs,
        fresh_ids: false,
    })
}

impl<'t> Filler<'t> {
    fn new(typechecked: &'t Typechecked, main_name: &str) -> Fill<Self> {
        let main = typechecked.module(main_name)?;
        let mut instances: Vec<Instance<'t>> = Vec::new();
        let mut own = HashMap::new();
        let mut exports = Vec::new();
        let mut protos: HashMap<&str, usize> = HashMap::new();
        for declaration in list(main, "declarations") {
            match text(declaration, "kind")? {
                "instance" => {
                    *protos.entry(text(declaration, "protoName")?).or_insert(0) += 1;
                    instances.push(instance(typechecked, main_name, declaration)?);
                }
                "import" => {
                    for definition in plain_import(typechecked, declaration)? {
                        own.insert(number(definition, "id")?, definition);
                    }
                }
                "export" => exports.push(declaration),
                "def" => {
                    own.insert(number(declaration, "id")?, declaration);
                }
                "typedef" => {}
                _ => return None,
            }
        }
        if instances.is_empty() {
            return None;
        }
        for export in exports {
            let alias = text(export, "protoName")?;
            let exported = instances
                .iter_mut()
                .find(|each| each.alias == Some(alias))
                .filter(|_| {
                    text(export, "defName") == Some("*") && export.get("qualifiedName").is_none()
                })?;
            exported.visible = true;
        }
        let mut by_declaration = HashMap::new();
        let mut by_instance = HashMap::new();
        let instance_declarations: Vec<&Value> = list(main, "declarations")
            .iter()
            .filter(|each| text(each, "kind") == Some("instance"))
            .collect();
        for (position, (each, declaration)) in
            instances.iter_mut().zip(instance_declarations).enumerate()
        {
            let proto = text(declaration, "protoName")?;
            each.fresh_ids = protos.get(proto).copied().unwrap_or(0) > 1;
            by_instance.insert(number(declaration, "id")?, position);
            if !each.fresh_ids {
                for id in each.declarations.keys().chain(each.constants.keys()) {
                    by_declaration.insert(*id, position);
                }
            }
        }
        Some(Self {
            typechecked,
            main,
            instances,
            by_declaration,
            by_instance,
            own,
            renamed: HashMap::new(),
            refreshed: HashMap::new(),
            pushed: Vec::new(),
            names: HashSet::new(),
            visiting: HashSet::new(),
            entries: Map::new(),
            next: typechecked.next,
        })
    }

    fn entry(&self, id: u64) -> Option<&'t Value> {
        self.typechecked.table.get(&id.to_string())
    }

    fn fresh(&mut self) -> u64 {
        let id = self.next;
        self.next = self.next.saturating_add(1);
        id
    }

    fn id_in(&mut self, within: Option<usize>, id: u64) -> u64 {
        let Some(position) = within.filter(|position| {
            self.instances
                .get(*position)
                .is_some_and(|each| each.fresh_ids)
        }) else {
            return id;
        };
        if let Some(found) = self.refreshed.get(&(position, id)) {
            return *found;
        }
        let new = self.fresh();
        self.refreshed.insert((position, id), new);
        new
    }

    fn push(&mut self, declaration: Value) {
        if let Some(name) = text(&declaration, "name")
            && self.names.insert(name.to_owned())
        {
            self.pushed.push(declaration);
        }
    }

    fn namespace_of(&self, within: Option<usize>) -> Option<String> {
        within
            .and_then(|position| self.instances.get(position))
            .map(|each| each.namespace.clone())
    }

    fn inlined(&self, within: usize, annotation: &Value) -> Fill<Value> {
        let typedefs = &self.instances.get(within)?.typedefs;
        let mut annotation = annotation.clone();
        for _ in 0..MOST_ALIASES {
            if text(&annotation, "kind") != Some("const") {
                return Some(annotation);
            }
            let alias = typedefs.get(text(&annotation, "name")?)?;
            annotation = alias.get("type")?.clone();
        }
        None
    }

    fn reach(&mut self, target: u64, within: Option<usize>) -> Fill<()> {
        if !self.visiting.insert((within, target)) {
            return Some(());
        }
        if let Some(position) = within
            && let Some((declaration, expr)) = self
                .instances
                .get(position)?
                .constants
                .get(&target)
                .copied()
        {
            let space = self.namespace_of(within)?;
            let id = self.id_in(within, target);
            let expr = self.expr(expr, None)?;
            let mut constant = json!({
                "kind": "def",
                "qualifier": "pureval",
                "expr": expr,
                "id": id,
                "name": namespaced(&space, text(declaration, "name")?),
                "depth": 0,
            });
            if let Some(annotation) = declaration.get("typeAnnotation") {
                constant["typeAnnotation"] = annotation.clone();
            }
            self.push(constant);
            return Some(());
        }
        let declaration = match within {
            Some(position) => *self.instances.get(position)?.declarations.get(&target)?,
            None => *self.own.get(&target)?,
        };
        let name = text(declaration, "name")?;
        let shown = self
            .namespace_of(within)
            .map_or_else(|| name.to_owned(), |space| namespaced(&space, name));
        if self.names.contains(&shown) {
            return Some(());
        }
        let mut copy = declaration.clone();
        copy["name"] = Value::String(shown);
        copy["id"] = json!(self.id_in(within, target));
        if text(declaration, "kind") == Some("var") {
            copy["typeAnnotation"] = self.inlined(within?, declaration.get("typeAnnotation")?)?;
        } else {
            copy["expr"] = self.expr(declaration.get("expr")?, within)?;
        }
        if let Value::Object(fields) = &mut copy {
            fields.remove("importedFrom");
            fields.remove("hidden");
        }
        self.push(copy);
        Some(())
    }

    fn owner(&self, entry: &Value, target: u64, within: Option<usize>) -> Option<usize> {
        if let Some(position) = within {
            return self
                .instances
                .get(position)
                .filter(|each| {
                    each.declarations.contains_key(&target) || each.constants.contains_key(&target)
                })
                .map(|_| position);
        }
        if self.own.contains_key(&target) {
            return None;
        }
        entry
            .get("importedFrom")
            .filter(|imported| text(imported, "kind") == Some("instance"))
            .and_then(|imported| number(imported, "id"))
            .and_then(|instance| self.by_instance.get(&instance).copied())
            .or_else(|| self.by_declaration.get(&target).copied())
    }

    fn used(&mut self, id: u64, written: &str, within: Option<usize>) -> Fill<String> {
        let Some(entry) = self.entry(id) else {
            return Some(written.to_owned());
        };
        let target = number(entry, "id")?;
        let kind = text(entry, "kind")?;
        let top = number(entry, "depth").unwrap_or(0) == 0 && kind != "param";
        let owner = self.owner(entry, target, within);
        let local = within.or(owner);
        if top {
            if owner.is_none() && !self.own.contains_key(&target) {
                return None;
            }
            self.reach(target, owner)?;
        }
        let refreshed = local.is_some_and(|position| {
            self.instances
                .get(position)
                .is_some_and(|each| each.fresh_ids)
        });
        if refreshed {
            let new_target = self.id_in(local, target);
            let new_kind = if kind == "const" { "def" } else { kind };
            let node = self.id_in(within, id);
            self.entries.insert(
                node.to_string(),
                json!({ "kind": new_kind, "id": new_target }),
            );
        }
        let base = self
            .renamed
            .get(&(local, target))
            .cloned()
            .unwrap_or_else(|| written.to_owned());
        Some(match (within, owner) {
            (Some(_), _) => namespaced(&self.namespace_of(within)?, &base),
            (None, Some(_)) if top => namespaced(&self.namespace_of(owner)?, text(entry, "name")?),
            _ => base,
        })
    }

    fn renamed_name(&self, written: &str, within: Option<usize>) -> String {
        match self.namespace_of(within) {
            Some(space) => namespaced(&space, written),
            None => written.to_owned(),
        }
    }

    fn expr(&mut self, expr: &Value, within: Option<usize>) -> Fill<Value> {
        let mut copy = expr.clone();
        if let Some(id) = number(expr, "id") {
            copy["id"] = json!(self.id_in(within, id));
        }
        match text(expr, "kind")? {
            "name" => {
                let id = number(expr, "id")?;
                copy["name"] = Value::String(self.used(id, text(expr, "name")?, within)?);
            }
            "app" => {
                let id = number(expr, "id")?;
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
                    let id = number(param, "id")?;
                    let unshadowed = format!("{}_{lambda}", text(param, "name")?);
                    self.renamed.insert((within, id), unshadowed.clone());
                    let mut renamed = param.clone();
                    renamed["id"] = json!(self.id_in(within, id));
                    renamed["name"] = Value::String(self.renamed_name(&unshadowed, within));
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
                    self.renamed.insert((within, def), unshadowed.clone());
                }
                let mut renamed = opdef.clone();
                renamed["id"] = json!(self.id_in(within, def));
                renamed["name"] = Value::String(self.renamed_name(&unshadowed, within));
                renamed["expr"] = self.expr(opdef.get("expr")?, within)?;
                copy["opdef"] = renamed;
                copy["expr"] = self.expr(expr.get("expr")?, within)?;
            }
            _ => {}
        }
        Some(copy)
    }

    fn named(&self, name: &str, qualifiers: &[&str]) -> Fill<Target> {
        let fits = |declaration: &Value, wanted: &str| {
            text(declaration, "name") == Some(wanted)
                && declaration
                    .get("expr")
                    .is_none_or(|expr| text(expr, "kind") != Some("lambda"))
                && match text(declaration, "kind") {
                    Some("var") => qualifiers.contains(&"var"),
                    Some("def") => text(declaration, "qualifier")
                        .is_some_and(|qualifier| qualifiers.contains(&qualifier)),
                    _ => false,
                }
        };
        let mut found: Vec<(Option<usize>, u64, &str)> = Vec::new();
        let (alias, wanted) = name
            .split_once(SEPARATOR)
            .map_or((None, name), |(alias, rest)| (Some(alias), rest));
        if alias.is_none() {
            found.extend(
                self.own
                    .iter()
                    .filter(|(_, declaration)| fits(declaration, wanted))
                    .map(|(id, _)| (None, *id, wanted)),
            );
        }
        for (position, each) in self.instances.iter().enumerate() {
            let seen = match alias {
                Some(_) => each.alias == alias,
                None => each.visible,
            };
            if seen {
                found.extend(
                    each.declarations
                        .iter()
                        .filter(|(_, declaration)| fits(declaration, wanted))
                        .map(|(id, _)| (Some(position), *id, wanted)),
                );
            }
        }
        let [(within, target, written)] = found.as_slice() else {
            return None;
        };
        let (within, target) = (*within, *target);
        let declaration = match within {
            Some(position) => *self.instances.get(position)?.declarations.get(&target)?,
            None => *self.own.get(&target)?,
        };
        let shown = self.renamed_name(written, within);
        Some((within, target, shown, kind_of(declaration)))
    }

    fn special(&mut self, (name, qualifier): (&str, &str), listed: &[Target]) -> Fill<()> {
        let mut args = Vec::with_capacity(listed.len());
        for (within, raw, shown, kind) in listed {
            self.reach(*raw, *within)?;
            let target = self.id_in(*within, *raw);
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

    fn played(&self, runs: &Runs) -> Vec<(usize, u64)> {
        let mut found = Vec::new();
        for (position, each) in self.instances.iter().enumerate() {
            let mut runs_here: Vec<(u64, &str)> = each
                .declarations
                .iter()
                .filter(|(_, declaration)| text(declaration, "qualifier") == Some("run"))
                .filter_map(|(id, declaration)| Some((*id, text(declaration, "name")?)))
                .filter(|(_, name)| match runs {
                    Runs::None => false,
                    Runs::Tests => name.ends_with(TESTS_END_WITH),
                    Runs::Named(names) => names.iter().any(|listed| {
                        listed == name
                            || each
                                .alias
                                .is_some_and(|alias| *listed == format!("{alias}{SEPARATOR}{name}"))
                    }),
                })
                .collect();
            runs_here.sort_unstable();
            found.extend(runs_here.into_iter().map(|(id, _)| (position, id)));
        }
        found
    }

    fn roots(&mut self) -> Fill<()> {
        let main = self.main;
        for declaration in list(main, "declarations") {
            match text(declaration, "kind") {
                Some("instance") => {
                    let overrides: Vec<&Value> = list(declaration, "overrides")
                        .iter()
                        .filter_map(|pair| pair.get(1))
                        .collect();
                    for expr in overrides {
                        self.expr(expr, None)?;
                    }
                }
                Some("def") => self.reach(number(declaration, "id")?, None)?,
                _ => {}
            }
        }
        Some(())
    }

    fn filled(mut self, check: &Setup<'_>) -> Fill<String> {
        self.roots()?;
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
        self.special(("q::init", "action"), &[init])?;
        self.special(("q::step", "action"), &[step])?;
        if !invariants.is_empty() {
            self.special(("q::inv", "val"), &invariants)?;
        }
        if !temporal.is_empty() {
            self.special(("q::temporalProps", "temporal"), &temporal)?;
        }
        for (position, run) in self.played(check.runs) {
            self.reach(run, Some(position))?;
        }
        let mut table = Map::new();
        for (key, entry) in &self.typechecked.table {
            let target = number(entry, "id")?;
            let constant = self
                .instances
                .iter()
                .any(|each| !each.fresh_ids && each.constants.contains_key(&target));
            let kind = if constant {
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
