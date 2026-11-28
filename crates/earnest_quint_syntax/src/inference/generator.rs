use crate::inference::application::Fresh;
use crate::inference::printing::{
    ErrorTree, Failure, Names, add, expression_text, leaf, row_names, row_text, tree, type_names,
    type_text,
};
use crate::inference::signatures::{Scheme, Signatures, has_signature};
use crate::inference::simplification::simplify_type;
use crate::inference::solver::{Context, Errors, solve};
use crate::inference::special::{
    Typed, field_access, field_names, item, matching, record, tuple, variant, with,
};
use crate::inference::substitution::{Constraint, Substitution, Substitutions, apply, compose};
use crate::inference::value::{Interner, Value};
use std::collections::{BTreeMap, HashMap, HashSet};

#[derive(Clone, Debug, Default)]
struct Scope {
    entries: Vec<(u64, Scheme)>,
    index: HashMap<u64, usize>,
}

impl Scope {
    fn set(&mut self, id: u64, scheme: Scheme) {
        if let Some(at) = self.index.get(&id) {
            self.entries[*at].1 = scheme;
        } else {
            self.index.insert(id, self.entries.len());
            self.entries.push((id, scheme));
        }
    }

    fn retain(&mut self, kept: &Scope) {
        self.entries.retain(|(id, _)| kept.index.contains_key(id));
        self.index = self
            .entries
            .iter()
            .enumerate()
            .map(|(at, (id, _))| (*id, at))
            .collect();
    }
}

fn variable(name: String) -> Value {
    Value::object(vec![
        ("kind", Value::text("var")),
        ("name", Value::Text(name)),
    ])
}

fn without(names: &[String], removed: &[String]) -> Vec<String> {
    names
        .iter()
        .filter(|name| !removed.contains(name))
        .cloned()
        .collect()
}

fn difference(first: &Names, second: &Names) -> Names {
    Names {
        types: without(&first.types, &second.types),
        rows: without(&first.rows, &second.rows),
    }
}

fn free_types(scheme: &Scheme) -> Names {
    let all = type_names(&scheme.value);
    Names {
        types: without(&all.types, &scheme.types),
        rows: without(&all.rows, &scheme.rows),
    }
}

fn prepended(names: &[String], known: &[String]) -> Vec<String> {
    let mut joined = Vec::new();
    for name in names.iter().chain(known) {
        add(&mut joined, name);
    }
    joined
}

fn add_bindings(free: &mut Names, substitutions: &[Substitution]) {
    for substitution in substitutions {
        if substitution.row {
            if free.rows.contains(&substitution.name) {
                for name in row_names(&substitution.value) {
                    add(&mut free.rows, &name);
                }
            }
        } else if free.types.contains(&substitution.name) {
            let names = type_names(&substitution.value);
            for name in &names.types {
                add(&mut free.types, name);
            }
            for name in &names.rows {
                add(&mut free.rows, name);
            }
        }
    }
}

fn generality(substitutions: &[Substitution], annotation: &Value) -> Result<(), ErrorTree> {
    let names = &type_names(annotation);
    let too_general = |row: bool| {
        substitutions
            .iter()
            .filter(move |substitution| {
                substitution.row == row
                    && substitution.value.kind() != "var"
                    && if row {
                        names.rows.contains(&substitution.name)
                    } else {
                        names.types.contains(&substitution.name)
                    }
            })
            .map(|substitution| {
                let expected = if substitution.row {
                    row_text(&substitution.value)
                } else {
                    type_text(&substitution.value)
                };
                leaf(
                    &format!("Checking variable {}", substitution.name),
                    &format!(
                        "Type annotation is too general: {} should be {expected}",
                        substitution.name
                    ),
                )
            })
            .collect::<Vec<_>>()
    };
    let mut errors = too_general(false);
    errors.extend(too_general(true));
    if errors.is_empty() {
        Ok(())
    } else {
        Err(tree(
            &format!("Checking type annotation {}", type_text(annotation)),
            Failure::Trees(errors),
        ))
    }
}

fn arity_error(opcode: &str, count: usize, expected: &str) -> Failure {
    Failure::Tree(leaf(
        &format!("Checking arity for application of {opcode}"),
        &format!("Operator expects {expected} arguments but was given {count}"),
    ))
}

pub(crate) struct Inferrer<'a> {
    context: &'a Context,
    signatures: Signatures,
    fresh: Fresh,
    pub(crate) types: BTreeMap<u64, Scheme>,
    pub(crate) errors: Errors,
    constraints: Vec<Constraint>,
    free_before: HashMap<u64, Names>,
    in_scope: Scope,
    saved: Scope,
    location: Option<&'a Value>,
    pub(crate) crash_place: Option<u64>,
    /// The type nodes of every type given an id so far, as the types table will hold them.
    type_nodes: usize,
    /// The most type nodes the types given ids have held at once.
    pub(crate) most_nodes: usize,
    /// The most type nodes inference keeps before it stops all type work.
    pub(crate) node_limit: usize,
    counted: HashMap<u64, usize>,
    /// Every type given an id, stored once by its content.
    interner: Interner,
}

fn nodes(value: &Value) -> usize {
    let mut count = 0;
    let mut waiting = vec![value];
    while let Some(value) = waiting.pop() {
        match value {
            Value::Object(fields) => {
                count += 1;
                waiting.extend(fields.iter().map(|(_, inner)| inner));
            }
            Value::List(items) => waiting.extend(items.iter()),
            _ => {}
        }
    }
    count
}

impl<'a> Inferrer<'a> {
    pub(crate) fn new(context: &'a Context) -> Self {
        Self {
            context,
            signatures: Signatures::default(),
            fresh: Fresh::default(),
            types: BTreeMap::new(),
            errors: Errors::default(),
            constraints: Vec::new(),
            free_before: HashMap::new(),
            in_scope: Scope::default(),
            saved: Scope::default(),
            location: None,
            crash_place: None,
            type_nodes: 0,
            most_nodes: 0,
            node_limit: usize::MAX,
            counted: HashMap::new(),
            interner: Interner::default(),
        }
    }

    pub(crate) fn with_types(context: &'a Context, types: BTreeMap<u64, Scheme>) -> Self {
        let mut inferrer = Self::new(context);
        inferrer.types = types
            .into_iter()
            .map(|(id, scheme)| {
                let value = inferrer.interner.intern(&scheme.value);
                (id, Scheme { value, ..scheme })
            })
            .collect();
        inferrer
    }

    pub(crate) fn infer(&mut self, declarations: &'a [Value]) {
        let before: HashSet<u64> = self.types.keys().copied().collect();
        for declaration in declarations {
            self.declaration(declaration);
        }
        let added: Vec<u64> = self
            .types
            .keys()
            .filter(|id| !before.contains(id))
            .copied()
            .collect();
        for id in added {
            if let Some(scheme) = self.types.get(&id) {
                let sound = self.context.crashed().is_none();
                let simplified = Scheme {
                    value: self
                        .interner
                        .intern(&simplify_type(self.context, &scheme.value)),
                    ..scheme.clone()
                };
                if sound && self.context.crashed().is_some() {
                    self.crash_place = Some(id);
                }
                self.count(id, &simplified.value);
                self.types.insert(id, simplified);
            }
        }
    }

    fn count(&mut self, id: u64, value: &Value) {
        let found = nodes(value);
        let before = self.counted.insert(id, found).unwrap_or(0);
        self.type_nodes = self.type_nodes.saturating_sub(before).saturating_add(found);
        self.most_nodes = self.most_nodes.max(self.type_nodes);
        if self.type_nodes > self.node_limit {
            self.context
                .crash(format!("more than {} type nodes", self.node_limit));
        }
    }

    fn location_text(&self) -> String {
        self.location.map_or_else(String::new, |expression| {
            format!("Generating constraints for {}", expression_text(expression))
        })
    }

    fn add_result(&mut self, id: u64, result: Result<Scheme, Failure>) {
        match result {
            Err(error) => {
                let error = tree(&self.location_text(), error);
                self.errors.set(id, error);
            }
            Ok(scheme) => {
                let scheme = Scheme {
                    value: self.interner.intern(&scheme.value),
                    ..scheme
                };
                self.count(id, &scheme.value);
                self.in_scope.set(id, scheme.clone());
                self.types.insert(id, scheme);
            }
        }
    }

    fn fetch(&self, id: u64) -> Result<Scheme, ErrorTree> {
        if let Some(error) = self.errors.get(id) {
            return Err(error.clone());
        }
        if let Some(scheme) = self.types.get(&id) {
            return Ok(scheme.clone());
        }
        self.context
            .crash(format!("Couldn't find any result for id {id}"));
        Err(leaf("", ""))
    }

    fn new_instance(&mut self, scheme: &Scheme) -> Value {
        let types: Substitutions = scheme
            .types
            .iter()
            .map(|name| Substitution {
                row: false,
                name: name.clone(),
                value: variable(self.fresh.var("_t")),
            })
            .collect();
        let rows: Substitutions = scheme
            .rows
            .iter()
            .map(|name| Substitution {
                row: true,
                name: name.clone(),
                value: variable(self.fresh.var("_t")),
            })
            .collect();
        let substitutions = compose(self.context, &types, &rows);
        apply(&substitutions, &scheme.value)
    }

    fn type_for_name(
        &mut self,
        name: &str,
        id: Option<u64>,
        arity: usize,
    ) -> Result<Value, ErrorTree> {
        if has_signature(name) {
            let Some(signature) = self.signatures.of(name, arity) else {
                self.context
                    .crash(format!("Could not parse the signature of {name}"));
                return Err(leaf("", ""));
            };
            return Ok(self.new_instance(&signature));
        }
        let context = self.context;
        let definition = id.and_then(|id| context.table.get(id));
        let Some((definition, defined)) = definition.and_then(|definition| {
            definition
                .id()
                .filter(|id| *id != 0)
                .map(|id| (definition, id))
        }) else {
            let location = self.location_text();
            return Err(leaf(
                &location,
                &format!("Signature not found for name: {name}"),
            ));
        };
        if matches!(definition.kind(), "var" | "const") {
            return self.fetch(defined).map(|scheme| scheme.value);
        }
        let annotation = definition.get("typeAnnotation");
        if annotation.truthy() {
            return Ok(annotation.clone());
        }
        let scheme = self.fetch(defined)?;
        Ok(self.new_instance(&scheme))
    }

    fn free_names_in_scope(&self) -> Names {
        let mut known = Names::default();
        for (_, scheme) in &self.in_scope.entries {
            let names = free_types(scheme);
            known = Names {
                types: prepended(&names.types, &known.types),
                rows: prepended(&names.rows, &known.rows),
            };
        }
        known
    }

    fn solve_constraints(&mut self) -> Option<Substitutions> {
        let constraint = Constraint::Conjunction(std::mem::take(&mut self.constraints));
        match solve(self.context, &constraint) {
            Err(errors) => {
                for (id, error) in errors.entries {
                    self.errors.set(id, error);
                }
                None
            }
            Ok(substitutions) => {
                let entries = self.in_scope.entries.clone();
                for (id, scheme) in entries {
                    let value = apply(&substitutions, &scheme.value);
                    self.add_result(id, Ok(Scheme { value, ..scheme }));
                }
                Some(substitutions)
            }
        }
    }

    fn declaration(&mut self, declaration: &'a Value) {
        if crate::inference::substitution::stopped() {
            return;
        }
        match declaration.kind() {
            "instance" => {
                for pair in declaration.items("overrides") {
                    if let Some(value) = pair.as_items().get(1) {
                        self.expression(value);
                    }
                }
                self.exit_instance(declaration);
            }
            "import" | "export" => {}
            _ => self.definition(declaration),
        }
        if matches!(declaration.kind(), "var" | "const") {
            self.saved = self.in_scope.clone();
            return;
        }
        let saved = std::mem::take(&mut self.saved);
        self.in_scope.retain(&saved);
        self.saved = self.in_scope.clone();
    }

    fn definition(&mut self, definition: &'a Value) {
        let Some(id) = definition.id() else {
            return;
        };
        match definition.kind() {
            "def" => {
                let free = self.free_names_in_scope();
                self.free_before.insert(id, free);
                self.expression(definition.get("expr"));
                self.exit_operator(definition, id);
            }
            "assume" => {
                let free = self.free_names_in_scope();
                self.free_before.insert(id, free);
                self.expression(definition.get("assumption"));
                self.exit_assume(definition, id);
            }
            "const" | "var" => {
                let annotation = definition.get("typeAnnotation").clone();
                self.add_result(id, Ok(Scheme::plain(annotation)));
            }
            _ => {}
        }
    }

    fn exit_operator(&mut self, definition: &Value, id: u64) {
        if !self.errors.is_empty() {
            return;
        }
        let body = definition.get("expr").id().unwrap_or(0);
        let annotation = definition.get("typeAnnotation");
        if let Ok(scheme) = self.fetch(body)
            && annotation.truthy()
        {
            self.constraints
                .push(Constraint::Equal(scheme.value, annotation.clone(), id));
        }
        let mut before = self.free_before.get(&id).cloned().unwrap_or_default();
        if !self.constraints.is_empty()
            && let Some(substitutions) = self.solve_constraints()
        {
            add_bindings(&mut before, &substitutions);
            if annotation.is_defined()
                && let Err(error) = generality(&substitutions, annotation)
            {
                self.errors.set(annotation.id().unwrap_or(id), error);
            }
        }
        let now = self.free_names_in_scope();
        let quantified = difference(&now, &before);
        if let Ok(scheme) = self.fetch(body) {
            self.add_result(
                id,
                Ok(Scheme {
                    types: quantified.types,
                    rows: quantified.rows,
                    value: scheme.value,
                }),
            );
        }
    }

    fn exit_assume(&mut self, definition: &Value, id: u64) {
        if !self.errors.is_empty() {
            return;
        }
        let body = definition.get("assumption").id().unwrap_or(0);
        if let Ok(scheme) = self.fetch(body) {
            let before = self.free_before.get(&id).cloned().unwrap_or_default();
            let quantified = difference(&self.free_names_in_scope(), &before);
            self.add_result(
                id,
                Ok(Scheme {
                    types: quantified.types,
                    rows: quantified.rows,
                    value: scheme.value.clone(),
                }),
            );
            self.constraints.push(Constraint::Equal(
                scheme.value,
                Value::object(vec![("kind", Value::text("bool"))]),
                id,
            ));
        }
    }

    fn exit_instance(&mut self, declaration: &Value) {
        if !self.errors.is_empty() {
            return;
        }
        for pair in declaration.items("overrides") {
            let items = pair.as_items();
            let (Some(name), Some(value)) = (items.first(), items.get(1)) else {
                continue;
            };
            let name_id = name.id().unwrap_or(0);
            let found = self
                .type_for_name(name.str("name"), name.id(), 2)
                .map(Scheme::plain)
                .map_err(Failure::Tree);
            self.add_result(name_id, found);
            let value_id = value.id().unwrap_or(0);
            if let Ok(original) = self.fetch(name_id)
                && let Ok(found) = self.fetch(value_id)
            {
                self.constraints
                    .push(Constraint::Equal(original.value, found.value, value_id));
            }
        }
        if !self.constraints.is_empty() {
            self.solve_constraints();
        }
    }

    fn expression(&mut self, expression: &'a Value) {
        if crate::inference::substitution::stopped() {
            return;
        }
        self.location = Some(expression);
        let id = expression.id().unwrap_or(0);
        match expression.kind() {
            "name" => {
                if !self.errors.is_empty() {
                    return;
                }
                let found = self
                    .type_for_name(expression.str("name"), expression.id(), 2)
                    .map(Scheme::plain)
                    .map_err(Failure::Tree);
                self.add_result(id, found);
            }
            kind @ ("bool" | "int" | "str") => {
                let literal = Value::object(vec![("kind", Value::text(kind))]);
                self.add_result(id, Ok(Scheme::plain(literal)));
            }
            "app" => {
                for argument in expression.items("args") {
                    self.expression(argument);
                }
                self.exit_application(expression, id);
            }
            "lambda" => {
                self.enter_lambda(expression);
                self.expression(expression.get("expr"));
                self.exit_lambda(expression, id);
            }
            "let" => {
                self.definition(expression.get("opdef"));
                self.expression(expression.get("expr"));
                if !self.errors.is_empty() {
                    return;
                }
                let body = expression.get("expr").id().unwrap_or(0);
                let found = self.fetch(body).map_err(Failure::Tree);
                self.add_result(id, found);
            }
            _ => {}
        }
    }

    fn enter_lambda(&mut self, expression: &Value) {
        for param in expression.items("params") {
            let name = param.str("name");
            let id = param.id().unwrap_or(0);
            let name = if name == "_" {
                self.fresh.var("_t")
            } else {
                format!("t_{name}_{id}")
            };
            let param_type = variable(name);
            self.add_result(id, Ok(Scheme::plain(param_type.clone())));
            let annotation = param.get("typeAnnotation");
            if annotation.truthy() {
                self.constraints
                    .push(Constraint::Equal(param_type, annotation.clone(), id));
            }
        }
    }

    fn exit_lambda(&mut self, expression: &Value, id: u64) {
        if !self.errors.is_empty() {
            return;
        }
        let body = expression.get("expr").id().unwrap_or(0);
        let result = match self.fetch(body) {
            Err(error) => Err(Failure::Tree(error)),
            Ok(result_type) => {
                let mut params = Vec::new();
                for param in expression.items("params") {
                    match self.fetch(param.id().unwrap_or(0)) {
                        Ok(scheme) => params.push(self.new_instance(&scheme)),
                        Err(_) => self.context.crash("Lambda variables not found".to_owned()),
                    }
                }
                Ok(Scheme::plain(Value::object(vec![
                    ("kind", Value::text("oper")),
                    ("args", Value::list(params)),
                    ("res", result_type.value),
                ])))
            }
        };
        self.add_result(id, result);
    }

    fn exit_application(&mut self, expression: &'a Value, id: u64) {
        if !self.errors.is_empty() {
            return;
        }
        let arguments = expression.items("args");
        let mut typed: Vec<Typed<'a>> = Vec::new();
        let mut failed = Vec::new();
        for argument in arguments {
            match self.fetch(argument.id().unwrap_or(0)) {
                Ok(scheme) => typed.push((argument, scheme.value)),
                Err(error) => failed.push(error),
            }
        }
        let opcode = expression.str("opcode");
        let defined = self.type_for_name(opcode, expression.id(), arguments.len());
        let result = variable(self.fresh.var("_t"));
        let count = typed.len();
        let constraints = if failed.is_empty() {
            match opcode {
                "Rec" if !count.is_multiple_of(2) => {
                    Err(arity_error(opcode, count, "even number of"))
                }
                "Rec" => record(id, &typed, &result),
                "field" | "item" | "variant" if count != 2 => Err(arity_error(opcode, count, "2")),
                "field" => field_access(id, &typed, &result),
                "fieldNames" if count != 1 => Err(arity_error(opcode, count, "1")),
                "fieldNames" => Ok(field_names(id, &typed, &result)),
                "with" if count != 3 => Err(arity_error(opcode, count, "3")),
                "with" => with(id, &typed, &result),
                "Tup" => Ok(tuple(id, &typed, &result)),
                "item" => item(id, &typed, &result),
                "variant" => variant(id, &typed, &result),
                "matchVariant" if count.is_multiple_of(2) => {
                    Err(arity_error(opcode, count, "odd number of"))
                }
                "matchVariant" => matching(id, &typed, &result),
                _ => defined.map_err(Failure::Tree).map(|signature| {
                    let applied = Value::object(vec![
                        ("kind", Value::text("oper")),
                        (
                            "args",
                            Value::list(typed.iter().map(|(_, found)| found.clone()).collect()),
                        ),
                        ("res", result.clone()),
                    ]);
                    vec![Constraint::Equal(signature, applied, id)]
                }),
            }
        } else {
            Err(Failure::Trees(failed))
        };
        let found = constraints.map(|constraints| {
            self.constraints.extend(constraints);
            Scheme::plain(result)
        });
        self.add_result(id, found);
    }
}
