use super::checks::{ModeChecker, NondetChecker, TypeOf, UpdatesChecker};
use super::signatures::{signature, standard_propagation};
use super::{
    Component, Effect, Entity, ErrorTree, Failure, KeyOrder, Kind, Scheme, StateVariable,
    Substitution, Substitutions, apply, compose, effect_text, entity_name_list, names_of,
    namespaced, unify,
};
use crate::json::Json;
use crate::names::{Entry, Raised, number};
use crate::printing_ir::expression_text;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::rc::Rc;

pub(crate) struct Crash(pub(crate) String);

#[derive(Clone)]
pub(crate) struct Ordered<T> {
    values: HashMap<u64, T>,
    order: Vec<u64>,
}

impl<T> Default for Ordered<T> {
    fn default() -> Self {
        Self {
            values: HashMap::new(),
            order: Vec::new(),
        }
    }
}

impl<T> Ordered<T> {
    pub(crate) fn set(&mut self, id: u64, value: T) {
        if self.values.insert(id, value).is_none() {
            self.order.push(id);
        }
    }

    pub(crate) fn get(&self, id: u64) -> Option<&T> {
        self.values.get(&id)
    }

    pub(crate) fn len(&self) -> usize {
        self.order.len()
    }

    pub(crate) fn since(&self, start: usize) -> impl Iterator<Item = (u64, &T)> {
        self.order[start..]
            .iter()
            .filter_map(|id| self.values.get(id).map(|value| (*id, value)))
    }
}

#[derive(Clone, Copy)]
enum Location<'a> {
    Empty,
    Inferring(&'a Json),
    Applying(&'a Json),
}

pub(crate) fn text<'a>(value: &'a Json, key: &str) -> &'a str {
    match value.get(key) {
        Some(Json::Text(text)) => text,
        _ => "",
    }
}

pub(crate) fn items<'a>(value: &'a Json, key: &str) -> &'a [Json] {
    match value.get(key) {
        Some(Json::List(items)) => items,
        _ => &[],
    }
}

pub(crate) fn id_of(value: &Json) -> u64 {
    number(value.get("id")).unwrap_or(0)
}

struct Inferrer<'a> {
    table: &'a BTreeMap<u64, Entry>,
    effects: Ordered<Scheme>,
    errors: Ordered<ErrorTree>,
    substitutions: Substitutions,
    fresh_effects: usize,
    fresh_entities: usize,
    origins: u64,
    location: Location<'a>,
    free: Vec<FreeNames>,
    depth: i64,
}

type Fetched = Result<Scheme, ErrorTree>;

impl<'a> Inferrer<'a> {
    fn location_text(&self) -> String {
        match self.location {
            Location::Empty => String::new(),
            Location::Inferring(expression) => {
                format!("Inferring effect for {}", expression_text(expression))
            }
            Location::Applying(expression) => format!(
                "Trying to infer effect for operator application in {}",
                expression_text(expression)
            ),
        }
    }

    fn fresh_effect(&mut self) -> Rc<str> {
        let name = format!("_e{}", self.fresh_effects).into();
        self.fresh_effects += 1;
        name
    }

    fn fresh_entity(&mut self) -> Rc<str> {
        let name = format!("_v{}", self.fresh_entities).into();
        self.fresh_entities += 1;
        name
    }

    fn add(&mut self, id: u64, result: Fetched) {
        match result {
            Ok(scheme) => self.effects.set(id, scheme),
            Err(tree) => {
                let tree = ErrorTree::wrapped(&self.location_text(), tree);
                self.errors.set(id, tree);
            }
        }
    }

    fn fetch(&self, id: u64) -> Result<Fetched, Crash> {
        if let Some(tree) = self.errors.get(id) {
            return Ok(Err(tree.clone()));
        }
        match self.effects.get(id) {
            Some(scheme) => Ok(Ok(scheme.clone())),
            None => Err(Crash(format!(
                "Couldn't find any result for id {id} while {}",
                self.location_text()
            ))),
        }
    }

    fn new_instance(&mut self, scheme: &Scheme) -> Effect {
        let effect_substitutions: Substitutions = scheme
            .effect_variables
            .iter()
            .map(|name| {
                Rc::new(Substitution::Effect(
                    Rc::clone(name),
                    Effect::Variable(self.fresh_effect()),
                ))
            })
            .collect();
        let entity_substitutions: Substitutions = scheme
            .entity_variables
            .iter()
            .map(|name| {
                Rc::new(Substitution::Entity(
                    Rc::clone(name),
                    Entity::Variable(self.fresh_entity()),
                ))
            })
            .collect();
        let substitutions = compose(&effect_substitutions, &entity_substitutions);
        apply(&substitutions, &scheme.effect)
    }

    fn current_free(&self) -> FreeNames {
        self.free.last().cloned().unwrap_or_default()
    }

    fn quantify(&self, effect: Effect) -> Scheme {
        let free = self.free.last();
        let names = names_of(&effect);
        let effects = names
            .effects
            .into_iter()
            .filter(|name| free.is_none_or(|free| !free.effects.contains(name)))
            .collect();
        let entities = names
            .entities
            .into_iter()
            .filter(|name| free.is_none_or(|free| !free.entities.contains(name)))
            .collect();
        Scheme {
            effect,
            effect_variables: effects,
            entity_variables: entities,
            order: KeyOrder::EffectLast,
        }
    }

    fn add_bindings_to_free_names(&mut self, substitutions: &Substitutions) {
        for substitution in substitutions {
            match &**substitution {
                Substitution::Effect(name, value) => {
                    if !self.free.iter().any(|free| free.effects.contains(name)) {
                        continue;
                    }
                    let added = names_of(value);
                    for free in &mut self.free {
                        if free.effects.contains(name) {
                            for found in &added.effects {
                                free.effects.add(found);
                            }
                            for found in &added.entities {
                                free.entities.add(found);
                            }
                        }
                    }
                }
                Substitution::Entity(name, value) => {
                    if !self.free.iter().any(|free| free.entities.contains(name)) {
                        continue;
                    }
                    let added = entity_name_list(value);
                    for free in &mut self.free {
                        if free.entities.contains(name) {
                            for found in &added {
                                free.entities.add(found);
                            }
                        }
                    }
                }
            }
        }
    }

    fn effect_for_name(
        &mut self,
        name: &str,
        id: u64,
        arity: usize,
    ) -> Result<Result<Effect, ErrorTree>, Crash> {
        if let Some(scheme) = signature(name, arity) {
            return Ok(Ok(self.new_instance(&scheme)));
        }
        let table = self.table;
        let Some(entry) = table.get(&id) else {
            return Ok(Err(self.not_found(name)));
        };
        let Some(definition) = entry.id().filter(|definition| *definition != 0) else {
            return Ok(Err(self.not_found(name)));
        };
        let fetched = self.fetch(definition)?;
        Ok(fetched.map(|scheme| {
            let effect = self.new_instance(&scheme);
            let from_instance = entry
                .get("importedFrom")
                .is_some_and(|from| text(from, "kind") == "instance");
            if from_instance {
                let namespaces: Vec<String> = match entry.get("namespaces") {
                    Some(Json::List(names)) => names
                        .iter()
                        .filter_map(|name| match name {
                            Json::Text(name) => Some(name.clone()),
                            _ => None,
                        })
                        .collect(),
                    _ => Vec::new(),
                };
                namespaced(&effect, &namespaces, &mut self.origins)
            } else {
                effect
            }
        }))
    }

    fn not_found(&self, name: &str) -> ErrorTree {
        ErrorTree::leaf(
            self.location_text(),
            format!("Signature not found for name: {name}"),
        )
    }

    fn declaration(&mut self, declaration: &'a Json) -> Result<(), Crash> {
        self.depth = 0;
        match text(declaration, "kind") {
            "const" | "var" | "typedef" | "assume" => self.definition(declaration),
            "def" => {
                self.depth = -1;
                self.definition(declaration)
            }
            "instance" => {
                for pair in items(declaration, "overrides") {
                    if let Json::List(pair) = pair
                        && let Some(value) = pair.get(1)
                    {
                        self.expression(value)?;
                    }
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }

    fn definition(&mut self, definition: &'a Json) -> Result<(), Crash> {
        self.depth += 1;
        match text(definition, "kind") {
            "const" => self.exit_const(definition),
            "var" => self.exit_var(definition),
            "def" => {
                if let Some(body) = definition.get("expr") {
                    self.expression(body)?;
                }
                self.exit_definition(definition)?;
            }
            "assume" => {
                if let Some(assumption) = definition.get("assumption") {
                    self.expression(assumption)?;
                }
            }
            _ => {}
        }
        self.depth -= 1;
        Ok(())
    }

    fn expression(&mut self, expression: &'a Json) -> Result<(), Crash> {
        self.location = Location::Inferring(expression);
        match text(expression, "kind") {
            "name" => self.exit_name(expression),
            "bool" | "int" | "str" => {
                self.add(
                    id_of(expression),
                    Ok(Scheme::plain(Effect::Concrete(Vec::new()))),
                );
                Ok(())
            }
            "app" => {
                for argument in items(expression, "args") {
                    self.expression(argument)?;
                }
                self.exit_app(expression)
            }
            "lambda" => {
                self.depth += 1;
                self.enter_lambda(expression);
                if let Some(body) = expression.get("expr") {
                    self.expression(body)?;
                }
                self.exit_lambda(expression)?;
                self.depth -= 1;
                Ok(())
            }
            "let" => {
                self.depth += 1;
                if let Some(definition) = expression.get("opdef") {
                    self.definition(definition)?;
                }
                if let Some(body) = expression.get("expr") {
                    self.expression(body)?;
                }
                if self.errors.len() == 0 {
                    let body = expression.get("expr").map_or(0, id_of);
                    let fetched = self.fetch(body)?;
                    self.add(id_of(expression), fetched);
                }
                self.depth -= 1;
                Ok(())
            }
            _ => Ok(()),
        }
    }

    fn exit_const(&mut self, definition: &Json) {
        let arity = definition
            .get("typeAnnotation")
            .and_then(Self::operator_arity);
        let scheme = match arity {
            Some(arity) => standard_propagation(arity),
            None => Scheme::plain(Effect::Concrete(Vec::new())),
        };
        self.add(id_of(definition), Ok(scheme));
    }

    fn operator_arity(annotation: &Json) -> Option<usize> {
        (text(annotation, "kind") == "oper").then(|| items(annotation, "args").len())
    }

    fn exit_var(&mut self, definition: &Json) {
        self.origins += 1;
        let variable = StateVariable {
            name: text(definition, "name").into(),
            reference: id_of(definition),
            origin: self.origins,
        };
        let effect = Effect::Concrete(vec![Component {
            kind: Kind::Read,
            entity: Entity::Concrete(vec![variable]),
        }]);
        self.add(id_of(definition), Ok(Scheme::plain(effect)));
    }

    fn exit_name(&mut self, expression: &Json) -> Result<(), Crash> {
        if self.errors.len() > 0 {
            return Ok(());
        }
        let id = id_of(expression);
        let found = self.effect_for_name(text(expression, "name"), id, 2)?;
        self.add(id, found.map(Scheme::plain));
        Ok(())
    }

    fn exit_app(&mut self, expression: &'a Json) -> Result<(), Crash> {
        if self.errors.len() > 0 {
            return Ok(());
        }
        self.location = Location::Applying(expression);
        let arguments = items(expression, "args");
        let mut params = Vec::new();
        let mut failures = Vec::new();
        for argument in arguments {
            match self.fetch(id_of(argument))? {
                Ok(scheme) => params.push(self.new_instance(&scheme)),
                Err(tree) => failures.push(tree),
            }
        }
        let result = Effect::Variable(self.fresh_effect());
        let arrow = if failures.is_empty() {
            Ok(apply(
                &self.substitutions,
                &Effect::Arrow(params.clone(), Box::new(result.clone())),
            ))
        } else {
            Err(failures)
        };
        let id = id_of(expression);
        let signature = self.effect_for_name(text(expression, "opcode"), id, arguments.len())?;
        let unified = match signature {
            Err(tree) => Err(tree),
            Ok(signature) => match arrow {
                Err(failures) => Err(ErrorTree::of(
                    self.location_text(),
                    Failure::Trees(failures),
                )),
                Ok(arrow) => unify(&apply(&self.substitutions, &signature), &arrow),
            },
        };
        match unified {
            Err(tree) => self.add(id, Err(tree)),
            Ok(found) => {
                let composed = compose(&self.substitutions, &found);
                self.substitutions.clone_from(&composed);
                for (param, argument) in params.iter().zip(arguments) {
                    let effect = apply(&composed, param);
                    self.add(id_of(argument), Ok(Scheme::plain(effect)));
                }
                self.add_bindings_to_free_names(&composed);
                let effect = apply(&composed, &result);
                self.add(id, Ok(Scheme::plain(effect)));
            }
        }
        Ok(())
    }

    fn exit_definition(&mut self, definition: &Json) -> Result<(), Crash> {
        if self.errors.len() > 0 {
            return Ok(());
        }
        let body = definition.get("expr").map_or(0, id_of);
        if let Ok(scheme) = self.fetch(body)? {
            let quantified = self.quantify(scheme.effect);
            self.add(id_of(definition), Ok(quantified));
        }
        if self.depth == 0 {
            self.substitutions = Vec::new();
        }
        Ok(())
    }

    fn enter_lambda(&mut self, lambda: &Json) {
        let mut names = self.current_free();
        for param in items(lambda, "params") {
            let name = text(param, "name");
            let variable: Rc<str> = if name == "_" {
                self.fresh_effect()
            } else {
                format!("e_{name}_{}", id_of(param)).into()
            };
            names.effects.add(&variable);
            self.add(id_of(param), Ok(Scheme::plain(Effect::Variable(variable))));
        }
        self.free.push(names);
    }

    fn exit_lambda(&mut self, lambda: &Json) -> Result<(), Crash> {
        if self.errors.len() > 0 {
            return Ok(());
        }
        let substitutions = self.substitutions.clone();
        self.add_bindings_to_free_names(&substitutions);
        let body = self.fetch(lambda.get("expr").map_or(0, id_of))?;
        let mut params = Vec::new();
        let mut failures = Vec::new();
        for param in items(lambda, "params") {
            let id = id_of(param);
            let fetched = self.fetch(id)?.map(|scheme| {
                let effect = self.new_instance(&scheme);
                apply(&self.substitutions, &effect)
            });
            self.add(id, fetched.clone().map(Scheme::plain));
            match fetched {
                Ok(effect) => params.push(effect),
                Err(tree) => failures.push(tree),
            }
        }
        let result = match body {
            Err(tree) => Err(tree),
            Ok(_) if !failures.is_empty() => Err(ErrorTree::of(
                self.location_text(),
                Failure::Trees(failures),
            )),
            Ok(scheme) => {
                if !scheme.effect_variables.is_empty() || !scheme.entity_variables.is_empty() {
                    return Err(Crash(format!(
                        "an unbound call of newInstance on the quantified effect {}",
                        effect_text(&scheme.effect)
                    )));
                }
                let arrow = Effect::Arrow(params, Box::new(scheme.effect));
                let effect = apply(&self.substitutions, &apply(&Vec::new(), &arrow));
                if let Effect::Arrow(_, result) = &effect
                    && matches!(**result, Effect::Arrow(..))
                {
                    let error = ErrorTree::leaf(
                        self.location_text(),
                        "Result cannot be an operator".to_owned(),
                    );
                    self.add(lambda.get("expr").map_or(0, id_of), Err(error));
                }
                Ok(Scheme::plain(effect))
            }
        };
        self.add(id_of(lambda), result);
        self.free.pop();
        Ok(())
    }
}

pub(crate) struct Analysis {
    pub(crate) effects: Json,
    pub(crate) inference: Vec<Vec<Raised>>,
    pub(crate) checks: Vec<Vec<Raised>>,
    pub(crate) schemes: Ordered<Scheme>,
}

pub(crate) fn analyze(
    modules: &[Json],
    table: &BTreeMap<u64, Entry>,
    types: &dyn Fn(u64) -> Option<TypeOf>,
) -> Result<Analysis, Crash> {
    let mut inferrer = Inferrer {
        table,
        effects: Ordered::default(),
        errors: Ordered::default(),
        substitutions: Vec::new(),
        fresh_effects: 0,
        fresh_entities: 0,
        origins: 0,
        location: Location::Empty,
        free: Vec::new(),
        depth: -1,
    };
    let mut modes = ModeChecker::default();
    let mut updates = UpdatesChecker::default();
    let mut nondets = NondetChecker::new(table, types);
    let mut inference = Vec::new();
    let mut checks = Vec::new();
    for module in modules {
        let declarations = items(module, "declarations");
        let (effects_before, errors_before) = (inferrer.effects.len(), inferrer.errors.len());
        for declaration in declarations {
            inferrer.declaration(declaration)?;
        }
        let by_inference: Vec<Raised> = inferrer
            .errors
            .since(errors_before)
            .map(|(id, tree)| Raised {
                code: "QNT000",
                message: tree.text(),
                reference: Some(id),
            })
            .collect();
        let mut by_checks = modes.check(declarations, &inferrer.effects)?;
        by_checks.extend(updates.check(inferrer.effects.since(effects_before)));
        by_checks.extend(nondets.check(declarations));
        inference.push(by_inference);
        checks.push(by_checks);
    }
    let mut ids: Vec<u64> = inferrer.effects.order.clone();
    ids.sort_unstable();
    let effects = Json::Object(
        ids.iter()
            .filter_map(|id| {
                inferrer
                    .effects
                    .get(*id)
                    .map(|scheme| (id.to_string(), scheme.to_json()))
            })
            .collect(),
    );
    Ok(Analysis {
        effects,
        inference,
        checks,
        schemes: inferrer.effects,
    })
}

pub(crate) fn analyze_added(
    previous: &Ordered<Scheme>,
    table: &BTreeMap<u64, Entry>,
    definitions: &[Json],
    types: &dyn Fn(u64) -> Option<TypeOf>,
) -> Result<(), Crash> {
    let mut inferrer = Inferrer {
        table,
        effects: previous.clone(),
        errors: Ordered::default(),
        substitutions: Vec::new(),
        fresh_effects: 0,
        fresh_entities: 0,
        origins: 0,
        location: Location::Empty,
        free: Vec::new(),
        depth: -1,
    };
    for definition in definitions {
        inferrer.declaration(definition)?;
    }
    ModeChecker::default().check(definitions, &inferrer.effects)?;
    UpdatesChecker::default().check(inferrer.effects.since(0));
    NondetChecker::new(table, types).check(definitions);
    Ok(())
}

#[derive(Clone, Debug, Default)]
struct NameSet {
    known: HashSet<Rc<str>>,
}

impl NameSet {
    fn contains(&self, name: &Rc<str>) -> bool {
        self.known.contains(name)
    }

    fn add(&mut self, name: &Rc<str>) {
        self.known.insert(Rc::clone(name));
    }
}

#[derive(Clone, Debug, Default)]
struct FreeNames {
    effects: NameSet,
    entities: NameSet,
}
